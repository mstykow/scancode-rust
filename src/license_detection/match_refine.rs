//! Match refinement - merge, filter, and finalize license matches.
//!
//! This module implements the final phase of license matching where raw matches
//! from all strategies are combined, refined, and finalized.
//!
//! Based on the Python ScanCode Toolkit implementation at:
//! reference/scancode-toolkit/src/licensedcode/match.py

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::Query;
use std::collections::HashMap;

/// Filter GPL matches with very short matched_length.
///
/// GPL rules that match only a tiny number of tokens are typically false positives
/// (e.g., matching "Copyright" in a comment). These should be filtered before merging
/// to prevent them from being combined with legitimate matches.
fn filter_short_gpl_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    const GPL_SHORT_THRESHOLD: usize = 3;

    matches
        .iter()
        .filter(|m| {
            let is_gpl = m.license_expression.to_lowercase().contains("gpl");
            let is_short = m.matched_length <= GPL_SHORT_THRESHOLD;
            !(is_gpl && is_short)
        })
        .cloned()
        .collect()
}

/// Parse rule ID from rule_identifier string.
///
/// Rule identifiers typically have the format "#42" where the numeric portion is the rule ID.
///
/// # Arguments
/// * `rule_identifier` - String like "#42" or "mit.LICENSE"
///
/// # Returns
/// * `Some(usize)` - Parsed rule ID if valid
/// * `None` - If rule_identifier is empty or doesn't contain a valid number
///
/// # Examples
/// ```
/// assert_eq!(parse_rule_id("#42"), Some(42));
/// assert_eq!(parse_rule_id("#0"), Some(0));
/// assert_eq!(parse_rule_id("invalid"), None);
/// ```
fn parse_rule_id(rule_identifier: &str) -> Option<usize> {
    let trimmed = rule_identifier.trim();
    if let Some(stripped) = trimmed.strip_prefix('#') {
        stripped.parse().ok()
    } else {
        trimmed.parse().ok()
    }
}

/// Merge overlapping and adjacent matches for the same rule.
///
/// This function combines matches that:
/// - Have the same rule_identifier
/// - Are adjacent (end_line + 1 == next.start_line)
/// - Overlap in their line ranges
///
/// When merging, the combined match covers the union of all line ranges.
///
/// # Arguments
/// * `matches` - Slice of LicenseMatch to potentially merge
///
/// # Returns
/// Vector of merged LicenseMatch with no overlaps/adjacency for same rule
///
/// Based on Python: `merge_matches()` (lines 800-910)
fn merge_overlapping_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    if matches.len() == 1 {
        return matches.to_vec();
    }

    let mut grouped: HashMap<String, Vec<&LicenseMatch>> = HashMap::new();

    for m in matches {
        grouped
            .entry(m.rule_identifier.clone())
            .or_default()
            .push(m);
    }

    let mut merged = Vec::new();

    for (_rid, rule_matches) in grouped {
        if rule_matches.len() == 1 {
            merged.push(rule_matches[0].clone());
            continue;
        }

        let mut sorted_matches = rule_matches.clone();
        sorted_matches.sort_by_key(|m| (m.start_line, m.end_line));

        let mut accum = sorted_matches[0].clone();

        for next_match in &sorted_matches[1..] {
            let is_adjacent = accum.end_line + 1 >= next_match.start_line;
            let is_overlapping =
                accum.start_line <= next_match.end_line && accum.end_line >= next_match.start_line;

            if is_adjacent || is_overlapping {
                accum.start_line = accum.start_line.min(next_match.start_line);
                accum.end_line = accum.end_line.max(next_match.end_line);
                accum.matched_length = accum.matched_length.max(next_match.matched_length);
                accum.score = accum.score.max(next_match.score);
            } else {
                merged.push(accum);
                accum = (*next_match).clone();
            }
        }

        merged.push(accum);
    }

    merged
}

/// Filter matches that are contained within other matches.
///
/// A match A is contained in match B if:
/// - A.start_line >= B.start_line
/// - A.end_line <= B.end_line
/// - A.matched_length <= B.matched_length
///
/// The containing (larger) match is kept, the contained (smaller) match is removed.
/// This function does NOT group by rule_identifier - matches from different rules
/// can contain each other.
///
/// # Arguments
/// * `matches` - Slice of LicenseMatch to filter
///
/// # Returns
/// Vector of LicenseMatch with contained matches removed
///
/// Based on Python: `filter_contained_matches()` (lines 950-1070)
fn filter_contained_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.len() < 2 {
        return matches.to_vec();
    }

    let mut sorted: Vec<&LicenseMatch> = matches.iter().collect();
    sorted.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| b.matched_length.cmp(&a.matched_length))
    });

    let mut kept = Vec::new();

    for current in sorted {
        let is_contained = kept.iter().any(|kept_match: &&LicenseMatch| {
            current.start_line >= kept_match.start_line
                && current.end_line <= kept_match.end_line
                && current.matched_length <= kept_match.matched_length
        });

        if !is_contained {
            kept.push(current);
        }
    }

    kept.into_iter().cloned().collect()
}

/// Filter matches to false positive rules.
///
/// Removes matches whose rule ID is in the index's false_positive_rids set.
///
/// # Arguments
/// * `index` - LicenseIndex containing false_positive_rids
/// * `matches` - Slice of LicenseMatch to filter
///
/// # Returns
/// Vector of LicenseMatch with false positive matches removed
///
/// Based on Python: `filter_false_positive_matches()` (lines 1950-1970)
fn filter_false_positive_matches(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    let mut filtered = Vec::new();

    for m in matches {
        if let Some(rid) = parse_rule_id(&m.rule_identifier)
            && index.false_positive_rids.contains(&rid)
        {
            continue;
        }

        filtered.push(m.clone());
    }

    filtered
}

/// Update match scores for all matches.
///
/// Ensures all matches have correctly computed scores using the formula:
/// `score = match_coverage * rule_relevance / 100`
///
/// This function is idempotent - safe to call multiple times.
///
/// # Arguments
/// * `matches` - Mutable slice of LicenseMatch to update
///
/// Based on Python: LicenseMatch.score() method and score calculation in refine_matches()
fn update_match_scores(matches: &mut [LicenseMatch]) {
    for m in matches.iter_mut() {
        m.score = m.match_coverage * m.rule_relevance as f32 / 100.0;
    }
}

/// Main refinement function - applies all refinement operations to match results.
///
/// This is the main entry point for Phase 4.6 match refinement. It:
/// 1. Merges overlapping/adjacent matches
/// 2. Filters contained matches
/// 3. Filters false positive matches
/// 4. Updates match scores
///
/// The operations are applied in sequence to produce final refined matches.
///
/// # Arguments
/// * `index` - LicenseIndex containing false_positive_rids
/// * `matches` - Vector of raw LicenseMatch from all strategies
/// * `_query` - Query (unused in Phase 4.6 but kept for API compatibility)
///
/// # Returns
/// Vector of refined LicenseMatch ready for detection assembly
///
/// Based on Python: `refine_matches()` (lines 2691-2833)
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    _query: &Query,
) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    let filtered = filter_short_gpl_matches(&matches);

    let mut refined = merge_overlapping_matches(&filtered);

    refined = filter_contained_matches(&refined);

    refined = filter_false_positive_matches(index, &refined);

    update_match_scores(&mut refined);

    refined
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_match(
        rule_identifier: &str,
        start_line: usize,
        end_line: usize,
        score: f32,
        coverage: f32,
        relevance: u8,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: None,
            start_line,
            end_line,
            matcher: "2-aho".to_string(),
            score,
            matched_length: 100,
            match_coverage: coverage,
            rule_relevance: relevance,
            rule_identifier: rule_identifier.to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: None,
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
        }
    }

    #[test]
    fn test_parse_rule_id_valid_hashes() {
        assert_eq!(parse_rule_id("#0"), Some(0));
        assert_eq!(parse_rule_id("#1"), Some(1));
        assert_eq!(parse_rule_id("#42"), Some(42));
        assert_eq!(parse_rule_id("#100"), Some(100));
        assert_eq!(parse_rule_id("#999"), Some(999));
    }

    #[test]
    fn test_parse_rule_id_plain_numbers() {
        assert_eq!(parse_rule_id("0"), Some(0));
        assert_eq!(parse_rule_id("42"), Some(42));
        assert_eq!(parse_rule_id("100"), Some(100));
    }

    #[test]
    fn test_parse_rule_id_invalid_formats() {
        assert_eq!(parse_rule_id(""), None);
        assert_eq!(parse_rule_id("#"), None);
        assert_eq!(parse_rule_id("#-1"), None);
        assert_eq!(parse_rule_id("invalid"), None);
        assert_eq!(parse_rule_id("#abc"), None);
        assert_eq!(parse_rule_id("mit.LICENSE"), None);
    }

    #[test]
    fn test_merge_overlapping_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 5, 15, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].rule_identifier, "#1");
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 15);
        assert_eq!(merged[0].score, 0.9);
    }

    #[test]
    fn test_merge_adjacent_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 11, 20, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].rule_identifier, "#1");
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 20);
        assert_eq!(merged[0].score, 0.9);
    }

    #[test]
    fn test_merge_no_overlap_different_rules() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 5, 15, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_merge_no_overlap_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 20, 30, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_merge_multiple_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 5, 0.8, 80.0, 100),
            create_test_match("#1", 6, 10, 0.9, 90.0, 100),
            create_test_match("#1", 11, 15, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 15);
    }

    #[test]
    fn test_merge_empty_matches() {
        let matches: Vec<LicenseMatch> = vec![];
        let merged = merge_overlapping_matches(&matches);
        assert_eq!(merged.len(), 0);
    }

    #[test]
    fn test_merge_single_match() {
        let matches = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let merged = merge_overlapping_matches(&matches);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 10);
    }

    #[test]
    fn test_filter_contained_matches_simple() {
        let matches = vec![
            create_test_match("#1", 1, 20, 0.9, 90.0, 100),
            create_test_match("#1", 5, 15, 0.85, 85.0, 100),
        ];

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_line, 1);
        assert_eq!(filtered[0].end_line, 20);
    }

    #[test]
    fn test_filter_contained_matches_multiple() {
        let matches = vec![
            create_test_match("#1", 1, 30, 0.9, 90.0, 100),
            create_test_match("#1", 5, 10, 0.8, 80.0, 100),
            create_test_match("#1", 15, 20, 0.85, 85.0, 100),
        ];

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_line, 1);
        assert_eq!(filtered[0].end_line, 30);
    }

    #[test]
    fn test_filter_contained_matches_different_rules() {
        let mut matches = vec![
            create_test_match("#1", 1, 20, 0.9, 90.0, 100),
            create_test_match("#2", 5, 15, 0.85, 85.0, 100),
        ];
        matches[0].matched_length = 200;
        matches[1].matched_length = 100;

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule_identifier, "#1");
    }

    #[test]
    fn test_filter_contained_matches_no_containment() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 15, 25, 0.85, 85.0, 100),
        ];

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_contained_matches_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        let filtered = filter_contained_matches(&matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_contained_matches_single() {
        let matches = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let filtered = filter_contained_matches(&matches);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_false_positive_matches_with_false_positive() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(42);

        let matches = vec![
            create_test_match("#42", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 15, 25, 0.85, 85.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule_identifier, "#1");
    }

    #[test]
    fn test_filter_false_positive_matches_no_false_positive() {
        let index = LicenseIndex::with_legalese_count(10);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 15, 25, 0.85, 85.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_false_positive_matches_all_false_positive() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(42);
        let _ = index.false_positive_rids.insert(43);

        let matches = vec![
            create_test_match("#42", 1, 10, 0.9, 90.0, 100),
            create_test_match("#43", 15, 25, 0.85, 85.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_false_positive_matches_empty() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches: Vec<LicenseMatch> = vec![];
        let filtered = filter_false_positive_matches(&index, &matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_update_match_scores_basic() {
        let mut matches = vec![create_test_match("#1", 1, 10, 0.5, 50.0, 100)];

        update_match_scores(&mut matches);

        assert_eq!(matches[0].score, 50.0);
    }

    #[test]
    fn test_update_match_scores_multiple() {
        let mut matches = vec![
            create_test_match("#1", 1, 10, 0.5, 50.0, 80),
            create_test_match("#2", 15, 25, 0.5, 50.0, 100),
        ];

        update_match_scores(&mut matches);

        assert_eq!(matches[0].score, 40.0);
        assert_eq!(matches[1].score, 50.0);
    }

    #[test]
    fn test_update_match_scores_idempotent() {
        let mut matches = vec![create_test_match("#1", 1, 10, 50.0, 50.0, 100)];

        update_match_scores(&mut matches);
        let score1 = matches[0].score;

        update_match_scores(&mut matches);
        let score2 = matches[0].score;

        assert_eq!(score1, score2);
    }

    #[test]
    fn test_update_match_scores_empty() {
        let mut matches: Vec<LicenseMatch> = vec![];
        update_match_scores(&mut matches);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_refine_matches_full_pipeline() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(99);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.5, 50.0, 100),
            create_test_match("#1", 5, 15, 0.5, 50.0, 100),
            create_test_match("#2", 20, 25, 0.5, 50.0, 80),
            create_test_match("#99", 30, 35, 0.5, 50.0, 100),
        ];

        let query = Query::new("test text", &index).unwrap();
        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 2);

        let rule1_match = refined.iter().find(|m| m.rule_identifier == "#1").unwrap();
        assert_eq!(rule1_match.start_line, 1);
        assert_eq!(rule1_match.end_line, 15);

        let rule2_match = refined.iter().find(|m| m.rule_identifier == "#2").unwrap();
        assert_eq!(rule2_match.score, 40.0);
    }

    #[test]
    fn test_refine_matches_empty() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches: Vec<LicenseMatch> = vec![];
        let query = Query::new("", &index).unwrap();

        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 0);
    }

    #[test]
    fn test_refine_matches_single() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches = vec![create_test_match("#1", 1, 10, 0.5, 50.0, 100)];
        let query = Query::new("test text", &index).unwrap();

        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 1);
        assert_eq!(refined[0].score, 50.0);
    }

    #[test]
    fn test_refine_matches_no_merging_needed() {
        let index = LicenseIndex::with_legalese_count(10);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 20, 30, 0.85, 85.0, 100),
        ];

        let query = Query::new("test text", &index).unwrap();

        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 2);
    }

    #[test]
    fn test_filter_short_gpl_matches_removes_short_gpl() {
        let matches = vec![
            LicenseMatch {
                license_expression: "gpl-2.0".to_string(),
                license_expression_spdx: "GPL-2.0".to_string(),
                matched_length: 1,
                ..create_test_match("#1", 1, 1, 1.0, 100.0, 100)
            },
            LicenseMatch {
                license_expression: "mit".to_string(),
                license_expression_spdx: "MIT".to_string(),
                matched_length: 1,
                ..create_test_match("#2", 5, 10, 1.0, 100.0, 100)
            },
        ];

        let filtered = filter_short_gpl_matches(&matches);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].license_expression, "mit");
    }

    #[test]
    fn test_filter_short_gpl_matches_keeps_long_gpl() {
        let matches = vec![LicenseMatch {
            license_expression: "gpl-2.0".to_string(),
            license_expression_spdx: "GPL-2.0".to_string(),
            matched_length: 10,
            ..create_test_match("#1", 1, 10, 1.0, 100.0, 100)
        }];

        let filtered = filter_short_gpl_matches(&matches);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_short_gpl_matches_keeps_short_non_gpl() {
        let matches = vec![LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            matched_length: 1,
            ..create_test_match("#1", 1, 1, 1.0, 100.0, 100)
        }];

        let filtered = filter_short_gpl_matches(&matches);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_short_gpl_matches_boundary_threshold() {
        let matches = vec![
            LicenseMatch {
                license_expression: "gpl-2.0".to_string(),
                license_expression_spdx: "GPL-2.0".to_string(),
                matched_length: 3,
                ..create_test_match("#1", 1, 1, 1.0, 100.0, 100)
            },
            LicenseMatch {
                license_expression: "gpl-3.0".to_string(),
                license_expression_spdx: "GPL-3.0".to_string(),
                matched_length: 4,
                ..create_test_match("#2", 5, 10, 1.0, 100.0, 100)
            },
        ];

        let filtered = filter_short_gpl_matches(&matches);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].matched_length, 4);
    }

    #[test]
    fn test_filter_short_gpl_matches_case_insensitive() {
        let matches = vec![LicenseMatch {
            license_expression: "GPL-2.0".to_string(),
            license_expression_spdx: "GPL-2.0".to_string(),
            matched_length: 2,
            ..create_test_match("#1", 1, 1, 1.0, 100.0, 100)
        }];

        let filtered = filter_short_gpl_matches(&matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_merge_partially_overlapping_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 15, 0.9, 90.0, 100),
            create_test_match("#1", 10, 25, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 25);
    }

    #[test]
    fn test_merge_matches_with_gap_larger_than_one() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 15, 25, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 10);
        assert_eq!(merged[1].start_line, 15);
        assert_eq!(merged[1].end_line, 25);
    }

    #[test]
    fn test_merge_preserves_max_score() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.7, 70.0, 100),
            create_test_match("#1", 5, 15, 0.95, 95.0, 100),
            create_test_match("#1", 12, 20, 0.8, 80.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].score, 0.95);
    }

    #[test]
    fn test_filter_contained_matches_partial_overlap_no_containment() {
        let mut m1 = create_test_match("#1", 1, 20, 0.9, 90.0, 100);
        m1.matched_length = 150;
        let mut m2 = create_test_match("#2", 15, 30, 0.85, 85.0, 100);
        m2.matched_length = 100;
        let matches = vec![m1, m2];

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_contained_matches_equal_start_different_end() {
        let mut m1 = create_test_match("#1", 1, 30, 0.9, 90.0, 100);
        m1.matched_length = 200;
        let mut m2 = create_test_match("#2", 1, 15, 0.85, 85.0, 100);
        m2.matched_length = 100;
        let matches = vec![m1, m2];

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].end_line, 30);
    }

    #[test]
    fn test_filter_contained_matches_nested_containment() {
        let mut outer = create_test_match("#1", 1, 50, 0.9, 90.0, 100);
        outer.matched_length = 300;
        let mut middle = create_test_match("#2", 10, 40, 0.85, 85.0, 100);
        middle.matched_length = 200;
        let mut inner = create_test_match("#3", 15, 35, 0.8, 80.0, 100);
        inner.matched_length = 100;
        let matches = vec![inner, middle, outer];

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_line, 1);
        assert_eq!(filtered[0].end_line, 50);
    }

    #[test]
    fn test_filter_contained_matches_same_boundaries_different_matched_length() {
        let mut m1 = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
        m1.matched_length = 200;
        let mut m2 = create_test_match("#2", 1, 10, 0.85, 85.0, 100);
        m2.matched_length = 100;
        let matches = vec![m1, m2];

        let filtered = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].matched_length, 200);
    }

    #[test]
    fn test_refine_matches_pipeline_preserves_non_overlapping_different_rules() {
        let index = LicenseIndex::with_legalese_count(10);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 20, 30, 0.85, 85.0, 100),
            create_test_match("#3", 40, 50, 0.8, 80.0, 100),
        ];

        let query = Query::new("test text", &index).unwrap();
        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 3);
    }

    #[test]
    fn test_refine_matches_complex_scenario() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(999);

        let mut m1 = create_test_match("#1", 1, 10, 0.7, 70.0, 100);
        m1.matched_length = 100;
        let mut m2 = create_test_match("#1", 8, 15, 0.8, 80.0, 100);
        m2.matched_length = 100;
        let mut m3 = create_test_match("#2", 20, 50, 0.9, 90.0, 100);
        m3.matched_length = 300;
        let mut m4 = create_test_match("#2", 25, 45, 0.85, 85.0, 100);
        m4.matched_length = 150;
        let m5 = create_test_match("#999", 60, 70, 0.9, 90.0, 100);

        let matches = vec![m1, m2, m3, m4, m5];

        let query = Query::new("test text", &index).unwrap();
        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 2);

        let rule1_match = refined.iter().find(|m| m.rule_identifier == "#1").unwrap();
        assert_eq!(rule1_match.start_line, 1);
        assert_eq!(rule1_match.end_line, 15);

        let rule2_match = refined.iter().find(|m| m.rule_identifier == "#2").unwrap();
        assert_eq!(rule2_match.start_line, 20);
        assert_eq!(rule2_match.end_line, 50);
    }

    #[test]
    fn test_parse_rule_id_with_whitespace() {
        assert_eq!(parse_rule_id("  #42  "), Some(42));
        assert_eq!(parse_rule_id("  42  "), Some(42));
    }

    #[test]
    fn test_filter_false_positive_matches_mixed_identifiers() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(42);

        let matches = vec![
            create_test_match("#42", 1, 10, 0.9, 90.0, 100),
            create_test_match("mit.LICENSE", 15, 25, 0.85, 85.0, 100),
            create_test_match("#1", 30, 40, 0.8, 80.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|m| m.rule_identifier == "mit.LICENSE"));
        assert!(filtered.iter().any(|m| m.rule_identifier == "#1"));
    }
}
