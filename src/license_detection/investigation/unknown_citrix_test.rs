//! Investigation test for PLAN-014: unknown/citrix.txt
//!
//! ## Issue
//! **Expected:** `["unknown", "gpl-1.0-plus", "free-unknown", "warranty-disclaimer", "free-unknown", "free-unknown", "unknown-license-reference", "commercial-license", "unknown"]`
//!
//! ## Fix Applied
//! Changed `has_unknown()` from `.contains("unknown")` to `== "unknown"`.
//! This prevents `free-unknown` and `unknown-license-reference` from being
//! incorrectly classified as weak matches.
//!
//! ## Remaining Issues
//! After the fix, `free-unknown` matches are preserved correctly, but there
//! are additional detection pipeline differences that need investigation.

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
        let path = PathBuf::from("testdata/license-golden/datadriven/unknown/citrix.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_plan_014_rust_detection() {
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
            eprintln!("  Number of matches: {}", det.matches.len());

            for (j, m) in det.matches.iter().enumerate() {
                eprintln!("    Match {}:", j + 1);
                eprintln!("      license_expression: {}", m.license_expression);
                eprintln!("      matcher: {}", m.matcher);
                eprintln!("      lines: {}-{}", m.start_line, m.end_line);
                eprintln!("      score: {:.2}", m.score);
                eprintln!("      rule_identifier: {}", m.rule_identifier);
            }
        }

        let all_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nAll license expressions: {:?}", all_expressions);

        assert!(
            all_expressions.iter().any(|e| *e == "free-unknown"),
            "free-unknown should be preserved (not replaced by unknown) after has_unknown fix"
        );

        let free_unknown_count = all_expressions
            .iter()
            .filter(|e| **e == "free-unknown")
            .count();
        assert_eq!(
            free_unknown_count, 3,
            "Should have 3 free-unknown matches (not being replaced by unknown)"
        );
    }

    #[test]
    fn test_plan_014_phase1_matches() {
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

        eprintln!("=== PHASE 1 MATCHES ===");

        let hash_matches = hash_match(&index, &whole_run);
        eprintln!("\nHash matches: {}", hash_matches.len());
        for m in &hash_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={})",
                m.license_expression, m.start_line, m.end_line, m.matcher
            );
        }

        let spdx_matches = spdx_lid_match(&index, &query);
        eprintln!("\nSPDX-LID matches: {}", spdx_matches.len());
        for m in &spdx_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={})",
                m.license_expression, m.start_line, m.end_line, m.matcher
            );
        }

        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("\nAho matches: {}", aho_matches.len());
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

        let mut all_phase1 = Vec::new();
        all_phase1.extend(hash_matches);
        all_phase1.extend(spdx_matches);
        all_phase1.extend(aho_matches);

        eprintln!("\n=== ALL PHASE 1 MATCHES ===");
        eprintln!("Total: {}", all_phase1.len());

        let free_unknown_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression.contains("free-unknown"))
            .collect();
        let unknown_license_ref_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression.contains("unknown-license-reference"))
            .collect();

        eprintln!(
            "\nfree-unknown matches in phase 1: {}",
            free_unknown_matches.len()
        );
        for m in &free_unknown_matches {
            eprintln!(
                "  {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        eprintln!(
            "\nunknown-license-reference matches in phase 1: {}",
            unknown_license_ref_matches.len()
        );
        for m in &unknown_license_ref_matches {
            eprintln!(
                "  {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }
    }

    #[test]
    fn test_plan_014_refine_pipeline() {
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

        let refined = refine_matches(&index, all_matches.clone(), &query);
        eprintln!("\n=== AFTER refine_matches ===");
        eprintln!("Count: {}", refined.len());
        for m in &refined {
            eprintln!(
                "  {} at lines {}-{} matcher={} rule={}",
                m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
            );
        }

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
                    "  {} at lines {}-{} rule={}",
                    m.license_expression, m.start_line, m.end_line, m.rule_identifier
                );
            }
        }

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
    fn test_plan_014_search_for_rules() {
        let Some(_engine) = get_engine() else { return };

        use crate::license_detection::index::build_index;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        eprintln!("=== SEARCHING FOR RULES ===");

        let free_unknown_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("free-unknown"))
            .collect();
        eprintln!("\nfree-unknown rules: {}", free_unknown_rules.len());
        for r in &free_unknown_rules {
            eprintln!("  {} -> {}", r.identifier, r.license_expression);
        }

        let unknown_license_ref_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("unknown-license-reference"))
            .collect();
        eprintln!(
            "\nunknown-license-reference rules: {}",
            unknown_license_ref_rules.len()
        );
        for r in &unknown_license_ref_rules {
            eprintln!("  {} -> {}", r.identifier, r.license_expression);
        }

        let commercial_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("commercial-license"))
            .collect();
        eprintln!("\ncommercial-license rules: {}", commercial_rules.len());
        for r in &commercial_rules {
            eprintln!("  {} -> {}", r.identifier, r.license_expression);
        }
    }
}
