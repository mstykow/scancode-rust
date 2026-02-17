//! Aho-Corasick exact matching for license detection.
//!
//! This module implements Aho-Corasick multi-pattern matching for license detection.
//! Token sequences from rules are encoded as bytes and used to build the automaton,
//! which can then efficiently find all matches in query token sequences.
//!
//! Based on the Python ScanCode Toolkit implementation at:
//! reference/scancode-toolkit/src/licensedcode/match_aho.py

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::QueryRun;

/// Matcher identifier for Aho-Corasick exact matching.
///
/// Corresponds to Python: `MATCH_AHO_EXACT = '2-aho'` (line 78)
pub const MATCH_AHO: &str = "2-aho";

/// Matcher order for Aho-Corasick exact matching.
///
/// Aho-Corasick matching runs after hash matching and SPDX-LID matching.
///
/// Corresponds to Python: `MATCH_AHO_EXACT_ORDER = 1` (line 79)
#[allow(dead_code)]
pub const MATCH_AHO_ORDER: u8 = 2;

/// Encode u16 token sequence as bytes.
///
/// Each token is encoded as 2 bytes in little-endian format.
/// This is necessary because Aho-Corasick works on bytes, not u16 values directly.
///
/// # Arguments
/// * `tokens` - Slice of token IDs to encode
///
/// # Returns
/// Byte vector where each token is represented as 2 little-endian bytes
///
/// # Examples
/// ```
/// let tokens = vec![1u16, 2, 3];
/// let bytes = tokens_to_bytes(&tokens);
/// assert_eq!(bytes, vec![1, 0, 2, 0, 3, 0]);
/// ```
fn tokens_to_bytes(tokens: &[u16]) -> Vec<u8> {
    tokens.iter().flat_map(|t| t.to_le_bytes()).collect()
}

/// Convert byte position to token position.
///
/// Since each token is encoded as 2 bytes, we divide the byte position by 2.
///
/// # Arguments
/// * `byte_pos` - Byte position in the encoded bytes
///
/// # Returns
/// Token position (byte_pos / 2)
#[inline]
fn byte_pos_to_token_pos(byte_pos: usize) -> usize {
    byte_pos / 2
}

/// Perform Aho-Corasick exact matching for a query run.
///
/// This function matches the query token sequence against all rules in the automaton,
/// finding all exact occurrences of rule token sequences. For each match, it verifies
/// that all positions are matchable and creates a LicenseMatch with proper coverage scores.
///
/// # Arguments
/// * `index` - The license index containing the automaton and rules
/// * `query_run` - The query run to match
///
/// # Returns
/// Vector of matches found by the Aho-Corasick automaton
///
/// Corresponds to Python: `exact_match()` (lines 84-138)
pub fn aho_match(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();

    let query_tokens = query_run.tokens();
    if query_tokens.is_empty() {
        return matches;
    }

    let encoded_query = tokens_to_bytes(query_tokens);
    let qbegin = query_run.start;

    let matchables = query_run.matchables(true);

    let automaton = &index.rules_automaton;

    for ac_match in automaton.find_overlapping_iter(&encoded_query) {
        let pattern_id = ac_match.pattern();
        let byte_start = ac_match.start();
        let byte_end = ac_match.end();

        let qstart = qbegin + byte_pos_to_token_pos(byte_start);
        let qend = qbegin + byte_pos_to_token_pos(byte_end);

        let is_entirely_matchable = (qstart..qend).all(|pos| matchables.contains(&pos));

        if !is_entirely_matchable {
            continue;
        }

        let Some(&rid) = index.pattern_id_to_rid.get(pattern_id.as_usize()) else {
            continue;
        };
        if rid >= index.rules_by_rid.len() {
            continue;
        }

        let matched_length = qend - qstart;

        // Skip zero-length matches (empty patterns)
        if matched_length == 0 {
            continue;
        }

        let rule = &index.rules_by_rid[rid];
        let rule_tids = &index.tids_by_rid[rid];
        let rule_length = rule.tokens.len();

        let match_coverage = if rule_length > 0 {
            (matched_length as f32 / rule_length as f32) * 100.0
        } else {
            100.0
        };

        let hispan_count = (0..matched_length)
            .filter(|&p| {
                rule_tids
                    .get(p)
                    .is_some_and(|tid| *tid < index.len_legalese as u16)
            })
            .count();

        let start_line = query_run.line_for_pos(qstart).unwrap_or(1);

        let end_line = if qend > qstart {
            // qend is exclusive, so the last matched token is at qend-1
            query_run
                .line_for_pos(qend.saturating_sub(1))
                .unwrap_or(start_line)
        } else {
            start_line
        };

        let score = if rule_length > 0 {
            matched_length as f32 / rule_length as f32
        } else {
            1.0
        };

        let matched_text = query_run.matched_text(start_line, end_line);

        let license_match = LicenseMatch {
            license_expression: rule.license_expression.clone(),
            license_expression_spdx: rule.license_expression.clone(),
            from_file: None,
            start_line,
            end_line,
            start_token: qstart,
            end_token: qend,
            matcher: MATCH_AHO.to_string(),
            score,
            matched_length,
            rule_length,
            match_coverage,
            rule_relevance: rule.relevance,
            rule_identifier: format!("#{}", rid),
            rule_url: String::new(),
            matched_text: Some(matched_text),
            referenced_filenames: rule.referenced_filenames.clone(),
            is_license_intro: rule.is_license_intro,
            is_license_clue: rule.is_license_clue,
            is_license_reference: rule.is_license_reference,
            is_license_tag: rule.is_license_tag,
            matched_token_positions: None,
            hilen: hispan_count,
        };

        matches.push(license_match);
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::test_utils::{
        create_mock_query_with_tokens, create_mock_rule, create_test_index_default,
    };
    use aho_corasick::{AhoCorasick, AhoCorasickBuilder};

    #[test]
    fn test_tokens_to_bytes_empty() {
        let tokens: Vec<u16> = vec![];
        let bytes = tokens_to_bytes(&tokens);
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_tokens_to_bytes_single() {
        let tokens = vec![1u16];
        let bytes = tokens_to_bytes(&tokens);
        assert_eq!(bytes, vec![1, 0]);
    }

    #[test]
    fn test_tokens_to_bytes_multiple() {
        let tokens = vec![1u16, 2, 3, 255, 256];
        let bytes = tokens_to_bytes(&tokens);
        assert_eq!(bytes, vec![1, 0, 2, 0, 3, 0, 255, 0, 0, 1]);
    }

    #[test]
    fn test_byte_pos_to_token_pos() {
        assert_eq!(byte_pos_to_token_pos(0), 0);
        assert_eq!(byte_pos_to_token_pos(1), 0);
        assert_eq!(byte_pos_to_token_pos(2), 1);
        assert_eq!(byte_pos_to_token_pos(3), 1);
        assert_eq!(byte_pos_to_token_pos(4), 2);
        assert_eq!(byte_pos_to_token_pos(10), 5);
    }

    #[test]
    fn test_aho_match_empty_query() {
        let index = create_test_index_default();
        let query = create_mock_query_with_tokens(&[], &index);
        let run = query.whole_query_run();

        let matches = aho_match(run.get_index(), &run);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_aho_match_no_automaton_patterns() {
        let mut index = create_test_index_default();
        index.rules_automaton = AhoCorasick::new::<_, &[u8]>([]).unwrap();

        let query = create_mock_query_with_tokens(&[0, 1, 2], &index);
        let run = query.whole_query_run();

        let matches = aho_match(run.get_index(), &run);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_aho_match_with_simple_pattern() {
        let mut index = create_test_index_default();

        let rule_tokens = vec![0u16, 1];
        let pattern_bytes = tokens_to_bytes(&rule_tokens);

        let automaton = AhoCorasickBuilder::new()
            .build(std::iter::once(pattern_bytes.as_slice()))
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("mit", vec![0, 1], false, false));
        index.tids_by_rid.push(vec![0, 1]);
        index.pattern_id_to_rid.push(0);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0, 1],
            line_by_pos: vec![1, 1],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: (0..2).collect(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matcher, MATCH_AHO);
        assert_eq!(matches[0].score, 1.0);
        assert_eq!(matches[0].match_coverage, 100.0);
    }

    #[test]
    fn test_aho_match_coverage() {
        let mut index = create_test_index_default();

        let rule_tokens = vec![0u16, 1, 2];
        let pattern_bytes = tokens_to_bytes(&rule_tokens);

        let automaton = AhoCorasickBuilder::new()
            .build(std::iter::once(pattern_bytes.as_slice()))
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("apache-2.0", vec![0, 1, 2], false, false));
        index.tids_by_rid.push(vec![0, 1, 2]);
        index.pattern_id_to_rid.push(0);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0, 1, 2],
            line_by_pos: vec![1, 1, 1],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: (0..3).collect(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_length, 3);
        assert_eq!(matches[0].match_coverage, 100.0);
    }

    #[test]
    fn test_aho_match_multiple_patterns() {
        let mut index = create_test_index_default();

        let pattern1 = tokens_to_bytes(&[0u16, 1]);
        let pattern2 = tokens_to_bytes(&[2u16, 3]);

        let automaton = AhoCorasickBuilder::new()
            .build([pattern1.as_slice(), pattern2.as_slice()])
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("mit", vec![0, 1], true, false));
        index
            .rules_by_rid
            .push(create_mock_rule("apache-2.0", vec![2, 3], true, false));
        index.tids_by_rid.push(vec![0, 1]);
        index.tids_by_rid.push(vec![2, 3]);
        index.pattern_id_to_rid.push(0);
        index.pattern_id_to_rid.push(1);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0, 1, 2, 3],
            line_by_pos: vec![1, 1, 2, 2],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: (0..4).collect(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].license_expression, "mit");
        assert_eq!(matches[0].matched_length, 2);
        assert_eq!(matches[1].license_expression, "apache-2.0");
        assert_eq!(matches[1].matched_length, 2);
    }

    #[test]
    fn test_aho_match_filters_non_matchable() {
        let mut index = create_test_index_default();

        let pattern = tokens_to_bytes(&[0u16, 1, 2]);

        let automaton = AhoCorasickBuilder::new()
            .build(std::iter::once(pattern.as_slice()))
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("mit", vec![0, 1, 2], false, false));
        index.tids_by_rid.push(vec![0, 1, 2]);
        index.pattern_id_to_rid.push(0);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0, 1, 2],
            line_by_pos: vec![1, 1, 1],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: std::collections::HashSet::new(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert!(
            matches.is_empty(),
            "Should not match non-matchable positions"
        );
    }

    #[test]
    fn test_aho_match_line_numbers() {
        let mut index = create_test_index_default();

        let pattern = tokens_to_bytes(&[0u16, 1]);

        let automaton = AhoCorasickBuilder::new()
            .build(std::iter::once(pattern.as_slice()))
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("mit", vec![0, 1], true, false));
        index.tids_by_rid.push(vec![0, 1]);
        index.pattern_id_to_rid.push(0);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0, 1],
            line_by_pos: vec![5, 5],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: (0..2).collect(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_line, 5);
        assert_eq!(matches[0].end_line, 5);
    }

    #[test]
    fn test_constants() {
        assert_eq!(MATCH_AHO, "2-aho");
        assert_eq!(MATCH_AHO_ORDER, 2);
    }

    #[test]
    fn test_aho_match_overlapping_patterns() {
        let mut index = create_test_index_default();

        let pattern1 = tokens_to_bytes(&[0u16, 1, 2]);
        let pattern2 = tokens_to_bytes(&[1u16, 2]);

        let automaton = AhoCorasickBuilder::new()
            .build([pattern1.as_slice(), pattern2.as_slice()])
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("mit-full", vec![0, 1, 2], true, false));
        index
            .rules_by_rid
            .push(create_mock_rule("mit-partial", vec![1, 2], true, false));
        index.tids_by_rid.push(vec![0, 1, 2]);
        index.tids_by_rid.push(vec![1, 2]);
        index.pattern_id_to_rid.push(0);
        index.pattern_id_to_rid.push(1);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0, 1, 2],
            line_by_pos: vec![1, 1, 1],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: (0..3).collect(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert!(!matches.is_empty(), "Should find overlapping matches");
    }

    #[test]
    fn test_aho_match_zero_length_pattern() {
        let mut index = create_test_index_default();

        let pattern = tokens_to_bytes(&[0u16]);

        let automaton = AhoCorasickBuilder::new()
            .build(std::iter::once(pattern.as_slice()))
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("single-token", vec![0], false, false));
        index.tids_by_rid.push(vec![0]);
        index.pattern_id_to_rid.push(0);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0],
            line_by_pos: vec![1],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: std::collections::HashSet::new(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert!(
            matches.is_empty(),
            "Should not match single low-value token"
        );
    }

    #[test]
    fn test_aho_match_long_query() {
        let mut index = create_test_index_default();

        let pattern = tokens_to_bytes(&[0u16, 1]);

        let automaton = AhoCorasickBuilder::new()
            .build(std::iter::once(pattern.as_slice()))
            .unwrap();

        index.rules_automaton = automaton;
        index
            .rules_by_rid
            .push(create_mock_rule("mit", vec![0, 1], true, false));
        index.tids_by_rid.push(vec![0, 1]);
        index.pattern_id_to_rid.push(0);

        let tokens: Vec<u16> = (0..1000).map(|i| i % 2).collect();
        let line_by_pos: Vec<usize> = (0..1000).map(|i| i / 80 + 1).collect();

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens,
            line_by_pos,
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: (0..1000).collect(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert!(
            matches.len() > 1,
            "Should find multiple matches in long query"
        );
    }

    #[test]
    fn test_aho_match_score_calculation() {
        let mut index = create_test_index_default();

        let rule_tokens = vec![0u16, 1, 2, 3, 4];
        let pattern_bytes = tokens_to_bytes(&rule_tokens);

        let automaton = AhoCorasickBuilder::new()
            .build(std::iter::once(pattern_bytes.as_slice()))
            .unwrap();

        index.rules_automaton = automaton;
        index.rules_by_rid.push(create_mock_rule(
            "apache-2.0",
            vec![0, 1, 2, 3, 4],
            true,
            false,
        ));
        index.tids_by_rid.push(vec![0, 1, 2, 3, 4]);
        index.pattern_id_to_rid.push(0);

        let query = crate::license_detection::query::Query {
            text: String::new(),
            tokens: vec![0, 1, 2, 3, 4],
            line_by_pos: vec![1, 1, 1, 1, 1],
            unknowns_by_pos: std::collections::HashMap::new(),
            stopwords_by_pos: std::collections::HashMap::new(),
            shorts_and_digits_pos: std::collections::HashSet::new(),
            high_matchables: (0..5).collect(),
            low_matchables: std::collections::HashSet::new(),
            has_long_lines: false,
            is_binary: false,
            query_run_ranges: Vec::new(),
            spdx_lines: Vec::new(),
            index: &index,
        };

        let run = query.whole_query_run();
        let matches = aho_match(run.get_index(), &run);

        assert_eq!(matches.len(), 1);
        assert!(
            (matches[0].score - 1.0).abs() < 0.001,
            "Full match should have score 1.0"
        );
        assert_eq!(matches[0].matched_length, 5);
        assert_eq!(matches[0].match_coverage, 100.0);
    }
}
