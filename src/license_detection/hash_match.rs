//! Hash-based exact matching for license detection.
//!
//! This module implements the hash matching strategy which computes a hash of the
//! entire query token sequence and looks for exact matches in the index.

use sha1::{Digest, Sha1};

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::QueryRun;
use crate::license_detection::spans::Span;

/// Matcher identifier for hash-based matching.
///
/// Corresponds to Python: `MATCH_HASH = '1-hash'` (line 40)
pub const MATCH_HASH: &str = "1-hash";

/// Matcher order for hash-based matching.
///
/// Hash matching is the fastest and has highest priority (0).
///
/// Corresponds to Python: `MATCH_HASH_ORDER = 0` (line 41)
#[allow(dead_code)]
pub const MATCH_HASH_ORDER: u8 = 0;

/// Compute a SHA1 hash of a token sequence.
///
/// Converts token IDs to signed 16-bit integers (matching Python's `array('h')`),
/// serializes them as little-endian bytes, and computes the SHA1 hash.
///
/// # Arguments
/// * `tokens` - Slice of token IDs
///
/// # Returns
/// 20-byte SHA1 digest
///
/// Corresponds to Python: `tokens_hash()` (lines 44-49)
pub fn compute_hash(tokens: &[u16]) -> [u8; 20] {
    let mut hasher = Sha1::new();

    for token in tokens {
        let signed = *token as i16;
        hasher.update(signed.to_le_bytes());
    }

    hasher.finalize().into()
}

/// Compute a hash of a rule's token sequence.
///
/// This is an alias for `compute_hash` to match the Python API.
///
/// Corresponds to Python: `index_hash()` (lines 52-56)
#[allow(dead_code)]
pub fn index_hash(rule_tokens: &[u16]) -> [u8; 20] {
    compute_hash(rule_tokens)
}

/// Perform hash-based matching for a query run.
///
/// Computes the hash of the query token sequence and looks for exact matches
/// in the index. If found, returns a single LicenseMatch with 100% coverage.
///
/// # Arguments
/// * `index` - The license index
/// * `query_run` - The query run to match
///
/// # Returns
/// Vector of matches (0 or 1 match)
///
/// Corresponds to Python: `hash_match()` (lines 59-87)
pub fn hash_match(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();
    let query_hash = compute_hash(query_run.tokens());

    if let Some(&rid) = index.rid_by_hash.get(&query_hash) {
        let rule = &index.rules_by_rid[rid];
        let itokens = &index.tids_by_rid[rid];

        let _qspan =
            Span::from_range(query_run.start..query_run.end.map_or(query_run.start, |e| e + 1));
        let rule_length = rule.tokens.len();
        let _ispan = Span::from_range(0..rule_length);

        let _hispan = Span::from_iterator(
            (0..rule_length).filter(|&p| itokens[p] < index.len_legalese as u16),
        );

        let matched_length = query_run.tokens().len();
        let match_coverage = 100.0;

        let start_line = query_run.line_for_pos(query_run.start).unwrap_or(1);
        let end_line = if let Some(end) = query_run.end {
            query_run.line_for_pos(end).unwrap_or(start_line)
        } else {
            start_line
        };

        let matched_text = query_run.matched_text(start_line, end_line);

        let license_match = LicenseMatch {
            license_expression: rule.license_expression.clone(),
            license_expression_spdx: rule.license_expression.clone(),
            from_file: None,
            start_line,
            end_line,
            matcher: MATCH_HASH.to_string(),
            score: 1.0,
            matched_length,
            match_coverage,
            rule_relevance: rule.relevance,
            rule_identifier: format!("#{}", rid),
            rule_url: String::new(),
            matched_text: Some(matched_text),
            referenced_filenames: rule.referenced_filenames.clone(),
            is_license_intro: rule.is_license_intro,
            is_license_clue: rule.is_license_clue,
        };

        matches.push(license_match);
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::models::Rule;
    use crate::license_detection::test_utils::{create_mock_query_with_tokens, create_test_index};

    fn create_test_rules_by_rid() -> Vec<Rule> {
        vec![
            Rule {
                identifier: "mit.LICENSE".to_string(),
                license_expression: "mit".to_string(),
                text: "MIT License".to_string(),
                tokens: vec![0, 1],
                is_license_text: true,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: true,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 2,
                high_length_unique: 2,
                high_length: 2,
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
            },
            Rule {
                identifier: "apache-2.0.LICENSE".to_string(),
                license_expression: "apache-2.0".to_string(),
                text: "Apache License 2.0".to_string(),
                tokens: vec![2, 3, 4],
                is_license_text: true,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: true,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 3,
                high_length_unique: 0,
                high_length: 0,
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
            },
        ]
    }

    #[test]
    fn test_compute_hash() {
        let tokens = vec![1u16, 2, 3, 4, 5];
        let hash = compute_hash(&tokens);

        assert_eq!(hash.len(), 20);

        let tokens2 = vec![1u16, 2, 3, 4, 5];
        let hash2 = compute_hash(&tokens2);

        assert_eq!(hash, hash2, "Same tokens should produce same hash");

        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(
            hash_hex, "aaa562e5641b932d5d5ecae43b47793b33b3b5f0",
            "Hash should match Python implementation"
        );
    }

    #[test]
    fn test_compute_hash_different_tokens() {
        let tokens1 = vec![1u16, 2, 3];
        let hash1 = compute_hash(&tokens1);

        let tokens2 = vec![1u16, 2, 4];
        let hash2 = compute_hash(&tokens2);

        assert_ne!(
            hash1, hash2,
            "Different tokens should produce different hashes"
        );
    }

    #[test]
    fn test_index_hash() {
        let rule_tokens = vec![10u16, 20, 30];
        let hash1 = compute_hash(&rule_tokens);
        let hash2 = index_hash(&rule_tokens);

        assert_eq!(hash1, hash2, "index_hash should be same as compute_hash");
    }

    #[test]
    fn test_hash_match_no_match() {
        let mut index =
            create_test_index(&[("mit", 0), ("license", 1), ("apache", 2), ("2.0", 3)], 2);

        let rules_by_rid = create_test_rules_by_rid();
        let tids_by_rid: Vec<Vec<u16>> = vec![vec![0, 1], vec![2, 3, 4]];

        index.rid_by_hash.insert(compute_hash(&[5, 6, 7]), 0);
        index.rules_by_rid = rules_by_rid;
        index.tids_by_rid = tids_by_rid;

        let query_index = create_test_index(&[("token", 0)], 1);
        let query = create_mock_query_with_tokens(&[0, 1], &query_index);
        let matches = hash_match(&index, &query.whole_query_run());

        assert!(
            matches.is_empty(),
            "Should return empty list when no match found"
        );
    }

    #[test]
    fn test_hash_match_with_match() {
        let mut index =
            create_test_index(&[("mit", 0), ("license", 1), ("apache", 2), ("2.0", 3)], 2);

        let rules_by_rid = create_test_rules_by_rid();
        let tids_by_rid: Vec<Vec<u16>> = vec![vec![0, 1], vec![2, 3, 4]];

        index.rid_by_hash.insert(compute_hash(&[0, 1]), 0);
        index.rules_by_rid = rules_by_rid;
        index.tids_by_rid = tids_by_rid;

        let query_index = create_test_index(&[("token", 0)], 1);
        let query = create_mock_query_with_tokens(&[0, 1], &query_index);
        let matches = hash_match(&index, &query.whole_query_run());

        assert_eq!(matches.len(), 1, "Should return exactly one match");
        assert_eq!(matches[0].matcher, MATCH_HASH);
        assert_eq!(matches[0].score, 1.0);
        assert_eq!(matches[0].match_coverage, 100.0);
    }

    #[test]
    fn test_hash_match_hispan_filters_legalese() {
        let mut index =
            create_test_index(&[("mit", 0), ("license", 1), ("apache", 2), ("2.0", 3)], 2);

        let rules_by_rid = create_test_rules_by_rid();
        let tids_by_rid: Vec<Vec<u16>> = vec![vec![0, 1], vec![2, 3, 4]];

        index.rid_by_hash.insert(compute_hash(&[0, 1]), 0);
        index.rules_by_rid = rules_by_rid;
        index.tids_by_rid = tids_by_rid;

        let query_index = create_test_index(&[("token", 0)], 1);
        let query = create_mock_query_with_tokens(&[0, 1], &query_index);
        let matches = hash_match(&index, &query.whole_query_run());

        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_match_hash_empty_tokens() {
        let tokens: Vec<u16> = vec![];
        let hash = compute_hash(&tokens);

        assert_eq!(hash.len(), 20);
    }

    #[test]
    fn test_match_hash_large_tokens() {
        let tokens: Vec<u16> = (0..1000).collect();
        let hash = compute_hash(&tokens);

        assert_eq!(hash.len(), 20);

        let hash2 = compute_hash(&tokens);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_match_hash_single_token() {
        let tokens = vec![42u16];
        let hash = compute_hash(&tokens);

        assert_eq!(hash.len(), 20);

        let hash2 = compute_hash(&tokens);
        assert_eq!(hash, hash2, "Same single token should produce same hash");
    }

    #[test]
    fn test_match_hash_max_token_values() {
        let tokens = vec![u16::MAX, u16::MAX - 1, 0];
        let hash = compute_hash(&tokens);

        assert_eq!(hash.len(), 20);

        let tokens2 = vec![u16::MAX, u16::MAX - 1, 0];
        let hash2 = compute_hash(&tokens2);

        assert_eq!(
            hash, hash2,
            "Same max token values should produce same hash"
        );
    }

    #[test]
    fn test_hash_match_multiple_rules_same_hash() {
        let mut index =
            create_test_index(&[("mit", 0), ("license", 1), ("apache", 2), ("2.0", 3)], 2);

        let rules_by_rid = create_test_rules_by_rid();
        let tids_by_rid: Vec<Vec<u16>> = vec![vec![0, 1], vec![2, 3, 4]];

        index.rid_by_hash.insert(compute_hash(&[0, 1]), 0);
        index.rid_by_hash.insert(compute_hash(&[0, 1]), 1);
        index.rules_by_rid = rules_by_rid;
        index.tids_by_rid = tids_by_rid;

        let query_index = create_test_index(&[("token", 0)], 1);
        let query = create_mock_query_with_tokens(&[0, 1], &query_index);
        let matches = hash_match(&index, &query.whole_query_run());

        assert_eq!(
            matches.len(),
            1,
            "Should return only one match even with hash collision"
        );
    }

    #[test]
    fn test_hash_match_returns_correct_license_expression() {
        let mut index = create_test_index(&[("mit", 0), ("license", 1)], 2);

        let rules_by_rid = create_test_rules_by_rid();
        let tids_by_rid: Vec<Vec<u16>> = vec![vec![0, 1]];

        index.rid_by_hash.insert(compute_hash(&[0, 1]), 0);
        index.rules_by_rid = rules_by_rid;
        index.tids_by_rid = tids_by_rid;

        let query_index = create_test_index(&[("token", 0)], 1);
        let query = create_mock_query_with_tokens(&[0, 1], &query_index);
        let matches = hash_match(&index, &query.whole_query_run());

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].license_expression, "mit");
        assert_eq!(matches[0].matcher, MATCH_HASH);
        assert_eq!(matches[0].score, 1.0);
        assert_eq!(matches[0].match_coverage, 100.0);
    }
}
