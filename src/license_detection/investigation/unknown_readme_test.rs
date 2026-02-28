//! Investigation test for PLAN-012: unknown/README.md
//!
//! ## Issue
//! **Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown-license-reference"]`
//! **Actual:** `["unknown"]`
//!
//! ## Pipeline stages to investigate:
//! 1. Aho matching - what matches are found?
//! 2. Seq matching - what candidates/matches?
//! 3. Filtering (contained, overlapping) - what gets filtered?
//! 4. Unknown matching - how are unknown-license-reference matches found?
//! 5. Detection grouping - how are matches grouped?

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
        let path = PathBuf::from("testdata/license-golden/datadriven/unknown/README.md");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_plan_012_rust_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, true)
            .expect("Detection should succeed");

        eprintln!("\n=== RUST DETECTIONS (with unknown_licenses=true) ===");
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

        assert_eq!(
            all_expressions,
            vec![
                "unknown-license-reference",
                "unknown-license-reference",
                "unknown-license-reference"
            ],
            "Expected 3 unknown-license-reference matches"
        );
    }

    #[test]
    fn test_plan_012_aho_matches() {
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

        // Check for unknown-license-reference matches in phase 1
        let unknown_ref_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression == "unknown-license-reference")
            .collect();

        eprintln!("\n=== UNKNOWN-LICENSE-REFERENCE MATCHES IN AHO ===");
        eprintln!("Count: {}", unknown_ref_matches.len());
        for m in &unknown_ref_matches {
            eprintln!(
                "  {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }
    }

    #[test]
    fn test_plan_012_unknown_license_reference_rules() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
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

        eprintln!("\n=== RULES MATCHING 'unknown-license-reference' ===");
        let unknown_ref_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("unknown-license-reference"))
            .take(10)
            .collect();

        eprintln!("Found {} rules (showing first 10)", unknown_ref_rules.len());
        for r in &unknown_ref_rules {
            eprintln!(
                "  {} - text preview: {:?}",
                r.identifier,
                &r.text.chars().take(50).collect::<String>()
            );
        }

        // Search for specific text patterns from the test file
        let clean_text = strip_utf8_bom_str(&text);

        // Line 47: "Copyright © 2018. All rights reserved."
        eprintln!("\n=== SEARCHING FOR COPYRIGHT/ALL RIGHTS RESERVED RULES ===");
        let copyright_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.text.contains("Copyright") && r.text.contains("All rights reserved"))
            .take(10)
            .collect();
        eprintln!(
            "Found {} rules with 'Copyright' AND 'All rights reserved' (showing first 10)",
            copyright_rules.len()
        );
        for r in &copyright_rules {
            eprintln!(
                "  {} - license_expression: {}",
                r.identifier, r.license_expression
            );
        }

        // Check if any of these patterns match the text
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();
        let aho_matches = aho_match(&index, &whole_run);

        eprintln!("\n=== ALL AHO MATCHES IN TEXT ===");
        for m in &aho_matches {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }
    }

    #[test]
    fn test_plan_012_full_pipeline_debug() {
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
            refine_matches, refine_matches_without_false_positive_filter, split_weak_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
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

        // Phase 1 matches
        let mut all_matches = Vec::new();
        let hash_matches = hash_match(&index, &whole_run);
        all_matches.extend(hash_matches);
        let spdx_matches = spdx_lid_match(&index, &query);
        all_matches.extend(spdx_matches);
        let aho_matches = aho_match(&index, &whole_run);
        all_matches.extend(aho_matches);

        eprintln!("\n=== PHASE 1 ALL MATCHES ===");
        eprintln!("Count: {}", all_matches.len());
        for m in &all_matches {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{}",
                m.license_expression, m.start_line, m.end_line, m.start_token, m.end_token
            );
        }

        // Step 1: refine_matches_without_false_positive_filter
        let merged_matches =
            refine_matches_without_false_positive_filter(&index, all_matches.clone(), &query);

        eprintln!("\n=== AFTER refine_matches_without_false_positive_filter ===");
        eprintln!("Count: {}", merged_matches.len());
        for m in &merged_matches {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} has_unknown={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.has_unknown()
            );
        }

        // Step 2: split_weak_matches
        let (good_matches, weak_matches) = split_weak_matches(&merged_matches);

        eprintln!("\n=== AFTER split_weak_matches ===");
        eprintln!("Good matches: {}", good_matches.len());
        for m in &good_matches {
            eprintln!(
                "  GOOD: {} at lines {}-{}",
                m.license_expression, m.start_line, m.end_line
            );
        }
        eprintln!("Weak matches: {}", weak_matches.len());
        for m in &weak_matches {
            eprintln!(
                "  WEAK: {} at lines {}-{} has_unknown={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.has_unknown()
            );
        }

        // Step 3: unknown_match
        let unknown_matches = unknown_match(&index, &query, &good_matches);
        eprintln!("\n=== UNKNOWN MATCHES ===");
        eprintln!("Count: {}", unknown_matches.len());
        for m in &unknown_matches {
            eprintln!(
                "  UNKNOWN: {} at lines {}-{}",
                m.license_expression, m.start_line, m.end_line
            );
        }

        // Combine good + unknown + weak
        let mut all_matches = good_matches;
        all_matches.extend(unknown_matches);
        all_matches.extend(weak_matches);

        eprintln!("\n=== BEFORE FINAL refine_matches ===");
        eprintln!("Count: {}", all_matches.len());
        for m in &all_matches {
            eprintln!(
                "  {} at lines {}-{}",
                m.license_expression, m.start_line, m.end_line
            );
        }

        // Step 5: Final refine WITH false positive filtering
        let refined = refine_matches(&index, all_matches, &query);

        eprintln!("\n=== AFTER FINAL refine_matches ===");
        eprintln!("Count: {}", refined.len());
        for m in &refined {
            eprintln!(
                "  {} at lines {}-{}",
                m.license_expression, m.start_line, m.end_line
            );
        }

        // Group into detections
        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);
        let groups = group_matches_by_region(&sorted);

        eprintln!("\n=== DETECTION GROUPS ===");
        for (i, group) in groups.iter().enumerate() {
            eprintln!(
                "Group {}: lines {}-{}",
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
    }

    #[test]
    fn test_plan_012_unknown_automaton_debug() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::query::Query;
        use crate::utils::text::strip_utf8_bom_str;

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, engine.index()).expect("Query creation failed");

        eprintln!("\n=== UNKNOWN AUTOMATON DEBUG ===");
        eprintln!("Query tokens: {}", query.tokens.len());
        eprintln!("len_legalese: {}", engine.index().len_legalese);

        // Count ngram matches in entire document
        let tokens = &query.tokens;
        let region_bytes: Vec<u8> = tokens.iter().flat_map(|tid| tid.to_le_bytes()).collect();

        let mut match_count = 0;
        for _ in engine.index().unknown_automaton.find_iter(&region_bytes) {
            match_count += 1;
        }
        eprintln!("Total ngram matches in document: {}", match_count);

        // Check hispan
        let hispan: usize = (0..tokens.len())
            .filter(|&pos| (tokens[pos] as usize) < engine.index().len_legalese)
            .count();
        eprintln!("Hispan (high-value legalese tokens): {}", hispan);

        // Check thresholds
        let region_length = tokens.len();
        eprintln!("Region length: {}", region_length);
        eprintln!("Passes length check (>= 24): {}", region_length >= 24);
        eprintln!("Passes hispan check (>= 5): {}", hispan >= 5);

        // Check with weak matches covered
        let whole_run = query.whole_query_run();
        let aho_matches = aho_match(engine.index(), &whole_run);

        let unknown_ref_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression == "unknown-license-reference")
            .collect();

        eprintln!("\n=== UNKNOWN-LICENSE-REFERENCE MATCHES ===");
        for m in &unknown_ref_matches {
            eprintln!(
                "  {} at tokens {}-{}",
                m.license_expression, m.start_token, m.end_token
            );
        }

        // Compute covered positions from weak matches
        let mut covered: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for m in &unknown_ref_matches {
            for pos in m.start_token..m.end_token {
                covered.insert(pos);
            }
        }
        eprintln!(
            "Covered positions from weak matches: {} tokens",
            covered.len()
        );

        // Find unmatched regions
        let mut unmatched_count = 0;
        let mut region_start = None;
        for pos in 0..tokens.len() {
            if !covered.contains(&pos) {
                if region_start.is_none() {
                    region_start = Some(pos);
                    unmatched_count += 1;
                }
            } else if region_start.is_some() {
                region_start = None;
            }
        }
        eprintln!("Unmatched regions count: {}", unmatched_count);
    }
}
