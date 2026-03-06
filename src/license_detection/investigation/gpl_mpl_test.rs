//! Investigation test for PLAN-002: gpl-2.0-plus_and_mpl-1.0.txt
//!
//! ## Issue
//! Should detect `mpl-1.0 OR gpl-2.0-plus` as single expression, but Rust detects them separately.
//!
//! **Expected:** `["mpl-1.0 OR gpl-2.0-plus"]`
//! **Actual:** `["mpl-1.0", "gpl-1.0-plus", "gpl-2.0-plus"]`
//!
//! ## Pipeline stages to investigate:
//! 1. Aho matching - what matches are found?
//! 2. Seq matching - what candidates/matches?
//! 3. Filtering (contained, overlapping) - what gets filtered?
//! 4. Detection grouping - how are matches grouped?
//! 5. Expression combination - how are expressions combined?

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::path::PathBuf;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    fn read_test_file() -> Option<String> {
        let path =
            PathBuf::from("testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_mpl-1.0.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_gpl_mpl_rust_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== RUST DETECTIONS ===");
        eprintln!("Number of detections: {}", detections.len());

        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            eprintln!("  detection_log: {:?}", det.detection_log);
            eprintln!("  Number of matches: {}", det.matches.len());

            for (j, m) in det.matches.iter().enumerate() {
                eprintln!("    Match {}:", j + 1);
                eprintln!("      license_expression: {}", m.license_expression);
                eprintln!("      matcher: {}", m.matcher);
                eprintln!("      lines: {}-{}", m.start_line, m.end_line);
                eprintln!("      score: {:.2}", m.score);
                eprintln!("      match_coverage: {:.1}", m.match_coverage);
                eprintln!("      rule_identifier: {}", m.rule_identifier);
                eprintln!("      matched_length: {}", m.matched_length);
                eprintln!("      rule_length: {}", m.rule_length);
            }
        }

        let all_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nAll license expressions: {:?}", all_expressions);

        assert_eq!(
            all_expressions,
            vec!["mpl-1.0 OR gpl-2.0-plus"],
            "Expected single combined expression"
        );
    }

    #[test]
    fn test_gpl_mpl_aho_matches() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        // Phase 1a: Hash matching
        let hash_matches = hash_match(&index, &whole_run);
        eprintln!("\n=== HASH MATCHES ===");
        eprintln!("Count: {}", hash_matches.len());
        for m in &hash_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={})",
                m.license_expression, m.start_line, m.end_line, m.matcher
            );
        }

        // Phase 1b: SPDX-LID matching
        let spdx_matches = spdx_lid_match(&index, &query);
        eprintln!("\n=== SPDX-LID MATCHES ===");
        eprintln!("Count: {}", spdx_matches.len());
        for m in &spdx_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={})",
                m.license_expression, m.start_line, m.end_line, m.matcher
            );
        }

        // Phase 1c: Aho matching
        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("\n=== AHO MATCHES ===");
        eprintln!("Count: {}", aho_matches.len());
        for m in &aho_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={}, coverage={:.1}%, rule={})",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.matcher,
                m.match_coverage,
                m.rule_identifier
            );
        }

        // Combine all phase 1 matches
        let mut all_phase1 = Vec::new();
        all_phase1.extend(hash_matches);
        all_phase1.extend(spdx_matches);
        all_phase1.extend(aho_matches);

        eprintln!("\n=== ALL PHASE 1 MATCHES ===");
        eprintln!("Total: {}", all_phase1.len());

        // Check for MPL and GPL matches
        let mpl_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression.contains("mpl"))
            .collect();
        let gpl_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression.contains("gpl"))
            .collect();

        eprintln!("\nMPL matches in phase 1: {}", mpl_matches.len());
        for m in &mpl_matches {
            eprintln!(
                "  {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        eprintln!("\nGPL matches in phase 1: {}", gpl_matches.len());
        for m in &gpl_matches {
            eprintln!(
                "  {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }
    }

    #[test]
    fn test_gpl_mpl_seq_matches() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::merge_overlapping_matches;
        use crate::license_detection::query::{PositionSpan, Query};
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match_with_candidates,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        let mut matched_qspans: Vec<PositionSpan> = Vec::new();

        // Phase 1 matches
        let hash_matches = hash_match(&index, &whole_run);
        let spdx_matches = spdx_lid_match(&index, &query);
        let aho_matches = aho_match(&index, &whole_run);

        let mut all_matches = Vec::new();
        all_matches.extend(hash_matches.clone());
        all_matches.extend(spdx_matches.clone());

        let merged_aho = merge_overlapping_matches(&aho_matches);
        for m in &merged_aho {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        all_matches.extend(merged_aho);

        eprintln!("\n=== PHASE 1 (before seq) ===");
        eprintln!("Total matches: {}", all_matches.len());
        for m in &all_matches {
            eprintln!(
                "  {} at lines {}-{} matcher={}",
                m.license_expression, m.start_line, m.end_line, m.matcher
            );
        }

        // Phase 2: Near-duplicate detection
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);

        eprintln!("\n=== NEAR-DUPE CANDIDATES ===");
        eprintln!("Count: {}", near_dupe_candidates.len());
        for c in &near_dupe_candidates {
            eprintln!(
                "  {} (rid={}, resemblance={:.3}, containment={:.3})",
                c.rule.license_expression,
                c.rid,
                c.score_vec_full.resemblance,
                c.score_vec_full.containment
            );
        }

        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                seq_match_with_candidates(&index, &whole_run, &near_dupe_candidates);

            eprintln!("\n=== NEAR-DUPE MATCHES ===");
            eprintln!("Count: {}", near_dupe_matches.len());
            for m in &near_dupe_matches {
                eprintln!(
                    "  {} at lines {}-{} matcher={} coverage={:.1}%",
                    m.license_expression, m.start_line, m.end_line, m.matcher, m.match_coverage
                );
            }
        }
    }

    #[test]
    fn test_gpl_mpl_refine_pipeline() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region, sort_matches_by_line,
        };
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
            refine_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        let mut all_matches = Vec::new();

        let hash_matches = hash_match(&index, &whole_run);
        all_matches.extend(hash_matches);

        let spdx_matches = spdx_lid_match(&index, &query);
        all_matches.extend(spdx_matches);

        let aho_matches = aho_match(&index, &whole_run);
        all_matches.extend(aho_matches);

        eprintln!("\n=== INITIAL MATCHES (pre-refine) ===");
        eprintln!("Count: {}", all_matches.len());

        // Step 1: Merge overlapping
        let merged = merge_overlapping_matches(&all_matches);
        eprintln!("\n=== AFTER merge_overlapping ===");
        eprintln!("Count: {}", merged.len());
        for m in &merged {
            eprintln!(
                "  {} at lines {}-{} tokens={}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }

        // Step 2: Filter contained
        let (non_contained, discarded_contained) = filter_contained_matches(&merged);
        eprintln!("\n=== AFTER filter_contained ===");
        eprintln!(
            "Kept: {}, Discarded: {}",
            non_contained.len(),
            discarded_contained.len()
        );

        for m in &discarded_contained {
            eprintln!(
                "  DISCARDED: {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        // Step 3: Filter overlapping
        let (kept_overlapping, discarded_overlapping) =
            filter_overlapping_matches(non_contained.clone(), &index);

        eprintln!("\n=== AFTER filter_overlapping ===");
        eprintln!(
            "Kept: {}, Discarded: {}",
            kept_overlapping.len(),
            discarded_overlapping.len()
        );

        for m in &discarded_overlapping {
            eprintln!(
                "  DISCARDED: {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        // Step 4: Full refine
        let refined = refine_matches(&index, all_matches.clone(), &query);
        eprintln!("\n=== AFTER refine_matches ===");
        eprintln!("Count: {}", refined.len());
        for m in &refined {
            eprintln!(
                "  {} at lines {}-{} matcher={}",
                m.license_expression, m.start_line, m.end_line, m.matcher
            );
        }

        // Step 5: Group by region
        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);
        let groups = group_matches_by_region(&sorted);

        eprintln!("\n=== DETECTION GROUPS ===");
        eprintln!("Number of groups: {}", groups.len());
        for (i, group) in groups.iter().enumerate() {
            eprintln!(
                "\nGroup {} (lines {}-{}):",
                i + 1,
                group.start_line,
                group.end_line
            );
            for m in &group.matches {
                eprintln!(
                    "  {} at lines {}-{}",
                    m.license_expression, m.start_line, m.end_line
                );
            }
        }

        // Step 6: Create detections
        let detections: Vec<_> = groups
            .iter()
            .map(|g| create_detection_from_group(g))
            .collect();

        eprintln!("\n=== FINAL DETECTIONS ===");
        for (i, det) in detections.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i + 1, det.license_expression);
        }
    }

    #[test]
    fn test_gpl_mpl_is_matchable_check() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::merge_overlapping_matches;
        use crate::license_detection::query::{PositionSpan, Query};
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        let mut matched_qspans: Vec<PositionSpan> = Vec::new();

        // Phase 1a: Hash matching
        let hash_matches = hash_match(&index, &whole_run);
        eprintln!("Hash matches: {}", hash_matches.len());

        // Phase 1b: SPDX-LID matching
        let spdx_matches = spdx_lid_match(&index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        for m in &merged_spdx {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        eprintln!("SPDX matches: {}", merged_spdx.len());

        // Phase 1c: Aho matching
        let aho_matches = aho_match(&index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        for m in &merged_aho {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        eprintln!("Aho matches: {}", merged_aho.len());

        eprintln!("\n=== AHO MATCHES (DETAILS) ===");
        for m in &merged_aho {
            eprintln!(
                "  {} at tokens {}-{} (lines {}-{}) coverage={:.1}%",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.match_coverage
            );
        }

        eprintln!("\n=== MATCHED QSPANS ===");
        for (i, span) in matched_qspans.iter().enumerate() {
            let positions = span.positions();
            eprintln!("  {}: {} positions", i + 1, positions.len());
        }

        // Check if matchable
        let whole_run = query.whole_query_run();
        let is_matchable = whole_run.is_matchable(false, &matched_qspans);

        eprintln!("\n=== IS_MATCHABLE CHECK ===");
        eprintln!(
            "whole_run: start={} end={:?}",
            whole_run.start, whole_run.end
        );
        eprintln!("matched_qspans count: {}", matched_qspans.len());
        eprintln!("is_matchable: {}", is_matchable);
        eprintln!("skip_seq_matching would be: {}", !is_matchable);

        if !is_matchable {
            eprintln!(
                "\nWARNING: Seq matching would be SKIPPED because aho matches cover everything!"
            );
            eprintln!("This is likely why the combined rule is not found.");
        }
    }

    #[test]
    fn test_gpl_mpl_combined_rule_candidate() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets,
        };
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);

        eprintln!("\n=== CHECKING FOR COMBINED RULE IN CANDIDATES ===");

        let combined_rule_expr = "mpl-1.0 OR gpl-2.0-plus";
        let mut found_combined = false;

        for c in &near_dupe_candidates {
            if c.rule.license_expression == combined_rule_expr {
                found_combined = true;
                eprintln!(
                    "FOUND: {} (rid={}, rule_id={})",
                    c.rule.license_expression, c.rid, c.rule.identifier
                );
                eprintln!(
                    "  resemblance={:.3}, containment={:.3}, is_highly_resemblant={}",
                    c.score_vec_full.resemblance,
                    c.score_vec_full.containment,
                    c.score_vec_full.is_highly_resemblant
                );
            }
        }

        if !found_combined {
            eprintln!(
                "NOT FOUND: Combined rule '{}' not in top {} candidates",
                combined_rule_expr, MAX_NEAR_DUPE_CANDIDATES
            );
            eprintln!("\nAll candidates:");
            for (i, c) in near_dupe_candidates.iter().enumerate() {
                eprintln!(
                    "  {}: {} (resemblance={:.3}, containment={:.3})",
                    i + 1,
                    c.rule.license_expression,
                    c.score_vec_full.resemblance,
                    c.score_vec_full.containment
                );
            }
        }

        assert!(
            found_combined,
            "Combined rule 'mpl-1.0 OR gpl-2.0-plus' should be found as near-dupe candidate"
        );
    }

    #[test]
    fn test_gpl_mpl_match_grouping_analysis() {
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        // Get all phase 1 matches
        let mut all_matches = Vec::new();
        all_matches.extend(hash_match(&index, &whole_run));
        all_matches.extend(spdx_lid_match(&index, &query));
        all_matches.extend(aho_match(&index, &whole_run));

        // Sort by line
        all_matches.sort_by(|a, b| a.start_line.cmp(&b.start_line));

        eprintln!("\n=== LINE DISTANCE ANALYSIS ===");
        for i in 0..all_matches.len().saturating_sub(1) {
            let cur = &all_matches[i];
            let next = &all_matches[i + 1];
            let line_gap = next.start_line.saturating_sub(cur.end_line);
            eprintln!(
                "Between {} (ends {}) and {} (starts {}): gap = {}",
                cur.license_expression,
                cur.end_line,
                next.license_expression,
                next.start_line,
                line_gap
            );
        }

        // Check the distance between MPL and GPL regions
        let mpl_matches: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression.contains("mpl"))
            .collect();
        let gpl_matches: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression.contains("gpl"))
            .collect();

        if let (Some(mpl), Some(gpl)) = (mpl_matches.first(), gpl_matches.first()) {
            let gap = gpl.start_line.saturating_sub(mpl.end_line);
            eprintln!(
                "\nGap between MPL (ends {}) and GPL (starts {}): {}",
                mpl.end_line, gpl.start_line, gap
            );
            eprintln!("LINES_THRESHOLD = 4 (matches within 4 lines are grouped)");
        }
    }

    #[test]
    fn test_gpl_mpl_failing_phase1_filtering_matches_python() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::{PositionSpan, Query};
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        let aho_matches = aho_match(&index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        eprintln!("=== PHASE 1 FILTERING ===");
        eprintln!("aho_matches: {}", aho_matches.len());
        eprintln!("merged_aho: {}", merged_aho.len());

        let (non_contained, discarded_contained) = filter_contained_matches(&merged_aho);
        eprintln!(
            "after filter_contained: {} kept, {} discarded",
            non_contained.len(),
            discarded_contained.len()
        );

        for m in &non_contained {
            eprintln!(
                "  KEPT: {} tokens {}-{}",
                m.license_expression, m.start_token, m.end_token
            );
        }

        let (filtered, discarded_overlapping) =
            filter_overlapping_matches(non_contained.clone(), &index);
        eprintln!(
            "after filter_overlapping: {} kept, {} discarded",
            filtered.len(),
            discarded_overlapping.len()
        );

        for m in &filtered {
            eprintln!(
                "  FINAL: {} tokens {}-{}",
                m.license_expression, m.start_token, m.end_token
            );
        }

        let mut matched_qspans: Vec<PositionSpan> = Vec::new();
        for m in &filtered {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }

        let whole_run = query.whole_query_run();
        let is_matchable = whole_run.is_matchable(false, &matched_qspans);

        eprintln!("\n=== IS_MATCHABLE CHECK ===");
        eprintln!("matched_qspans: {}", matched_qspans.len());
        eprintln!("is_matchable: {}", is_matchable);

        assert_eq!(
            filtered.len(),
            3,
            "Phase 1 aho filtering should produce 3 matches (like Python), not 4"
        );

        assert!(
            is_matchable,
            "After proper filtering, is_matchable should be True (position 0 should remain uncovered)"
        );
    }

    #[test]
    fn test_gpl_mpl_main_pipeline_skip_seq_matching_bug() {
        // This test demonstrates the bug: the main pipeline currently skips seq matching
        // because it only calls filter_contained_matches (not filter_overlapping_matches)
        // after aho matching, leaving 4 matches that cover all high_matchable positions.
        //
        // EXPECTED: is_matchable should be True (position 0 uncovered)
        // ACTUAL: is_matchable is False (all positions covered by 4 matches)
        //
        // This test will FAIL until the fix is applied.

        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::{PositionSpan, Query};
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        let aho_matches = aho_match(&index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        // This is what the main pipeline CURRENTLY does (buggy behavior)
        let (filtered_aho, _) = filter_contained_matches(&merged_aho);

        eprintln!("=== CURRENT MAIN PIPELINE BEHAVIOR (BUGGY) ===");
        eprintln!("filtered_aho count: {}", filtered_aho.len());

        let mut matched_qspans: Vec<PositionSpan> = Vec::new();
        for m in &filtered_aho {
            eprintln!(
                "  {} tokens {}-{}",
                m.license_expression, m.start_token, m.end_token
            );
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }

        let whole_run = query.whole_query_run();
        let is_matchable = whole_run.is_matchable(false, &matched_qspans);

        eprintln!("\nis_matchable (current buggy behavior): {}", is_matchable);

        // This assertion should FAIL until the fix is applied
        assert!(
            is_matchable,
            "BUG: is_matchable should be True but is False because main pipeline only calls \
             filter_contained_matches without filter_overlapping_matches. \
             Fix: Add filter_overlapping_matches after filter_contained_matches in mod.rs:193-198"
        );
    }
}
