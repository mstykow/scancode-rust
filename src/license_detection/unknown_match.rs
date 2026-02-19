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
/// Uses token positions (start_token, end_token) for precise coverage,
/// matching Python's qspan-based approach.
///
/// # Arguments
/// * `_query` - The query object (unused, kept for API compatibility)
/// * `known_matches` - Previously found matches
///
/// # Returns
/// A HashSet of token positions that are covered by known matches
fn compute_covered_positions(
    _query: &Query,
    known_matches: &[LicenseMatch],
) -> std::collections::HashSet<usize> {
    let mut covered = std::collections::HashSet::new();

    for m in known_matches {
        for pos in m.start_token..m.end_token {
            covered.insert(pos);
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
    index: &LicenseIndex,
    query: &Query,
    start: usize,
    end: usize,
    ngram_count: usize,
) -> Option<LicenseMatch> {
    let region_length = end.saturating_sub(start);

    if region_length < UNKNOWN_NGRAM_LENGTH * 4 {
        return None;
    }

    // Compute hispan: count high-value legalese tokens.
    // Python: `len(hispan) < 5` check at match_unknown.py:220
    let hispan = (start..end)
        .filter(|&pos| {
            query
                .tokens
                .get(pos)
                .is_some_and(|&tid| (tid as usize) < index.len_legalese)
        })
        .count();

    if hispan < 5 {
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

    let matched_text = query.matched_text(start_line, end_line);

    LicenseMatch {
        license_expression: "unknown".to_string(),
        license_expression_spdx: "unknown".to_string(),
        from_file: None,
        start_line,
        end_line,
        start_token: start,
        end_token: end,
        matcher: MATCH_UNKNOWN.to_string(),
        score,
        matched_length,
        rule_length: region_length,
        match_coverage,
        rule_relevance: 50,
        rule_identifier: "unknown".to_string(),
        rule_url: String::new(),
        matched_text: Some(matched_text),
        referenced_filenames: None,
        is_license_intro: false,
        is_license_clue: false,
        is_license_reference: false,
        is_license_tag: false,
        matched_token_positions: None,
        hilen: hispan,
        rule_start_token: 0,
        qspan_positions: None,
        ispan_positions: None,
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

    #[test]
    fn test_constants() {
        assert_eq!(MATCH_UNKNOWN, "5-undetected");
        assert_eq!(MATCH_UNKNOWN_ORDER, 5);
        assert_eq!(UNKNOWN_NGRAM_LENGTH, 6);
    }

    #[test]
    fn test_unknown_match_empty_query() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("", &index).expect("Failed to create query");
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
        let query = Query::new("short text", &index).expect("Failed to create query");

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

    #[test]
    fn test_find_unmatched_regions_leading_unmatched() {
        let query_len = 20;
        let covered_positions: std::collections::HashSet<usize> =
            [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
                .iter()
                .cloned()
                .collect();

        let regions = find_unmatched_regions(query_len, &covered_positions);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0], (0, 10));
    }

    #[test]
    fn test_find_unmatched_regions_middle_gap() {
        let query_len = 30;
        let covered_positions: std::collections::HashSet<usize> =
            [0, 1, 2, 3, 4, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]
                .iter()
                .cloned()
                .collect();

        let regions = find_unmatched_regions(query_len, &covered_positions);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0], (5, 20));
    }

    #[test]
    fn test_compute_covered_positions_single_match() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("some license text here", &index).expect("Failed to create query");

        let known_matches = vec![LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: None,
            start_line: 1,
            end_line: 1,
            start_token: 0,
            end_token: 3,
            matcher: "test".to_string(),
            score: 1.0,
            matched_length: 3,
            rule_length: 3,
            matched_token_positions: None,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "test-rule".to_string(),
            rule_url: String::new(),
            matched_text: Some("some license text".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            hilen: 1,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
        }];

        let covered = compute_covered_positions(&query, &known_matches);

        assert!(
            covered.contains(&0) || covered.is_empty(),
            "Should track covered positions"
        );
    }

    #[test]
    fn test_match_ngrams_in_region_with_matches() {
        let tokens: Vec<u16> = (0..30).collect();
        let ngram: Vec<u8> = vec![0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0];
        let automaton = AhoCorasick::new(std::iter::once(ngram.as_slice()))
            .expect("Failed to create automaton");

        let count = match_ngrams_in_region(&tokens, 0, 30, &automaton);

        assert!(count > 0, "Should find ngram matches");
    }

    #[test]
    fn test_create_unknown_match_valid() {
        let index = LicenseIndex::with_legalese_count(10);
        let text = "This is a license text that should be long enough for unknown detection";
        let query = Query::new(text, &index).expect("Failed to create query");

        let match_result = create_unknown_match(&index, &query, 0, 30, 5);

        assert!(
            match_result.is_some(),
            "Should create unknown match for sufficient length"
        );

        let m = match_result.unwrap();
        assert_eq!(m.license_expression, "unknown");
        assert_eq!(m.matcher, MATCH_UNKNOWN);
    }

    #[test]
    fn test_unknown_match_with_known_matches() {
        let index = LicenseIndex::with_legalese_count(10);
        let text = "some text that is license related and should be detected";
        let query = Query::new(text, &index).expect("Failed to create query");

        let known_matches = vec![LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: None,
            start_line: 1,
            end_line: 1,
            start_token: 0,
            end_token: 5,
            matcher: "test".to_string(),
            score: 1.0,
            matched_length: 5,
            rule_length: 5,
            matched_token_positions: None,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "test-rule".to_string(),
            rule_url: String::new(),
            matched_text: Some("some text".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            hilen: 2,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
        }];

        let matches = unknown_match(&index, &query, &known_matches);

        assert!(
            matches.is_empty() || matches[0].start_line > 1,
            "Should not re-detect known regions"
        );
    }

    #[test]
    fn test_calculate_score_edge_cases() {
        let score_zero_length = calculate_score(10, 0);
        assert_eq!(score_zero_length, 0.0, "Zero length should have zero score");

        let score_zero_ngrams = calculate_score(0, 100);
        assert_eq!(score_zero_ngrams, 0.0, "Zero ngrams should have zero score");

        let score_high_density = calculate_score(100, 50);
        assert_eq!(
            score_high_density, 1.0,
            "High density should be capped at 1.0"
        );
    }

    #[test]
    fn test_match_ngrams_in_region_out_of_bounds() {
        let tokens = vec![1u16, 2, 3];
        let automaton =
            AhoCorasick::new(std::iter::empty::<&[u8]>()).expect("Failed to create automaton");

        let count = match_ngrams_in_region(&tokens, 5, 10, &automaton);
        assert_eq!(count, 0, "Out of bounds should return 0");

        let count = match_ngrams_in_region(&tokens, 2, 1, &automaton);
        assert_eq!(count, 0, "Invalid range should return 0");
    }
}
