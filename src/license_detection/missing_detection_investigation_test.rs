//! Investigation test for PLAN-063: Missing lgpl-2.1-plus detection in e2fsprogs.txt
//!
//! Python finds 5 matches:
//! 1. gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert at lines 3-11
//! 2. bsd-new at lines 16-40
//! 3. lgpl-2.1-plus at lines 44-47
//! 4. lgpl-2.1-plus at lines 53-56
//! 5. lgpl-2.1-plus at lines 80-83
//!
//! Rust finds 4 matches (missing one lgpl-2.1-plus).

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
        let path = PathBuf::from("testdata/license-golden/datadriven/lic1/e2fsprogs.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_e2fsprogs_detection_count() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine.detect(&text).expect("Detection should succeed");

        let all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();

        eprintln!("Total detections: {}", detections.len());
        eprintln!("Total matches: {}", all_matches.len());

        for (i, m) in all_matches.iter().enumerate() {
            eprintln!(
                "{}. {} at lines {}-{} (matcher={})",
                i + 1,
                m.license_expression,
                m.start_line,
                m.end_line,
                m.matcher
            );
        }

        let lgpl_matches: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();

        eprintln!("\nLGPL-2.1-plus matches: {}", lgpl_matches.len());
        for (i, m) in lgpl_matches.iter().enumerate() {
            eprintln!(
                "  {}. lines {}-{} matcher={} score={:.1}",
                i + 1,
                m.start_line,
                m.end_line,
                m.matcher,
                m.score
            );
        }

        assert_eq!(
            lgpl_matches.len(),
            3,
            "Expected 3 lgpl-2.1-plus matches (Python finds 3)"
        );
    }

    #[test]
    fn test_e2fsprogs_refine_pipeline_step_by_step() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
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

        eprintln!("=== Initial matches: {} ===", all_matches.len());

        let lgpl_initial: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("LGPL-2.1-plus initial: {}", lgpl_initial.len());
        for (i, m) in lgpl_initial.iter().enumerate() {
            eprintln!(
                "  {}. lines {}-{} rule={} tokens={}-{}",
                i + 1,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.start_token,
                m.end_token
            );
        }

        let merged = merge_overlapping_matches(&all_matches);
        let lgpl_after_merge: Vec<_> = merged
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!(
            "\nAfter merge_overlapping: {} LGPL-2.1-plus",
            lgpl_after_merge.len()
        );
        for (i, m) in lgpl_after_merge.iter().enumerate() {
            eprintln!(
                "  {}. lines {}-{} rule={} tokens={}-{}",
                i + 1,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.start_token,
                m.end_token
            );
        }

        let (non_contained, discarded_contained) = filter_contained_matches(&merged);
        let lgpl_after_contained: Vec<_> = non_contained
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!(
            "\nAfter filter_contained: {} LGPL-2.1-plus (discarded {})",
            lgpl_after_contained.len(),
            discarded_contained.len()
        );

        let lgpl_discarded: Vec<_> = discarded_contained
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        if !lgpl_discarded.is_empty() {
            eprintln!("\nDiscarded LGPL-2.1-plus in filter_contained:");
            for (i, m) in lgpl_discarded.iter().enumerate() {
                eprintln!(
                    "  {}. lines {}-{} rule={}",
                    i + 1,
                    m.start_line,
                    m.end_line,
                    m.rule_identifier
                );
            }
        }

        let (kept, discarded_overlapping) = filter_overlapping_matches(non_contained, &index);
        let lgpl_after_overlapping: Vec<_> = kept
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!(
            "\nAfter filter_overlapping: {} LGPL-2.1-plus (discarded {})",
            lgpl_after_overlapping.len(),
            discarded_overlapping.len()
        );

        let lgpl_discarded_overlap: Vec<_> = discarded_overlapping
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        if !lgpl_discarded_overlap.is_empty() {
            eprintln!("\nDiscarded LGPL-2.1-plus in filter_overlapping:");
            for (i, m) in lgpl_discarded_overlap.iter().enumerate() {
                eprintln!(
                    "  {}. lines {}-{} rule={}",
                    i + 1,
                    m.start_line,
                    m.end_line,
                    m.rule_identifier
                );
            }
        }

        let refined = refine_matches(&index, all_matches.clone(), &query);
        let lgpl_final: Vec<_> = refined
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!(
            "\nFinal refined matches: {} LGPL-2.1-plus",
            lgpl_final.len()
        );
        for (i, m) in lgpl_final.iter().enumerate() {
            eprintln!(
                "  {}. lines {}-{} rule={}",
                i + 1,
                m.start_line,
                m.end_line,
                m.rule_identifier
            );
        }
    }

    #[test]
    fn test_e2fsprogs_grouping_phase() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, post_process_detections, sort_matches_by_line,
        };
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::refine_matches;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::license_detection::spdx_mapping::build_spdx_mapping;
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);
        let spdx_mapping =
            build_spdx_mapping(&index.licenses_by_key.values().cloned().collect::<Vec<_>>());

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        let mut all_matches = Vec::new();
        all_matches.extend(hash_match(&index, &whole_run));
        all_matches.extend(spdx_lid_match(&index, &query));
        all_matches.extend(aho_match(&index, &whole_run));

        let refined = refine_matches(&index, all_matches, &query);

        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        eprintln!("=== Sorted refined matches: {} ===", sorted.len());
        for m in &sorted {
            eprintln!(
                "  {} at lines {}-{}",
                m.license_expression, m.start_line, m.end_line
            );
        }

        let groups = group_matches_by_region(&sorted);
        eprintln!("\n=== Groups: {} ===", groups.len());
        for (i, group) in groups.iter().enumerate() {
            eprintln!(
                "Group {} (lines {}-{}):",
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

        let lgpl_in_groups: Vec<_> = groups
            .iter()
            .flat_map(|g| g.matches.iter())
            .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("\nLGPL-2.1-plus in groups: {}", lgpl_in_groups.len());
        for (i, m) in lgpl_in_groups.iter().enumerate() {
            eprintln!(
                "  {}. lines {}-{} rule={}",
                i + 1,
                m.start_line,
                m.end_line,
                m.rule_identifier
            );
        }

        let detections: Vec<_> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(&mut detection, group, &spdx_mapping);
                detection
            })
            .collect();

        eprintln!(
            "\n=== Detections before post_process: {} ===",
            detections.len()
        );
        for (i, d) in detections.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i + 1, d.license_expression);
        }

        let processed = post_process_detections(detections, 0.0);
        eprintln!(
            "\n=== Detections after post_process: {} ===",
            processed.len()
        );
        for (i, d) in processed.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i + 1, d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "  {} at lines {}-{}",
                    m.license_expression, m.start_line, m.end_line
                );
            }
        }
    }
}
