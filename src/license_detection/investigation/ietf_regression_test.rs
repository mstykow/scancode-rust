//! Investigation test for regression caused by filter_overlapping_matches fix.
//!
//! ## ACTUAL ISSUE (discovered during investigation)
//! The file `gpl-2.0-plus_and_mit_1.txt` is missing one `mit` match.
//!
//! **Expected:** `["gpl-2.0-plus", "mit", "mit", "gpl-1.0-plus"]`
//! **Actual:** `["gpl-2.0-plus", "mit", "gpl-1.0-plus"]` (missing one "mit")
//!
//! ## Original reported issue (not reproducible)
//! The file `ietf_1.txt` was reported to detect extra `other-permissive` but both
//! Python and Rust correctly detect only `["ietf"]`.
//!
//! ## Investigation approach
//! Use the incremental test method to find where the second `mit` is incorrectly filtered:
//! 1. Check raw aho matches (before any filtering)
//! 2. Check after `merge_overlapping_matches`
//! 3. Check after `filter_contained_matches`
//! 4. Check after `filter_overlapping_matches`
//! 5. Compare with Python reference at each stage

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

    fn read_gpl_mit_file() -> Option<String> {
        let path =
            PathBuf::from("testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_mit_1.txt");
        std::fs::read_to_string(&path).ok()
    }

    fn read_ietf_file() -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic4/ietf_1.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_ietf_correctly_detects_only_ietf() {
        // This test verifies ietf_1.txt correctly detects only "ietf"
        // (the original reported issue was not reproducible)
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_ietf_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let all_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("ietf_1.txt expressions: {:?}", all_expressions);

        assert_eq!(
            all_expressions,
            vec!["ietf"],
            "ietf_1.txt: Expected only ietf, got {:?}",
            all_expressions
        );
    }

    #[test]
    fn test_gpl_mit_missing_second_mit() {
        // This test demonstrates the actual regression
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_gpl_mit_file() else {
            return;
        };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== GPL-MIT DETECTIONS ===");
        for (i, det) in detections.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i + 1, det.license_expression);
            for m in &det.matches {
                eprintln!(
                    "  {} at lines {}-{} (matcher={})",
                    m.license_expression, m.start_line, m.end_line, m.matcher
                );
            }
        }

        let all_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nAll expressions: {:?}", all_expressions);

        // Expected: ["gpl-2.0-plus", "mit", "mit", "gpl-1.0-plus"]
        // Actual: ["gpl-2.0-plus", "mit", "gpl-1.0-plus"] (missing one "mit")
        assert_eq!(
            all_expressions,
            vec!["gpl-2.0-plus", "mit", "mit", "gpl-1.0-plus"],
            "gpl-2.0-plus_and_mit_1.txt: Expected 4 expressions including 2 mits, got {:?}",
            all_expressions
        );
    }

    #[test]
    fn test_gpl_mit_pipeline_incremental() {
        // Incremental pipeline analysis to find where the second "mit" is lost
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_gpl_mit_file() else {
            return;
        };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::Query;
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

        // === STEP 0: Raw Aho matches ===
        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("\n=== STEP 0: RAW AHO MATCHES ===");
        eprintln!("Count: {}", aho_matches.len());
        for m in &aho_matches {
            eprintln!(
                "  {} at tokens {}-{} (lines {}-{}, matcher={}, coverage={:.1}%, matched_len={}, rule={})",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matcher,
                m.match_coverage,
                m.matched_length,
                m.rule_identifier
            );
        }

        // === STEP 1: After merge_overlapping_matches ===
        let merged = merge_overlapping_matches(&aho_matches);
        eprintln!("\n=== STEP 1: AFTER merge_overlapping_matches ===");
        eprintln!("Count: {}", merged.len());
        for m in &merged {
            eprintln!(
                "  {} at tokens {}-{} (lines {}-{}, matched_len={}, rule={})",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.rule_identifier
            );
        }

        // === STEP 2: After filter_contained_matches ===
        let (non_contained, discarded_contained) = filter_contained_matches(&merged);
        eprintln!("\n=== STEP 2: AFTER filter_contained_matches ===");
        eprintln!(
            "Kept: {}, Discarded: {}",
            non_contained.len(),
            discarded_contained.len()
        );

        eprintln!("\n  KEPT:");
        for m in &non_contained {
            eprintln!(
                "    {} at tokens {}-{} (lines {}-{}, matched_len={}, rule={})",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.rule_identifier
            );
        }

        eprintln!("\n  DISCARDED (contained):");
        for m in &discarded_contained {
            eprintln!(
                "    {} at tokens {}-{} (lines {}-{}, matched_len={}, rule={})",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.rule_identifier
            );
        }

        // === STEP 3: After filter_overlapping_matches ===
        let (kept_overlapping, discarded_overlapping) =
            filter_overlapping_matches(non_contained.clone(), &index);

        eprintln!("\n=== STEP 3: AFTER filter_overlapping_matches ===");
        eprintln!(
            "Kept: {}, Discarded: {}",
            kept_overlapping.len(),
            discarded_overlapping.len()
        );

        eprintln!("\n  KEPT:");
        for m in &kept_overlapping {
            eprintln!(
                "    {} at tokens {}-{} (lines {}-{}, matched_len={}, rule={})",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.rule_identifier
            );
        }

        eprintln!("\n  DISCARDED (overlapping):");
        for m in &discarded_overlapping {
            eprintln!(
                "    {} at tokens {}-{} (lines {}-{}, matched_len={}, rule={})",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.rule_identifier
            );
        }

        // Count MIT matches at each stage
        let mit_after_merge: Vec<_> = merged
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();
        let mit_after_contained: Vec<_> = non_contained
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();
        let mit_after_overlapping: Vec<_> = kept_overlapping
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();

        eprintln!("\n=== MIT MATCH COUNT AT EACH STAGE ===");
        eprintln!("After merge: {}", mit_after_merge.len());
        eprintln!("After filter_contained: {}", mit_after_contained.len());
        eprintln!("After filter_overlapping: {}", mit_after_overlapping.len());
    }

    #[test]
    fn test_gpl_mit_overlapping_detail() {
        // Detailed analysis of overlapping matches
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_gpl_mit_file() else {
            return;
        };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::Query;
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
        let merged = merge_overlapping_matches(&aho_matches);
        let (non_contained, _) = filter_contained_matches(&merged);

        eprintln!("\n=== OVERLAPPING ANALYSIS ===");
        eprintln!(
            "Input to filter_overlapping_matches: {} matches",
            non_contained.len()
        );

        // Sort for analysis (same as filter_overlapping_matches does)
        let mut matches = non_contained.clone();
        matches.sort_by(|a, b| {
            a.qstart()
                .cmp(&b.qstart())
                .then_with(|| b.hilen.cmp(&a.hilen))
                .then_with(|| b.matched_length.cmp(&a.matched_length))
                .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        });

        eprintln!("\nSorted matches:");
        for (i, m) in matches.iter().enumerate() {
            eprintln!(
                "  [{}] {} at tokens {}-{} (lines {}-{}, matched_len={}, hilen={}, license={})",
                i,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length,
                m.hilen,
                m.license_expression
            );
        }

        // Check for overlaps
        eprintln!("\n=== PAIRWISE OVERLAP CHECK ===");
        for i in 0..matches.len().saturating_sub(1) {
            for j in (i + 1)..matches.len() {
                let current = &matches[i];
                let next = &matches[j];

                let current_end = current.end_token;
                let next_start = next.start_token;

                if next_start >= current_end {
                    continue; // No overlap
                }

                let overlap = current.qoverlap(next);
                if overlap == 0 {
                    continue;
                }

                let overlap_ratio_to_next = overlap as f64 / next.matched_length as f64;
                let overlap_ratio_to_current = overlap as f64 / current.matched_length as f64;

                eprintln!(
                    "\n  Overlap between [{}] {} and [{}] {}:",
                    i, current.rule_identifier, j, next.rule_identifier
                );
                eprintln!(
                    "    Current: {} tokens {}-{} (lines {}-{}), len={}, hilen={}",
                    current.license_expression,
                    current.start_token,
                    current.end_token,
                    current.start_line,
                    current.end_line,
                    current.matched_length,
                    current.hilen
                );
                eprintln!(
                    "    Next: {} tokens {}-{} (lines {}-{}), len={}, hilen={}",
                    next.license_expression,
                    next.start_token,
                    next.end_token,
                    next.start_line,
                    next.end_line,
                    next.matched_length,
                    next.hilen
                );
                eprintln!("    Overlap: {} tokens", overlap);
                eprintln!(
                    "    Overlap ratio to current: {:.2}%",
                    overlap_ratio_to_current * 100.0
                );
                eprintln!(
                    "    Overlap ratio to next: {:.2}%",
                    overlap_ratio_to_next * 100.0
                );
                eprintln!(
                    "    Different licenses: {}",
                    current.license_expression != next.license_expression
                );
            }
        }

        // Now run the actual filter
        let (kept, discarded) = filter_overlapping_matches(non_contained, &index);
        eprintln!("\n=== FILTER RESULT ===");
        eprintln!("Kept: {} matches", kept.len());
        eprintln!("Discarded: {} matches", discarded.len());

        for m in &kept {
            eprintln!(
                "  KEPT: {} ({}) at lines {}-{}",
                m.rule_identifier, m.license_expression, m.start_line, m.end_line
            );
        }
        for m in &discarded {
            eprintln!(
                "  DISCARDED: {} ({}) at lines {}-{}",
                m.rule_identifier, m.license_expression, m.start_line, m.end_line
            );
        }
    }

    #[test]
    fn test_gpl_mit_mit30_vs_mit31() {
        // Specific test to investigate why mit_30.RULE is being filtered
        // Python keeps mit_30.RULE (qspan 202-203, len=2)
        // Rust produces mit_30.RULE (tokens 202-204, len=2) AND mit_31.RULE (tokens 202-205, len=3)
        // Then filter_contained discards mit_30.RULE as contained in mit_31.RULE
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_gpl_mit_file() else {
            return;
        };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::Query;
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

        // Get raw aho matches
        let aho_matches = aho_match(&index, &whole_run);

        // Find MIT matches specifically
        eprintln!("\n=== MIT RAW AHO MATCHES ===");
        let mit_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| {
                m.rule_identifier.contains("mit_30") || m.rule_identifier.contains("mit_31")
            })
            .collect();

        for m in &mit_matches {
            eprintln!(
                "  {} at tokens {}-{} (len={}, hilen={}, start_line={}, end_line={})",
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.matched_length,
                m.hilen,
                m.start_line,
                m.end_line
            );
        }

        // Check if mit_30.RULE exists
        let mit_30 = mit_matches
            .iter()
            .find(|m| m.rule_identifier == "mit_30.RULE");
        let mit_31 = mit_matches
            .iter()
            .find(|m| m.rule_identifier == "mit_31.RULE");

        eprintln!("\nmit_30.RULE found: {}", mit_30.is_some());
        eprintln!("mit_31.RULE found: {}", mit_31.is_some());

        if let (Some(m30), Some(m31)) = (mit_30, mit_31) {
            eprintln!("\n=== COMPARISON ===");
            eprintln!(
                "mit_30: start={}, end={}, len={}",
                m30.start_token, m30.end_token, m30.matched_length
            );
            eprintln!(
                "mit_31: start={}, end={}, len={}",
                m31.start_token, m31.end_token, m31.matched_length
            );

            // Check if mit_30 is contained in mit_31
            let same_start = m30.start_token == m31.start_token;
            let m30_end_before_m31 = m30.end_token <= m31.end_token;

            eprintln!("Same start: {}", same_start);
            eprintln!("mit_30 end <= mit_31 end: {}", m30_end_before_m31);

            if same_start && m30_end_before_m31 && m30.matched_length < m31.matched_length {
                eprintln!("\nISSUE: mit_30.RULE will be filtered as contained in mit_31.RULE!");
                eprintln!(
                    "But Python's mit_30.RULE has different token range and is NOT contained."
                );
            }
        }

        // Now trace through the actual filtering
        let merged = merge_overlapping_matches(&aho_matches);

        eprintln!("\n=== AFTER merge_overlapping_matches ===");
        let mit_merged: Vec<_> = merged
            .iter()
            .filter(|m| {
                m.rule_identifier.contains("mit_30")
                    || m.rule_identifier.contains("mit_31")
                    || m.rule_identifier == "mit.LICENSE"
            })
            .collect();
        for m in &mit_merged {
            eprintln!(
                "  {} at tokens {}-{}",
                m.rule_identifier, m.start_token, m.end_token
            );
        }

        let (non_contained, discarded_contained) = filter_contained_matches(&merged);

        eprintln!("\n=== AFTER filter_contained_matches ===");
        let mit_kept: Vec<_> = non_contained
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();
        let mit_discarded: Vec<_> = discarded_contained
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();

        eprintln!("MIT kept: {}", mit_kept.len());
        for m in &mit_kept {
            eprintln!(
                "  KEPT: {} at tokens {}-{}",
                m.rule_identifier, m.start_token, m.end_token
            );
        }

        eprintln!("MIT discarded: {}", mit_discarded.len());
        for m in &mit_discarded {
            eprintln!(
                "  DISCARDED: {} at tokens {}-{}",
                m.rule_identifier, m.start_token, m.end_token
            );
        }
    }
}
