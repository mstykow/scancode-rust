//! Token ID equivalence test between Python and Rust.
//!
//! This test verifies that token IDs are assigned consistently between
//! Python ScanCode Toolkit and Rust scancode-rust.
//!
//! Run with: cargo test token_id_equivalence --lib -- --nocapture

#[cfg(test)]
mod tests {
    use crate::license_detection::index::dictionary::TokenDictionary;

    #[test]
    fn test_hash_index_population() {
        use crate::license_detection::index::build_index;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use std::path::PathBuf;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let rules = load_rules_from_directory(&rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(&licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        eprintln!("=== Hash Index Population ===");
        eprintln!("Total rules: {}", index.rules_by_rid.len());
        eprintln!("Rules with hashes: {}", index.rid_by_hash.len());
        eprintln!(
            "Approx matchable rules: {}",
            index.approx_matchable_rids.len()
        );
        eprintln!("Regular rules: {}", index.regular_rids.len());
        eprintln!("False positive rules: {}", index.false_positive_rids.len());

        for (i, rule) in index.rules_by_rid.iter().enumerate().take(10) {
            let has_hash = index.rid_by_hash.values().any(|&rid| rid == i);
            eprintln!(
                "Rule {}: {} - has_hash={}, is_license_text={}",
                i, rule.identifier, has_hash, rule.is_license_text
            );
        }

        let mit_rule = index
            .rules_by_rid
            .iter()
            .position(|r| r.license_expression == "mit" && r.is_license_text);
        if let Some(rid) = mit_rule {
            let rule = &index.rules_by_rid[rid];
            let has_hash = index.rid_by_hash.values().any(|&r| r == rid);
            eprintln!("\nMIT license text rule (rid={}):", rid);
            eprintln!("  identifier: {}", rule.identifier);
            eprintln!("  has_hash: {}", has_hash);
            eprintln!("  text length: {} chars", rule.text.len());
        }

        assert!(
            index.rid_by_hash.len() > 100,
            "Should have at least 100 hashed rules, got {}",
            index.rid_by_hash.len()
        );
    }

    #[test]
    fn test_legalese_count_matches_python() {
        let legalese_words = crate::license_detection::rules::legalese::get_legalese_words();

        eprintln!("Legalese word count: {}", legalese_words.len());

        let unique_ids: std::collections::HashSet<u16> =
            legalese_words.iter().map(|(_, id)| *id).collect();

        eprintln!("Unique token IDs in legalese: {}", unique_ids.len());

        let max_id = unique_ids.iter().max().copied().unwrap_or(0);
        eprintln!("Max legalese token ID: {}", max_id);

        assert_eq!(
            legalese_words.len(),
            4506,
            "Python has 4506 legalese entries"
        );
        assert_eq!(
            unique_ids.len(),
            4356,
            "Python has 4356 unique token IDs in legalese"
        );
    }

    #[test]
    fn test_specific_legalese_token_ids() {
        let legalese_words = crate::license_detection::rules::legalese::get_legalese_words();
        let dict: std::collections::HashMap<&str, u16> = legalese_words
            .iter()
            .map(|(word, id)| (*word, *id))
            .collect();

        assert_eq!(dict.get("3orgplv2"), Some(&0), "First token should be ID 0");
        assert_eq!(dict.get("4suite"), Some(&1), "Second token should be ID 1");
        assert_eq!(dict.get("abandon"), Some(&2), "Third token should be ID 2");
        assert_eq!(
            dict.get("abandons"),
            Some(&2),
            "abandons should also map to ID 2"
        );
        assert_eq!(dict.get("abandoned"), Some(&3), "abandoned should be ID 3");
        assert_eq!(dict.get("apache"), Some(&244), "apache should be ID 244");
        assert_eq!(dict.get("gpl"), Some(&1864), "gpl should be ID 1864");
        assert_eq!(
            dict.get("license"),
            Some(&2432),
            "license IS in legalese (common license word)"
        );
        assert_eq!(dict.get("the"), None, "the is NOT in legalese (too common)");
    }

    #[test]
    fn test_dictionary_initialization() {
        let legalese_words = crate::license_detection::rules::legalese::get_legalese_words();
        let dict = TokenDictionary::new_with_legalese(&legalese_words);

        assert_eq!(
            dict.legalese_count(),
            4506,
            "Dictionary legalese count should match legalese entries"
        );

        assert_eq!(dict.get("3orgplv2"), Some(0));
        assert_eq!(dict.get("4suite"), Some(1));
        assert_eq!(dict.get("abandon"), Some(2));
        assert_eq!(dict.get("apache"), Some(244));
        assert_eq!(dict.get("gpl"), Some(1864));
    }

    #[test]
    fn test_non_legalese_token_assignment() {
        let legalese_words = crate::license_detection::rules::legalese::get_legalese_words();
        let mut dict = TokenDictionary::new_with_legalese(&legalese_words);

        let initial_count = dict.legalese_count();
        eprintln!("Initial legalese count: {}", initial_count);

        let id1 = dict.get_or_assign("new_token_1");
        let id2 = dict.get_or_assign("new_token_2");
        let id3 = dict.get_or_assign("new_token_1");

        eprintln!("new_token_1 assigned ID: {}", id1);
        eprintln!("new_token_2 assigned ID: {}", id2);
        eprintln!("new_token_1 again: {}", id3);

        assert!(
            id1 >= initial_count as u16,
            "New token should get ID >= legalese_count"
        );
        assert_eq!(id2, id1 + 1, "Second new token should be ID+1");
        assert_eq!(id3, id1, "Same token should get same ID");

        assert!(!dict.is_legalese_token(id1), "New token is not legalese");
        assert!(!dict.is_legalese_token(id2), "New token is not legalese");
    }

    #[test]
    fn test_hash_computation_matches_python() {
        use crate::license_detection::hash_match::compute_hash;

        let tokens: Vec<u16> = vec![1, 2, 3, 4, 5];
        let hash = compute_hash(&tokens);
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        eprintln!("Hash of [1,2,3,4,5]: {}", hash_hex);

        assert_eq!(
            hash_hex, "aaa562e5641b932d5d5ecae43b47793b33b3b5f0",
            "Hash should match Python implementation"
        );
    }

    #[test]
    fn test_hash_of_empty_tokens() {
        use crate::license_detection::hash_match::compute_hash;

        let tokens: Vec<u16> = vec![];
        let hash = compute_hash(&tokens);
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        eprintln!("Hash of []: {}", hash_hex);

        assert_eq!(hash_hex.len(), 40, "SHA1 hash should be 40 hex chars");
    }

    #[test]
    fn test_legalese_vs_stopwords_distinction() {
        let legalese_words = crate::license_detection::rules::legalese::get_legalese_words();
        let legalese_set: std::collections::HashSet<&str> =
            legalese_words.iter().map(|(word, _)| *word).collect();

        let common_stopwords = [
            "div", "p", "a", "br", "img", "span", "table", "td", "tr", "th",
        ];
        let stopwords_set: std::collections::HashSet<&str> =
            common_stopwords.iter().copied().collect();

        let mut overlap: Vec<&str> = legalese_set.intersection(&stopwords_set).copied().collect();
        overlap.sort();

        if !overlap.is_empty() {
            eprintln!(
                "WARNING: {} words in both legalese and common stopwords:",
                overlap.len()
            );
            for word in &overlap {
                eprintln!("  - {}", word);
            }
        }

        assert!(
            overlap.is_empty(),
            "Legalese and common stopwords should not overlap, but found: {:?}",
            overlap
        );
    }

    #[test]
    fn test_tokenization_produces_same_tokens_as_python() {
        use crate::license_detection::tokenize::tokenize;

        let test_cases = vec![
            ("MIT License", vec!["mit", "license"]),
            ("Apache License 2.0", vec!["apache", "license", "2", "0"]),
            (
                "Copyright (c) 2023 Author",
                vec!["copyright", "c", "2023", "author"],
            ),
            (
                "Licensed under the MIT license",
                vec!["licensed", "under", "the", "mit", "license"],
            ),
            ("GPL-2.0-or-later", vec!["gpl", "2", "0", "or", "later"]),
            (
                "See the LICENSE file for details",
                vec!["see", "the", "license", "file", "for", "details"],
            ),
        ];

        for (input, expected) in test_cases {
            let tokens = tokenize(input);
            eprintln!("Input: {:?}", input);
            eprintln!("  Tokens: {:?}", tokens);
            eprintln!("  Expected: {:?}", expected);
            assert_eq!(tokens, expected, "Tokenization mismatch for: {:?}", input);
        }
    }

    #[test]
    fn test_critical_token_ids_for_hash_match() {
        let legalese_words = crate::license_detection::rules::legalese::get_legalese_words();
        let dict: std::collections::HashMap<&str, u16> = legalese_words
            .iter()
            .map(|(word, id)| (*word, *id))
            .collect();

        eprintln!("=== Critical token IDs for common license words ===");
        for word in &[
            "license",
            "copyright",
            "permission",
            "without",
            "warranty",
            "conditions",
            "redistribution",
            "derived",
            "works",
            "source",
            "code",
            "distribution",
            "modify",
            "modifications",
            "notice",
            "included",
            "provided",
            "above",
            "following",
            "disclaimer",
            "warranties",
            "merchantability",
            "fitness",
            "particular",
            "purpose",
            "infringement",
            "liability",
            "damages",
            "arising",
            "use",
            "data",
            "business",
            "interruption",
            "theory",
            "contract",
            "tort",
            "otherwise",
            "advised",
            "possibility",
        ] {
            if let Some(&id) = dict.get(word) {
                eprintln!("  {} -> {}", word, id);
            } else {
                eprintln!("  {} -> NOT IN LEGALESE", word);
            }
        }

        assert!(
            dict.contains_key("license"),
            "license IS in legalese (common license word, ID 2432)"
        );
    }

    #[test]
    fn test_a2_c_regression_debug() {
        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use std::path::PathBuf;
        use std::sync::Arc;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let test_file = PathBuf::from("testdata/license-golden/datadriven/lic2/a2.c");
        let text = match std::fs::read_to_string(&test_file) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Skipping test: cannot read test file: {}", e);
                return;
            }
        };

        let rules = load_rules_from_directory(&rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(&licenses_path, false).expect("Failed to load licenses");
        let index = Arc::new(build_index(rules, licenses));

        eprintln!("=== a2.c Debug ===");
        eprintln!("File length: {} bytes", text.len());

        let query = Query::new(&text, &index).expect("Failed to create query");
        let whole_run = query.whole_query_run();

        eprintln!("Query tokens: {}", whole_run.tokens().len());
        eprintln!(
            "Query run: start={}, end={:?}",
            whole_run.start, whole_run.end
        );

        let hash = crate::license_detection::hash_match::compute_hash(whole_run.tokens());
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        eprintln!("Query hash: {}", hash_hex);

        if let Some(&rid) = index.rid_by_hash.get(&hash) {
            let rule = &index.rules_by_rid[rid];
            eprintln!("HASH MATCH FOUND!");
            eprintln!("  Rule ID: {}", rid);
            eprintln!("  License expression: {}", rule.license_expression);
            eprintln!("  is_license_text: {}", rule.is_license_text);
        } else {
            eprintln!("No hash match - this is expected for multi-license file");
        }

        let detections = crate::license_detection::LicenseDetectionEngine {
            index: index.clone(),
            spdx_mapping: crate::license_detection::spdx_mapping::build_spdx_mapping(
                &index.licenses_by_key.values().cloned().collect::<Vec<_>>(),
            ),
        }
        .detect(&text, false)
        .expect("Detection failed");

        eprintln!("\nDetections:");
        for d in &detections {
            eprintln!("  detection license_expression: {:?}", d.license_expression);
            eprintln!("  Individual matches:");
            for m in &d.matches {
                eprintln!(
                    "    - {} (matcher: {}, lines {}-{}, score: {:.2})",
                    m.license_expression, m.matcher, m.start_line, m.end_line, m.score
                );
            }
        }

        let expected = vec!["gpl-2.0-plus", "bsd-top-gpl-addition"];
        let individual_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nExpected (individual match expressions): {:?}", expected);
        eprintln!(
            "Actual (individual match expressions): {:?}",
            individual_expressions
        );

        assert!(
            individual_expressions.iter().any(|e| e.contains("gpl")),
            "Should detect GPL in individual matches"
        );
        assert!(
            individual_expressions.iter().any(|e| e.contains("bsd")),
            "Should detect BSD in individual matches"
        );
    }
}
