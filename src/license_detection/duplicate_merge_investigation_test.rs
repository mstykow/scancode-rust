//! Investigation tests for PLAN-058: Duplicate License Detections Merged
//!
//! This module traces through the license detection pipeline to find where
//! two separate matches for the same license expression are incorrectly merged.

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use once_cell::sync::Lazy;
    use std::path::PathBuf;
    use std::sync::Once;

    static TEST_ENGINE: Lazy<Option<LicenseDetectionEngine>> = Lazy::new(|| {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            eprintln!("Reference data not available at {:?}", data_path);
            return None;
        }
        match LicenseDetectionEngine::new(&data_path) {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("Failed to create engine: {:?}", e);
                None
            }
        }
    });

    static INIT: Once = Once::new();

    fn ensure_engine() -> Option<&'static LicenseDetectionEngine> {
        INIT.call_once(|| {
            let _ = &*TEST_ENGINE;
        });
        TEST_ENGINE.as_ref()
    }

    fn print_match_details(m: &crate::license_detection::models::LicenseMatch, prefix: &str) {
        eprintln!(
            "{}: expr={}, start_token={}, end_token={}, start_line={}, end_line={}, qspan_positions={:?}",
            prefix,
            m.license_expression,
            m.start_token,
            m.end_token,
            m.start_line,
            m.end_line,
            m.qspan_positions
        );
        eprintln!(
            "{}:   rule_identifier={}, matcher={}, matched_length={}, hilen={}",
            prefix, m.rule_identifier, m.matcher, m.matched_length, m.hilen
        );
    }

    #[test]
    fn test_bzip2_106_c_full_pipeline() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content =
            include_str!("../../testdata/license-golden/datadriven/lic2/1908-bzip2/bzip2.106.c");

        eprintln!("\n=== FULL DETECTION PIPELINE for bzip2.106.c ===");
        let detections = engine.detect(content).expect("Detection should succeed");

        eprintln!("Number of detections: {}", detections.len());

        let mut all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();
        all_matches.sort_by_key(|m| m.start_line);

        for (i, m) in all_matches.iter().enumerate() {
            print_match_details(m, &format!("Match[{}]", i));
        }

        let expressions: Vec<&str> = all_matches
            .iter()
            .map(|m| m.license_expression.as_str())
            .collect();
        eprintln!("Final expressions: {:?}", expressions);

        // Document current behavior (1 match instead of expected 2)
        eprintln!("\nEXPECTED: 2 matches (Python produces matches at lines 7-17 and 27-34)");
        eprintln!("ACTUAL: {} matches", all_matches.len());
    }

    #[test]
    fn test_bzip2_106_c_aho_matches_directly() {
        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::query::Query;

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content =
            include_str!("../../testdata/license-golden/datadriven/lic2/1908-bzip2/bzip2.106.c");
        let index = engine.index();

        let query = Query::new(content, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== DIRECT AHO MATCH for bzip2.106.c ===");
        eprintln!("Query tokens: {} tokens", query.tokens.len());
        eprintln!(
            "Query run: start={}, end={:?}",
            whole_run.start, whole_run.end
        );

        // First check which bzip2 rules are loaded
        eprintln!("\n=== BZIP2 RULES IN INDEX ===");
        let bzip2_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("bzip2"))
            .collect();
        eprintln!("Found {} bzip2 rules", bzip2_rules.len());
        for rule in bzip2_rules.iter().take(5) {
            eprintln!(
                "  Rule: {}, tokens: {}, text preview: {:?}",
                rule.identifier,
                rule.tokens.len(),
                &rule.text.chars().take(80).collect::<String>()
            );
        }

        // Check the second rule more carefully
        eprintln!("\n=== CHECKING SECOND BZIP2 RULE ===");
        let rule_21 = bzip2_rules
            .iter()
            .find(|r| r.identifier == "bzip2-libbzip-2010_21.RULE");
        if let Some(rule) = rule_21 {
            eprintln!("Found rule: {}", rule.identifier);
            eprintln!(
                "  tokens (first 10): {:?}",
                &rule.tokens[..10.min(rule.tokens.len())]
            );
            eprintln!("  text: {:?}", rule.text);

            // Check if tokens appear in query
            eprintln!("\n  Looking for token sequence in query...");
            let first_token = rule.tokens[0];
            let positions: Vec<_> = query
                .tokens
                .iter()
                .enumerate()
                .filter(|(_, t)| **t == first_token)
                .map(|(i, _)| i)
                .collect();
            eprintln!(
                "  First token {} found at positions: {:?}",
                first_token, positions
            );

            // Show query tokens around those positions
            for pos in &positions {
                let start = pos.saturating_sub(2);
                let end = (*pos + 10).min(query.tokens.len());
                eprintln!(
                    "  Query tokens at {}..{}: {:?}",
                    start,
                    end,
                    &query.tokens[start..end]
                );
            }

            // Show lines in the content
            eprintln!("\n  Content lines at positions 84-96:");
            for i in 84..96.min(query.tokens.len()) {
                let line = query.line_by_pos.get(i).copied().unwrap_or(0);
                eprintln!("    token[{}] = {} (line {})", i, query.tokens[i], line);
            }

            // Show full rule tokens
            eprintln!("\n  Rule 21 has {} tokens", rule.tokens.len());

            // Find what token 8579 represents
            eprintln!(
                "\n  Investigating token 8579 (appears in query where rule expects different tokens)..."
            );

            // Show text content at lines 27-34
            eprintln!("\n  File content at lines 27-34:");
            for (line_num, line) in content.lines().enumerate() {
                if (26..35).contains(&line_num) {
                    eprintln!("    Line {}: {:?}", line_num + 1, line);
                }
            }

            // Check the full match at position 84
            if positions.contains(&84) {
                let pos = 84;
                let end = (pos + rule.tokens.len()).min(query.tokens.len());
                eprintln!(
                    "\n  Checking full sequence at position {} ({} tokens):",
                    pos,
                    end - pos
                );
                for i in 0..(end - pos) {
                    let q_tok = query.tokens[pos + i];
                    let r_tok = rule.tokens[i];
                    let match_str = if q_tok == r_tok { "MATCH" } else { "DIFF" };
                    let line = query.line_by_pos.get(pos + i).copied().unwrap_or(0);
                    eprintln!(
                        "    [{}] query[{}]={} rule[{}]={} line={} {}",
                        i,
                        pos + i,
                        q_tok,
                        i,
                        r_tok,
                        line,
                        match_str
                    );
                }
            }

            // Check if full sequence matches at any position
            for pos in &positions {
                let query_slice =
                    &query.tokens[*pos..(*pos + rule.tokens.len()).min(query.tokens.len())];
                if query_slice.len() == rule.tokens.len() {
                    let matches = query_slice
                        .iter()
                        .zip(rule.tokens.iter())
                        .all(|(a, b)| a == b);
                    if matches {
                        eprintln!("  FULL MATCH at position {}", pos);
                    } else {
                        // Show first difference
                        for (i, (a, b)) in query_slice.iter().zip(rule.tokens.iter()).enumerate() {
                            if a != b {
                                // Check if this position is matchable
                                let is_matchable = whole_run.matchables(true).contains(&(*pos + i));
                                eprintln!(
                                    "  First diff at offset {}: query={}, rule={}, pos={}, is_matchable={}",
                                    i,
                                    a,
                                    b,
                                    *pos + i,
                                    is_matchable
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }

        let aho_matches = aho_match(index, &whole_run);
        eprintln!("\nNumber of aho matches: {}", aho_matches.len());

        for (i, m) in aho_matches.iter().enumerate() {
            print_match_details(m, &format!("AhoMatch[{}]", i));
        }
    }

    #[test]
    fn test_apache_2_0_and_apache_2_0() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = include_str!(
            "../../testdata/license-golden/datadriven/lic2/apache-2.0_and_apache-2.0.txt"
        );

        eprintln!("\n=== FULL DETECTION PIPELINE for apache-2.0_and_apache-2.0.txt ===");
        let detections = engine.detect(content).expect("Detection should succeed");

        eprintln!("Number of detections: {}", detections.len());

        let mut all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();
        all_matches.sort_by_key(|m| m.start_line);

        for (i, m) in all_matches.iter().enumerate() {
            print_match_details(m, &format!("Match[{}]", i));
        }

        let expressions: Vec<&str> = all_matches
            .iter()
            .map(|m| m.license_expression.as_str())
            .collect();
        eprintln!("Final expressions: {:?}", expressions);

        assert_eq!(
            all_matches.len(),
            2,
            "Expected 2 matches for apache-2.0_and_apache-2.0.txt, got {}",
            all_matches.len()
        );
    }

    #[test]
    fn test_aladdin_md5_and_not_rsa_md5() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = include_str!(
            "../../testdata/license-golden/datadriven/lic2/aladdin-md5_and_not_rsa-md5.txt"
        );

        eprintln!("\n=== FULL DETECTION PIPELINE for aladdin-md5_and_not_rsa-md5.txt ===");
        let detections = engine.detect(content).expect("Detection should succeed");

        eprintln!("Number of detections: {}", detections.len());

        let mut all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();
        all_matches.sort_by_key(|m| m.start_line);

        for (i, m) in all_matches.iter().enumerate() {
            print_match_details(m, &format!("Match[{}]", i));
        }

        let expressions: Vec<&str> = all_matches
            .iter()
            .map(|m| m.license_expression.as_str())
            .collect();
        eprintln!("Final expressions: {:?}", expressions);

        assert_eq!(
            all_matches.len(),
            2,
            "Expected 2 matches for aladdin-md5_and_not_rsa-md5.txt, got {}",
            all_matches.len()
        );
    }

    #[test]
    fn test_qcontains_identical_ranges() {
        use crate::license_detection::models::LicenseMatch;

        let m1 = LicenseMatch {
            rid: 0,
            license_expression: "test".to_string(),
            license_expression_spdx: "TEST".to_string(),
            start_token: 10,
            end_token: 20,
            start_line: 1,
            end_line: 5,
            qspan_positions: None,
            matched_length: 10,
            rule_length: 10,
            rule_identifier: "test.LICENSE".to_string(),
            rule_url: "".to_string(),
            matcher: "2-aho".to_string(),
            score: 100.0,
            match_coverage: 100.0,
            rule_relevance: 100,
            hilen: 5,
            rule_start_token: 0,
            ..Default::default()
        };

        let m2 = LicenseMatch {
            rid: 0,
            license_expression: "test".to_string(),
            license_expression_spdx: "TEST".to_string(),
            start_token: 10,
            end_token: 20,
            start_line: 1,
            end_line: 5,
            qspan_positions: None,
            matched_length: 10,
            rule_length: 10,
            rule_identifier: "test.LICENSE".to_string(),
            rule_url: "".to_string(),
            matcher: "2-aho".to_string(),
            score: 100.0,
            match_coverage: 100.0,
            rule_relevance: 100,
            hilen: 5,
            rule_start_token: 0,
            ..Default::default()
        };

        eprintln!("\n=== qcontains with IDENTICAL ranges ===");
        eprintln!(
            "m1: start_token={}, end_token={}, qspan_positions={:?}",
            m1.start_token, m1.end_token, m1.qspan_positions
        );
        eprintln!(
            "m2: start_token={}, end_token={}, qspan_positions={:?}",
            m2.start_token, m2.end_token, m2.qspan_positions
        );

        let result = m1.qcontains(&m2);
        eprintln!(
            "m1.qcontains(m2) = {} (expected: true, since ranges are identical)",
            result
        );

        let result2 = m2.qcontains(&m1);
        eprintln!(
            "m2.qcontains(m1) = {} (expected: true, since ranges are identical)",
            result2
        );

        assert!(result, "Identical ranges should contain each other");
        assert!(result2, "Identical ranges should contain each other");
    }

    #[test]
    fn test_qcontains_different_ranges() {
        use crate::license_detection::models::LicenseMatch;

        let m1 = LicenseMatch {
            rid: 0,
            license_expression: "test".to_string(),
            license_expression_spdx: "TEST".to_string(),
            start_token: 10,
            end_token: 20,
            start_line: 1,
            end_line: 5,
            qspan_positions: None,
            matched_length: 10,
            rule_length: 10,
            rule_identifier: "test.LICENSE".to_string(),
            rule_url: "".to_string(),
            matcher: "2-aho".to_string(),
            score: 100.0,
            match_coverage: 100.0,
            rule_relevance: 100,
            hilen: 5,
            rule_start_token: 0,
            ..Default::default()
        };

        let m2 = LicenseMatch {
            rid: 0,
            license_expression: "test".to_string(),
            license_expression_spdx: "TEST".to_string(),
            start_token: 30,
            end_token: 40,
            start_line: 10,
            end_line: 15,
            qspan_positions: None,
            matched_length: 10,
            rule_length: 10,
            rule_identifier: "test.LICENSE".to_string(),
            rule_url: "".to_string(),
            matcher: "2-aho".to_string(),
            score: 100.0,
            match_coverage: 100.0,
            rule_relevance: 100,
            hilen: 5,
            rule_start_token: 0,
            ..Default::default()
        };

        eprintln!("\n=== qcontains with DIFFERENT non-overlapping ranges ===");
        eprintln!(
            "m1: start_token={}, end_token={}",
            m1.start_token, m1.end_token
        );
        eprintln!(
            "m2: start_token={}, end_token={}",
            m2.start_token, m2.end_token
        );

        let result = m1.qcontains(&m2);
        eprintln!("m1.qcontains(m2) = {} (expected: false)", result);

        let result2 = m2.qcontains(&m1);
        eprintln!("m2.qcontains(m1) = {} (expected: false)", result2);

        assert!(!result, "m1 should NOT contain m2");
        assert!(!result2, "m2 should NOT contain m1");
    }

    #[test]
    fn test_tokenize_backslash_n_in_rust() {
        use crate::license_detection::tokenize::tokenize_without_stopwords;

        // C string literal with backslash-n (the actual bytes in the file)
        let c_literal = "modify\\nit"; // This is: m o d i f y \ n i t
        eprintln!("\n=== TOKENIZE BACKSLASH-N IN RUST ===");
        eprintln!("C literal: {:?}", c_literal);
        let rust_tokens = tokenize_without_stopwords(c_literal);
        eprintln!("Rust tokenized: {:?}", rust_tokens);

        // Actual newline
        let actual_newline = "modify\nit";
        eprintln!("Actual newline: {:?}", actual_newline);
        let rust_tokens_newline = tokenize_without_stopwords(actual_newline);
        eprintln!("Rust tokenized: {:?}", rust_tokens_newline);

        // Python tokenizes 'modify\\nit' as ['modify', 'nit'] because \n is kept together
        // Rust should do the same for compatibility
    }

    #[test]
    fn test_lookup_token_8579_and_7054() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let index = engine.index();

        // Look up what tokens 8579 and 7054 represent
        eprintln!("\n=== LOOKING UP TOKENS 8579 and 7054 ===");

        for (token_str, &tid) in index.dictionary.tokens_to_ids() {
            if tid == 8579 {
                eprintln!("Token 8579 = {:?}", token_str);
            }
            if tid == 7054 {
                eprintln!("Token 7054 = {:?}", token_str);
            }
        }

        // Also check if 'n' (from \n escape sequence) has a token
        for (token_str, &tid) in index.dictionary.tokens_to_ids() {
            if token_str == "n" {
                eprintln!("Token 'n' = {}", tid);
            }
        }
    }

    #[test]
    fn test_qcontains_overlapping_but_not_contained() {
        use crate::license_detection::models::LicenseMatch;

        // m1 is from token 10-20
        // m2 is from token 15-25
        // They overlap but neither contains the other
        let m1 = LicenseMatch {
            rid: 0,
            license_expression: "test".to_string(),
            license_expression_spdx: "TEST".to_string(),
            start_token: 10,
            end_token: 20,
            start_line: 1,
            end_line: 5,
            qspan_positions: None,
            matched_length: 10,
            rule_length: 10,
            rule_identifier: "test.LICENSE".to_string(),
            rule_url: "".to_string(),
            matcher: "2-aho".to_string(),
            score: 100.0,
            match_coverage: 100.0,
            rule_relevance: 100,
            hilen: 5,
            rule_start_token: 0,
            ..Default::default()
        };

        let m2 = LicenseMatch {
            rid: 0,
            license_expression: "test".to_string(),
            license_expression_spdx: "TEST".to_string(),
            start_token: 15,
            end_token: 25,
            start_line: 5,
            end_line: 10,
            qspan_positions: None,
            matched_length: 10,
            rule_length: 10,
            rule_identifier: "test.LICENSE".to_string(),
            rule_url: "".to_string(),
            matcher: "2-aho".to_string(),
            score: 100.0,
            match_coverage: 100.0,
            rule_relevance: 100,
            hilen: 5,
            rule_start_token: 0,
            ..Default::default()
        };

        eprintln!("\n=== qcontains with OVERLAPPING but NOT CONTAINED ranges ===");
        eprintln!(
            "m1: start_token={}, end_token={}",
            m1.start_token, m1.end_token
        );
        eprintln!(
            "m2: start_token={}, end_token={}",
            m2.start_token, m2.end_token
        );

        let result = m1.qcontains(&m2);
        eprintln!(
            "m1.qcontains(m2) = {} (expected: false, since m2 extends past m1)",
            result
        );

        let result2 = m2.qcontains(&m1);
        eprintln!(
            "m2.qcontains(m1) = {} (expected: false, since m1 starts before m2)",
            result2
        );

        assert!(!result, "m1 should NOT contain m2 (m2 extends past m1)");
        assert!(!result2, "m2 should NOT contain m1 (m1 starts before m2)");
    }
}
