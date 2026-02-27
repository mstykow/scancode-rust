//! Investigation tests for PLAN-004: ijg.txt extra detections
//!
//! Issue: Multiple extra detections: warranty-disclaimer, extra ijg, and free-unknown.
//!
//! Expected (Python): `["ijg"]` - single match, lines 12-96, matcher=3-seq, coverage=99.56%
//! Actual (Rust): `["ijg", "warranty-disclaimer", "ijg", "free-unknown", "free-unknown"]`

#[cfg(test)]
mod tests {
    use crate::license_detection::aho_match::aho_match;
    use crate::license_detection::hash_match::hash_match;
    use crate::license_detection::match_refine::{
        filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
        refine_matches, refine_matches_without_false_positive_filter, split_weak_matches,
    };
    use crate::license_detection::query::Query;
    use crate::license_detection::seq_match::{
        compute_candidates_with_msets, seq_match, seq_match_with_candidates,
        MAX_NEAR_DUPE_CANDIDATES,
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

    fn read_ijg_test_file() -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic4/ijg.txt");
        std::fs::read_to_string(&path).ok()
    }

    /// Python reference result: single ijg match, lines 12-96, matcher=3-seq
    #[test]
    fn test_ijg_python_reference_result() {
        let Some(_engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        eprintln!("\n=== Python Reference Result (from scancode CLI) ===");
        eprintln!("Number of detections: 1");
        eprintln!("Detection[0]: ijg");
        eprintln!("  Match: ijg.LICENSE (lines=12-96, matcher=3-seq, coverage=99.56%)");
        eprintln!("\n=== Expected vs Actual ===");
        eprintln!("EXPECTED: 1 detection, 1 match");
        eprintln!("ACTUAL: 3 detections, 5 matches");
    }

    /// Full detection test to show the current behavior
    #[test]
    fn test_ijg_full_detection() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== Full Detection Results ===");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("Number of detections: {}", detections.len());
        for (i, d) in detections.iter().enumerate() {
            eprintln!("Detection[{}]: {:?}", i, d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "  Match: {} (rule={}, lines={}-{}, tokens={}-{}, matcher={}, coverage={:.1}%)",
                    m.license_expression,
                    m.rule_identifier,
                    m.start_line,
                    m.end_line,
                    m.start_token,
                    m.end_token,
                    m.matcher,
                    m.match_coverage
                );
            }
        }

        let expressions: Vec<_> = detections
            .iter()
            .filter_map(|d| d.license_expression.as_ref())
            .collect();
        eprintln!("\nFinal expressions: {:?}", expressions);

        // FAILING ASSERTION: Should be 1 detection with "ijg"
        assert_eq!(
            detections.len(),
            1,
            "Should have 1 detection, got {}",
            detections.len()
        );
        assert_eq!(
            expressions,
            vec!["ijg"],
            "Should have single 'ijg' expression, got {:?}",
            expressions
        );
    }

    /// Step-by-step pipeline trace to find divergence point
    #[test]
    fn test_ijg_pipeline_trace() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PIPELINE TRACE FOR ijg.txt ===");
        eprintln!(
            "Text length: {} bytes, {} lines",
            text.len(),
            text.lines().count()
        );

        let index = &engine.index;
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        eprintln!("Query tokens: {}", query.tokens.len());

        // Step 1: Hash matching
        let hash_matches = hash_match(index, &query_run);
        eprintln!("\n--- Step 1: Hash matching ---");
        eprintln!("Hash matches: {}", hash_matches.len());

        // Step 2: SPDX-LID matching
        let spdx_matches = spdx_lid_match(index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        eprintln!("\n--- Step 2: SPDX-LID matching ---");
        eprintln!(
            "SPDX-LID matches: {} (merged: {})",
            spdx_matches.len(),
            merged_spdx.len()
        );

        // Step 3: Aho-Corasick matching
        let aho_matches = aho_match(index, &query_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        eprintln!("\n--- Step 3: Aho-Corasick matching ---");
        eprintln!(
            "Aho matches: {} (merged: {})",
            aho_matches.len(),
            merged_aho.len()
        );
        for m in merged_aho.iter().take(15) {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }
        if merged_aho.len() > 15 {
            eprintln!("  ... and {} more", merged_aho.len() - 15);
        }

        // Step 4: Near-duplicate candidate selection
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &query_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("\n--- Step 4: Near-duplicate candidates ---");
        eprintln!("Near-dupe candidates: {}", near_dupe_candidates.len());
        for c in near_dupe_candidates.iter().take(10) {
            let rule = &index.rules_by_rid[c.rid];
            eprintln!(
                "  {} (rid={}, resemblance={:.3}, containment={:.3})",
                rule.identifier, c.rid, c.score_vec_full.resemblance, c.score_vec_full.containment
            );
        }

        // Step 5: Sequence matching with near-dupe candidates
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &query_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);
        eprintln!("\n--- Step 5: Near-dupe sequence matching ---");
        eprintln!(
            "Near-dupe matches: {} (merged: {})",
            near_dupe_matches.len(),
            merged_near_dupe.len()
        );
        for m in merged_near_dupe.iter().take(15) {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        // Step 6: Regular sequence matching (no candidates)
        let seq_matches = seq_match(index, &query_run);
        let merged_seq = merge_overlapping_matches(&seq_matches);
        eprintln!("\n--- Step 6: Regular sequence matching ---");
        eprintln!(
            "Seq matches: {} (merged: {})",
            seq_matches.len(),
            merged_seq.len()
        );
        for m in merged_seq.iter().take(10) {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        // Combine all matches
        let mut all_matches = Vec::new();
        all_matches.extend(merged_spdx.clone());
        all_matches.extend(merged_aho.clone());
        all_matches.extend(merged_near_dupe.clone());
        all_matches.extend(merged_seq.clone());

        eprintln!("\n--- Step 7: All matches combined ---");
        eprintln!("Total matches before refine: {}", all_matches.len());

        // Step 8: First refine (without FP filter)
        let refined_initial =
            refine_matches_without_false_positive_filter(index, all_matches.clone(), &query);
        eprintln!("\n--- Step 8: Initial refine (no FP filter) ---");
        eprintln!("Refined matches: {}", refined_initial.len());
        for m in refined_initial.iter().take(15) {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        // Step 9: Split weak matches
        let (good_matches, weak_matches) = split_weak_matches(&refined_initial);
        eprintln!("\n--- Step 9: Split weak matches ---");
        eprintln!("Good matches: {}", good_matches.len());
        eprintln!("Weak matches: {}", weak_matches.len());
        for m in weak_matches.iter().take(10) {
            eprintln!(
                "  WEAK: {} (rule={}, lines={}-{}, coverage={:.1}%)",
                m.license_expression, m.rule_identifier, m.start_line, m.end_line, m.match_coverage
            );
        }

        // Step 10: Unknown matching
        let unknown_matches = unknown_match(index, &query, &good_matches);
        eprintln!("\n--- Step 10: Unknown matching ---");
        eprintln!("Unknown matches: {}", unknown_matches.len());

        // Combine good + unknown
        let mut combined = good_matches.clone();
        combined.extend(unknown_matches.clone());

        // Step 11: Final refine (with FP filter)
        let refined_final = refine_matches(index, combined.clone(), &query);
        eprintln!("\n--- Step 11: Final refine (with FP filter) ---");
        eprintln!("Final refined matches: {}", refined_final.len());
        for m in refined_final.iter() {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        // Final check
        eprintln!("\n=== COMPARISON ===");
        eprintln!("Python: 1 match (ijg.LICENSE, lines 12-96)");
        eprintln!("Rust: {} matches", refined_final.len());

        let ijg_matches: Vec<_> = refined_final
            .iter()
            .filter(|m| m.license_expression == "ijg")
            .collect();
        let warranty_matches: Vec<_> = refined_final
            .iter()
            .filter(|m| m.license_expression == "warranty-disclaimer")
            .collect();
        let free_unknown_matches: Vec<_> = refined_final
            .iter()
            .filter(|m| m.license_expression == "free-unknown")
            .collect();

        eprintln!("  ijg matches: {}", ijg_matches.len());
        eprintln!("  warranty-disclaimer matches: {}", warranty_matches.len());
        eprintln!("  free-unknown matches: {}", free_unknown_matches.len());
    }

    /// Focus on the key divergence: Why doesn't the large ijg match cover warranty-disclaimer?
    #[test]
    fn test_ijg_warranty_containment() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== CONTAINMENT CHECK: ijg vs warranty-disclaimer ===");

        let index = &engine.index;
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        // Get aho matches
        let aho_matches = aho_match(index, &query_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        // Find ijg and warranty-disclaimer matches
        let ijg_matches: Vec<_> = merged_aho
            .iter()
            .filter(|m| m.license_expression == "ijg")
            .collect();
        let warranty_matches: Vec<_> = merged_aho
            .iter()
            .filter(|m| m.license_expression == "warranty-disclaimer")
            .collect();

        eprintln!("\nijg aho matches: {}", ijg_matches.len());
        for m in &ijg_matches {
            eprintln!(
                "  lines={}-{}, tokens={}-{}, rule={}",
                m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
            );
        }

        eprintln!(
            "\nwarranty-disclaimer aho matches: {}",
            warranty_matches.len()
        );
        for m in &warranty_matches {
            eprintln!(
                "  lines={}-{}, tokens={}-{}, rule={}",
                m.start_line, m.end_line, m.start_token, m.end_token, m.rule_identifier
            );
        }

        // Check if warranty-disclaimer lines 26-29 are covered by any ijg match
        // The text shows the IJG license starts at line 12 ("LEGAL ISSUES") and goes to line ~59
        eprintln!("\n=== EXPECTED COVERAGE ===");
        eprintln!("Python's ijg.LICENSE match: lines 12-96");
        eprintln!("Rust's ijg matches: fragmented");
        eprintln!("warranty-disclaimer: lines 26-29");
        eprintln!("Is warranty-disclaimer covered by ijg? Should be YES if ijg covers lines 12-96");

        // Check qcontains
        if let (Some(ijg), Some(warranty)) = (ijg_matches.first(), warranty_matches.first()) {
            eprintln!("\nqcontains check:");
            eprintln!("  ijg.qcontains(warranty): {}", ijg.qcontains(warranty));
            eprintln!("  warranty.qcontains(ijg): {}", warranty.qcontains(ijg));
        }
    }

    /// Check the near-dupe seq matching specifically for ijg.LICENSE
    #[test]
    fn test_ijg_near_dupe_matching() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== NEAR-DUPE SEQ MATCHING FOR ijg.LICENSE ===");

        let index = &engine.index;
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        // Find ijg.LICENSE rule
        let ijg_license_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "ijg.LICENSE");
        eprintln!("ijg.LICENSE rid: {:?}", ijg_license_rid);

        // Get near-dupe candidates
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &query_run, true, MAX_NEAR_DUPE_CANDIDATES);

        eprintln!("\nNear-dupe candidates (looking for ijg.LICENSE):");
        for (pos, c) in near_dupe_candidates.iter().enumerate() {
            let rule = &index.rules_by_rid[c.rid];
            if rule.identifier.contains("ijg") {
                eprintln!(
                    "  [{}] {} (rid={}, resemblance={:.3}, containment={:.3})",
                    pos,
                    rule.identifier,
                    c.rid,
                    c.score_vec_full.resemblance,
                    c.score_vec_full.containment
                );
            }
        }

        // Run seq match with candidates
        let near_dupe_matches = seq_match_with_candidates(index, &query_run, &near_dupe_candidates);
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        eprintln!("\nijg near-dupe seq matches:");
        for m in merged_near_dupe
            .iter()
            .filter(|m| m.license_expression == "ijg")
        {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        // Compare with Python's result
        eprintln!("\n=== COMPARISON ===");
        eprintln!("Python ijg.LICENSE match: lines 12-96, coverage=99.56%");
        eprintln!("Rust should produce a similar large ijg match");
    }

    /// Check the seq_match output directly (without near-dupe candidates)
    #[test]
    fn test_ijg_regular_seq_match() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== REGULAR SEQ MATCH FOR ijg.txt ===");

        let index = &engine.index;
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        // Run regular seq match
        let seq_matches = seq_match(index, &query_run);
        let merged_seq = merge_overlapping_matches(&seq_matches);

        eprintln!(
            "Seq matches: {} (merged: {})",
            seq_matches.len(),
            merged_seq.len()
        );

        // Find ijg matches
        let ijg_seq_matches: Vec<_> = merged_seq
            .iter()
            .filter(|m| m.license_expression == "ijg")
            .collect();

        eprintln!("\nijg seq matches: {}", ijg_seq_matches.len());
        for m in &ijg_seq_matches {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        // Find the largest ijg match
        if let Some(largest) = ijg_seq_matches.iter().max_by_key(|m| m.matched_length) {
            eprintln!(
                "\nLargest ijg seq match: lines {}-{}, coverage={:.1}%",
                largest.start_line, largest.end_line, largest.match_coverage
            );
        }
    }

    /// FAILING TEST: After pipeline, should have single ijg detection
    #[test]
    fn test_ijg_should_have_single_detection() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        // FAILING ASSERTION: This will fail until the bug is fixed
        assert_eq!(
            detections.len(),
            1,
            "Expected 1 detection, got {}",
            detections.len()
        );

        let detection = &detections[0];
        assert_eq!(
            detection.license_expression.as_deref(),
            Some("ijg"),
            "Expected 'ijg' expression, got {:?}",
            detection.license_expression
        );

        // Should have one match (or combined matches that express as single ijg)
        assert_eq!(
            detection.matches.len(),
            1,
            "Expected 1 match in detection, got {}",
            detection.matches.len()
        );

        let match_info = &detection.matches[0];
        assert_eq!(match_info.license_expression, "ijg");
        assert_eq!(match_info.matcher, "3-seq");
        assert_eq!(match_info.start_line, 12);
        assert_eq!(match_info.end_line, 96);
    }

    /// FAILING TEST: warranty-disclaimer should be contained within ijg match
    #[test]
    fn test_ijg_warranty_should_be_contained() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = &engine.index;
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        // Get all matches after pipeline
        let aho_matches = aho_match(index, &query_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        let near_dupe_candidates =
            compute_candidates_with_msets(index, &query_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = seq_match_with_candidates(index, &query_run, &near_dupe_candidates);
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        let mut all_matches = merged_aho.clone();
        all_matches.extend(merged_near_dupe);

        let refined = refine_matches_without_false_positive_filter(index, all_matches, &query);

        // After filter_contained_matches, warranty-disclaimer should be discarded
        // if ijg covers it
        let (kept, discarded) = filter_contained_matches(&refined);

        let warranty_in_kept: Vec<_> = kept
            .iter()
            .filter(|m| m.license_expression == "warranty-disclaimer")
            .collect();
        let warranty_in_discarded: Vec<_> = discarded
            .iter()
            .filter(|m| m.license_expression == "warranty-disclaimer")
            .collect();

        eprintln!("\n=== warranty-disclaimer after filter_contained ===");
        eprintln!("Kept: {}", warranty_in_kept.len());
        eprintln!("Discarded: {}", warranty_in_discarded.len());

        // FAILING ASSERTION: warranty-disclaimer should be contained within ijg
        // and thus discarded
        assert!(
            warranty_in_kept.is_empty(),
            "warranty-disclaimer should be contained in ijg and discarded, but {} kept",
            warranty_in_kept.len()
        );
    }

    /// FAILING TEST: free-unknown should not appear in final detections
    #[test]
    fn test_ijg_no_free_unknown() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let free_unknown_count = detections
            .iter()
            .filter(|d| d.license_expression.as_deref() == Some("free-unknown"))
            .count();

        // FAILING ASSERTION: free-unknown should not be detected
        assert_eq!(
            free_unknown_count, 0,
            "Expected 0 free-unknown detections, got {}",
            free_unknown_count
        );
    }

    /// CRITICAL DIVERGENCE TEST: Compare what the engine produces vs what manual pipeline produces
    ///
    /// The key difference:
    /// - Manual pipeline: near-dupe produces ijg.LICENSE (lines 12-96), which survives refining
    /// - Engine: produces fragmented matches (ijg_26.RULE, warranty-disclaimer, ijg_9.RULE)
    #[test]
    fn test_ijg_engine_vs_manual_pipeline() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== CRITICAL DIVERGENCE TEST ===");

        // Run full engine
        let engine_detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\nEngine detections: {}", engine_detections.len());
        for d in &engine_detections {
            eprintln!("  {}", d.license_expression.as_deref().unwrap_or("none"));
            for m in &d.matches {
                eprintln!(
                    "    {} (rule={}, lines={}-{}, matcher={})",
                    m.license_expression, m.rule_identifier, m.start_line, m.end_line, m.matcher
                );
            }
        }

        // Run manual pipeline
        let index = &engine.index;
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        // Get near-dupe matches ONLY (like the test that worked)
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &query_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = seq_match_with_candidates(index, &query_run, &near_dupe_candidates);
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        eprintln!("\nManual near-dupe matches: {}", merged_near_dupe.len());
        for m in &merged_near_dupe {
            eprintln!(
                "  {} (rule={}, lines={}-{}, coverage={:.1}%)",
                m.license_expression, m.rule_identifier, m.start_line, m.end_line, m.match_coverage
            );
        }

        // Get aho matches
        let aho_matches = aho_match(index, &query_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        eprintln!("\nAho matches: {}", merged_aho.len());
        for m in &merged_aho {
            eprintln!(
                "  {} (rule={}, lines={}-{})",
                m.license_expression, m.rule_identifier, m.start_line, m.end_line
            );
        }

        // Combine: just aho + near-dupe (no regular seq)
        let mut combined = merged_aho.clone();
        combined.extend(merged_near_dupe.clone());
        eprintln!("\nCombined (aho + near-dupe): {} matches", combined.len());

        // Refine
        let refined = refine_matches_without_false_positive_filter(index, combined, &query);
        eprintln!("After refine: {} matches", refined.len());
        for m in &refined {
            eprintln!(
                "  {} (rule={}, lines={}-{}, coverage={:.1}%)",
                m.license_expression, m.rule_identifier, m.start_line, m.end_line, m.match_coverage
            );
        }

        eprintln!("\n=== KEY FINDING ===");
        eprintln!("If refine produces ijg.LICENSE (lines 12-96), the issue is in how the engine combines matches.");
        eprintln!("If refine produces fragmented matches, the issue is in the refine logic.");
    }

    /// CRITICAL TEST: Why does engine NOT produce ijg.LICENSE match?
    /// The engine's near-dupe matching should find ijg.LICENSE but it doesn't show up in final.
    #[test]
    fn test_ijg_near_dupe_subtraction_issue() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== NEAR-DUPE SUBTRACTION ISSUE ===");

        let index = &engine.index;
        let mut query = Query::new(&text, index).expect("Query should be created");
        let original_whole_run = query.whole_query_run();

        eprintln!(
            "Original whole_run: tokens {}-{:?}",
            original_whole_run.start, original_whole_run.end
        );

        // Simulate what the engine does: run near-dupe and subtract spans
        let near_dupe_candidates = compute_candidates_with_msets(
            index,
            &original_whole_run,
            true,
            MAX_NEAR_DUPE_CANDIDATES,
        );

        eprintln!("\nNear-dupe candidates: {}", near_dupe_candidates.len());
        for c in &near_dupe_candidates {
            let rule = &index.rules_by_rid[c.rid];
            eprintln!(
                "  {} (rid={}, resemblance={:.3})",
                rule.identifier, c.rid, c.score_vec_full.resemblance
            );
        }

        let near_dupe_matches =
            seq_match_with_candidates(index, &original_whole_run, &near_dupe_candidates);
        eprintln!(
            "\nNear-dupe matches before merge: {}",
            near_dupe_matches.len()
        );
        for m in near_dupe_matches.iter().take(5) {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{})",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token
            );
        }

        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);
        eprintln!(
            "\nNear-dupe matches after merge: {}",
            merged_near_dupe.len()
        );
        for m in &merged_near_dupe {
            eprintln!(
                "  {} (rule={}, lines={}-{}, tokens={}-{}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        // Now subtract the spans (like the engine does)
        let mut subtracted_spans: Vec<crate::license_detection::query::PositionSpan> = Vec::new();
        for m in &near_dupe_matches {
            if m.end_token > m.start_token {
                let span = crate::license_detection::query::PositionSpan::new(
                    m.start_token,
                    m.end_token - 1,
                );
                query.subtract(&span);
                subtracted_spans.push(span);
            }
        }

        eprintln!("\nSubtracted {} spans", subtracted_spans.len());

        // Get the NEW whole_run after subtraction
        let new_whole_run = query.whole_query_run();
        eprintln!(
            "\nAfter subtraction, whole_run: tokens {}-{:?}",
            new_whole_run.start, new_whole_run.end
        );

        // Run regular seq matching on the SUBTRACTED query
        const MAX_SEQ_CANDIDATES: usize = 70;
        let seq_candidates =
            compute_candidates_with_msets(index, &new_whole_run, false, MAX_SEQ_CANDIDATES);

        eprintln!(
            "\nSeq candidates on subtracted query: {}",
            seq_candidates.len()
        );

        let seq_matches = seq_match_with_candidates(index, &new_whole_run, &seq_candidates);
        eprintln!("Seq matches: {}", seq_matches.len());

        // Combine all
        let mut all_matches = Vec::new();
        all_matches.extend(merged_near_dupe.clone());
        all_matches.extend(seq_matches.clone());

        eprintln!("\nAll matches before refine: {}", all_matches.len());

        // Refine (need to re-create query since we mutated it)
        let query2 = Query::new(&text, index).expect("Query should be created");
        let refined = refine_matches_without_false_positive_filter(index, all_matches, &query2);

        eprintln!("\nAfter refine: {} matches", refined.len());
        for m in &refined {
            eprintln!(
                "  {} (rule={}, lines={}-{}, coverage={:.1}%)",
                m.license_expression, m.rule_identifier, m.start_line, m.end_line, m.match_coverage
            );
        }

        eprintln!("\n=== KEY INSIGHT ===");
        eprintln!("If ijg.LICENSE is in near-dupe matches but NOT in final refined, the issue is in refine.");
        eprintln!("If ijg.LICENSE is NOT in near-dupe matches, the issue is in seq_match_with_candidates.");
    }

    /// TRACE ENGINE BEHAVIOR: What does the engine actually produce at each phase?
    /// This replicates the EXACT engine logic to find the divergence.
    #[test]
    fn test_ijg_exact_engine_trace() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== EXACT ENGINE TRACE ===");

        let index = &engine.index;
        let mut query = Query::new(&text, index).expect("Query should be created");
        let mut matched_qspans: Vec<crate::license_detection::query::PositionSpan> = Vec::new();

        // Phase 1a: Hash matching (skip - empty for this file)
        // Phase 1b: SPDX-LID matching (skip - empty for this file)

        // Phase 1c: Aho-Corasick matching (EXACTLY like the engine)
        {
            let whole_run = query.whole_query_run();
            let aho_matches = aho_match(index, &whole_run);
            let merged_aho = merge_overlapping_matches(&aho_matches);

            eprintln!("\n=== Phase 1c: Aho matching ===");
            eprintln!("Aho matches: {}", merged_aho.len());

            for m in &merged_aho {
                if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                    matched_qspans.push(crate::license_detection::query::PositionSpan::new(
                        m.start_token,
                        m.end_token - 1,
                    ));
                }
                if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                    let span = crate::license_detection::query::PositionSpan::new(
                        m.start_token,
                        m.end_token.saturating_sub(1),
                    );
                    query.subtract(&span);
                }
            }
            eprintln!("matched_qspans after aho: {}", matched_qspans.len());
        }

        // Check is_matchable (like engine line 211)
        let whole_run = query.whole_query_run();
        let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
        eprintln!("\nskip_seq_matching: {}", skip_seq_matching);

        // Phase 2: Near-duplicate detection (EXACTLY like the engine)
        let mut seq_all_matches = Vec::new();
        if !skip_seq_matching {
            // Phase 2: Near-dupe
            {
                let whole_run = query.whole_query_run();
                let near_dupe_candidates = compute_candidates_with_msets(
                    index,
                    &whole_run,
                    true,
                    MAX_NEAR_DUPE_CANDIDATES,
                );

                eprintln!("\n=== Phase 2: Near-dupe candidates ===");
                eprintln!("Candidates: {}", near_dupe_candidates.len());
                for c in near_dupe_candidates.iter().take(5) {
                    let rule = &index.rules_by_rid[c.rid];
                    eprintln!(
                        "  {} (resemblance={:.3})",
                        rule.identifier, c.score_vec_full.resemblance
                    );
                }

                if !near_dupe_candidates.is_empty() {
                    let near_dupe_matches =
                        seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);

                    eprintln!(
                        "\nNear-dupe matches (before span subtraction): {}",
                        near_dupe_matches.len()
                    );
                    for m in near_dupe_matches.iter().take(5) {
                        eprintln!(
                            "  {} (rule={}, lines={}-{}, tokens={}-{})",
                            m.license_expression,
                            m.rule_identifier,
                            m.start_line,
                            m.end_line,
                            m.start_token,
                            m.end_token
                        );
                    }

                    // CRITICAL: This is where the engine subtracts spans
                    for m in &near_dupe_matches {
                        if m.end_token > m.start_token {
                            let span = crate::license_detection::query::PositionSpan::new(
                                m.start_token,
                                m.end_token - 1,
                            );
                            query.subtract(&span);
                            matched_qspans.push(span);
                        }
                    }

                    seq_all_matches.extend(near_dupe_matches);
                }
            }

            // Phase 3: Regular sequence matching
            const MAX_SEQ_CANDIDATES: usize = 70;
            {
                let whole_run = query.whole_query_run(); // NEW whole_run after subtraction!

                eprintln!("\n=== Phase 3: Regular seq (whole_run after subtraction) ===");
                eprintln!(
                    "whole_run: start={} end={:?}",
                    whole_run.start, whole_run.end
                );

                let candidates =
                    compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
                eprintln!("Candidates: {}", candidates.len());

                if !candidates.is_empty() {
                    let matches = seq_match_with_candidates(index, &whole_run, &candidates);
                    eprintln!("Seq matches: {}", matches.len());
                    for m in matches.iter().take(5) {
                        eprintln!(
                            "  {} (rule={}, lines={}-{})",
                            m.license_expression, m.rule_identifier, m.start_line, m.end_line
                        );
                    }
                    seq_all_matches.extend(matches);
                }
            }

            // Phase 4: Query run matching (skip for now - usually empty)

            // Merge ONCE
            let merged_seq = merge_overlapping_matches(&seq_all_matches);
            eprintln!("\nSeq matches (merged): {}", merged_seq.len());

            // Add to all_matches (we need to add aho too)
            let aho_matches = aho_match(index, &query.whole_query_run());
            let merged_aho = merge_overlapping_matches(&aho_matches);

            let mut all_matches = Vec::new();
            all_matches.extend(merged_aho);
            all_matches.extend(merged_seq);

            eprintln!("\nAll matches before refine: {}", all_matches.len());

            // Refine
            let refined = refine_matches_without_false_positive_filter(index, all_matches, &query);

            eprintln!("\n=== After refine ===");
            eprintln!("Refined matches: {}", refined.len());
            for m in refined.iter() {
                eprintln!(
                    "  {} (rule={}, lines={}-{}, coverage={:.1}%)",
                    m.license_expression,
                    m.rule_identifier,
                    m.start_line,
                    m.end_line,
                    m.match_coverage
                );
            }
        }
    }

    /// ISOLATED TEST: Check if matched_qspans affect near-dupe candidates.
    #[test]
    fn test_ijg_near_dupe_with_matched_qspans() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== TESTING NEAR-DUPE WITH MATCHED_QSPANS ===");

        let index = &engine.index;
        let query = Query::new(&text, index).expect("Query should be created");
        let whole_run = query.whole_query_run();

        // Get aho matches (like engine does)
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        // Check if any aho matches have is_license_text=true
        eprintln!("\n=== Aho matches detail ===");
        for m in &merged_aho {
            eprintln!(
                "  {} (rule={}, lines={}-{}, is_license_text={}, rule_length={}, coverage={:.1}%)",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.is_license_text,
                m.rule_length,
                m.match_coverage
            );
        }

        // Build matched_qspans from 100% coverage aho matches
        let mut matched_qspans: Vec<crate::license_detection::query::PositionSpan> = Vec::new();
        for m in &merged_aho {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(crate::license_detection::query::PositionSpan::new(
                    m.start_token,
                    m.end_token - 1,
                ));
            }
        }
        eprintln!("\nmatched_qspans: {}", matched_qspans.len());

        // Check is_matchable
        let is_matchable = whole_run.is_matchable(false, &matched_qspans);
        eprintln!(
            "is_matchable(include_low=false, matched_qspans): {}",
            is_matchable
        );

        // CRITICAL: Get matchable_tokens count
        let matchable_tokens = whole_run.matchable_tokens();
        eprintln!("matchable_tokens count: {}", matchable_tokens.len());

        // Get high matchables count
        let high_matchables = whole_run.high_matchables();
        eprintln!("high_matchables count: {}", high_matchables.len());

        // Compute candidates
        let candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("\nNear-dupe candidates: {}", candidates.len());
        for c in candidates.iter().take(5) {
            let rule = &index.rules_by_rid[c.rid];
            eprintln!(
                "  {} (resemblance={:.3})",
                rule.identifier, c.score_vec_full.resemblance
            );
        }

        eprintln!("\n=== KEY FINDING ===");
        if candidates.is_empty() && !is_matchable {
            eprintln!("CRITICAL: is_matchable=false means skip_seq_matching should be true!");
            eprintln!("This means near-dupe matching is skipped when aho matches cover all high matchables.");
        }
    }

    /// CHECK QUERY SUBTRACTION: Does the engine subtract from query during aho?
    #[test]
    fn test_ijg_query_subtraction_check() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_ijg_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== QUERY SUBTRACTION CHECK ===");

        let index = &engine.index;
        let mut query = Query::new(&text, index).expect("Query should be created");
        let original_whole_run = query.whole_query_run();
        eprintln!(
            "Original whole_run: start={} end={:?}",
            original_whole_run.start, original_whole_run.end
        );

        // Get aho matches
        let aho_matches = aho_match(index, &original_whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        // Check what happens with query subtraction
        for m in &merged_aho {
            if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                eprintln!(
                    "SUBTRACTING: {} (rule={}, lines={}-{}, rule_length={}, coverage={:.1}%)",
                    m.license_expression,
                    m.rule_identifier,
                    m.start_line,
                    m.end_line,
                    m.rule_length,
                    m.match_coverage
                );
                let span = crate::license_detection::query::PositionSpan::new(
                    m.start_token,
                    m.end_token.saturating_sub(1),
                );
                query.subtract(&span);
            }
        }

        let after_subtraction_whole_run = query.whole_query_run();
        eprintln!(
            "\nAfter subtraction whole_run: start={} end={:?}",
            after_subtraction_whole_run.start, after_subtraction_whole_run.end
        );

        // Now compute near-dupe candidates
        let candidates = compute_candidates_with_msets(
            index,
            &after_subtraction_whole_run,
            true,
            MAX_NEAR_DUPE_CANDIDATES,
        );
        eprintln!(
            "\nNear-dupe candidates after subtraction: {}",
            candidates.len()
        );
        for c in candidates.iter().take(5) {
            let rule = &index.rules_by_rid[c.rid];
            eprintln!(
                "  {} (resemblance={:.3})",
                rule.identifier, c.score_vec_full.resemblance
            );
        }
    }
}
