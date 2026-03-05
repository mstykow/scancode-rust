//! Investigation test for PLAN-013: unknown/cigna-go-you-mobile-app-eula.txt
//!
//! ## Issue
//! **Expected:** `["proprietary-license", "proprietary-license", "unknown-license-reference", "warranty-disclaimer", "proprietary-license", "warranty-disclaimer", "unknown-license-reference", "unknown"]`
//! **Actual:** `["proprietary-license", "proprietary-license", "unknown", "warranty-disclaimer", "warranty-disclaimer", "warranty-disclaimer", "unknown-license-reference", "unknown"]`
//!
//! ## Differences
//! - Position 2: Expected `unknown-license-reference`, Actual `unknown`
//! - Position 4: Expected `proprietary-license`, Actual `warranty-disclaimer`
//! - Extra `warranty-disclaimer`, missing `proprietary-license`

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
            "testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt",
        );
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_plan_013_rust_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, true)
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
                eprintln!("      tokens: {}-{}", m.start_token, m.end_token);
                eprintln!("      score: {:.2}", m.score);
                eprintln!("      match_coverage: {:.1}", m.match_coverage);
                eprintln!("      rule_identifier: {}", m.rule_identifier);
            }
        }

        let all_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nAll license expressions: {:?}", all_expressions);

        let expected = vec![
            "proprietary-license",
            "proprietary-license",
            "unknown-license-reference",
            "warranty-disclaimer",
            "proprietary-license",
            "warranty-disclaimer",
            "unknown-license-reference",
            "unknown",
        ];

        eprintln!("\nExpected: {:?}", expected);
        eprintln!("Actual:   {:?}", all_expressions);
    }

    #[test]
    fn test_plan_013_debug_pipeline() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, seq_match_with_candidates, MAX_NEAR_DUPE_CANDIDATES,
        };
        use crate::license_detection::unknown_match::unknown_match;
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

        eprintln!("\n=== QUERY INFO ===");
        eprintln!(
            "Whole run: start={:?}, end={:?}",
            whole_run.start, whole_run.end
        );

        // STEP 1: Raw aho matches
        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("\n=== STEP 1: RAW AHO MATCHES ===");
        eprintln!("Count: {}", aho_matches.len());

        // STEP 2: After merge_overlapping_matches
        let merged = merge_overlapping_matches(&aho_matches);
        eprintln!("\n=== STEP 2: AFTER merge_overlapping_matches ===");
        eprintln!("Count: {}", merged.len());

        // STEP 3: After filter_contained_matches
        let (non_contained, _discarded_contained) = filter_contained_matches(&merged);
        eprintln!("\n=== STEP 3: AFTER filter_contained_matches ===");
        eprintln!("Kept: {}", non_contained.len());

        // STEP 4: After filter_overlapping_matches
        let (non_overlapping, _discarded_overlapping) =
            filter_overlapping_matches(non_contained, &index);
        eprintln!("\n=== STEP 4: AFTER filter_overlapping_matches ===");
        eprintln!("Kept: {}", non_overlapping.len());

        // Check if the rule we expect exists
        eprintln!("\n=== CHECKING FOR RULE unknown-license-reference_298.RULE ===");
        let target_rule = "unknown-license-reference_298.RULE";
        let mut found_rule = false;
        for rule in &index.rules_by_rid {
            if rule.identifier == target_rule {
                found_rule = true;
                eprintln!("Found rule: {}", rule.identifier);
                eprintln!("  license_expression: {}", rule.license_expression);
                eprintln!("  text: {}", rule.text);
                eprintln!("  tokens: {:?}", rule.tokens);
                eprintln!("  is_small: {}", rule.is_small);
                eprintln!("  is_tiny: {}", rule.is_tiny);
                eprintln!("  is_license_reference: {}", rule.is_license_reference);
                eprintln!("  relevance: {}", rule.relevance);
            }
        }
        if !found_rule {
            eprintln!("Rule {} NOT FOUND in index!", target_rule);
        }

        // STEP 5: Sequence matching - check candidates more closely
        eprintln!("\n=== STEP 5: SEQUENCE MATCHING ===");
        eprintln!("Checking near-dupe candidates (high_resemblance=true)...");
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("Near-dupe candidates: {}", near_dupe_candidates.len());

        eprintln!("\nChecking regular candidates (high_resemblance=false)...");
        let regular_candidates =
            compute_candidates_with_msets(&index, &whole_run, false, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("Regular candidates: {}", regular_candidates.len());
        for cand in regular_candidates.iter().take(10) {
            let rule = &index.rules_by_rid[cand.rid as usize];
            eprintln!(
                "  {} (rid={}): score={:?}",
                rule.identifier, cand.rid, cand.score_vec_full
            );
        }

        let seq_matches = seq_match_with_candidates(&index, &whole_run, &near_dupe_candidates);
        eprintln!("\nSequence matches from near-dupe: {}", seq_matches.len());

        let seq_matches_regular =
            seq_match_with_candidates(&index, &whole_run, &regular_candidates);
        eprintln!(
            "Sequence matches from regular: {}",
            seq_matches_regular.len()
        );
        for m in &seq_matches_regular {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={} coverage={:.1}%",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier,
                m.match_coverage
            );
        }

        // STEP 6: Unknown match detection
        eprintln!("\n=== STEP 6: UNKNOWN MATCH DETECTION ===");
        let good_matches: Vec<_> = non_overlapping
            .iter()
            .chain(seq_matches.iter())
            .cloned()
            .collect();
        let unknown_matches = unknown_match(&index, &query, &good_matches);
        eprintln!("Unknown matches: {}", unknown_matches.len());
    }
}
