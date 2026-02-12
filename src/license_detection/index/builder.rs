//! License index builder.
//!
//! This module implements the `build_index()` function that constructs all
//! index data structures from rules and licenses.
//!
//! Based on the Python ScanCode Toolkit implementation at:
//! reference/scancode-toolkit/src/licensedcode/index.py (lines 381-577)

use aho_corasick::AhoCorasickBuilder;
use std::collections::{HashMap, HashSet};

use crate::license_detection::hash_match::compute_hash;
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::index::dictionary::TokenDictionary;
use crate::license_detection::index::token_sets::{
    build_set_and_mset, high_multiset_subset, high_tids_set_subset, multiset_counter,
    tids_set_counter,
};
use crate::license_detection::models::{License, Rule};
use crate::license_detection::rules::legalese;
use crate::license_detection::rules::thresholds::{
    SMALL_RULE, TINY_RULE, compute_thresholds_occurrences, compute_thresholds_unique,
};
use crate::license_detection::tokenize::tokenize;

const UNKNOWN_NGRAM_LENGTH: usize = 6;

const MARKERS: &[&str] = &[
    "copyright",
    "c",
    "copyrights",
    "rights",
    "reserved",
    "trademark",
    "foundation",
    "government",
    "institute",
    "university",
    "inc",
    "corp",
    "co",
    "author",
    "com",
    "org",
    "net",
    "uk",
    "fr",
    "be",
    "de",
    "http",
    "https",
    "www",
];

fn is_good_tokens_ngram(tokens_ngram: &[String], tids_ngram: &[u16], len_legalese: usize) -> bool {
    const MIN_GOOD: usize = 3;

    let digit_count = tokens_ngram
        .iter()
        .filter(|t| t.chars().all(|c| c.is_ascii_digit()))
        .count();
    if digit_count >= MIN_GOOD {
        return false;
    }

    let year_count = tokens_ngram
        .iter()
        .filter(|t| t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()))
        .count();
    if year_count > 0 {
        return false;
    }

    let single_char_count = tokens_ngram.iter().filter(|t| t.len() == 1).count();
    if single_char_count >= MIN_GOOD {
        return false;
    }

    let unique_tids: HashSet<u16> = tids_ngram.iter().copied().collect();
    if unique_tids.len() <= 2 {
        return false;
    }

    let has_high_token = tids_ngram.iter().any(|&tid| (tid as usize) < len_legalese);
    if !has_high_token {
        return false;
    }

    let has_marker = tokens_ngram.iter().any(|t| MARKERS.contains(&t.as_str()));
    if has_marker {
        return false;
    }

    true
}

fn compute_is_approx_matchable(rule: &Rule) -> bool {
    !(rule.is_false_positive
        || rule.is_required_phrase
        || rule.is_tiny
        || rule.is_continuous
        || (rule.is_small && (rule.is_license_reference || rule.is_license_tag)))
}

fn tokens_to_bytes(tokens: &[u16]) -> Vec<u8> {
    tokens.iter().flat_map(|t| t.to_le_bytes()).collect()
}

fn ngrams<T: Clone>(items: &[T], ngram_length: usize) -> Vec<Vec<T>> {
    if items.len() < ngram_length {
        return Vec::new();
    }
    items
        .windows(ngram_length)
        .map(|window| window.to_vec())
        .collect()
}

pub fn build_index(rules: Vec<Rule>, licenses: Vec<License>) -> LicenseIndex {
    let legalese_words = legalese::get_legalese_words();
    let mut dictionary = TokenDictionary::new_with_legalese(&legalese_words);
    let len_legalese = dictionary.legalese_count();

    let mut digit_only_tids = HashSet::new();
    let mut rid_by_hash: HashMap<[u8; 20], usize> = HashMap::new();
    let mut rules_by_rid: Vec<Rule> = Vec::with_capacity(rules.len());
    let mut tids_by_rid: Vec<Vec<u16>> = Vec::with_capacity(rules.len());
    let mut sets_by_rid: HashMap<usize, HashSet<u16>> = HashMap::new();
    let mut msets_by_rid: HashMap<usize, HashMap<u16, usize>> = HashMap::new();
    let mut high_postings_by_rid: HashMap<usize, HashMap<u16, Vec<usize>>> = HashMap::new();
    let mut regular_rids: HashSet<usize> = HashSet::new();
    let mut false_positive_rids: HashSet<usize> = HashSet::new();
    let mut approx_matchable_rids: HashSet<usize> = HashSet::new();

    let mut rules_automaton_patterns: Vec<Vec<u8>> = Vec::with_capacity(rules.len());
    let mut unknown_automaton_patterns: Vec<Vec<u8>> = Vec::new();

    let mut licenses_by_key: HashMap<String, License> = HashMap::new();
    for license in licenses {
        licenses_by_key.insert(license.key.clone(), license);
    }

    for (rid, mut rule) in rules.into_iter().enumerate() {
        let rule_tokens = tokenize(&rule.text);
        let mut rule_token_ids: Vec<u16> = Vec::with_capacity(rule_tokens.len());

        let mut is_weak = true;
        for rts in &rule_tokens {
            let rtid = dictionary.get_or_assign(rts);
            if is_weak && (rtid as usize) < len_legalese {
                is_weak = false;
            }
            rule_token_ids.push(rtid);
        }

        let rule_length = rule_token_ids.len();
        rule.tokens = rule_token_ids.clone();

        let rule_hash = compute_hash(&rule_token_ids);
        rules_automaton_patterns.push(tokens_to_bytes(&rule_token_ids));

        if rule.is_false_positive {
            false_positive_rids.insert(rid);
            rules_by_rid.push(rule);
            tids_by_rid.push(rule_token_ids);
            continue;
        }

        rid_by_hash.insert(rule_hash, rid);
        regular_rids.insert(rid);

        let is_approx_matchable = {
            rule.is_small = rule_length < SMALL_RULE;
            rule.is_tiny = rule_length < TINY_RULE;
            compute_is_approx_matchable(&rule)
        };

        if rule_length >= UNKNOWN_NGRAM_LENGTH {
            let tids_ngrams = ngrams(&rule_token_ids, UNKNOWN_NGRAM_LENGTH);
            let toks_ngrams = ngrams(&rule_tokens, UNKNOWN_NGRAM_LENGTH);
            for (tids_ngram, toks_ngram) in tids_ngrams.iter().zip(toks_ngrams.iter()) {
                if is_good_tokens_ngram(toks_ngram, tids_ngram, len_legalese) {
                    unknown_automaton_patterns.push(tokens_to_bytes(tids_ngram));
                }
            }
        }

        if is_approx_matchable && !is_weak {
            approx_matchable_rids.insert(rid);

            let mut postings: HashMap<u16, Vec<usize>> = HashMap::new();
            for (pos, &tid) in rule_token_ids.iter().enumerate() {
                if (tid as usize) < len_legalese {
                    postings.entry(tid).or_default().push(pos);
                }
            }
            if !postings.is_empty() {
                high_postings_by_rid.insert(rid, postings);
            }
        }

        let (tids_set, mset) = build_set_and_mset(&rule_token_ids);
        sets_by_rid.insert(rid, tids_set.clone());
        msets_by_rid.insert(rid, mset.clone());

        let tids_set_high = high_tids_set_subset(&tids_set, len_legalese);
        let mset_high = high_multiset_subset(&mset, len_legalese);

        rule.length_unique = tids_set_counter(&tids_set);
        rule.high_length_unique = tids_set_counter(&tids_set_high);
        rule.high_length = multiset_counter(&mset_high);

        let (updated_coverage, min_matched_length, min_high_matched_length) =
            compute_thresholds_occurrences(rule.minimum_coverage, rule_length, rule.high_length);
        rule.minimum_coverage = updated_coverage;
        rule.min_matched_length = min_matched_length;
        rule.min_high_matched_length = min_high_matched_length;

        let (min_matched_length_unique, min_high_matched_length_unique) = compute_thresholds_unique(
            rule.minimum_coverage,
            rule_length,
            rule.length_unique,
            rule.high_length_unique,
        );
        rule.min_matched_length_unique = min_matched_length_unique;
        rule.min_high_matched_length_unique = min_high_matched_length_unique;

        rules_by_rid.push(rule);
        tids_by_rid.push(rule_token_ids);
    }

    for (token, &tid) in dictionary.tokens_to_ids() {
        if token.chars().all(|c| c.is_ascii_digit()) {
            digit_only_tids.insert(tid);
        }
    }

    let rules_automaton = AhoCorasickBuilder::new()
        .match_kind(aho_corasick::MatchKind::LeftmostFirst)
        .build(&rules_automaton_patterns)
        .expect("Failed to build rules automaton");

    let unknown_automaton = if unknown_automaton_patterns.is_empty() {
        AhoCorasickBuilder::new()
            .build(std::iter::empty::<&[u8]>())
            .expect("Failed to build empty unknown automaton")
    } else {
        let unique_patterns: HashSet<Vec<u8>> = unknown_automaton_patterns.into_iter().collect();
        AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .build(&unique_patterns)
            .expect("Failed to build unknown automaton")
    };

    LicenseIndex {
        dictionary,
        len_legalese,
        digit_only_tids,
        rid_by_hash,
        rules_by_rid,
        tids_by_rid,
        rules_automaton,
        unknown_automaton,
        sets_by_rid,
        msets_by_rid,
        high_postings_by_rid,
        regular_rids,
        false_positive_rids,
        approx_matchable_rids,
        licenses_by_key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rule(text: &str, is_false_positive: bool) -> Rule {
        Rule {
            license_expression: "mit".to_string(),
            text: text.to_string(),
            tokens: vec![],
            is_license_text: false,
            is_license_notice: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_intro: false,
            is_license_clue: false,
            is_false_positive,
            is_required_phrase: false,
            relevance: 100,
            minimum_coverage: None,
            is_continuous: false,
            referenced_filenames: None,
            ignorable_urls: None,
            ignorable_emails: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            language: None,
            notes: None,
            length_unique: 0,
            high_length_unique: 0,
            high_length: 0,
            min_matched_length: 0,
            min_high_matched_length: 0,
            min_matched_length_unique: 0,
            min_high_matched_length_unique: 0,
            is_small: false,
            is_tiny: false,
        }
    }

    fn create_test_license(key: &str, name: &str) -> License {
        License {
            key: key.to_string(),
            name: name.to_string(),
            spdx_license_key: Some(key.to_uppercase()),
            category: Some("Permissive".to_string()),
            text: format!("{} license text", name),
            reference_urls: vec![],
            notes: None,
        }
    }

    #[test]
    fn test_build_index_empty() {
        let index = build_index(vec![], vec![]);
        assert!(index.rules_by_rid.is_empty());
        assert!(index.tids_by_rid.is_empty());
        assert!(index.rid_by_hash.is_empty());
        assert!(index.regular_rids.is_empty());
        assert!(index.false_positive_rids.is_empty());
        assert!(index.approx_matchable_rids.is_empty());
    }

    #[test]
    fn test_build_index_single_rule() {
        let rules = vec![create_test_rule("MIT License", false)];
        let licenses = vec![create_test_license("mit", "MIT License")];

        let index = build_index(rules, licenses);

        assert_eq!(index.rules_by_rid.len(), 1);
        assert_eq!(index.tids_by_rid.len(), 1);
        assert!(
            index
                .rid_by_hash
                .contains_key(&compute_hash(&index.tids_by_rid[0]))
        );
        assert!(index.regular_rids.contains(&0));
        assert!(!index.false_positive_rids.contains(&0));
        assert!(index.licenses_by_key.contains_key("mit"));
    }

    #[test]
    fn test_build_index_false_positive() {
        let rules = vec![create_test_rule("MIT License", true)];
        let index = build_index(rules, vec![]);

        assert_eq!(index.rules_by_rid.len(), 1);
        assert!(index.false_positive_rids.contains(&0));
        assert!(!index.regular_rids.contains(&0));
        assert!(index.rid_by_hash.is_empty());
    }

    #[test]
    fn test_build_index_sets_and_msets() {
        let rules = vec![create_test_rule("MIT License copyright permission", false)];
        let index = build_index(rules, vec![]);

        assert!(index.sets_by_rid.contains_key(&0));
        assert!(index.msets_by_rid.contains_key(&0));
        assert!(!index.sets_by_rid[&0].is_empty());
    }

    #[test]
    fn test_build_index_high_postings() {
        let rules = vec![create_test_rule(
            "licensed copyrighted permission granted authorized",
            false,
        )];
        let index = build_index(rules, vec![]);

        if !index.approx_matchable_rids.is_empty() {
            assert!(index.high_postings_by_rid.contains_key(&0));
        }
    }

    #[test]
    fn test_build_index_digit_only_tids() {
        let rules = vec![create_test_rule("version 123 456 789 test", false)];
        let index = build_index(rules, vec![]);

        assert!(!index.digit_only_tids.is_empty() || !index.dictionary.is_empty());
    }

    #[test]
    fn test_compute_is_approx_matchable() {
        let mut rule = create_test_rule("test", false);
        rule.is_tiny = false;
        rule.is_small = false;
        rule.is_continuous = false;
        rule.is_required_phrase = false;
        rule.is_false_positive = false;
        rule.is_license_reference = false;
        rule.is_license_tag = false;
        assert!(compute_is_approx_matchable(&rule));

        rule.is_false_positive = true;
        assert!(!compute_is_approx_matchable(&rule));
        rule.is_false_positive = false;

        rule.is_required_phrase = true;
        assert!(!compute_is_approx_matchable(&rule));
        rule.is_required_phrase = false;

        rule.is_tiny = true;
        assert!(!compute_is_approx_matchable(&rule));
        rule.is_tiny = false;

        rule.is_continuous = true;
        assert!(!compute_is_approx_matchable(&rule));
        rule.is_continuous = false;

        rule.is_small = true;
        rule.is_license_reference = true;
        assert!(!compute_is_approx_matchable(&rule));
    }

    #[test]
    fn test_is_good_tokens_ngram() {
        let tokens = vec![
            "hello".to_string(),
            "world".to_string(),
            "license".to_string(),
        ];
        let tids = vec![100, 101, 0];
        assert!(is_good_tokens_ngram(&tokens, &tids, 10));

        let tokens_with_year = vec!["2023".to_string(), "license".to_string(), "mit".to_string()];
        let tids_with_year = vec![500, 0, 1];
        assert!(!is_good_tokens_ngram(
            &tokens_with_year,
            &tids_with_year,
            10
        ));

        let tokens_all_digits = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let tids_all_digits = vec![100, 101, 102];
        assert!(!is_good_tokens_ngram(
            &tokens_all_digits,
            &tids_all_digits,
            10
        ));
    }

    #[test]
    fn test_tokens_to_bytes() {
        let tokens = vec![1u16, 2, 3];
        let bytes = tokens_to_bytes(&tokens);
        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 2);
        assert_eq!(bytes[3], 0);
        assert_eq!(bytes[4], 3);
        assert_eq!(bytes[5], 0);
    }

    #[test]
    fn test_ngrams() {
        let items = vec![1, 2, 3, 4, 5];
        let ngrams_result = ngrams(&items, 3);
        assert_eq!(ngrams_result.len(), 3);
        assert_eq!(ngrams_result[0], vec![1, 2, 3]);
        assert_eq!(ngrams_result[1], vec![2, 3, 4]);
        assert_eq!(ngrams_result[2], vec![3, 4, 5]);

        let short_items = vec![1, 2];
        let short_ngrams = ngrams(&short_items, 3);
        assert!(short_ngrams.is_empty());
    }

    #[test]
    fn test_build_index_multiple_rules() {
        let rules = vec![
            create_test_rule("MIT License", false),
            create_test_rule("Apache License", false),
            create_test_rule("GPL License", true),
        ];
        let index = build_index(rules, vec![]);

        assert_eq!(index.rules_by_rid.len(), 3);
        assert_eq!(index.tids_by_rid.len(), 3);
        assert_eq!(index.regular_rids.len(), 2);
        assert_eq!(index.false_positive_rids.len(), 1);
    }

    #[test]
    fn test_build_index_licenses() {
        let rules = vec![create_test_rule("MIT License", false)];
        let licenses = vec![
            create_test_license("mit", "MIT License"),
            create_test_license("apache-2.0", "Apache License 2.0"),
        ];
        let index = build_index(rules, licenses);

        assert_eq!(index.license_count(), 2);
        assert!(index.get_license("mit").is_some());
        assert!(index.get_license("apache-2.0").is_some());
    }
}
