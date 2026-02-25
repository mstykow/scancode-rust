//! CDDL Rule Selection Investigation Test
//!
//! This test file investigates the CDDL 1.0 vs CDDL 1.1 rule selection issue.
//! When scanning `cddl-1.0_or_gpl-2.0-glassfish.txt`, Rust incorrectly matches
//! a CDDL 1.1 rule instead of the CDDL 1.0 rule.

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use crate::license_detection::index::token_sets::build_set_and_mset;
    use crate::license_detection::query::Query;
    use crate::license_detection::seq_match::{
        MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match_with_candidates,
    };
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            eprintln!("Reference data not available at {:?}", data_path);
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    fn read_test_file(name: &str) -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic1").join(name);
        match std::fs::read_to_string(&path) {
            Ok(content) => Some(content),
            Err(e) => {
                eprintln!("Failed to read {:?}: {}", path, e);
                None
            }
        }
    }

    fn find_rule_rid(
        index: &crate::license_detection::index::LicenseIndex,
        identifier_contains: &str,
    ) -> Option<usize> {
        for (rid, rule) in index.rules_by_rid.iter().enumerate() {
            if rule.identifier.contains(identifier_contains) {
                return Some(rid);
            }
        }
        None
    }

    #[test]
    fn test_detect_all_phases() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        #[allow(unused_imports)]
        use crate::license_detection::index::LicenseIndex;
        use crate::license_detection::match_refine::merge_overlapping_matches;
        use crate::license_detection::query::Query;
        use crate::license_detection::seq_match::{
            MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match,
            seq_match_with_candidates,
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        eprintln!("\n========================================");
        eprintln!("All Phases Trace");
        eprintln!("========================================");

        // Phase 2: Near-duplicate
        eprintln!("\nPhase 2: Near-duplicate detection");
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);
        eprintln!("  Near-dupe matches: {}", near_dupe_matches.len());
        for m in &near_dupe_matches {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            if is_cddl10 || is_cddl11 {
                eprintln!(
                    "    {} (rid={}, coverage={:.1}%, start={}, end={}){}",
                    rule.license_expression,
                    m.rid,
                    m.match_coverage,
                    m.start_token,
                    m.end_token,
                    marker
                );
            }
        }

        // Phase 3: Regular seq_match
        eprintln!("\nPhase 3: Regular seq_match");
        let seq_matches = seq_match(index, &whole_run);
        eprintln!("  Seq matches: {}", seq_matches.len());
        for m in &seq_matches {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            if is_cddl10 || is_cddl11 {
                eprintln!(
                    "    {} (rid={}, coverage={:.1}%, start={}, end={}){}",
                    rule.license_expression,
                    m.rid,
                    m.match_coverage,
                    m.start_token,
                    m.end_token,
                    marker
                );
            }
        }

        // Combine all matches
        let mut all_seq_matches = near_dupe_matches.clone();
        all_seq_matches.extend(seq_matches.clone());
        let merged_all = merge_overlapping_matches(&all_seq_matches);

        eprintln!("\nAfter merging all seq matches: {}", merged_all.len());
        for m in &merged_all {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            if is_cddl10 || is_cddl11 {
                eprintln!(
                    "    {} (rid={}, coverage={:.1}%, start={}, end={}, qspan_positions={:?}){}",
                    rule.license_expression,
                    m.rid,
                    m.match_coverage,
                    m.start_token,
                    m.end_token,
                    m.qspan_positions.as_ref().map(|p| p.len()),
                    marker
                );
            }
        }

        // Check qcontains behavior
        let cddl10_main = merged_all
            .iter()
            .find(|m| cddl10_rid == Some(m.rid) && m.match_coverage > 90.0);
        let cddl11_main = merged_all.iter().find(|m| {
            cddl11_rid == Some(m.rid) && m.qspan_positions.is_some() && m.start_token == 0
        });

        if let (Some(m10), Some(m11)) = (cddl10_main, cddl11_main) {
            eprintln!("\n--- Comparison check ---");
            eprintln!(
                "CDDL 1.0: start={}, end={}, hilen={}, matched_length={}",
                m10.start_token, m10.end_token, m10.hilen, m10.matched_length
            );
            eprintln!(
                "CDDL 1.1: start={}, end={}, hilen={}, matched_length={}",
                m11.start_token, m11.end_token, m11.hilen, m11.matched_length
            );

            // Check sorting order for filter_overlapping_matches
            // Sort by: qstart ASC, hilen DESC, matched_length DESC, matcher_order ASC
            let m10_first = m11
                .qstart()
                .cmp(&m10.qstart())
                .then_with(|| m10.hilen.cmp(&m11.hilen))
                .then_with(|| m10.matched_length.cmp(&m11.matched_length))
                .then_with(|| m11.matcher_order().cmp(&m10.matcher_order()));
            eprintln!("\nSorting order: {:?}", m10_first);
            if m10_first == std::cmp::Ordering::Less {
                eprintln!("  -> CDDL 1.1 (current) comes before CDDL 1.0 (next)");
            } else {
                eprintln!("  -> CDDL 1.0 (current) comes before CDDL 1.1 (next)");
            }

            // Simulate filter_overlapping_matches logic
            let overlap = m11.qoverlap(m10);
            let overlap_ratio_to_m10 = overlap as f64 / m10.matched_length as f64;
            let overlap_ratio_to_m11 = overlap as f64 / m11.matched_length as f64;
            eprintln!("\nfilter_overlapping_matches simulation:");
            eprintln!("  overlap: {}", overlap);
            eprintln!(
                "  overlap_ratio_to_next (CDDL 1.0): {:.3}",
                overlap_ratio_to_m10
            );
            eprintln!(
                "  overlap_ratio_to_current (CDDL 1.1): {:.3}",
                overlap_ratio_to_m11
            );

            #[allow(dead_code)]
            const OVERLAP_SMALL: f64 = 0.10;
            #[allow(dead_code)]
            const OVERLAP_MEDIUM: f64 = 0.40;
            const OVERLAP_LARGE: f64 = 0.70;
            const OVERLAP_EXTRA_LARGE: f64 = 0.90;

            let large_current = overlap_ratio_to_m11 >= OVERLAP_LARGE;
            let extra_large_current = overlap_ratio_to_m11 >= OVERLAP_EXTRA_LARGE;
            eprintln!("  large_current (>= 0.70): {}", large_current);
            eprintln!("  extra_large_current (>= 0.90): {}", extra_large_current);

            // current = CDDL 1.1, next = CDDL 1.0
            let current_len = m11.matched_length;
            let next_len = m10.matched_length;
            let current_hilen = m11.hilen;
            let next_hilen = m10.hilen;

            eprintln!("\n  Checking line 586: extra_large_current && current_len <= next_len");
            eprintln!(
                "    extra_large_current={} && current_len({}) <= next_len({}) = {}",
                extra_large_current,
                current_len,
                next_len,
                extra_large_current && current_len <= next_len
            );

            eprintln!(
                "\n  Checking line 597: large_current && current_len <= next_len && current_hilen <= next_hilen"
            );
            eprintln!(
                "    large_current={} && current_len({}) <= next_len({}) && current_hilen({}) <= next_hilen({}) = {}",
                large_current,
                current_len,
                next_len,
                current_hilen,
                next_hilen,
                large_current && current_len <= next_len && current_hilen <= next_hilen
            );
        }

        // Check what's happening after refine_matches
        use crate::license_detection::match_refine::refine_matches;
        eprintln!("\nCalling refine_matches()...");
        let refined = refine_matches(index, merged_all.clone(), &query);
        eprintln!("  After refine_matches: {} matches", refined.len());

        // Check ALL matches in refined
        eprintln!("  ALL matches in refined:");
        for m in &refined {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            eprintln!(
                "    {} (rid={}, coverage={:.1}%, start={}, end={}){}",
                rule.license_expression,
                m.rid,
                m.match_coverage,
                m.start_token,
                m.end_token,
                marker
            );
        }

        // Now check final detection
        eprintln!("\nFinal detection via engine.detect():");
        let detections = engine.detect(&text).expect("Detection should succeed");
        for d in &detections {
            eprintln!("  Detection: {:?}", d.license_expression);
        }
    }

    #[test]
    fn test_cddl_10_detection_basic() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        let detections = engine.detect(&text).expect("Detection should succeed");

        eprintln!("\n========================================");
        eprintln!("CDDL 1.0 Detection Test");
        eprintln!("========================================");
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("Number of detections: {}", detections.len());

        for (i, d) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", d.license_expression);
            eprintln!("  license_expression_spdx: {:?}", d.license_expression_spdx);
            for (j, m) in d.matches.iter().enumerate() {
                eprintln!(
                    "  Match {}: {} (matcher: {}, score: {:.2})",
                    j + 1,
                    m.license_expression,
                    m.matcher,
                    m.score
                );
                eprintln!("    rule_identifier: {}", m.rule_identifier);
                eprintln!("    lines: {}-{}", m.start_line, m.end_line);
            }
        }

        let has_cddl10 = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map(|s| s.contains("cddl-1.0"))
                .unwrap_or(false)
        });
        assert!(
            has_cddl10,
            "Expected CDDL 1.0 in final detection, got: {:?}",
            detections
                .first()
                .and_then(|d| d.license_expression.as_ref())
        );
    }

    #[test]
    fn test_cddl_11_detection_basic() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.1_or_gpl-2.0-classpath-glassfish.txt") else {
            return;
        };

        let detections = engine.detect(&text).expect("Detection should succeed");

        eprintln!("\n========================================");
        eprintln!("CDDL 1.1 Detection Test");
        eprintln!("========================================");
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("Number of detections: {}", detections.len());

        for (i, d) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", d.license_expression);
            for (j, m) in d.matches.iter().enumerate() {
                eprintln!(
                    "  Match {}: {} (rule: {})",
                    j + 1,
                    m.license_expression,
                    m.rule_identifier
                );
            }
        }

        let actual = detections
            .first()
            .and_then(|d| d.license_expression.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("none");

        assert!(
            actual.contains("cddl-1.1") || actual.contains("CDDL-1.1"),
            "Expected CDDL 1.1 expression, got: {}",
            actual
        );
    }

    #[test]
    #[allow(unused_variables)]
    fn test_investigate_rule_matching_cddl_10() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n========================================");
        eprintln!("CDDL 1.0 Rule Matching Investigation");
        eprintln!("========================================");

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        eprintln!("\nRule IDs found:");
        eprintln!("  cddl-1.0_or_gpl-2.0-glassfish: {:?}", cddl10_rid);
        eprintln!(
            "  cddl-1.1_or_gpl-2.0-classpath-glassfish: {:?}",
            cddl11_rid
        );

        if let Some(rid) = cddl10_rid {
            let rule = &index.rules_by_rid[rid];
            eprintln!("\nCDDL 1.0 Rule details:");
            eprintln!("  identifier: {}", rule.identifier);
            eprintln!("  license_expression: {}", rule.license_expression);
            eprintln!("  text length: {}", rule.text.len());
            eprintln!("  tokens count: {}", rule.tokens.len());
            eprintln!("  is_license_notice: {}", rule.is_license_notice);
            if let Some(urls) = &rule.ignorable_urls {
                eprintln!("  ignorable_urls: {:?}", urls);
            }
        }

        if let Some(rid) = cddl11_rid {
            let rule = &index.rules_by_rid[rid];
            eprintln!("\nCDDL 1.1 Rule details:");
            eprintln!("  identifier: {}", rule.identifier);
            eprintln!("  license_expression: {}", rule.license_expression);
            eprintln!("  text length: {}", rule.text.len());
            eprintln!("  tokens count: {}", rule.tokens.len());
            eprintln!("  is_license_notice: {}", rule.is_license_notice);
            if let Some(urls) = &rule.ignorable_urls {
                eprintln!("  ignorable_urls: {:?}", urls);
            }
        }

        eprintln!("\nQuery details:");
        eprintln!("  tokens count: {}", query.tokens.len());
        eprintln!("  high_matchables: {}", query.high_matchables.len());
        eprintln!("  low_matchables: {}", query.low_matchables.len());
    }

    #[test]
    fn test_investigate_sequence_matching_cddl_10() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n========================================");
        eprintln!("CDDL 1.0 Sequence Matching Investigation");
        eprintln!("========================================");

        let candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);

        eprintln!("\nNear-duplicate candidates found: {}", candidates.len());

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        for (i, candidate) in candidates.iter().enumerate() {
            let rule = &index.rules_by_rid[candidate.rid];
            let is_cddl10 = cddl10_rid == Some(candidate.rid);
            let is_cddl11 = cddl11_rid == Some(candidate.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };

            eprintln!(
                "  Candidate {}: rid={} score={:.3} expr={}{}",
                i + 1,
                candidate.rid,
                candidate.score_vec_rounded.resemblance,
                rule.license_expression,
                marker
            );
        }

        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(index, &whole_run, &candidates);
            eprintln!("\nSequence matches from candidates: {}", matches.len());

            for (i, m) in matches.iter().enumerate() {
                let rule = &index.rules_by_rid[m.rid];
                let is_cddl10 = cddl10_rid == Some(m.rid);
                let is_cddl11 = cddl11_rid == Some(m.rid);
                let marker = if is_cddl10 {
                    " <-- CDDL 1.0"
                } else if is_cddl11 {
                    " <-- CDDL 1.1"
                } else {
                    ""
                };

                eprintln!(
                    "  Match {}: {} (rid={}, score={:.2}, coverage={:.1}%){}",
                    i + 1,
                    rule.license_expression,
                    m.rid,
                    m.score,
                    m.match_coverage,
                    marker
                );
            }
        }
    }

    #[test]
    fn test_compare_query_tokens_vs_rules() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n========================================");
        eprintln!("Query Tokens vs Rule Tokens Comparison");
        eprintln!("========================================");

        let query_tokens = whole_run.tokens();
        let query_set: HashSet<u16> = query_tokens.iter().copied().collect();
        let (_, query_mset) = build_set_and_mset(query_tokens);

        eprintln!(
            "Query: {} tokens, {} unique in set, {} in mset",
            query_tokens.len(),
            query_set.len(),
            query_mset.len()
        );

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        for (rule_name, rid_opt) in [("CDDL 1.0", cddl10_rid), ("CDDL 1.1", cddl11_rid)] {
            if let Some(rid) = rid_opt {
                let rule = &index.rules_by_rid[rid];
                let rule_set = index.sets_by_rid.get(&rid);
                let rule_mset = index.msets_by_rid.get(&rid);

                if let (Some(rs), Some(rm)) = (rule_set, rule_mset) {
                    let intersection: HashSet<u16> = query_set.intersection(rs).copied().collect();
                    let union_size = query_set.len() + rs.len() - intersection.len();
                    let resemblance = intersection.len() as f32 / union_size as f32;

                    eprintln!("\n{} Rule (rid={}):", rule_name, rid);
                    eprintln!(
                        "  tokens: {}, set: {}, mset: {}",
                        rule.tokens.len(),
                        rs.len(),
                        rm.len()
                    );
                    eprintln!("  intersection with query: {}", intersection.len());
                    eprintln!("  union size: {}", union_size);
                    eprintln!("  resemblance: {:.3}", resemblance);
                    eprintln!("  is_highly_resemblant (>= 0.8): {}", resemblance >= 0.8);
                }
            }
        }
    }

    #[test]
    fn test_key_distinguishing_features() {
        let Some(engine) = get_engine() else { return };
        let Some(text_10) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };
        let Some(text_11) = read_test_file("cddl-1.1_or_gpl-2.0-classpath-glassfish.txt") else {
            return;
        };

        let index = engine.index();

        eprintln!("\n========================================");
        eprintln!("Key Distinguishing Features");
        eprintln!("========================================");

        let cddl_10_url = "CDDL+GPL.html";
        let cddl_11_url = "CDDL+GPL_1_1.html";

        eprintln!("\nCDDL 1.0 test file:");
        eprintln!(
            "  Contains '{}': {}",
            cddl_10_url,
            text_10.contains(cddl_10_url)
        );
        eprintln!(
            "  Contains '{}': {}",
            cddl_11_url,
            text_10.contains(cddl_11_url)
        );
        eprintln!("  Contains 'Oracle': {}", text_10.contains("Oracle"));
        eprintln!(
            "  Contains 'Sun Microsystems': {}",
            text_10.contains("Sun Microsystems")
        );
        eprintln!(
            "  Contains 'Classpath Exception': {}",
            text_10.contains("Classpath Exception")
        );

        eprintln!("\nCDDL 1.1 test file:");
        eprintln!(
            "  Contains '{}': {}",
            cddl_11_url,
            text_11.contains(cddl_11_url)
        );
        eprintln!(
            "  Contains '{}': {}",
            cddl_10_url,
            text_11.contains(cddl_10_url)
        );
        eprintln!("  Contains 'Oracle': {}", text_11.contains("Oracle"));
        eprintln!(
            "  Contains 'Sun Microsystems': {}",
            text_11.contains("Sun Microsystems")
        );
        eprintln!(
            "  Contains 'Classpath Exception': {}",
            text_11.contains("Classpath Exception")
        );

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        if let Some(rid) = cddl10_rid {
            let rule = &index.rules_by_rid[rid];
            eprintln!("\nCDDL 1.0 Rule text:");
            eprintln!(
                "  Contains '{}': {}",
                cddl_10_url,
                rule.text.contains(cddl_10_url)
            );
            eprintln!(
                "  Contains '{}': {}",
                cddl_11_url,
                rule.text.contains(cddl_11_url)
            );
            eprintln!("  Contains 'Oracle': {}", rule.text.contains("Oracle"));
            eprintln!(
                "  Contains 'Sun Microsystems': {}",
                rule.text.contains("Sun Microsystems")
            );
        }

        if let Some(rid) = cddl11_rid {
            let rule = &index.rules_by_rid[rid];
            eprintln!("\nCDDL 1.1 Rule text:");
            eprintln!(
                "  Contains '{}': {}",
                cddl_11_url,
                rule.text.contains(cddl_11_url)
            );
            eprintln!(
                "  Contains '{}': {}",
                cddl_10_url,
                rule.text.contains(cddl_10_url)
            );
            eprintln!("  Contains 'Oracle': {}", rule.text.contains("Oracle"));
            eprintln!(
                "  Contains 'Sun Microsystems': {}",
                rule.text.contains("Sun Microsystems")
            );
        }
    }

    #[test]
    fn test_aho_corasick_matching_cddl() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n========================================");
        eprintln!("Aho-Corasick Matching Investigation");
        eprintln!("========================================");

        use crate::license_detection::aho_match::aho_match;
        let aho_matches = aho_match(index, &whole_run);

        eprintln!("Aho-Corasick matches found: {}", aho_matches.len());

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        for (i, m) in aho_matches.iter().enumerate() {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };

            eprintln!(
                "  Match {}: {} (rid={}, matcher={}, score={:.2}){}",
                i + 1,
                rule.license_expression,
                m.rid,
                m.matcher,
                m.score,
                marker
            );
        }
    }

    #[test]
    fn test_hash_matching_cddl() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n========================================");
        eprintln!("Hash Matching Investigation");
        eprintln!("========================================");

        use crate::license_detection::hash_match::hash_match;
        let hash_matches = hash_match(index, &whole_run);

        eprintln!("Hash matches found: {}", hash_matches.len());

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        for (i, m) in hash_matches.iter().enumerate() {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };

            eprintln!(
                "  Match {}: {} (rid={}, matcher={}, score={:.2}){}",
                i + 1,
                rule.license_expression,
                m.rid,
                m.matcher,
                m.score,
                marker
            );
        }
    }

    #[test]
    fn test_detect_full_pipeline_matches() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else {
            return;
        };

        use crate::license_detection::aho_match::aho_match;
        #[allow(unused_imports)]
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region, sort_matches_by_line,
        };
        use crate::license_detection::match_refine::merge_overlapping_matches;
        use crate::license_detection::seq_match::seq_match;
        use crate::license_detection::spdx_lid::spdx_lid_match;

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        let cddl10_rid = find_rule_rid(index, "cddl-1.0_or_gpl-2.0-glassfish");
        let cddl11_rid = find_rule_rid(index, "cddl-1.1_or_gpl-2.0-classpath-glassfish");

        eprintln!("\n========================================");
        eprintln!("FULL PIPELINE - Matching detect() behavior");
        eprintln!("========================================");

        let mut all_matches: Vec<crate::license_detection::models::LicenseMatch> = Vec::new();

        // Phase 1b: SPDX-LID
        let spdx_matches = spdx_lid_match(index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        all_matches.extend(merged_spdx);
        eprintln!("After SPDX-LID: {} matches", all_matches.len());

        // Phase 1c: Aho-Corasick
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        all_matches.extend(merged_aho);
        eprintln!("After Aho: {} matches", all_matches.len());

        // Phase 2: Near-duplicate
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let mut seq_all_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };

        // Phase 3: Regular seq_match
        let seq_matches = seq_match(index, &whole_run);
        eprintln!("Phase 2 (near-dupe) matches: {}", seq_all_matches.len());
        eprintln!("Phase 3 (regular seq) matches: {}", seq_matches.len());

        // Count CDDL matches in each phase
        let cddl10_in_phase2 = seq_all_matches
            .iter()
            .filter(|m| cddl10_rid == Some(m.rid))
            .count();
        let cddl11_in_phase2 = seq_all_matches
            .iter()
            .filter(|m| cddl11_rid == Some(m.rid))
            .count();
        let cddl10_in_phase3 = seq_matches
            .iter()
            .filter(|m| cddl10_rid == Some(m.rid))
            .count();
        let cddl11_in_phase3 = seq_matches
            .iter()
            .filter(|m| cddl11_rid == Some(m.rid))
            .count();

        eprintln!(
            "  CDDL 1.0 in phase2: {}, CDDL 1.1 in phase2: {}",
            cddl10_in_phase2, cddl11_in_phase2
        );
        eprintln!(
            "  CDDL 1.0 in phase3: {}, CDDL 1.1 in phase3: {}",
            cddl10_in_phase3, cddl11_in_phase3
        );

        seq_all_matches.extend(seq_matches);

        // Merge all sequence matches
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        eprintln!("After merge seq: {} matches", merged_seq.len());

        // Check CDDL presence after merge
        let cddl10_after_merge = merged_seq
            .iter()
            .filter(|m| cddl10_rid == Some(m.rid))
            .count();
        let cddl11_after_merge = merged_seq
            .iter()
            .filter(|m| cddl11_rid == Some(m.rid))
            .count();
        eprintln!(
            "  CDDL 1.0 after merge: {}, CDDL 1.1 after merge: {}",
            cddl10_after_merge, cddl11_after_merge
        );

        all_matches.extend(merged_seq);
        eprintln!("Total before refine: {} matches", all_matches.len());

        // Show CDDL matches before refine
        eprintln!("\nCDDL matches before refine:");
        for m in all_matches
            .iter()
            .filter(|m| cddl10_rid == Some(m.rid) || cddl11_rid == Some(m.rid))
        {
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let marker = if is_cddl10 { "CDDL 1.0" } else { "CDDL 1.1" };
            eprintln!(
                "  {} (rid={}, matcher={}, score={:.2}, coverage={:.1}%, start={}, end={})",
                marker, m.rid, m.matcher, m.score, m.match_coverage, m.start_token, m.end_token
            );
        }

        // CRITICAL: refine_matches does an INITIAL merge of ALL matches
        eprintln!("\nCRITICAL: merge_overlapping_matches on ALL matches:");

        // First, show all CDDL 1.1 matches before merge
        eprintln!("\nCDDL 1.1 matches BEFORE merge:");
        for m in all_matches.iter().filter(|m| cddl11_rid == Some(m.rid)) {
            eprintln!(
                "  rid={}, matcher={}, score={:.2}, coverage={:.1}%, start={}, end={}, hilen={}, matched_length={}",
                m.rid,
                m.matcher,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.matched_length
            );
            if let Some(positions) = &m.qspan_positions {
                eprintln!(
                    "    qspan_positions: {} positions (first 10: {:?})",
                    positions.len(),
                    &positions.iter().take(10).copied().collect::<Vec<_>>()
                );
            } else {
                eprintln!("    qspan_positions: None (contiguous range)");
            }
        }

        eprintln!("\nCDDL 1.0 matches BEFORE merge:");
        for m in all_matches.iter().filter(|m| cddl10_rid == Some(m.rid)) {
            eprintln!(
                "  rid={}, matcher={}, score={:.2}, coverage={:.1}%, start={}, end={}, hilen={}, matched_length={}",
                m.rid,
                m.matcher,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.matched_length
            );
            if let Some(positions) = &m.qspan_positions {
                eprintln!(
                    "    qspan_positions: {} positions (first 10: {:?})",
                    positions.len(),
                    &positions.iter().take(10).copied().collect::<Vec<_>>()
                );
            } else {
                eprintln!("    qspan_positions: None (contiguous range)");
            }
        }

        let first_merged = merge_overlapping_matches(&all_matches);
        // Check the merge logic - what's happening with CDDL 1.1 matches?
        eprintln!("\nAnalyzing CDDL 1.1 merge behavior:");
        let cddl11_matches: Vec<_> = all_matches
            .iter()
            .filter(|m| cddl11_rid == Some(m.rid))
            .collect();
        if cddl11_matches.len() >= 2 {
            let m1 = cddl11_matches[0];
            let m2 = cddl11_matches[1];
            eprintln!(
                "  Match 1: start={}, end={}, hilen={}, matched_length={}",
                m1.start_token,
                m1.end_token,
                m1.hilen(),
                m1.matched_length
            );
            eprintln!(
                "  Match 2: start={}, end={}, hilen={}, matched_length={}",
                m2.start_token,
                m2.end_token,
                m2.hilen(),
                m2.matched_length
            );

            // Check surround condition
            let m1_surround_m2 = m1.start_token <= m2.start_token && m1.end_token >= m2.end_token;
            let m2_surround_m1 = m2.start_token <= m1.start_token && m2.end_token >= m1.end_token;
            eprintln!("  m1 surround m2: {}", m1_surround_m2);
            eprintln!("  m2 surround m1: {}", m2_surround_m1);

            // Check qcontains
            eprintln!("  m1 qcontains m2: {}", m1.qcontains(m2));
            eprintln!("  m2 qcontains m1: {}", m2.qcontains(m1));

            // Check qspan/ispan lengths
            eprintln!(
                "  m1 qspan len: {}, ispan len: {}",
                m1.qspan().len(),
                m1.ispan().len()
            );
            eprintln!(
                "  m2 qspan len: {}, ispan len: {}",
                m2.qspan().len(),
                m2.ispan().len()
            );

            // Check if m2's positions are subset of m1's positions
            let m1_positions: std::collections::HashSet<usize> = m1.qspan().into_iter().collect();
            let m2_positions: std::collections::HashSet<usize> = m2.qspan().into_iter().collect();
            let m2_subset_of_m1 = m2_positions.iter().all(|p| m1_positions.contains(p));
            eprintln!("  All m2 positions in m1: {}", m2_subset_of_m1);

            // Check the union size
            let union_size = m1_positions.union(&m2_positions).count();
            eprintln!(
                "  Union of positions: {} (should be {} if m2 is subset)",
                union_size,
                m1.qspan().len()
            );

            // Check the ispan relationship
            let m1_ispan: std::collections::HashSet<usize> = m1.ispan().into_iter().collect();
            let m2_ispan: std::collections::HashSet<usize> = m2.ispan().into_iter().collect();
            let m2_ispan_subset_of_m1 = m2_ispan.iter().all(|p| m1_ispan.contains(p));
            eprintln!("  All m2 ispan in m1 ispan: {}", m2_ispan_subset_of_m1);
            let ispan_union_size = m1_ispan.union(&m2_ispan).count();
            eprintln!("  Union of ispan: {}", ispan_union_size);
        }

        eprintln!(
            "\n  After first merge: {} matches (down from {})",
            first_merged.len(),
            all_matches.len()
        );
        for m in first_merged
            .iter()
            .filter(|m| cddl10_rid == Some(m.rid) || cddl11_rid == Some(m.rid))
        {
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let marker = if is_cddl10 { "CDDL 1.0" } else { "CDDL 1.1" };
            eprintln!(
                "  {} (rid={}, matcher={}, score={:.2}, coverage={:.1}%, start={}, end={}, hilen={}, matched_length={})",
                marker,
                m.rid,
                m.matcher,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.matched_length
            );
            if let Some(positions) = &m.qspan_positions {
                eprintln!(
                    "    qspan_positions: {} positions (first 10: {:?})",
                    positions.len(),
                    &positions.iter().take(10).copied().collect::<Vec<_>>()
                );
            } else {
                eprintln!("    qspan_positions: None (contiguous range)");
            }
        }

        // Run refine step by step
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, restore_non_overlapping,
        };

        // Step 1: filter_contained
        let (non_contained, discarded_contained) = filter_contained_matches(&first_merged);
        eprintln!("\nAfter filter_contained:");
        eprintln!("  Kept: {}", non_contained.len());
        eprintln!("  Discarded: {}", discarded_contained.len());
        for m in non_contained
            .iter()
            .filter(|m| cddl10_rid == Some(m.rid) || cddl11_rid == Some(m.rid))
        {
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let marker = if is_cddl10 { "CDDL 1.0" } else { "CDDL 1.1" };
            eprintln!(
                "  KEPT: {} (rid={}, matcher={}, start={}, end={})",
                marker, m.rid, m.matcher, m.start_token, m.end_token
            );
        }
        for m in discarded_contained
            .iter()
            .filter(|m| cddl10_rid == Some(m.rid) || cddl11_rid == Some(m.rid))
        {
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let marker = if is_cddl10 { "CDDL 1.0" } else { "CDDL 1.1" };
            eprintln!(
                "  DISCARDED: {} (rid={}, matcher={}, start={}, end={})",
                marker, m.rid, m.matcher, m.start_token, m.end_token
            );
        }

        // Step 2: filter_overlapping
        let (kept_overlapping, discarded_overlapping) =
            filter_overlapping_matches(non_contained.clone(), index);
        eprintln!("\nAfter filter_overlapping:");
        eprintln!("  Kept: {}", kept_overlapping.len());
        eprintln!("  Discarded: {}", discarded_overlapping.len());

        // Show detailed analysis of the two CDDL matches
        let cddl10 = non_contained.iter().find(|m| cddl10_rid == Some(m.rid));
        let cddl11 = non_contained.iter().find(|m| cddl11_rid == Some(m.rid));
        if let (Some(m10), Some(m11)) = (cddl10, cddl11) {
            eprintln!("\n  Detailed CDDL comparison BEFORE filter_overlapping:");
            eprintln!(
                "    CDDL 1.0: qstart={}, end={}, hilen={}, matched_length={}",
                m10.qstart(),
                m10.end_token,
                m10.hilen(),
                m10.matched_length
            );
            eprintln!(
                "    CDDL 1.1: qstart={}, end={}, hilen={}, matched_length={}",
                m11.qstart(),
                m11.end_token,
                m11.hilen(),
                m11.matched_length
            );

            // Sorting order
            let sort_order = m11
                .qstart()
                .cmp(&m10.qstart())
                .then_with(|| m10.hilen().cmp(&m11.hilen()))
                .then_with(|| m10.matched_length.cmp(&m11.matched_length));
            eprintln!(
                "    Sort order: {:?} (Less=CDDL1.1 first, Greater=CDDL1.0 first)",
                sort_order
            );

            // Overlap analysis
            let overlap = m11.qoverlap(m10);
            let overlap_ratio_to_10 = overlap as f64 / m10.matched_length as f64;
            let overlap_ratio_to_11 = overlap as f64 / m11.matched_length as f64;
            eprintln!("    Overlap: {} positions", overlap);
            eprintln!("    Overlap ratio to CDDL 1.0: {:.3}", overlap_ratio_to_10);
            eprintln!("    Overlap ratio to CDDL 1.1: {:.3}", overlap_ratio_to_11);

            // Check filter conditions
            let extra_large_current = overlap_ratio_to_11 >= 0.90;
            let large_current = overlap_ratio_to_11 >= 0.70;
            let extra_large_next = overlap_ratio_to_10 >= 0.90;
            let large_next = overlap_ratio_to_10 >= 0.70;

            eprintln!(
                "    extra_large_current (CDDL 1.1 >= 0.90): {}",
                extra_large_current
            );
            eprintln!("    large_current (CDDL 1.1 >= 0.70): {}", large_current);
            eprintln!(
                "    extra_large_next (CDDL 1.0 >= 0.90): {}",
                extra_large_next
            );
            eprintln!("    large_next (CDDL 1.0 >= 0.70): {}", large_next);

            eprintln!(
                "    current_len({}) <= next_len({}): {}",
                m11.matched_length,
                m10.matched_length,
                m11.matched_length <= m10.matched_length
            );
            eprintln!(
                "    current_hilen({}) <= next_hilen({}): {}",
                m11.hilen(),
                m10.hilen(),
                m11.hilen() <= m10.hilen()
            );

            // Line 586 condition: if extra_large_current && current_len <= next_len
            eprintln!(
                "    Line 586: extra_large_current && current_len <= next_len = {} && {} = {}",
                extra_large_current,
                m11.matched_length <= m10.matched_length,
                extra_large_current && m11.matched_length <= m10.matched_length
            );

            // Line 597 condition: if large_current && current_len <= next_len && current_hilen <= next_hilen
            eprintln!(
                "    Line 597: large_current && current_len <= next_len && current_hilen <= next_hilen = {} && {} && {} = {}",
                large_current,
                m11.matched_length <= m10.matched_length,
                m11.hilen() <= m10.hilen(),
                large_current
                    && m11.matched_length <= m10.matched_length
                    && m11.hilen() <= m10.hilen()
            );
        }

        eprintln!("  ALL KEPT matches:");
        for m in &kept_overlapping {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            eprintln!(
                "     KEPT: {} (rid={}, score={:.2}, coverage={:.1}%, start={}, end={}){}",
                rule.license_expression,
                m.rid,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token,
                marker
            );
        }
        eprintln!("   Discarded: {} matches", discarded_overlapping.len());
        for m in &discarded_overlapping {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            eprintln!(
                "     DISCARDED: {} (rid={}, score={:.2}, coverage={:.1}%, start={}, end={}){}",
                rule.license_expression,
                m.rid,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token,
                marker
            );
        }

        eprintln!("\n5e. RESTORE NON-OVERLAPPING (discarded_contained)");
        let mut matches_after_first_restore = kept_overlapping.clone();
        if !discarded_contained.is_empty() {
            let (restored_contained, _) =
                restore_non_overlapping(&kept_overlapping, discarded_contained.clone());
            eprintln!(
                "   Restored {} matches from discarded_contained",
                restored_contained.len()
            );
            for m in &restored_contained {
                let rule = &index.rules_by_rid[m.rid];
                let is_cddl10 = cddl10_rid == Some(m.rid);
                let is_cddl11 = cddl11_rid == Some(m.rid);
                let marker = if is_cddl10 {
                    " <-- CDDL 1.0"
                } else if is_cddl11 {
                    " <-- CDDL 1.1"
                } else {
                    ""
                };
                eprintln!(
                    "     RESTORED: {} (rid={}, score={:.2}, coverage={:.1}%, start={}, end={}){}",
                    rule.license_expression,
                    m.rid,
                    m.score,
                    m.match_coverage,
                    m.start_token,
                    m.end_token,
                    marker
                );
            }
            matches_after_first_restore.extend(restored_contained);
        }

        eprintln!("\n5f. RESTORE NON-OVERLAPPING (discarded_overlapping)");
        let mut final_matches = matches_after_first_restore.clone();
        if !discarded_overlapping.is_empty() {
            let (restored_overlapping, _) = restore_non_overlapping(
                &matches_after_first_restore,
                discarded_overlapping.clone(),
            );
            eprintln!(
                "   Restored {} matches from discarded_overlapping",
                restored_overlapping.len()
            );
            for m in &restored_overlapping {
                let rule = &index.rules_by_rid[m.rid];
                let is_cddl10 = cddl10_rid == Some(m.rid);
                let is_cddl11 = cddl11_rid == Some(m.rid);
                let marker = if is_cddl10 {
                    " <-- CDDL 1.0"
                } else if is_cddl11 {
                    " <-- CDDL 1.1"
                } else {
                    ""
                };
                eprintln!(
                    "     RESTORED: {} (rid={}, score={:.2}, coverage={:.1}%, start={}, end={}){}",
                    rule.license_expression,
                    m.rid,
                    m.score,
                    m.match_coverage,
                    m.start_token,
                    m.end_token,
                    marker
                );
            }
            final_matches.extend(restored_overlapping);
        }

        eprintln!("\n5g. FINAL filter_contained_matches");
        let (non_contained_final, _) = filter_contained_matches(&final_matches);
        eprintln!(
            "   After final filter_contained: {} matches",
            non_contained_final.len()
        );
        for m in &non_contained_final {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            eprintln!(
                "     MATCH: {} (rid={}, score={:.2}, coverage={:.1}%, start={}, end={}){}",
                rule.license_expression,
                m.rid,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token,
                marker
            );
        }
        for m in &discarded_overlapping {
            let rule = &index.rules_by_rid[m.rid];
            let is_cddl10 = cddl10_rid == Some(m.rid);
            let is_cddl11 = cddl11_rid == Some(m.rid);
            let marker = if is_cddl10 {
                " <-- CDDL 1.0"
            } else if is_cddl11 {
                " <-- CDDL 1.1"
            } else {
                ""
            };
            eprintln!(
                "     DISCARDED: {} (rid={}, score={:.2}, coverage={:.1}%, start={}, end={}){}",
                rule.license_expression,
                m.rid,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token,
                marker
            );
        }

        eprintln!("\n6. FINAL DETECTION (via engine.detect)");
        let detections = engine.detect(&text).expect("Detection should succeed");
        for (i, d) in detections.iter().enumerate() {
            eprintln!("   Detection {}: {:?}", i + 1, d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "     - {} (rule: {}, matcher: {}, score: {:.2})",
                    m.license_expression, m.rule_identifier, m.matcher, m.score
                );
            }
        }

        let has_cddl10 = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map(|s| s.contains("cddl-1.0"))
                .unwrap_or(false)
        });
        assert!(
            has_cddl10,
            "Expected CDDL 1.0 in final detection, got: {:?}",
            detections
                .first()
                .and_then(|d| d.license_expression.as_ref())
        );
    }
}
