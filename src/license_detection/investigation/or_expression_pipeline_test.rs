//! Investigation test for OR expression handling.
//!
//! ## Issue
//! The BSL-1.0_or_MIT.txt file should produce `["mit OR boost-1.0"]` as a single
//! combined expression, but Rust produces `["mit", "boost-1.0"]` as separate matches.
//!
//! **Expected (Python):** `["mit OR boost-1.0"]`
//! **Actual (Rust):** `["mit", "boost-1.0"]`
//!
//! ## Root Cause Found
//!
//! The bug is in how `detect()` handles the relationship between aho matches and seq matches:
//!
//! 1. Aho matches are found and refined via `refine_aho_matches()`
//! 2. The aho matches' spans are subtracted from the query via `query.subtract(&span)`
//! 3. Seq matching runs and finds the OR expression `mit OR boost-1.0`
//! 4. During refinement, the OR expression is filtered out because its span overlaps
//!    with the already-matched aho matches (mit + boost-1.0 separately)
//!
//! **Python Behavior:** Python's `get_exact_matches()` returns immediately if aho matches
//! cover the entire text, so seq matching is skipped. When seq matching runs, Python
//! preserves the higher-quality OR expression match.
//!
//! **Rust Behavior:** Rust runs seq matching even when aho matches exist, then filters
//! based on overlap, which incorrectly removes the OR expression.
//!
//! ## Fix Location
//!
//! The fix should be in `src/license_detection/mod.rs` in the `detect()` function:
//! - Either skip seq matching when aho matches already cover the content well
//! - Or prioritize OR expression matches over individual license matches during refinement
//! - Or preserve the higher-scoring OR match during overlap filtering

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
        let path = PathBuf::from(
            "testdata/license-golden/datadriven/external/fossology-tests/Dual-license/BSL-1.0_or_MIT.txt",
        );
        std::fs::read_to_string(&path).ok()
    }

    /// Test final detection output - this FAILS showing the bug
    #[test]
    fn test_bsl_mit_or_final_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== FINAL DETECTIONS ===");
        eprintln!("Number of detections: {}", detections.len());

        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            eprintln!("  Number of matches: {}", det.matches.len());

            for (j, m) in det.matches.iter().enumerate() {
                eprintln!("    Match {}:", j + 1);
                eprintln!("      license_expression: {}", m.license_expression);
                eprintln!("      matcher: {}", m.matcher);
                eprintln!("      lines: {}-{}", m.start_line, m.end_line);
                eprintln!("      rule_identifier: {}", m.rule_identifier);
            }
        }

        // EXPECTED: Single detection with "mit OR boost-1.0"
        // ACTUAL: "mit" detection with two separate matches
        assert_eq!(
            detections.len(),
            1,
            "Should have exactly 1 detection, got {}",
            detections.len()
        );

        assert_eq!(
            detections[0].license_expression,
            Some("mit OR boost-1.0".to_string()),
            "Expected 'mit OR boost-1.0', got '{:?}'",
            detections[0].license_expression
        );
    }

    /// Full pipeline investigation - shows OR expression IS found in seq matching
    /// but is then filtered out during refinement
    #[test]
    fn test_bsl_mit_or_full_pipeline() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, seq_match_with_candidates,
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

        eprintln!("\n{}", "=".repeat(80));
        eprintln!("PHASE 1: AHO-CORASICK MATCHES");
        eprintln!("{}", "=".repeat(80));

        let whole_run = query.whole_query_run();
        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("Raw aho matches count: {}", aho_matches.len());

        let aho_or_count = aho_matches
            .iter()
            .filter(|m| m.license_expression.contains(" OR "))
            .count();
        eprintln!("OR expression matches in aho: {}", aho_or_count);

        eprintln!("\n{}", "=".repeat(80));
        eprintln!("PHASE 2: SEQUENCE MATCHING");
        eprintln!("{}", "=".repeat(80));

        let candidates = compute_candidates_with_msets(&index, &whole_run, false, 70);
        eprintln!("Sequence candidates count: {}", candidates.len());

        if let Some(cand) = candidates.first() {
            eprintln!("Top candidate: {} (rid={})", cand.rule.identifier, cand.rid);
        }

        let seq_matches = seq_match_with_candidates(&index, &whole_run, &candidates);
        eprintln!("Sequence matches count: {}", seq_matches.len());

        let seq_or_matches: Vec<_> = seq_matches
            .iter()
            .filter(|m| m.license_expression.contains(" OR "))
            .cloned()
            .collect();
        eprintln!("\nOR expression matches in seq: {}", seq_or_matches.len());
        for m in &seq_or_matches {
            eprintln!(
                "  '{}' at lines {}-{} rule={} score={:.2}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.score
            );
        }

        eprintln!("\n{}", "=".repeat(80));
        eprintln!("INVESTIGATION SUMMARY");
        eprintln!("{}", "=".repeat(80));

        eprintln!("\nPipeline Stage Analysis:");
        eprintln!("  Phase 1 (Aho):  {} OR matches found", aho_or_count);
        eprintln!("  Phase 2 (Seq):  {} OR matches found", seq_or_matches.len());

        eprintln!("\nKEY FINDING: The OR expression 'mit OR boost-1.0' IS found in seq matching!");
        eprintln!("The bug is that it gets filtered out during refinement because");
        eprintln!("the aho matches (mit, boost-1.0) have already claimed those regions.");

        assert!(
            !seq_or_matches.is_empty(),
            "OR expression should be found in sequence matching"
        );
    }

    /// Test mimicking detect() WITHOUT query subtraction - PASSES
    /// This proves the bug is in query subtraction / overlap handling
    #[test]
    fn test_bsl_mit_or_detect_flow_no_subtract() {
        let Some(_engine) = get_engine() else { return };
        let text = read_test_file().expect("Test file should exist");

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, sort_matches_by_line,
        };
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            merge_overlapping_matches, refine_aho_matches, refine_matches,
            refine_matches_without_false_positive_filter, split_weak_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, seq_match_with_candidates,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::license_detection::spdx_mapping::build_spdx_mapping;
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses.clone());
        let spdx_mapping = build_spdx_mapping(&licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let mut all_matches = Vec::new();

        // Phase 1b: SPDX-LID matching
        let spdx_matches = spdx_lid_match(&index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        all_matches.extend(merged_spdx);

        // Phase 1c: AHO matching with refinement
        let whole_run = query.whole_query_run();
        let aho_matches = aho_match(&index, &whole_run);
        let refined_aho = refine_aho_matches(&index, aho_matches, &query);
        all_matches.extend(refined_aho);

        // Phase 2-4: Sequence matching (WITHOUT query subtraction!)
        let candidates = compute_candidates_with_msets(&index, &whole_run, false, 70);
        let seq_matches = seq_match_with_candidates(&index, &whole_run, &candidates);
        let merged_seq = merge_overlapping_matches(&seq_matches);
        all_matches.extend(merged_seq);

        // Final refinement
        let merged_matches =
            refine_matches_without_false_positive_filter(&index, all_matches.clone(), &query);
        let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
        let all_matches: Vec<_> = good_matches.into_iter().chain(weak_matches).collect();
        let refined = refine_matches(&index, all_matches, &query);

        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);
        let detections: Vec<_> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(&mut detection, group, &spdx_mapping);
                detection
            })
            .collect();

        eprintln!("\n=== DETECTIONS (without query subtraction) ===");
        for (i, det) in detections.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i + 1, det.license_expression);
        }

        // This PASSES because we didn't subtract the aho match regions
        assert!(
            detections
                .iter()
                .any(|d| d.license_expression.as_deref() == Some("mit OR boost-1.0")),
            "Expected detection with 'mit OR boost-1.0' but got: {:?}",
            detections.iter().map(|d| &d.license_expression).collect::<Vec<_>>()
        );
    }
}
