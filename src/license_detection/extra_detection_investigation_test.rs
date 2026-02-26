//! Investigation tests for PLAN-062: Extra Detections Investigation
//!
//! This module traces through the license detection pipeline to find where
//! extra detections are created that don't appear in Python output.

#[cfg(test)]
mod tests {
    use crate::license_detection::aho_match::aho_match;
    use crate::license_detection::hash_match::hash_match;
    use crate::license_detection::match_refine::{
        filter_contained_matches, filter_false_positive_license_lists_matches,
        filter_overlapping_matches, merge_overlapping_matches, refine_matches,
    };
    use crate::license_detection::models::LicenseMatch;
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

    fn print_match_summary(m: &LicenseMatch, prefix: &str) {
        eprintln!(
            "{}: {} (rid={}, matcher={}, coverage={:.1}%, lines={}-{})",
            prefix,
            m.license_expression,
            m.rid,
            m.matcher,
            m.match_coverage,
            m.start_line,
            m.end_line
        );
    }

    fn read_test_file(name: &str) -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic1").join(name);
        std::fs::read_to_string(&path).ok()
    }

    fn read_unknown_test_file(name: &str) -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/unknown").join(name);
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_gfdl_11_gnome_full_detection() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== FULL DETECTION for gfdl-1.1-en_gnome_1.RULE ===");
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
        eprintln!("EXPECTED: 2 expressions (gfdl-1.1, gfdl-1.1-plus)");
        eprintln!("ACTUAL: {} expressions", expressions.len());
    }

    #[test]
    fn test_gfdl_11_gnome_step_by_step() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== STEP-BY-STEP PIPELINE for gfdl-1.1-en_gnome_1.RULE ===");
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

        // Step 3: Aho-Corasick matching
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        eprintln!("\n--- Step 3: Aho-Corasick matching ---");
        eprintln!(
            "Aho matches: {} (merged: {})",
            aho_matches.len(),
            merged_aho.len()
        );
        for m in merged_aho.iter().take(10) {
            print_match_summary(m, "  Aho");
        }
        if merged_aho.len() > 10 {
            eprintln!("  ... and {} more", merged_aho.len() - 10);
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

        // Step 5: Regular sequence matching
        let seq_matches = seq_match(index, &whole_run);
        eprintln!("\n--- Step 5: Regular sequence matching ---");
        eprintln!("Seq matches: {}", seq_matches.len());

        // Combine all matches
        let mut all_matches = Vec::new();
        all_matches.extend(merged_spdx.clone());
        all_matches.extend(merged_aho.clone());
        all_matches.extend(merge_overlapping_matches(&near_dupe_matches));
        all_matches.extend(merge_overlapping_matches(&seq_matches));

        eprintln!("\n--- Step 6: All matches combined (before unknown) ---");
        eprintln!("Total matches: {}", all_matches.len());

        // Group by license expression
        let mut by_expr: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for m in &all_matches {
            *by_expr.entry(&m.license_expression).or_insert(0) += 1;
        }
        eprintln!("By expression: {:?}", by_expr);

        // Step 7: Unknown matching
        let unknown_matches = unknown_match(index, &query, &all_matches);
        eprintln!("\n--- Step 7: Unknown matching ---");
        eprintln!("Unknown matches: {}", unknown_matches.len());

        // Step 8: Refine matches
        eprintln!("\n--- Step 8: Refine matches ---");
        let refined = refine_matches(index, all_matches.clone(), &query);
        eprintln!(
            "Refined matches: {} (from {})",
            refined.len(),
            all_matches.len()
        );

        // Show what each filter does
        let step_matches = all_matches.clone();

        // Merge first
        let merged = merge_overlapping_matches(&step_matches);
        eprintln!("  After merge: {}", merged.len());

        // Filter contained
        let (non_contained, discarded_contained) = filter_contained_matches(&merged);
        eprintln!(
            "  After filter_contained: {} (discarded: {})",
            non_contained.len(),
            discarded_contained.len()
        );

        // Filter overlapping
        let (kept, discarded_overlapping) =
            filter_overlapping_matches(non_contained.clone(), index);
        eprintln!(
            "  After filter_overlapping: {} (discarded: {})",
            kept.len(),
            discarded_overlapping.len()
        );

        // Filter false positive lists
        let (kept_fp, discarded_fp) = filter_false_positive_license_lists_matches(kept);
        eprintln!(
            "  After filter_false_positive_lists: {} (discarded: {})",
            kept_fp.len(),
            discarded_fp.len()
        );

        // Final result
        eprintln!("\n--- FINAL RESULT ---");
        let mut by_expr: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for m in &refined {
            *by_expr.entry(&m.license_expression).or_insert(0) += 1;
        }
        eprintln!("By expression: {:?}", by_expr);
    }

    #[test]
    fn test_gfdl_rules_in_index() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let index = engine.index();

        eprintln!("\n=== GFDL RULES IN INDEX ===");
        let gfdl_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("gfdl"))
            .collect();

        eprintln!("Found {} GFDL rules", gfdl_rules.len());
        for rule in gfdl_rules.iter().take(20) {
            eprintln!(
                "  {} ({}): is_false_positive={}, is_license_text={}, relevance={}",
                rule.identifier,
                rule.license_expression,
                rule.is_false_positive,
                rule.is_license_text,
                rule.relevance
            );
        }

        // Check other-copyleft rules
        eprintln!("\n=== other-copyleft RULES IN INDEX ===");
        let other_copyleft: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("other-copyleft"))
            .collect();

        eprintln!("Found {} other-copyleft rules", other_copyleft.len());
        for rule in other_copyleft.iter().take(10) {
            eprintln!(
                "  {} ({}): is_false_positive={}, text_preview={:?}",
                rule.identifier,
                rule.license_expression,
                rule.is_false_positive,
                &rule.text.chars().take(100).collect::<String>()
            );
        }
    }

    #[test]
    fn test_gfdl_11_gnome_aho_matches_detail() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== AHO MATCH DETAILS for gfdl-1.1-en_gnome_1.RULE ===");

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
            for m in matches.iter().take(5) {
                eprintln!(
                    "    lines {}-{}, coverage={:.1}%, rid={}, identifier={}",
                    m.start_line, m.end_line, m.match_coverage, m.rid, m.rule_identifier
                );
            }
            if matches.len() > 5 {
                eprintln!("    ... and {} more", matches.len() - 5);
            }
        }

        // After merge
        let merged = merge_overlapping_matches(&aho_matches);
        eprintln!("\nAfter merge: {} matches", merged.len());

        let mut by_expr_merged: std::collections::HashMap<&str, Vec<&LicenseMatch>> =
            std::collections::HashMap::new();
        for m in &merged {
            by_expr_merged
                .entry(&m.license_expression)
                .or_default()
                .push(m);
        }

        for (expr, matches) in &by_expr_merged {
            eprintln!("  {}: {} matches", expr, matches.len());
        }
    }

    #[test]
    fn test_gfdl_11_gnome_seq_matches_detail() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== SEQ MATCH DETAILS for gfdl-1.1-en_gnome_1.RULE ===");

        let seq_matches = seq_match(index, &whole_run);
        eprintln!("Raw seq matches: {}", seq_matches.len());

        // Group by license expression
        let mut by_expr: std::collections::HashMap<&str, Vec<&LicenseMatch>> =
            std::collections::HashMap::new();
        for m in &seq_matches {
            by_expr.entry(&m.license_expression).or_default().push(m);
        }

        for (expr, matches) in &by_expr {
            eprintln!("\n{}: {} matches", expr, matches.len());
            for m in matches.iter().take(5) {
                eprintln!(
                    "    lines {}-{}, coverage={:.1}%, rid={}, identifier={}",
                    m.start_line, m.end_line, m.match_coverage, m.rid, m.rule_identifier
                );
            }
            if matches.len() > 5 {
                eprintln!("    ... and {} more", matches.len() - 5);
            }
        }

        // After merge
        let merged = merge_overlapping_matches(&seq_matches);
        eprintln!("\nAfter merge: {} matches", merged.len());

        let mut by_expr_merged: std::collections::HashMap<&str, Vec<&LicenseMatch>> =
            std::collections::HashMap::new();
        for m in &merged {
            by_expr_merged
                .entry(&m.license_expression)
                .or_default()
                .push(m);
        }

        for (expr, matches) in &by_expr_merged {
            eprintln!("  {}: {} matches", expr, matches.len());
        }
    }

    #[test]
    fn test_gfdl_11_gnome_qspan_detail() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== QSPAN DETAIL ===");

        // Get near-dupe matches
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        // Find a gfdl-1.1 match that spans tokens 1-74
        let gfdl_1_74: Vec<_> = merged_near_dupe
            .iter()
            .filter(|m| {
                m.license_expression == "gfdl-1.1" && m.start_token <= 1 && m.end_token >= 74
            })
            .collect();

        if let Some(gfdl) = gfdl_1_74.first() {
            eprintln!(
                "GFDL match: tokens {}-{}, len={}",
                gfdl.start_token, gfdl.end_token, gfdl.matched_length
            );

            if let Some(qspan) = &gfdl.qspan_positions {
                eprintln!("Qspan has {} tokens", qspan.len());
                eprintln!("First 10: {:?}", &qspan[..10.min(qspan.len())]);
                eprintln!("Last 10: {:?}", &qspan[qspan.len().saturating_sub(10)..]);

                // Check if tokens 41-55 are in the qspan
                eprintln!("\nTokens 41-55 in qspan:");
                for t in 41..55 {
                    let in_qspan = qspan.contains(&t);
                    eprintln!("  Token {}: {}", t, in_qspan);
                }
            }
        }

        // Get other-copyleft matches
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        let other_copyleft: Vec<_> = merged_aho
            .iter()
            .filter(|m| m.license_expression == "other-copyleft")
            .collect();

        for oc in &other_copyleft {
            eprintln!(
                "\nother-copyleft: tokens {}-{}, start_token={}, end_token={}",
                oc.start_token, oc.end_token, oc.start_token, oc.end_token
            );
            eprintln!("  qspan_positions: {:?}", oc.qspan_positions);
        }
    }

    #[test]
    fn test_gfdl_11_gnome_qcontains_debug() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== QCONTAINS DEBUG ===");

        // Get near-dupe matches
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        // Find a gfdl-1.1 match that spans tokens 1-74
        let gfdl_1_74: Vec<_> = merged_near_dupe
            .iter()
            .filter(|m| {
                m.license_expression == "gfdl-1.1" && m.start_token <= 1 && m.end_token >= 74
            })
            .collect();

        eprintln!("GFDL-1.1 matches spanning tokens 1-74: {}", gfdl_1_74.len());
        for m in &gfdl_1_74 {
            eprintln!(
                "  tokens {}-{}, lines {}-{}, len={}, hilen={}",
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.hilen()
            );
            eprintln!(
                "  qspan_positions: {:?}",
                m.qspan_positions.as_ref().map(|p| (p.first(), p.last()))
            );
        }

        // Get other-copyleft matches
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        let other_copyleft: Vec<_> = merged_aho
            .iter()
            .filter(|m| m.license_expression == "other-copyleft")
            .collect();

        eprintln!("\nother-copyleft matches:");
        for m in &other_copyleft {
            eprintln!(
                "  tokens {}-{}, lines {}-{}, len={}, hilen={}",
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.hilen()
            );
            eprintln!(
                "  qspan_positions: {:?}",
                m.qspan_positions.as_ref().map(|p| (p.first(), p.last()))
            );
        }

        // Test qcontains manually
        if let Some(gfdl) = gfdl_1_74.first() {
            for oc in &other_copyleft {
                let contains = gfdl.qcontains(oc);
                eprintln!(
                    "\ngfdl.qcontains(other_copyleft at {}-{})? {}",
                    oc.start_token, oc.end_token, contains
                );

                // Check qspan containment
                if let (Some(gfdl_qspan), Some(oc_qspan)) =
                    (&gfdl.qspan_positions, &oc.qspan_positions)
                {
                    eprintln!(
                        "  gfdl qspan: {}..{}",
                        gfdl_qspan.first().unwrap_or(&0),
                        gfdl_qspan.last().unwrap_or(&0) + 1
                    );
                    eprintln!(
                        "  oc qspan: {}..{}",
                        oc_qspan.first().unwrap_or(&0),
                        oc_qspan.last().unwrap_or(&0) + 1
                    );

                    // Manual check: all oc qspan tokens in gfdl qspan?
                    let gfdl_set: std::collections::HashSet<_> =
                        gfdl_qspan.iter().copied().collect();
                    let all_contained = oc_qspan.iter().all(|t| gfdl_set.contains(t));
                    eprintln!("  All oc tokens in gfdl qspan? {}", all_contained);
                }
            }
        }
    }

    #[test]
    fn test_gfdl_11_gnome_containment_debug() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== CONTAINMENT DEBUG ===");

        // Get near-dupe matches with gfdl-1.1 in the gap
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        // Find gfdl-1.1 matches in gap
        let gfdl_gap: Vec<_> = merged_near_dupe
            .iter()
            .filter(|m| m.license_expression == "gfdl-1.1" && m.start_token <= 74)
            .collect();

        eprintln!("GFDL-1.1 near-dupe matches in gap (tokens 0-74):");
        for m in &gfdl_gap {
            eprintln!(
                "  tokens {}-{}, lines {}-{}, len={}, hilen={}",
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.hilen()
            );
        }

        // Get other-copyleft matches
        let aho_matches = aho_match(index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        let other_copyleft: Vec<_> = merged_aho
            .iter()
            .filter(|m| m.license_expression == "other-copyleft")
            .collect();

        eprintln!("\nother-copyleft matches:");
        for m in &other_copyleft {
            eprintln!(
                "  tokens {}-{}, lines {}-{}, len={}, hilen={}",
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.hilen()
            );
        }

        // Check containment manually
        eprintln!("\nManual containment check:");
        for oc in &other_copyleft {
            for gfdl in &gfdl_gap {
                if gfdl.start_token <= oc.start_token && gfdl.end_token >= oc.end_token {
                    eprintln!(
                        "  other-copyleft ({}-{}) IS contained in gfdl-1.1 ({}-{})",
                        oc.start_token, oc.end_token, gfdl.start_token, gfdl.end_token
                    );
                }
            }
        }

        // Now check after filter_contained_matches
        // Combine all matches
        let mut all_matches = Vec::new();
        all_matches.extend(merged_aho.clone());
        all_matches.extend(merged_near_dupe.clone());
        all_matches.extend(merge_overlapping_matches(&seq_match(index, &whole_run)));

        // First merge
        let merged = merge_overlapping_matches(&all_matches);
        eprintln!("\nAfter first merge: {} matches", merged.len());

        // Show matches in gap after merge
        eprintln!("Matches in gap (tokens 0-78) after merge:");
        for m in merged
            .iter()
            .filter(|m| m.end_token > 0 && m.start_token < 78)
        {
            eprintln!(
                "  {}: tokens {}-{}, len={}, hilen={}",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.matched_length,
                m.hilen()
            );
        }

        // Apply filter_contained
        let (non_contained, discarded) = filter_contained_matches(&merged);
        eprintln!(
            "\nAfter filter_contained: {} kept, {} discarded",
            non_contained.len(),
            discarded.len()
        );

        // Check if other-copyleft was discarded
        eprintln!("Discarded matches:");
        for m in discarded.iter().filter(|m| m.start_token < 78) {
            eprintln!(
                "  {}: tokens {}-{}, len={}, hilen={}",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.matched_length,
                m.hilen()
            );
        }
    }

    #[test]
    fn test_gfdl_11_gnome_gap_analysis() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== GAP ANALYSIS for lines 1-19 ===");

        // Get all raw matches (before any processing)
        let aho_matches = aho_match(index, &whole_run);
        let seq_matches = seq_match(index, &whole_run);
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };

        // Find matches that start before token 78 (line 20)
        let gap_end = 78;

        eprintln!("\nAho matches in gap (tokens 0-{}):", gap_end);
        for m in aho_matches.iter().filter(|m| m.start_token < gap_end) {
            eprintln!(
                "  {}: tokens {}-{}, lines {}-{}, len={}",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length
            );
        }

        eprintln!("\nSeq matches in gap (tokens 0-{}):", gap_end);
        for m in seq_matches.iter().filter(|m| m.start_token < gap_end) {
            eprintln!(
                "  {}: tokens {}-{}, lines {}-{}, len={}",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length
            );
        }

        eprintln!("\nNear-dupe matches in gap (tokens 0-{}):", gap_end);
        for m in near_dupe_matches.iter().filter(|m| m.start_token < gap_end) {
            eprintln!(
                "  {}: tokens {}-{}, lines {}-{}, len={}",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length
            );
        }

        // Check what the first 20 lines of the file look like
        let lines: Vec<&str> = text.lines().take(20).collect();
        eprintln!("\n=== First 20 lines of the file ===");
        for (i, line) in lines.iter().enumerate() {
            eprintln!(
                "Line {}: {}",
                i + 1,
                line.chars().take(80).collect::<String>()
            );
        }
    }

    #[test]
    fn test_gfdl_11_gnome_overlap_analysis() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== OVERLAP ANALYSIS for other-copyleft matches ===");

        // Get aho matches (where other-copyleft comes from)
        let aho_matches = aho_match(index, &whole_run);
        let other_copyleft_aho: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression == "other-copyleft")
            .collect();

        eprintln!("other-copyleft Aho matches:");
        for m in &other_copyleft_aho {
            eprintln!(
                "  lines {}-{}, tokens {}-{}, len={}, coverage={:.1}%",
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length,
                m.match_coverage
            );
        }

        // Find what other matches overlap with other-copyleft
        let all_matches = merge_overlapping_matches(&aho_matches);
        eprintln!("\nMatches that overlap with other-copyleft:");
        for oc in &other_copyleft_aho {
            eprintln!(
                "\nChecking other-copyleft at tokens {}-{}:",
                oc.start_token, oc.end_token
            );
            for m in &all_matches {
                // Check if they overlap
                let overlap_start = m.start_token.max(oc.start_token);
                let overlap_end = m.end_token.min(oc.end_token);
                if overlap_start < overlap_end {
                    let overlap = overlap_end - overlap_start;
                    let oc_overlap_ratio = overlap as f64 / (oc.end_token - oc.start_token) as f64;
                    let m_overlap_ratio = overlap as f64 / (m.end_token - m.start_token) as f64;
                    eprintln!(
                        "  {}: tokens {}-{}, overlap={}, oc_ratio={:.2}%, m_ratio={:.2}%",
                        m.license_expression,
                        m.start_token,
                        m.end_token,
                        overlap,
                        oc_overlap_ratio * 100.0,
                        m_overlap_ratio * 100.0
                    );
                }
            }
        }

        // Check containment
        eprintln!("\n=== CONTAINMENT CHECK ===");
        for oc in &other_copyleft_aho {
            for m in &all_matches {
                if m.start_token <= oc.start_token && m.end_token >= oc.end_token {
                    eprintln!(
                        "other-copyleft (tokens {}-{}) IS CONTAINED in {} (tokens {}-{})",
                        oc.start_token,
                        oc.end_token,
                        m.license_expression,
                        m.start_token,
                        m.end_token
                    );
                }
            }
        }
    }

    #[test]
    fn test_gfdl_11_gnome_merge_debug() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== MERGE DEBUG for gfdl-1.1-en_gnome_1.RULE ===");

        // Get seq matches
        let seq_matches = seq_match(index, &whole_run);
        eprintln!("Raw seq matches: {}", seq_matches.len());

        // Group by expression before merge
        let mut by_expr: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for m in &seq_matches {
            *by_expr.entry(&m.license_expression).or_insert(0) += 1;
        }
        eprintln!("Before merge by expression: {:?}", by_expr);

        // After merge
        let merged = merge_overlapping_matches(&seq_matches);
        eprintln!("\nMerged seq matches: {}", merged.len());

        let mut by_expr_merged: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for m in &merged {
            *by_expr_merged.entry(&m.license_expression).or_insert(0) += 1;
        }
        eprintln!("After merge by expression: {:?}", by_expr_merged);

        // Show gfdl matches after merge
        let gfdl_merged: Vec<_> = merged
            .iter()
            .filter(|m| m.license_expression.contains("gfdl"))
            .collect();
        eprintln!("\nGFDL matches after merge: {}", gfdl_merged.len());
        for m in gfdl_merged.iter().take(10) {
            eprintln!(
                "  {}: lines {}-{}, tokens {}-{}, len={}, coverage={:.1}%",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length,
                m.match_coverage
            );
        }

        // Also check near-dupe matches
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        let gfdl_near_dupe: Vec<_> = merged_near_dupe
            .iter()
            .filter(|m| m.license_expression.contains("gfdl"))
            .collect();
        eprintln!("\nGFDL matches from near-dupe: {}", gfdl_near_dupe.len());
        for m in gfdl_near_dupe.iter().take(5) {
            eprintln!(
                "  {}: lines {}-{}, tokens {}-{}, len={}, coverage={:.1}%",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length,
                m.match_coverage
            );
        }
    }

    #[test]
    fn test_gfdl_11_gnome_why_gfdl_starts_late() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== WHY DOES GFDL START AT LINE 20? ===");
        eprintln!("Total query tokens: {}", query.tokens.len());

        // Check all gfdl-1.1 seq matches
        let seq_matches = seq_match(index, &whole_run);
        let gfdl_seq: Vec<_> = seq_matches
            .iter()
            .filter(|m| m.license_expression == "gfdl-1.1" && m.matcher == "3-seq")
            .collect();

        eprintln!("\nGFDL-1.1 sequence matches ({} total):", gfdl_seq.len());
        for m in gfdl_seq.iter().take(10) {
            eprintln!(
                "  lines {}-{} (tokens {}-{}), coverage={:.1}%, len={}, hilen={}",
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage,
                m.matched_length,
                m.hilen()
            );
        }

        // Check what the largest gfdl-1.1 seq match covers
        if let Some(largest) = gfdl_seq.iter().max_by_key(|m| m.matched_length) {
            eprintln!(
                "\nLargest gfdl-1.1 match: lines {}-{}, tokens {}-{}, len={}, coverage={:.1}%",
                largest.start_line,
                largest.end_line,
                largest.start_token,
                largest.end_token,
                largest.matched_length,
                largest.match_coverage
            );
        }

        // Check if there's a gfdl-1.1 match that starts earlier
        if let Some(earliest) = gfdl_seq.iter().min_by_key(|m| m.start_token) {
            eprintln!(
                "Earliest gfdl-1.1 match: lines {}-{}, tokens {}-{}, len={}",
                earliest.start_line,
                earliest.end_line,
                earliest.start_token,
                earliest.end_token,
                earliest.matched_length
            );
        }

        // Compare with what Python found: lines 1-608, coverage 99.03%
        eprintln!("\nPython result: lines 1-608, coverage 99.03%");
    }

    #[test]
    fn test_gfdl_11_gnome_refined_matches_detail() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== REFINED MATCHES DETAIL for gfdl-1.1-en_gnome_1.RULE ===");

        let _hash_matches = hash_match(index, &whole_run);
        let spdx_matches = spdx_lid_match(index, &query);
        let aho_matches = aho_match(index, &whole_run);
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };
        let seq_matches = seq_match(index, &whole_run);

        let mut all_matches = Vec::new();
        all_matches.extend(merge_overlapping_matches(&spdx_matches));
        all_matches.extend(merge_overlapping_matches(&aho_matches));
        all_matches.extend(merge_overlapping_matches(&near_dupe_matches));
        all_matches.extend(merge_overlapping_matches(&seq_matches));

        let refined = refine_matches(index, all_matches.clone(), &query);

        eprintln!("Refined matches: {}", refined.len());
        for m in &refined {
            eprintln!(
                "  {}: lines {}-{} (tokens {}-{}), coverage={:.1}%, len={}, hilen={}, matcher={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage,
                m.matched_length,
                m.hilen(),
                m.matcher
            );
        }

        // Check if other-copyleft is contained within gfdl-1.1
        let gfdl_matches: Vec<_> = refined
            .iter()
            .filter(|m| m.license_expression.contains("gfdl"))
            .collect();
        let other_copyleft: Vec<_> = refined
            .iter()
            .filter(|m| m.license_expression.contains("other-copyleft"))
            .collect();

        eprintln!("\n=== CONTAINMENT CHECK ===");
        for oc in &other_copyleft {
            eprintln!(
                "other-copyleft: lines {}-{}, tokens {}-{}",
                oc.start_line, oc.end_line, oc.start_token, oc.end_token
            );
            for gfdl in &gfdl_matches {
                let contained =
                    gfdl.start_token <= oc.start_token && gfdl.end_token >= oc.end_token;
                eprintln!(
                    "  contained in {} (tokens {}-{})? {}",
                    gfdl.license_expression, gfdl.start_token, gfdl.end_token, contained
                );
            }
        }
    }

    #[test]
    fn test_gfdl_11_gnome_where_extra_matches_come_from() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== TRACING EXTRA MATCHES for gfdl-1.1-en_gnome_1.RULE ===");

        // Collect matches from each source separately
        let hash_matches = hash_match(index, &whole_run);
        let spdx_matches = spdx_lid_match(index, &query);
        let aho_matches = aho_match(index, &whole_run);
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };
        let seq_matches = seq_match(index, &whole_run);

        eprintln!("\nMatches by source:");
        eprintln!("  Hash: {}", hash_matches.len());
        eprintln!("  SPDX: {}", spdx_matches.len());
        eprintln!("  Aho: {}", aho_matches.len());
        eprintln!("  Near-dupe: {}", near_dupe_matches.len());
        eprintln!("  Seq: {}", seq_matches.len());

        // Show license expressions from each source
        fn show_expressions(matches: &[LicenseMatch], name: &str) {
            let mut exprs: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for m in matches {
                exprs.insert(&m.license_expression);
            }
            eprintln!("  {} expressions: {:?}", name, exprs);
        }

        show_expressions(&hash_matches, "Hash");
        show_expressions(&spdx_matches, "SPDX");
        show_expressions(&aho_matches, "Aho");
        show_expressions(&near_dupe_matches, "Near-dupe");
        show_expressions(&seq_matches, "Seq");

        // Look for other-copyleft specifically
        eprintln!("\n=== other-copyleft matches ===");
        for (name, matches) in [
            ("Aho", aho_matches.as_slice()),
            ("Seq", seq_matches.as_slice()),
            ("Near-dupe", near_dupe_matches.as_slice()),
        ] {
            let other_copyleft: Vec<_> = matches
                .iter()
                .filter(|m| m.license_expression.contains("other-copyleft"))
                .collect();
            if !other_copyleft.is_empty() {
                eprintln!("{}: {} other-copyleft matches", name, other_copyleft.len());
                for m in other_copyleft.iter().take(3) {
                    eprintln!(
                        "    lines {}-{}, coverage={:.1}%, rule={}",
                        m.start_line, m.end_line, m.match_coverage, m.rule_identifier
                    );
                }
            }
        }
    }

    #[test]
    fn test_plan_080_swrule_detection_ucware() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let Some(text) = read_unknown_test_file("ucware-eula.txt") else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query creation should succeed");

        eprintln!("\n=== PLAN-080: swrule detection investigation ===");

        // Find swrule.LICENSE in the index
        let swrule_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "swrule.LICENSE");

        if let Some(rid) = swrule_rid {
            let rule = &index.rules_by_rid[rid];
            eprintln!("\nswrule.LICENSE properties:");
            eprintln!("  tokens length: {}", rule.tokens.len());
            eprintln!("  length_unique: {}", rule.length_unique);
            eprintln!("  high_length: {}", rule.high_length);
            eprintln!("  high_length_unique: {}", rule.high_length_unique);
            eprintln!("  min_matched_length: {}", rule.min_matched_length);
            eprintln!(
                "  min_high_matched_length: {}",
                rule.min_high_matched_length
            );
            eprintln!(
                "  min_matched_length_unique: {}",
                rule.min_matched_length_unique
            );
            eprintln!(
                "  min_high_matched_length_unique: {}",
                rule.min_high_matched_length_unique
            );
            eprintln!(
                "  is_approx_matchable: {}",
                index.approx_matchable_rids.contains(&rid)
            );

            // Show legalese tokens in swrule
            let legalese_tokens: Vec<_> = rule
                .tokens
                .iter()
                .filter(|&&tid| (tid as usize) < index.len_legalese)
                .collect();
            eprintln!("  legalese tokens count: {}", legalese_tokens.len());
        } else {
            eprintln!("swrule.LICENSE not found in index!");
        }

        // Check query runs and candidate selection
        eprintln!("\nQuery properties:");
        eprintln!("  tokens length: {}", query.tokens.len());
        eprintln!("  query runs: {}", query.query_runs().len());

        for (i, query_run) in query.query_runs().iter().enumerate() {
            let candidates = compute_candidates_with_msets(index, query_run, false, 70);

            eprintln!("\nQuery run {}: {} candidates", i, candidates.len());

            // Check if swrule is among candidates
            let swrule_in_candidates = candidates
                .iter()
                .enumerate()
                .find(|(_, c)| c.rule.identifier == "swrule.LICENSE");
            if let Some((pos, c)) = swrule_in_candidates {
                eprintln!("  swrule found at position {}!", pos + 1);
                eprintln!("    containment: {}", c.score_vec_full.containment);
                eprintln!("    resemblance: {}", c.score_vec_full.resemblance);
                eprintln!("    matched_length: {}", c.score_vec_full.matched_length);
                eprintln!(
                    "    is_highly_resemblant: {}",
                    c.score_vec_full.is_highly_resemblant
                );
            } else if swrule_rid.is_some() {
                eprintln!("  swrule NOT in top 70 candidates");
                eprintln!("  Step 1 position: 329 (containment=0.694)");
                eprintln!("  Python shows position 66 (containment=0.765)");
                eprintln!("  --> Difference is in step 2 multiset ranking!");

                // Show candidates around position 65-70 to see what's there
                eprintln!("\n  Candidates around positions 60-70:");
                for (pos, c) in candidates.iter().enumerate().skip(59).take(15) {
                    eprintln!(
                        "    {}: {} (cont={:.3}, resembl={:.4})",
                        pos + 1,
                        c.rule.identifier,
                        c.score_vec_full.containment,
                        c.score_vec_full.resemblance
                    );
                }

                // Get all step 1 candidates to see if swrule was filtered
                let all_step1_candidates = {
                    let query_tokens = query_run.matchable_tokens();
                    let query_token_ids: Vec<u16> = query_tokens
                        .iter()
                        .filter_map(|&tid| if tid >= 0 { Some(tid as u16) } else { None })
                        .collect();

                    let (query_set, query_mset) =
                        crate::license_detection::index::token_sets::build_set_and_mset(
                            &query_token_ids,
                        );
                    let len_legalese = index.len_legalese;

                    let mut candidates: Vec<(String, f32, f32, usize)> = Vec::new();

                    for (rid, rule) in index.rules_by_rid.iter().enumerate() {
                        if !index.approx_matchable_rids.contains(&rid) {
                            continue;
                        }

                        let Some(rule_set) = index.sets_by_rid.get(&rid) else {
                            continue;
                        };

                        let intersection: std::collections::HashSet<u16> =
                            query_set.intersection(rule_set).copied().collect();
                        if intersection.is_empty() {
                            continue;
                        }

                        let high_set_intersection =
                            crate::license_detection::index::token_sets::high_tids_set_subset(
                                &intersection,
                                len_legalese,
                            );
                        if high_set_intersection.is_empty() {
                            continue;
                        }

                        let high_matched_length =
                            crate::license_detection::index::token_sets::tids_set_counter(
                                &high_set_intersection,
                            );
                        if high_matched_length < rule.min_high_matched_length_unique {
                            continue;
                        }

                        let matched_length =
                            crate::license_detection::index::token_sets::tids_set_counter(
                                &intersection,
                            );
                        if matched_length < rule.min_matched_length_unique {
                            continue;
                        }

                        // Compute resemblance
                        let qset_len = query_set.len();
                        let iset_len = rule.length_unique;
                        if qset_len == 0 || iset_len == 0 {
                            continue;
                        }

                        let union_len = qset_len + iset_len - matched_length;
                        let resemblance = matched_length as f32 / union_len as f32;
                        let containment = matched_length as f32 / iset_len as f32;

                        candidates.push((rule.identifier.clone(), containment, resemblance, rid));
                    }

                    candidates.sort_by(|a, b| {
                        b.1.partial_cmp(&a.1)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| {
                                b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)
                            })
                    });
                    candidates
                };

                // Find swrule's position
                for (i, (id, cont, resembl, _rid)) in all_step1_candidates.iter().enumerate() {
                    if id.contains("swrule") {
                        eprintln!(
                            "  swrule position in step1: {} (containment={:.3}, resemblance={:.3})",
                            i, cont, resembl
                        );
                        break;
                    }
                }

                eprintln!("  Total step1 candidates: {}", all_step1_candidates.len());
                eprintln!("  Top 10 candidates:");
                for (i, (id, cont, resembl, _rid)) in
                    all_step1_candidates.iter().take(10).enumerate()
                {
                    eprintln!(
                        "    {}: {} (cont={:.3}, resembl={:.3})",
                        i + 1,
                        id,
                        cont,
                        resembl
                    );
                }
            }
        }

        // Run full detection
        let detections = engine.detect(&text, false).expect("Detection failed");

        // Check for swrule in detections
        let has_swrule = detections.iter().any(|d| {
            d.matches
                .iter()
                .any(|m| m.rule_identifier.contains("swrule"))
        });
        eprintln!("\nHas swrule detection: {}", has_swrule);

        eprintln!("\nAll matches:");
        for d in &detections {
            for m in &d.matches {
                eprintln!(
                    "  {} (rule: {}, matcher: {}, score: {:.1}, lines: {}-{})",
                    m.license_expression,
                    m.rule_identifier,
                    m.matcher,
                    m.score,
                    m.start_line,
                    m.end_line
                );
            }
        }
    }
}
