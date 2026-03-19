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
use crate::license_detection::index::dictionary::{
    KnownToken, TokenDictionary, TokenId, TokenKind,
};
use crate::license_detection::index::token_sets::{
    build_set_and_mset, high_multiset_subset, high_tids_set_subset, multiset_counter,
    tids_set_counter,
};
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::{License, Rule};
use crate::license_detection::rules::legalese;
use crate::license_detection::rules::thresholds::{
    compute_thresholds_occurrences, compute_thresholds_unique, SMALL_RULE, TINY_RULE,
};
use crate::license_detection::tokenize::{
    parse_required_phrase_spans, tokenize, tokenize_with_stopwords,
};

const UNKNOWN_NGRAM_LENGTH: usize = 6;
const LICENSE_TOKEN_STRINGS: &[&str] = &["license", "licence", "licensed"];

const DEPRECATED_SPDX_SUBS: &[(&str, &str)] = &[
    ("ecos-2.0", "gpl-2.0-or-later with ecos-exception-2.0"),
    (
        "gpl-2.0-with-autoconf-exception",
        "gpl-2.0-only with autoconf-exception-2.0",
    ),
    (
        "gpl-2.0-with-bison-exception",
        "gpl-2.0-only with bison-exception-2.2",
    ),
    (
        "gpl-2.0-with-classpath-exception",
        "gpl-2.0-only with classpath-exception-2.0",
    ),
    (
        "gpl-2.0-with-font-exception",
        "gpl-2.0-only with font-exception-2.0",
    ),
    (
        "gpl-2.0-with-gcc-exception",
        "gpl-2.0-only with gcc-exception-2.0",
    ),
    (
        "gpl-3.0-with-autoconf-exception",
        "gpl-3.0-only with autoconf-exception-3.0",
    ),
    (
        "gpl-3.0-with-gcc-exception",
        "gpl-3.0-only with gcc-exception-3.1",
    ),
    (
        "wxwindows",
        "lgpl-2.0-or-later with wxwindows-exception-3.1",
    ),
];

fn add_deprecated_spdx_aliases(rid_by_spdx_key: &mut HashMap<String, usize>) {
    for (deprecated, replacement) in DEPRECATED_SPDX_SUBS {
        if let Some(&rid) = rid_by_spdx_key.get(*replacement) {
            rid_by_spdx_key.insert(deprecated.to_string(), rid);
        }
    }
}

fn prepare_rule_text(text: &str) -> String {
    text.lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
pub fn generate_url_variants(text: &str, ignorable_urls: &Option<Vec<String>>) -> Vec<String> {
    let Some(urls) = ignorable_urls else {
        return vec![];
    };
    if urls.is_empty() {
        return vec![];
    }

    let mut variants = Vec::new();
    let current = text.to_string();

    for url in urls {
        let url_lower = url.to_lowercase();
        if url_lower.starts_with("https://") {
            let http_url = format!("http://{}", &url[8..]);
            if current.contains(url) {
                let variant = current.replace(url, &http_url);
                variants.push(variant);
            }
        } else if url_lower.starts_with("http://") {
            let https_url = format!("https://{}", &url[7..]);
            if current.contains(url) {
                let variant = current.replace(url, &https_url);
                variants.push(variant);
            }
        }
    }

    variants
}

fn build_rule_from_license(license: &License) -> Option<Rule> {
    let has_stored_minimum_coverage = license.minimum_coverage.is_some();

    let text = if license.text.is_empty() {
        "unknown-spdx license identifier".to_string()
    } else {
        prepare_rule_text(&license.text)
    };

    Some(Rule {
        identifier: format!("{}.LICENSE", license.key),
        license_expression: license.key.clone(),
        text,
        tokens: vec![],
        is_license_text: true,
        is_license_notice: false,
        is_license_reference: false,
        is_license_tag: false,
        is_license_intro: false,
        is_license_clue: false,
        is_false_positive: false,
        is_required_phrase: false,
        is_from_license: true,
        relevance: 100,
        minimum_coverage: license.minimum_coverage,
        has_stored_minimum_coverage,
        is_continuous: false,
        required_phrase_spans: vec![],
        stopwords_by_pos: HashMap::new(),
        referenced_filenames: None,
        ignorable_urls: license.ignorable_urls.clone(),
        ignorable_emails: license.ignorable_emails.clone(),
        ignorable_copyrights: license.ignorable_copyrights.clone(),
        ignorable_holders: license.ignorable_holders.clone(),
        ignorable_authors: license.ignorable_authors.clone(),
        language: None,
        notes: license.notes.clone(),
        length_unique: 0,
        high_length_unique: 0,
        high_length: 0,
        min_matched_length: 0,
        min_high_matched_length: 0,
        min_matched_length_unique: 0,
        min_high_matched_length_unique: 0,
        is_small: false,
        is_tiny: false,
        starts_with_license: false,
        ends_with_license: false,
        is_deprecated: license.is_deprecated,
        spdx_license_key: license.spdx_license_key.clone(),
        other_spdx_license_keys: license.other_spdx_license_keys.clone(),
    })
}

fn build_rules_from_licenses(licenses: &[License]) -> Vec<Rule> {
    licenses
        .iter()
        .filter_map(build_rule_from_license)
        .collect()
}

fn get_essential_spdx_tokens() -> &'static [&'static str] {
    &["spdx", "license", "licence", "identifier", "licenseref"]
}

fn collect_spdx_tokens(licenses: &[License]) -> HashSet<String> {
    let mut tokens: HashSet<String> = HashSet::new();
    for &tok in get_essential_spdx_tokens() {
        tokens.insert(tok.to_string());
    }
    for license in licenses {
        if let Some(ref spdx_key) = license.spdx_license_key {
            for token in tokenize(spdx_key) {
                tokens.insert(token);
            }
        }
        for spdx_key in &license.other_spdx_license_keys {
            for token in tokenize(spdx_key) {
                tokens.insert(token);
            }
        }
    }
    tokens
}

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

pub fn is_good_tokens_ngram(tokens_ngram: &[String], known_tokens_ngram: &[KnownToken]) -> bool {
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

    let unique_tids: HashSet<TokenId> = known_tokens_ngram.iter().map(|token| token.id).collect();
    if unique_tids.len() <= 2 {
        return false;
    }

    let has_high_token = known_tokens_ngram
        .iter()
        .any(|token| token.kind == TokenKind::Legalese);
    if !has_high_token {
        return false;
    }

    let has_marker = tokens_ngram.iter().any(|t| MARKERS.contains(&t.as_str()));
    if has_marker {
        return false;
    }

    true
}

pub fn compute_is_approx_matchable(rule: &Rule) -> bool {
    !(rule.is_false_positive
        || rule.is_required_phrase
        || rule.is_tiny
        || rule.is_continuous
        || (rule.is_small && (rule.is_license_reference || rule.is_license_tag)))
}

pub fn tokens_to_bytes(tokens: &[TokenId]) -> Vec<u8> {
    tokens.iter().flat_map(|t| t.to_le_bytes()).collect()
}

pub fn ngrams<T: Clone>(items: &[T], ngram_length: usize) -> Vec<Vec<T>> {
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

    // Pre-assign SPDX tokens before processing rules (Python: index.py:301-314)
    // This ensures SPDX tokens get consistent IDs matching Python
    {
        let spdx_tokens = collect_spdx_tokens(&licenses);
        let mut sorted_tokens: Vec<&String> = spdx_tokens.iter().collect();
        sorted_tokens.sort();
        for token in sorted_tokens {
            if dictionary.lookup(token).is_none() {
                let _ = dictionary.intern(token);
            }
        }
    }

    let license_token_ids: HashSet<TokenId> = LICENSE_TOKEN_STRINGS
        .iter()
        .filter_map(|&token| dictionary.lookup(token).map(|token| token.id))
        .collect();

    let mut rid_by_hash: HashMap<[u8; 20], usize> = HashMap::new();
    let mut rules_by_rid: Vec<Rule> = Vec::with_capacity(rules.len());
    let mut tids_by_rid: Vec<Vec<TokenId>> = Vec::with_capacity(rules.len());
    let mut sets_by_rid: HashMap<usize, HashSet<TokenId>> = HashMap::new();
    let mut msets_by_rid: HashMap<usize, HashMap<TokenId, usize>> = HashMap::new();
    let mut high_postings_by_rid: HashMap<usize, HashMap<TokenId, Vec<usize>>> = HashMap::new();
    let mut false_positive_rids: HashSet<usize> = HashSet::new();
    let mut approx_matchable_rids: HashSet<usize> = HashSet::new();

    let mut rules_automaton_patterns: Vec<Vec<u8>> = Vec::with_capacity(rules.len());
    let mut pattern_id_to_rid: Vec<usize> = Vec::with_capacity(rules.len());
    let mut unknown_automaton_patterns: Vec<Vec<u8>> = Vec::new();

    let mut licenses_by_key: HashMap<String, License> = HashMap::new();
    for license in licenses {
        licenses_by_key.insert(license.key.clone(), license);
    }

    let license_rules =
        build_rules_from_licenses(&licenses_by_key.values().cloned().collect::<Vec<_>>());

    let mut all_rules: Vec<Rule> = license_rules.into_iter().chain(rules).collect();
    all_rules.sort();

    let mut rid_by_spdx_key: HashMap<String, usize> = HashMap::new();
    let mut unknown_spdx_rid: Option<usize> = None;

    for (rid, mut rule) in all_rules.into_iter().enumerate() {
        rule.required_phrase_spans = parse_required_phrase_spans(&rule.text);
        let (rule_tokens, stopwords_by_pos) = tokenize_with_stopwords(&rule.text);
        rule.stopwords_by_pos = stopwords_by_pos;
        let mut known_rule_tokens: Vec<KnownToken> = Vec::with_capacity(rule_tokens.len());
        let mut rule_token_ids: Vec<TokenId> = Vec::with_capacity(rule_tokens.len());

        let mut is_weak = true;
        for rts in &rule_tokens {
            let known_token = dictionary.intern(rts);
            if is_weak && known_token.kind == TokenKind::Legalese {
                is_weak = false;
            }
            known_rule_tokens.push(known_token);
            rule_token_ids.push(known_token.id);
        }

        let rule_length = rule_token_ids.len();
        rule.tokens = rule_token_ids.clone();

        rule.starts_with_license = rule_token_ids
            .first()
            .map(|&tid| license_token_ids.contains(&tid))
            .unwrap_or(false);
        rule.ends_with_license = rule_token_ids
            .last()
            .map(|&tid| license_token_ids.contains(&tid))
            .unwrap_or(false);

        let rule_hash = compute_hash(&rule_token_ids);

        // Only add non-empty patterns to the automaton
        // Empty patterns (from non-ASCII text like Japanese) would match everywhere
        if !rule_token_ids.is_empty() {
            rules_automaton_patterns.push(tokens_to_bytes(&rule_token_ids));
            pattern_id_to_rid.push(rid);
        }

        if rule.is_false_positive {
            false_positive_rids.insert(rid);
            rules_by_rid.push(rule);
            tids_by_rid.push(rule_token_ids);
            continue;
        }

        rid_by_hash.insert(rule_hash, rid);

        // Match Python indexing order: approx-matchable membership is decided
        // before compute_thresholds() later derives final is_small/is_tiny flags.
        let is_approx_matchable = compute_is_approx_matchable(&rule);

        if rule_length >= UNKNOWN_NGRAM_LENGTH {
            let known_ngrams = ngrams(&known_rule_tokens, UNKNOWN_NGRAM_LENGTH);
            let toks_ngrams = ngrams(&rule_tokens, UNKNOWN_NGRAM_LENGTH);
            for (known_ngram, toks_ngram) in known_ngrams.iter().zip(toks_ngrams.iter()) {
                if is_good_tokens_ngram(toks_ngram, known_ngram) {
                    let token_ids: Vec<TokenId> =
                        known_ngram.iter().map(|token| token.id).collect();
                    unknown_automaton_patterns.push(tokens_to_bytes(&token_ids));
                }
            }
        }

        if is_approx_matchable && !is_weak {
            approx_matchable_rids.insert(rid);

            let mut postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
            for (pos, token) in known_rule_tokens.iter().enumerate() {
                if token.kind == TokenKind::Legalese {
                    postings.entry(token.id).or_default().push(pos);
                }
            }
            if !postings.is_empty() {
                high_postings_by_rid.insert(rid, postings);
            }
        }

        let (tids_set, mset) = build_set_and_mset(&rule_token_ids);

        sets_by_rid.insert(rid, tids_set.clone());
        msets_by_rid.insert(rid, mset.clone());

        let tids_set_high = high_tids_set_subset(&tids_set, &dictionary);
        let mset_high = high_multiset_subset(&mset, &dictionary);

        rule.length_unique = tids_set_counter(&tids_set);
        rule.high_length_unique = tids_set_counter(&tids_set_high);
        rule.high_length = multiset_counter(&mset_high);

        let (updated_coverage, min_matched_length, min_high_matched_length) =
            compute_thresholds_occurrences(rule.minimum_coverage, rule_length, rule.high_length);
        if !rule.has_stored_minimum_coverage {
            rule.minimum_coverage = updated_coverage;
        }
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
        rule.is_small = rule_length < SMALL_RULE;
        rule.is_tiny = rule_length < TINY_RULE;

        if let Some(ref spdx_key) = rule.spdx_license_key {
            rid_by_spdx_key.insert(spdx_key.to_lowercase(), rid);
        }
        for alias in &rule.other_spdx_license_keys {
            rid_by_spdx_key.insert(alias.to_lowercase(), rid);
        }

        if rule.license_expression == "unknown-spdx" {
            unknown_spdx_rid = Some(rid);
        }

        rules_by_rid.push(rule);
        tids_by_rid.push(rule_token_ids);
    }

    add_deprecated_spdx_aliases(&mut rid_by_spdx_key);

    let rules_automaton = AhoCorasickBuilder::new()
        .match_kind(aho_corasick::MatchKind::Standard)
        .build(&rules_automaton_patterns)
        .expect("Failed to build rules automaton");

    let unknown_automaton = if unknown_automaton_patterns.is_empty() {
        AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::Standard)
            .build(std::iter::empty::<&[u8]>())
            .expect("Failed to build empty unknown automaton")
    } else {
        let unique_patterns: HashSet<Vec<u8>> = unknown_automaton_patterns.into_iter().collect();
        AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::Standard)
            .build(&unique_patterns)
            .expect("Failed to build unknown automaton")
    };

    LicenseIndex {
        dictionary,
        len_legalese,
        rid_by_hash,
        rules_by_rid,
        tids_by_rid,
        rules_automaton,
        unknown_automaton,
        sets_by_rid,
        msets_by_rid,
        high_postings_by_rid,
        false_positive_rids,
        approx_matchable_rids,
        licenses_by_key,
        pattern_id_to_rid,
        rid_by_spdx_key,
        unknown_spdx_rid,
    }
}

#[cfg(test)]
mod tests;
