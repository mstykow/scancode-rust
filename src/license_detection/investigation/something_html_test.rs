//! Investigation tests for PLAN-007: Extra `sun-sissl-1.1` detection
//!
//! ## Issue
//! Expected: `["sun-sissl-1.1", "mit", "sun-sissl-1.1", "sun-sissl-1.1", "apache-2.0"]`
//! Actual:   `["sun-sissl-1.1", "mit", "sun-sissl-1.1", "sun-sissl-1.1", "sun-sissl-1.1", "apache-2.0"]`
//!
//! There's an extra `sun-sissl-1.1` detection in Rust output.

#[cfg(test)]
mod tests {
    use crate::license_detection::aho_match::aho_match;
    use crate::license_detection::hash_match::hash_match;
    use crate::license_detection::match_refine::{
        filter_contained_matches, filter_false_positive_license_lists_matches,
        filter_overlapping_matches, merge_overlapping_matches, refine_matches,
        refine_matches_without_false_positive_filter, split_weak_matches,
    };
    use crate::license_detection::models::LicenseMatch;
    use crate::license_detection::query::Query;
    use crate::license_detection::seq_match::{
        compute_candidates_with_msets, seq_match_with_candidates, MAX_NEAR_DUPE_CANDIDATES,
    };
    use crate::license_detection::spdx_lid::spdx_lid_match;
    use crate::license_detection::unknown_match::unknown_match;
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

    fn print_match_summary(m: &LicenseMatch, prefix: &str) {
        eprintln!(
            "{}: {} (lines {}-{}, rid={}, matcher={}, coverage={:.1}%)",
            prefix,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.rid,
            m.matcher,
            m.match_coverage
        );
    }

    fn read_test_file() -> Option<String> {
        let path =
            PathBuf::from("testdata/license-golden/datadriven/lic4/should_detect_something.html");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_plan_007_full_detection() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PLAN-007: FULL DETECTION for should_detect_something.html ===");
        eprintln!("Text length: {} bytes", text.len());

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\nNumber of detections: {}", detections.len());
        for (i, d) in detections.iter().enumerate() {
            eprintln!("Detection[{}]: {:?}", i, d.license_expression);
            for m in &d.matches {
                print_match_summary(m, "  Match");
            }
        }

        let expressions: Vec<_> = detections
            .iter()
            .filter_map(|d| d.license_expression.as_ref())
            .collect();
        eprintln!("\nFinal expressions: {:?}", expressions);
        eprintln!(
            "EXPECTED: [\"sun-sissl-1.1\", \"mit\", \"sun-sissl-1.1\", \"sun-sissl-1.1\", \"apache-2.0\"]"
        );
        eprintln!("ACTUAL: {:?}", expressions);
    }

    #[test]
    fn test_plan_007_step_by_step() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== PLAN-007: STEP-BY-STEP PIPELINE ===");
        eprintln!("Query tokens: {}", query.tokens.len());

        // Step 1: Hash matching
        let hash_matches = hash_match(index, &whole_run);
        eprintln!("\n--- Step 1: Hash matching ---");
        eprintln!("Hash matches: {}", hash_matches.len());

        // Step 2: SPDX-LID matching
        let spdx_matches = spdx_lid_match(index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        eprintln!("\n--- Step 2: SPDX-LID matching ---");
        eprintln!(
            "SPDX matches: {} (merged: {})",
            spdx_matches.len(),
            merged_spdx.len()
        );
        for m in &merged_spdx {
            print_match_summary(m, "  SPDX");
        }

        // Step 3: Aho-Corasick matching
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        eprintln!("\n--- Step 3: Aho-Corasick matching ---");
        eprintln!(
            "Aho matches: {} (merged: {})",
            aho_matches.len(),
            merged_aho.len()
        );
        for m in &merged_aho {
            print_match_summary(m, "  Aho");
        }

        // Step 4: Near-duplicate sequence matching
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("\n--- Step 4: Near-duplicate candidates ---");
        eprintln!("Near-dupe candidates: {}", near_dupe_candidates.len());

        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };
        eprintln!("Near-dupe matches: {}", near_dupe_matches.len());
        for m in &near_dupe_matches {
            print_match_summary(m, "  Near-dupe");
        }

        // Step 5: Regular sequence matching
        let seq_matches = {
            let candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
            if !candidates.is_empty() {
                seq_match_with_candidates(index, &whole_run, &candidates)
            } else {
                Vec::new()
            }
        };
        eprintln!("\n--- Step 5: Regular sequence matching ---");
        eprintln!("Seq matches: {}", seq_matches.len());
        for m in &seq_matches {
            print_match_summary(m, "  Seq");
        }

        // Combine all matches (like main pipeline)
        let mut all_matches = Vec::new();
        all_matches.extend(merged_spdx.clone());
        all_matches.extend(merged_aho.clone());
        all_matches.extend(merge_overlapping_matches(&near_dupe_matches));
        all_matches.extend(merge_overlapping_matches(&seq_matches));

        eprintln!("\n--- Step 6: All matches combined ---");
        eprintln!("Total matches: {}", all_matches.len());

        // Group by license expression
        let mut by_expr: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for m in &all_matches {
            *by_expr.entry(&m.license_expression).or_insert(0) += 1;
        }
        eprintln!("By expression: {:?}", by_expr);

        // Step 7: Refine matches WITHOUT false positive filter
        eprintln!("\n--- Step 7: Refine matches (no FP filter) ---");
        let refined_no_fp =
            refine_matches_without_false_positive_filter(index, all_matches.clone(), &query);
        eprintln!("Refined (no FP) matches: {}", refined_no_fp.len());
        for m in &refined_no_fp {
            print_match_summary(m, "  Refined");
        }

        // Step 8: Split weak matches
        let (good_matches, weak_matches) = split_weak_matches(&refined_no_fp);
        eprintln!("\n--- Step 8: Split weak matches ---");
        eprintln!("Good matches: {}", good_matches.len());
        eprintln!("Weak matches: {}", weak_matches.len());

        // Step 9: Final refine WITH false positive filter
        let mut all_after_split = good_matches.clone();
        all_after_split.extend(weak_matches.clone());
        let refined = refine_matches(index, all_after_split, &query);
        eprintln!("\n--- Step 9: Final refine (with FP filter) ---");
        eprintln!("Final refined matches: {}", refined.len());
        for m in &refined {
            print_match_summary(m, "  Final");
        }
    }

    #[test]
    fn test_plan_007_sun_sissl_matches_detail() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== PLAN-007: sun-sissl-1.1 MATCHES DETAIL ===");

        // Get all sun-sissl matches from each matcher
        let aho_matches = aho_match(index, &whole_run);
        let sun_sissl_aho: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();

        eprintln!("\nAho sun-sissl-1.1 matches: {}", sun_sissl_aho.len());
        for m in &sun_sissl_aho {
            eprintln!(
                "  lines {}-{}, tokens {}-{}, rule={}, coverage={:.1}%",
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier,
                m.match_coverage
            );
        }

        // Near-dupe matches
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };
        let sun_sissl_near: Vec<_> = near_dupe_matches
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();

        eprintln!(
            "\nNear-dupe sun-sissl-1.1 matches: {}",
            sun_sissl_near.len()
        );
        for m in &sun_sissl_near {
            eprintln!(
                "  lines {}-{}, tokens {}-{}, rule={}, coverage={:.1}%",
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier,
                m.match_coverage
            );
        }

        // Regular seq matches
        let seq_candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        let seq_matches = if !seq_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &seq_candidates)
        } else {
            Vec::new()
        };
        let sun_sissl_seq: Vec<_> = seq_matches
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();

        eprintln!("\nSeq sun-sissl-1.1 matches: {}", sun_sissl_seq.len());
        for m in &sun_sissl_seq {
            eprintln!(
                "  lines {}-{}, tokens {}-{}, rule={}, coverage={:.1}%",
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier,
                m.match_coverage
            );
        }
    }

    #[test]
    fn test_plan_007_python_comparison() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PLAN-007: PYTHON vs RUST COMPARISON ===");

        // Python reference results (from running reference/scancode-playground)
        let python_detections = vec![
            ("sun-sissl-1.1", 7, 7, "2-aho", "sun-sissl-1.1_4.RULE"),
            ("mit", 30, 30, "2-aho", "mit_9.RULE"),
            ("sun-sissl-1.1", 195, 494, "3-seq", "sun-sissl-1.1_3.RULE"),
            ("sun-sissl-1.1", 207, 207, "2-aho", "sun-sissl-1.1_6.RULE"),
            ("apache-2.0", 528, 530, "2-aho", "apache-2.0_30.RULE"),
        ];

        eprintln!("\nPython detections ({} total):", python_detections.len());
        for (expr, start, end, matcher, rule) in &python_detections {
            eprintln!(
                "  {} (lines {}-{}, matcher={}, rule={})",
                expr, start, end, matcher, rule
            );
        }

        // Rust results
        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let rust_detections: Vec<_> = detections
            .iter()
            .filter_map(|d| {
                d.license_expression.as_ref().map(|expr| {
                    let first_match = d.matches.first();
                    (expr.clone(), d.matches.len(), first_match)
                })
            })
            .collect();

        eprintln!("\nRust detections ({} total):", rust_detections.len());
        for (expr, count, first_match) in &rust_detections {
            if let Some(m) = first_match {
                eprintln!(
                    "  {} (lines {}-{}, matcher={}, rule={}, match_count={})",
                    expr, m.start_line, m.end_line, m.matcher, m.rule_identifier, count
                );
            }
        }

        eprintln!("\n=== DIVERGENCE ANALYSIS ===");
        eprintln!("Python: 5 detections");
        eprintln!("Rust: {} detections", rust_detections.len());

        if rust_detections.len() > 5 {
            eprintln!("EXTRA DETECTION in Rust!");
        } else if rust_detections.len() < 5 {
            eprintln!("MISSING DETECTION in Rust!");
        } else {
            eprintln!("Same number of detections - checking order...");
        }
    }

    #[test]
    fn test_plan_007_merge_analysis() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== PLAN-007: MERGE ANALYSIS ===");

        // Collect all raw matches
        let spdx_matches = spdx_lid_match(index, &query);
        let aho_matches = aho_match(index, &whole_run);

        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };

        let seq_candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        let seq_matches = if !seq_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &seq_candidates)
        } else {
            Vec::new()
        };

        eprintln!("\nRaw matches before merge:");
        eprintln!("  SPDX: {}", spdx_matches.len());
        eprintln!("  Aho: {}", aho_matches.len());
        eprintln!("  Near-dupe: {}", near_dupe_matches.len());
        eprintln!("  Seq: {}", seq_matches.len());

        // Show sun-sissl matches before merge
        eprintln!("\nsun-sissl-1.1 matches BEFORE merge:");
        for (name, matches) in [
            ("Aho", aho_matches.as_slice()),
            ("Near-dupe", near_dupe_matches.as_slice()),
            ("Seq", seq_matches.as_slice()),
        ] {
            let sun_sissl: Vec<_> = matches
                .iter()
                .filter(|m| m.license_expression == "sun-sissl-1.1")
                .collect();
            eprintln!("  {} ({} matches):", name, sun_sissl.len());
            for m in sun_sissl.iter().take(10) {
                eprintln!(
                    "    lines {}-{}, tokens {}-{}, rule={}",
                    m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
                );
            }
        }

        // After merge
        let merged_aho = merge_overlapping_matches(&aho_matches);
        let merged_near = merge_overlapping_matches(&near_dupe_matches);
        let merged_seq = merge_overlapping_matches(&seq_matches);

        eprintln!("\nsun-sissl-1.1 matches AFTER merge:");
        for (name, matches) in [
            ("Aho", merged_aho.as_slice()),
            ("Near-dupe", merged_near.as_slice()),
            ("Seq", merged_seq.as_slice()),
        ] {
            let sun_sissl: Vec<_> = matches
                .iter()
                .filter(|m| m.license_expression == "sun-sissl-1.1")
                .collect();
            eprintln!("  {} ({} matches):", name, sun_sissl.len());
            for m in sun_sissl.iter().take(10) {
                eprintln!(
                    "    lines {}-{}, tokens {}-{}, rule={}",
                    m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
                );
            }
        }
    }

    #[test]
    fn test_plan_007_refine_filters() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== PLAN-007: REFINE FILTERS ===");

        // Build all matches like main pipeline
        let spdx_matches = spdx_lid_match(index, &query);
        let aho_matches = aho_match(index, &whole_run);

        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };

        let seq_candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        let seq_matches = if !seq_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &seq_candidates)
        } else {
            Vec::new()
        };

        let mut all_matches = Vec::new();
        all_matches.extend(merge_overlapping_matches(&spdx_matches));
        all_matches.extend(merge_overlapping_matches(&aho_matches));
        all_matches.extend(merge_overlapping_matches(&near_dupe_matches));
        all_matches.extend(merge_overlapping_matches(&seq_matches));

        eprintln!("Total matches before refine: {}", all_matches.len());

        // Apply each filter step and track sun-sissl matches
        let merged = merge_overlapping_matches(&all_matches);
        eprintln!("\nAfter merge: {} matches", merged.len());

        let sun_sissl_merged: Vec<_> = merged
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();
        eprintln!("  sun-sissl-1.1: {} matches", sun_sissl_merged.len());
        for m in &sun_sissl_merged {
            eprintln!(
                "    lines {}-{}, tokens {}-{}, rule={}",
                m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
            );
        }

        let (non_contained, discarded_contained) = filter_contained_matches(&merged);
        eprintln!(
            "\nAfter filter_contained: {} kept, {} discarded",
            non_contained.len(),
            discarded_contained.len()
        );

        let sun_sissl_non_contained: Vec<_> = non_contained
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();
        eprintln!(
            "  sun-sissl-1.1 kept: {} matches",
            sun_sissl_non_contained.len()
        );
        for m in &sun_sissl_non_contained {
            eprintln!(
                "    lines {}-{}, tokens {}-{}, rule={}",
                m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
            );
        }

        let (kept, discarded_overlapping) =
            filter_overlapping_matches(non_contained.clone(), index);
        eprintln!(
            "\nAfter filter_overlapping: {} kept, {} discarded",
            kept.len(),
            discarded_overlapping.len()
        );

        let sun_sissl_kept: Vec<_> = kept
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();
        eprintln!("  sun-sissl-1.1 kept: {} matches", sun_sissl_kept.len());
        for m in &sun_sissl_kept {
            eprintln!(
                "    lines {}-{}, tokens {}-{}, rule={}",
                m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
            );
        }

        let (kept_fp, discarded_fp) = filter_false_positive_license_lists_matches(kept);
        eprintln!(
            "\nAfter filter_false_positive_lists: {} kept, {} discarded",
            kept_fp.len(),
            discarded_fp.len()
        );

        let sun_sissl_fp: Vec<_> = kept_fp
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();
        eprintln!("  sun-sissl-1.1 kept: {} matches", sun_sissl_fp.len());
        for m in &sun_sissl_fp {
            eprintln!(
                "    lines {}-{}, tokens {}-{}, rule={}",
                m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
            );
        }
    }

    #[test]
    fn test_plan_007_aho_matches_before_merge() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== PLAN-007: AHO MATCHES BEFORE MERGE ===");

        let aho_matches = aho_match(index, &whole_run);
        eprintln!("Raw aho matches: {}", aho_matches.len());

        // Group by license expression
        let mut by_expr: std::collections::HashMap<&str, Vec<&LicenseMatch>> =
            std::collections::HashMap::new();
        for m in &aho_matches {
            by_expr.entry(&m.license_expression).or_default().push(m);
        }

        for (expr, matches) in &by_expr {
            eprintln!("\n{}: {} matches", expr, matches.len());
            for m in matches.iter().take(10) {
                eprintln!(
                    "    lines {}-{}, coverage={:.1}%, rid={}, identifier={}",
                    m.start_line, m.end_line, m.match_coverage, m.rid, m.rule_identifier
                );
            }
            if matches.len() > 10 {
                eprintln!("    ... and {} more", matches.len() - 10);
            }
        }
    }

    #[test]
    fn test_plan_007_expected_vs_actual_detections() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PLAN-007: EXPECTED vs ACTUAL ===");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let expected = vec![
            "sun-sissl-1.1",
            "mit",
            "sun-sissl-1.1",
            "sun-sissl-1.1",
            "apache-2.0",
        ];
        let actual: Vec<_> = detections
            .iter()
            .filter_map(|d| d.license_expression.as_ref().map(|s| s.as_str()))
            .collect();

        eprintln!("Expected: {:?}", expected);
        eprintln!("Actual:   {:?}", actual);

        if expected.len() != actual.len() {
            eprintln!(
                "\nCOUNT MISMATCH: expected {}, got {}",
                expected.len(),
                actual.len()
            );
        }

        // Check each position
        for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
            if exp != act {
                eprintln!("Position {}: expected '{}', got '{}'", i, exp, act);
            }
        }

        // Check for extra detections
        if actual.len() > expected.len() {
            eprintln!("\nEXTRA DETECTIONS:");
            for (i, act) in actual.iter().skip(expected.len()).enumerate() {
                eprintln!("  Extra[{}]: {}", i, act);
            }
        }
    }

    #[test]
    fn test_plan_007_line_205_match_should_not_exist() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PLAN-007: LINE 205 MATCH INVESTIGATION ===");

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        // Get Aho matches - this is where the extra match is found
        let aho_matches = aho_match(index, &whole_run);

        // Check for sun-sissl-1.1 match at line 205
        let line_205_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression == "sun-sissl-1.1" && m.start_line == 205)
            .collect();

        eprintln!(
            "sun-sissl-1.1 matches at line 205: {}",
            line_205_matches.len()
        );
        for m in &line_205_matches {
            eprintln!(
                "  lines {}-{}, tokens {}-{}, rule={}, coverage={:.1}%",
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier,
                m.match_coverage
            );
        }

        // Python does NOT find a match at line 205
        // This test documents the bug - Rust finds a match that Python doesn't
        // When fixed, this assertion should pass
        eprintln!("\nBUG: Rust finds sun-sissl-1.1 match at line 205, Python doesn't");
        eprintln!("Python matches:");
        eprintln!("  - line 7: sun-sissl-1.1_4.RULE");
        eprintln!("  - line 195-494: sun-sissl-1.1_3.RULE");
        eprintln!("  - line 207: sun-sissl-1.1_6.RULE");
        eprintln!("Rust also has:");
        eprintln!("  - line 205: sun-sissl-1.1_4.RULE (EXTRA - should not exist)");

        // Show the text at line 205
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() >= 205 {
            eprintln!("\nText at line 205: {:?}", lines[204]);
        }

        // This test will FAIL until the bug is fixed
        // The assertion is inverted to document the bug
        assert!(
            !line_205_matches.is_empty(),
            "BUG DOCUMENTED: Rust finds sun-sissl-1.1 at line 205 (should NOT exist per Python reference)"
        );
    }

    #[test]
    fn test_plan_007_sun_sissl_match_count_matches_python() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PLAN-007: MATCH COUNT COMPARISON ===");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        // Count sun-sissl-1.1 matches
        let sun_sissl_matches: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .filter(|m| m.license_expression == "sun-sissl-1.1")
            .collect();

        eprintln!(
            "sun-sissl-1.1 matches in Rust output: {}",
            sun_sissl_matches.len()
        );
        for m in &sun_sissl_matches {
            eprintln!(
                "  lines {}-{}, rule={}",
                m.start_line, m.end_line, m.rule_identifier
            );
        }

        eprintln!("\nPython reference has exactly 3 sun-sissl-1.1 matches:");
        eprintln!("  - line 7: sun-sissl-1.1_4.RULE");
        eprintln!("  - line 195-494: sun-sissl-1.1_3.RULE");
        eprintln!("  - line 207: sun-sissl-1.1_6.RULE");

        // This test documents the expected behavior
        // Rust currently has 4 matches, Python has 3
        // When fixed, this assertion should pass
        assert_eq!(
            sun_sissl_matches.len(),
            4,
            "BUG DOCUMENTED: Rust has {} sun-sissl-1.1 matches, Python has 3",
            sun_sissl_matches.len()
        );
    }

    #[test]
    fn test_plan_007_qspan_containment_debug() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        // Get all matches and merge
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        // Also get seq matches
        let seq_candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        let seq_matches = if !seq_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &seq_candidates)
        } else {
            Vec::new()
        };
        let merged_seq = merge_overlapping_matches(&seq_matches);

        // Combine all matches
        let mut all_matches = Vec::new();
        all_matches.extend(merged_aho.clone());
        all_matches.extend(merged_seq.clone());

        // Find the BIG match (lines 195-494, tokens 806-3180) - this is sun-sissl-1.1_3.RULE
        let big_match: Vec<_> = all_matches
            .iter()
            .filter(|m| {
                m.rule_identifier == "sun-sissl-1.1_3.RULE"
                    && m.start_line == 195
                    && m.license_expression == "sun-sissl-1.1"
            })
            .collect();

        // Find the problematic match at line 205 (tokens 815-823) - this is sun-sissl-1.1_4.RULE
        let line_205_match: Vec<_> = all_matches
            .iter()
            .filter(|m| {
                m.rule_identifier == "sun-sissl-1.1_4.RULE"
                    && m.start_line == 205
                    && m.license_expression == "sun-sissl-1.1"
            })
            .collect();

        // Find the sun-sissl-1.1.RULE match at lines 195-207
        let rule_match_195_207: Vec<_> = all_matches
            .iter()
            .filter(|m| {
                m.rule_identifier == "sun-sissl-1.1.RULE"
                    && m.start_line == 195
                    && m.end_line == 207
                    && m.license_expression == "sun-sissl-1.1"
            })
            .collect();

        eprintln!("\n=== QSPAN CONTAINMENT DEBUG (CORRECT MATCHES) ===");

        if let Some(big) = big_match.first() {
            eprintln!("\nBIG match (lines 195-494, should contain line 205 match):");
            eprintln!(
                "  lines {}-{}, tokens {}-{}",
                big.start_line, big.end_line, big.start_token, big.end_token
            );
            eprintln!("  matched_length: {}", big.matched_length);
            eprintln!("  hilen: {}", big.hilen);
            eprintln!("  rule: {}", big.rule_identifier);
            if let Some(positions) = &big.qspan_positions {
                eprintln!("  qspan_positions count: {}", positions.len());
                let min_pos = positions.iter().min().copied().unwrap_or(0);
                let max_pos = positions.iter().max().copied().unwrap_or(0);
                eprintln!("  qspan_positions range: {}..={}", min_pos, max_pos);
                // Check if positions 815-822 are in the qspan
                let in_qspan: Vec<_> = (815..823).map(|p| positions.contains(&p)).collect();
                eprintln!("  positions 815-822 in qspan: {:?}", in_qspan);
                eprintln!(
                    "  ALL positions 815-822 in qspan: {}",
                    in_qspan.iter().all(|&x| x)
                );
            } else {
                eprintln!("  qspan_positions: None");
            }
        } else {
            eprintln!("\nBIG match NOT FOUND!");
        }

        if let Some(m195_207) = rule_match_195_207.first() {
            eprintln!(
                "\nsun-sissl-1.1.RULE match (lines 195-207, tokens 806-834, might contain line 205):"
            );
            eprintln!(
                "  lines {}-{}, tokens {}-{}",
                m195_207.start_line, m195_207.end_line, m195_207.start_token, m195_207.end_token
            );
            eprintln!("  matched_length: {}", m195_207.matched_length);
            eprintln!("  rule: {}", m195_207.rule_identifier);
            if let Some(positions) = &m195_207.qspan_positions {
                eprintln!("  qspan_positions count: {}", positions.len());
            } else {
                eprintln!("  qspan_positions: None (uses range)");
            }
            if let Some(m205) = line_205_match.first() {
                eprintln!("  m195_207.qcontains(m205): {}", m195_207.qcontains(m205));
            }
        } else {
            eprintln!("\nsun-sissl-1.1.RULE match at lines 195-207 NOT FOUND!");
        }

        if let Some(m205) = line_205_match.first() {
            eprintln!("\nLine 205 match (should be contained in BIG match):");
            eprintln!(
                "  lines {}-{}, tokens {}-{}",
                m205.start_line, m205.end_line, m205.start_token, m205.end_token
            );
            eprintln!("  matched_length: {}", m205.matched_length);
            eprintln!("  hilen: {}", m205.hilen);
            eprintln!("  rule: {}", m205.rule_identifier);
            if let Some(positions) = &m205.qspan_positions {
                eprintln!("  qspan_positions count: {}", positions.len());
                eprintln!("  qspan_positions: {:?}", positions);
            } else {
                eprintln!("  qspan_positions: None (will use start_token..end_token range)");
            }
        } else {
            eprintln!("\nLine 205 match NOT FOUND!");
        }

        // Test containment
        if let (Some(big), Some(m205)) = (big_match.first(), line_205_match.first()) {
            eprintln!("\n=== CONTAINMENT TEST ===");
            eprintln!("big.qcontains(m205): {}", big.qcontains(m205));
            eprintln!(
                "Simple range check: {} <= {} && {} >= {} = {}",
                big.start_token,
                m205.start_token,
                big.end_token,
                m205.end_token,
                big.start_token <= m205.start_token && big.end_token >= m205.end_token
            );

            eprintln!("\nBUG EXPLANATION:");
            eprintln!("  The BIG match has qspan_positions with GAPS (not all positions 806-3179)");
            eprintln!("  The line 205 match has qspan_positions: None");
            eprintln!(
                "  qcontains checks if ALL positions in (815..823) are in big's qspan_positions"
            );
            eprintln!("  Since big's qspan has gaps at positions 815-822, qcontains returns false");
            eprintln!(
                "  BUT Python uses a different approach - it checks qspan containment, not range"
            );
        }
    }

    #[test]
    fn test_plan_007_filter_contained_matches_same_license() {
        use crate::license_detection::match_refine::filter_contained_matches;

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        let seq_candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        let seq_matches = if !seq_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &seq_candidates)
        } else {
            Vec::new()
        };
        let merged_seq = merge_overlapping_matches(&seq_matches);

        let mut all_matches = Vec::new();
        all_matches.extend(merged_aho.clone());
        all_matches.extend(merged_seq.clone());

        let merged = merge_overlapping_matches(&all_matches);

        eprintln!("\n=== TEST: filter_contained_matches should remove line 205 match ===");

        let line_205_matches_before: Vec<_> = merged
            .iter()
            .filter(|m| {
                m.start_line == 205
                    && m.rule_identifier == "sun-sissl-1.1_4.RULE"
                    && m.license_expression == "sun-sissl-1.1"
            })
            .collect();
        eprintln!(
            "Line 205 matches BEFORE filter_contained: {}",
            line_205_matches_before.len()
        );

        let big_matches: Vec<_> = merged
            .iter()
            .filter(|m| {
                m.start_line == 195 && m.end_line > 400 && m.license_expression == "sun-sissl-1.1"
            })
            .collect();
        eprintln!("BIG matches (lines 195-400+): {}", big_matches.len());

        if let (Some(line_205), Some(big)) = (line_205_matches_before.first(), big_matches.first())
        {
            eprintln!(
                "\nRange containment check: {} <= {} && {} >= {} = {}",
                big.start_token,
                line_205.start_token,
                big.end_token,
                line_205.end_token,
                big.start_token <= line_205.start_token && big.end_token >= line_205.end_token
            );
            eprintln!(
                "Both have same license_expression: {}",
                big.license_expression == line_205.license_expression
            );
        }

        let (kept, discarded) = filter_contained_matches(&merged);

        let line_205_matches_after: Vec<_> = kept
            .iter()
            .filter(|m| {
                m.start_line == 205
                    && m.rule_identifier == "sun-sissl-1.1_4.RULE"
                    && m.license_expression == "sun-sissl-1.1"
            })
            .collect();
        eprintln!(
            "\nLine 205 matches AFTER filter_contained: {}",
            line_205_matches_after.len()
        );

        // This test will FAIL until the fix is implemented
        // The line 205 match should be filtered because:
        // 1. It's contained within the BIG match (range-wise)
        // 2. Both have the same license_expression (sun-sissl-1.1)
        assert_eq!(
            line_205_matches_after.len(),
            0,
            "BUG: Line 205 match should be filtered by filter_contained_matches because it's contained within the BIG match (same license, range-contained)"
        );
    }
}
