//! Unknown license detection using ngram matching.
//!
//! This module implements detection of license-like text that doesn't match
//! any known rule. It uses an ngram-based approach to identify regions with
//! sufficient license-like content.
//!
//! # Signature Difference from Other Matchers
//!
//! Unlike other matchers (`hash_match`, `aho_match`, `seq_match`) which take
//! `&QueryRun` as their query parameter, `unknown_match` takes `&Query` directly
//! and requires a `known_matches` parameter.
//!
//! ## Why This Design?
//!
//! The unknown matcher has a fundamentally different purpose: it finds regions
//! of text that are **NOT** covered by previously detected licenses. This requires
//! knowledge of what has already been matched.
//!
//! Other matchers operate independently:
//! - `hash_match`: Looks for exact hash matches of the entire text
//! - `aho_match`: Finds all occurrences of rule patterns
//! - `seq_match`: Finds approximate matches using set similarity
//!
//! The unknown matcher operates on **gaps**:
//! - First computes which positions are already covered by known matches
//! - Then searches only the uncovered regions for license-like content
//! - This prevents re-detecting known licenses as "unknown"
//!
//! ## Python Parity
//!
//! The Python reference (`match_unknowns()` in `match_unknown.py`) has a similar
//! conceptual design but different signature:
//!
//! ```python
//! def match_unknowns(idx, query_run, automaton, unknown_ngram_length=6, **kwargs):
//!     matched_ngrams = get_matched_ngrams(...)
//!     qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
//!     qspan = Span().union(*qspans)
//! ```
//!
//! The Python version receives `query_run` but operates on the automaton matches
//! within that run. The Rust version explicitly receives `known_matches` because:
//!
//! 1. The Rust detection pipeline calls matchers in sequence and accumulates results
//! 2. By the time unknown_match is called, we already have the complete list of
//!    matches from hash, SPDX-LID, aho, and seq matchers
//! 3. Passing `known_matches` directly is more explicit and avoids recomputing
//!    coverage information
//!
//! This design choice maintains functional parity with Python while being more
//! idiomatic for Rust's explicit data flow patterns.

use aho_corasick::AhoCorasick;

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::Query;

/// Matcher name for unknown license detection.
///
/// Corresponds to Python: `MATCH_UNKNOWN = '6-unknown'` (line 46)
pub const MATCH_UNKNOWN: &str = "5-undetected";

/// Matcher order for unknown license detection.
///
/// Corresponds to Python: `MATCH_UNKNOWN_ORDER = 6` (line 47)
#[allow(dead_code)]
pub const MATCH_UNKNOWN_ORDER: u8 = 5;

/// Length of ngrams for unknown detection.
///
/// Corresponds to Python: `UNKNOWN_NGRAM_LENGTH = 6` (line 49)
const UNKNOWN_NGRAM_LENGTH: usize = 6;

/// Minimum number of ngram matches required for a region.
const MIN_NGRAM_MATCHES: usize = 3;

/// Minimum region length in tokens.
const MIN_REGION_LENGTH: usize = 5;

/// Perform unknown license matching on a query.
///
/// This function identifies regions of the query that are not covered by known
/// matches and attempts to license-like content in those regions using an
/// ngram-based approach.
///
/// # Arguments
/// * `index` - The license index containing the unknown_automaton
/// * `query` - The query object containing tokenized text
/// * `known_matches` - Previously found matches that cover recognized license text
///
/// # Returns
/// A vector of LicenseMatch objects representing unknown license detections
///
/// # Simplified Implementation Notes
///
/// This is a simplified version for Phase 4.5 that:
/// - Assumes unknown_automaton is pre-built (will build in future phase)
/// - Uses simple region gap detection for unmatched areas
/// - Creates matches based on ngram count threshold
///
/// The full Python implementation includes:
/// - Ngram building from approx-matchable rules via add_ngrams()
/// - Filtering of ngrams via is_good_tokens_ngram()
/// - More sophisticated region grouping
///
/// Corresponds to Python: `match_unknowns()` (lines 132-239)
pub fn unknown_match(
    index: &LicenseIndex,
    query: &Query,
    known_matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    let mut unknown_matches = Vec::new();

    if query.tokens.is_empty() {
        return unknown_matches;
    }

    let query_len = query.tokens.len();

    let covered_positions = compute_covered_positions(query, known_matches);

    let unmatched_regions = find_unmatched_regions(query_len, &covered_positions);

    let automaton = &index.unknown_automaton;

    for region in unmatched_regions {
        let start = region.0;
        let end = region.1;

        let region_length = end - start;
        if region_length < MIN_REGION_LENGTH {
            continue;
        }

        let ngram_matches = match_ngrams_in_region(&query.tokens, start, end, automaton);

        if ngram_matches < MIN_NGRAM_MATCHES {
            continue;
        }

        if let Some(match_result) = create_unknown_match(index, query, start, end, ngram_matches) {
            unknown_matches.push(match_result);
        }
    }

    unknown_matches
}

/// Compute the set of query positions covered by known matches.
///
/// # Arguments
/// * `query` - The query object
/// * `known_matches` - Previously found matches
///
/// # Returns
/// A HashSet of token positions that are covered by known matches
fn compute_covered_positions(
    query: &Query,
    known_matches: &[LicenseMatch],
) -> std::collections::HashSet<usize> {
    let mut covered = std::collections::HashSet::new();

    for match_result in known_matches {
        let start_line = match_result.start_line;
        let end_line = match_result.end_line;

        for pos in 0..query.line_by_pos.len() {
            let line = query.line_by_pos[pos];
            if line >= start_line && line <= end_line {
                covered.insert(pos);
            }
        }
    }

    covered
}

/// Find regions of the query that are not covered by known matches.
///
/// # Arguments
/// * `query_len` - Total length of the query in tokens
/// * `covered_positions` - Set of positions covered by known matches
///
/// # Returns
/// A vector of (start, end) tuples representing unmatched regions
fn find_unmatched_regions(
    query_len: usize,
    covered_positions: &std::collections::HashSet<usize>,
) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();

    if query_len == 0 {
        return regions;
    }

    let mut region_start = None;

    for pos in 0..query_len {
        if !covered_positions.contains(&pos) {
            if region_start.is_none() {
                region_start = Some(pos);
            }
        } else if let Some(start) = region_start {
            regions.push((start, pos));
            region_start = None;
        }
    }

    if let Some(start) = region_start {
        regions.push((start, query_len));
    }

    regions
}

/// Count ngram matches in a specific region using the unknown automaton.
///
/// # Arguments
/// * `tokens` - The query tokens
/// * `start` - Start position of the region
/// * `end` - End position of the region (exclusive)
/// * `automaton` - The unknown ngram automaton
///
/// # Returns
/// The number of ngram matches found in the region
fn match_ngrams_in_region(
    tokens: &[u16],
    start: usize,
    end: usize,
    automaton: &AhoCorasick,
) -> usize {
    if start >= end || end > tokens.len() {
        return 0;
    }

    let region_tokens = &tokens[start..end];

    let region_bytes: Vec<u8> = region_tokens
        .iter()
        .flat_map(|tid| tid.to_le_bytes())
        .collect();

    let mut match_count = 0;

    for _ in automaton.find_iter(&region_bytes) {
        match_count += 1;
    }

    match_count
}

/// Create a LicenseMatch for an unknown license region.
///
/// # Arguments
/// * `index` - The license index
/// * `query` - The query object
/// * `start` - Start position of the region
/// * `end` - End position of the region (exclusive)
/// * `ngram_count` - Number of ngram matches in the region
///
/// # Returns
/// Option containing the LicenseMatch, or None if the region doesn't meet thresholds
fn create_unknown_match(
    _index: &LicenseIndex,
    query: &Query,
    start: usize,
    end: usize,
    ngram_count: usize,
) -> Option<LicenseMatch> {
    let region_length = end.saturating_sub(start);

    if region_length < UNKNOWN_NGRAM_LENGTH * 4 {
        return None;
    }

    let start_line = query.line_by_pos.get(start).copied().unwrap_or(1);
    let end_line = query
        .line_by_pos
        .get(end.saturating_sub(1))
        .copied()
        .unwrap_or(start_line);

    let matched_length = region_length;
    let match_coverage = 100.0;

    let score = calculate_score(ngram_count, region_length);

    LicenseMatch {
        license_expression: "unknown".to_string(),
        license_expression_spdx: "unknown".to_string(),
        from_file: None,
        start_line,
        end_line,
        matcher: MATCH_UNKNOWN.to_string(),
        score,
        matched_length,
        match_coverage,
        rule_relevance: 50,
        rule_identifier: "unknown".to_string(),
        rule_url: String::new(),
        matched_text: None,
        referenced_filenames: None,
    }
    .into()
}

/// Calculate match score based on ngram count and region length.
///
/// # Arguments
/// * `ngram_count` - Number of ngram matches found
/// * `region_length` - Length of the region in tokens
///
/// # Returns
/// A score between 0.0 and 1.0
fn calculate_score(ngram_count: usize, region_length: usize) -> f32 {
    if region_length == 0 {
        return 0.0;
    }

    let density = ngram_count as f32 / region_length as f32;

    density.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::index::LicenseIndex;
    use crate::license_detection::query::Query;

    fn create_test_query(text: &str) -> Query {
        let index = LicenseIndex::with_legalese_count(10);
        Query::new(text, index).expect("Failed to create query")
    }

    #[test]
    fn test_constants() {
        assert_eq!(MATCH_UNKNOWN, "5-undetected");
        assert_eq!(MATCH_UNKNOWN_ORDER, 5);
        assert_eq!(UNKNOWN_NGRAM_LENGTH, 6);
    }

    #[test]
    fn test_unknown_match_empty_query() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = create_test_query("");
        let known_matches = vec![];

        let matches = unknown_match(&index, &query, &known_matches);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_unmatched_regions_no_coverage() {
        let query_len = 10;
        let covered_positions = std::collections::HashSet::new();

        let regions = find_unmatched_regions(query_len, &covered_positions);

        assert_eq!(regions, vec![(0, 10)]);
    }

    #[test]
    fn test_find_unmatched_regions_full_coverage() {
        let query_len = 10;
        let covered_positions: std::collections::HashSet<usize> = (0..10).collect();

        let regions = find_unmatched_regions(query_len, &covered_positions);

        assert!(regions.is_empty());
    }

    #[test]
    fn test_find_unmatched_regions_partial_coverage() {
        let query_len = 20;
        let covered_positions: std::collections::HashSet<usize> =
            [0, 1, 2, 12, 13, 14, 15, 16, 17, 18, 19]
                .iter()
                .cloned()
                .collect();

        let regions = find_unmatched_regions(query_len, &covered_positions);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0], (3, 12));
    }

    #[test]
    fn test_find_unmatched_regions_trailing_unmatched() {
        let query_len = 20;
        let covered_positions: std::collections::HashSet<usize> =
            [0, 1, 2, 3, 4, 5].iter().cloned().collect();

        let regions = find_unmatched_regions(query_len, &covered_positions);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0], (6, 20));
    }

    #[test]
    fn test_match_ngrams_in_region() {
        let tokens = vec![1u16, 2, 3, 4, 5, 6, 7, 8];
        let automaton =
            AhoCorasick::new(std::iter::empty::<&[u8]>()).expect("Failed to create automaton");

        let count = match_ngrams_in_region(&tokens, 0, 8, &automaton);

        assert_eq!(count, 0);
    }

    #[test]
    fn test_create_unknown_match_too_short() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = create_test_query("short text");

        let match_result = create_unknown_match(&index, &query, 0, 5, 10);

        assert!(match_result.is_none());
    }

    #[test]
    fn test_calculate_score() {
        let score1 = calculate_score(5, 10);
        let score2 = calculate_score(10, 10);
        let score3 = calculate_score(0, 10);

        assert!(score2 > score1);
        assert!(score2 <= 1.0);
        assert_eq!(score3, 0.0);
    }
}
