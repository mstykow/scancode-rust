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

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

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
        eprintln!("\n=== Refined Matches: {} ===", refined.len());
        for m in &refined {
            eprintln!(
                "  {} at lines {}-{} rule={} score={:.2} matched_len={} hilen={} matcher={} len()={} coverage={:.2}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.score,
                m.matched_length,
                m.hilen(),
                m.matcher,
                m.len(),
                m.match_coverage
            );
        }

        eprintln!("\n=== split_weak_matches test ===");
        use crate::license_detection::match_refine::split_weak_matches;
        let (good, weak) = split_weak_matches(&refined);
        eprintln!("Good matches: {}", good.len());
        eprintln!("Weak matches: {}", weak.len());
        for m in &weak {
            eprintln!(
                "  WEAK: {} at lines {}-{} rule={} matcher={} len()={} coverage={:.2}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.matcher,
                m.len(),
                m.match_coverage
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

    #[test]
    fn debug_apache_header_engine_flow() {
        let Some(engine) = get_engine() else { return };
        let path =
            PathBuf::from("testdata/license-golden/datadriven/external/glc/Apache-2.0-Header.t2");
        let text = std::fs::read_to_string(&path).expect("Failed to read file");

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
            refine_matches, refine_matches_without_false_positive_filter, restore_non_overlapping,
            split_weak_matches,
        };
        use crate::license_detection::query;
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let index = engine.index();
        let clean_text = strip_utf8_bom_str(&text);
        let mut query = crate::license_detection::query::Query::new(clean_text, index).unwrap();

        let mut all_matches = Vec::new();
        let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

        // Phase 1a: Hash matching
        {
            let whole_run = query.whole_query_run();
            let hash_matches = hash_match(index, &whole_run);
            eprintln!("Hash matches: {}", hash_matches.len());
            if !hash_matches.is_empty() {
                eprintln!("Hash matches found - would return early");
            }
        }

        // Phase 1b: SPDX-LID matching
        {
            let spdx_matches = spdx_lid_match(index, &query);
            let merged_spdx = merge_overlapping_matches(&spdx_matches);
            eprintln!(
                "SPDX matches: {} (merged: {})",
                spdx_matches.len(),
                merged_spdx.len()
            );
            for m in &merged_spdx {
                if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
                if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                    let span =
                        query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
                    query.subtract(&span);
                }
            }
            all_matches.extend(merged_spdx);
        }

        // Phase 1c: Aho-Corasick matching
        {
            let whole_run = query.whole_query_run();
            let aho_matches = aho_match(index, &whole_run);
            let merged_aho = merge_overlapping_matches(&aho_matches);
            eprintln!(
                "Aho matches: {} (merged: {})",
                aho_matches.len(),
                merged_aho.len()
            );

            // Check warranty matches before filtering
            let warranty_before: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression.contains("warranty"))
                .collect();
            eprintln!(
                "Warranty matches before filter_contained: {}",
                warranty_before.len()
            );
            for m in &warranty_before {
                eprintln!(
                    "  WARRANTY BEFORE: {} at lines {}-{} rule={} start_token={} end_token={}",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.rule_identifier,
                    m.start_token,
                    m.end_token
                );
            }

            // Print ALL matches for debugging
            eprintln!("\n=== ALL merged_aho matches (sorted by start_token) ===");
            let mut sorted_matches = merged_aho.clone();
            sorted_matches.sort_by_key(|m| m.start_token);
            for m in &sorted_matches {
                eprintln!(
                    "  {} at lines {}-{} rule={} start_token={} end_token={} hilen={} len={} is_license_text={}",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.rule_identifier,
                    m.start_token,
                    m.end_token,
                    m.hilen,
                    m.matched_length,
                    m.is_license_text
                );
            }

            // Find apache and warranty matches for comparison
            let apache_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression == "apache-2.0")
                .collect();
            let warranty_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression.contains("warranty"))
                .collect();

            eprintln!("\n=== Detailed qspan containment check ===");
            eprintln!(
                "Found {} apache matches, {} warranty matches",
                apache_matches.len(),
                warranty_matches.len()
            );

            // Check containment between each apache and warranty pair
            for (i, apache) in apache_matches.iter().enumerate() {
                for (j, warranty) in warranty_matches.iter().enumerate() {
                    eprintln!(
                        "Apache[{}]: qspan {}-{} lines {}-{} rule={} hilen={} is_license_text={}",
                        i,
                        apache.start_token,
                        apache.end_token,
                        apache.start_line,
                        apache.end_line,
                        apache.rule_identifier,
                        apache.hilen,
                        apache.is_license_text
                    );
                    eprintln!(
                        "Warranty[{}]: qspan {}-{} lines {}-{} rule={} hilen={} is_license_text={}",
                        j,
                        warranty.start_token,
                        warranty.end_token,
                        warranty.start_line,
                        warranty.end_line,
                        warranty.rule_identifier,
                        warranty.hilen,
                        warranty.is_license_text
                    );
                    eprintln!(
                        "  apache.qcontains(warranty) = {}",
                        apache.qcontains(warranty)
                    );
                    eprintln!(
                        "  warranty.qcontains(apache) = {}",
                        warranty.qcontains(apache)
                    );
                }
            }

            // Find apache and warranty matches for comparison
            let apache_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression == "apache-2.0")
                .collect();
            let warranty_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression.contains("warranty"))
                .collect();

            eprintln!("\n=== Detailed qspan containment check ===");
            eprintln!(
                "Found {} apache matches, {} warranty matches",
                apache_matches.len(),
                warranty_matches.len()
            );

            // Check containment between each apache and warranty pair
            for (i, apache) in apache_matches.iter().enumerate() {
                for (j, warranty) in warranty_matches.iter().enumerate() {
                    eprintln!(
                        "Apache[{}]: qspan {}-{} lines {}-{} rule={} hilen={} is_license_text={}",
                        i,
                        apache.start_token,
                        apache.end_token,
                        apache.start_line,
                        apache.end_line,
                        apache.rule_identifier,
                        apache.hilen,
                        apache.is_license_text
                    );
                    eprintln!(
                        "Warranty[{}]: qspan {}-{} lines {}-{} rule={} hilen={} is_license_text={}",
                        j,
                        warranty.start_token,
                        warranty.end_token,
                        warranty.start_line,
                        warranty.end_line,
                        warranty.rule_identifier,
                        warranty.hilen,
                        warranty.is_license_text
                    );
                    eprintln!(
                        "  apache.qcontains(warranty) = {}",
                        apache.qcontains(warranty)
                    );
                    eprintln!(
                        "  warranty.qcontains(apache) = {}",
                        warranty.qcontains(apache)
                    );
                }
            }

            // Find apache and warranty matches for comparison
            let apache_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression == "apache-2.0")
                .collect();
            let warranty_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression.contains("warranty"))
                .collect();

            eprintln!("\n=== Detailed qspan containment check ===");
            eprintln!(
                "Found {} apache matches, {} warranty matches",
                apache_matches.len(),
                warranty_matches.len()
            );

            // Check containment between each apache and warranty pair
            for (i, apache) in apache_matches.iter().enumerate() {
                for (j, warranty) in warranty_matches.iter().enumerate() {
                    eprintln!(
                        "Apache[{}]: qspan {}-{} lines {}-{} rule={} hilen={}",
                        i,
                        apache.start_token,
                        apache.end_token,
                        apache.start_line,
                        apache.end_line,
                        apache.rule_identifier,
                        apache.hilen
                    );
                    eprintln!(
                        "Warranty[{}]: qspan {}-{} lines {}-{} rule={} hilen={}",
                        j,
                        warranty.start_token,
                        warranty.end_token,
                        warranty.start_line,
                        warranty.end_line,
                        warranty.rule_identifier,
                        warranty.hilen
                    );
                    eprintln!(
                        "  apache.qcontains(warranty) = {}",
                        apache.qcontains(warranty)
                    );
                    eprintln!(
                        "  warranty.qcontains(apache) = {}",
                        warranty.qcontains(apache)
                    );
                }
            }

            // Find apache and warranty matches for comparison
            let apache_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression == "apache-2.0")
                .collect();
            let warranty_matches: Vec<_> = merged_aho
                .iter()
                .filter(|m| m.license_expression.contains("warranty"))
                .collect();

            eprintln!("\n=== Detailed qspan containment check ===");
            eprintln!(
                "Found {} apache matches, {} warranty matches",
                apache_matches.len(),
                warranty_matches.len()
            );

            // Check containment between each apache and warranty pair
            for (i, apache) in apache_matches.iter().enumerate() {
                for (j, warranty) in warranty_matches.iter().enumerate() {
                    eprintln!(
                        "Apache[{}]: qspan {}-{} lines {}-{} rule={}",
                        i,
                        apache.start_token,
                        apache.end_token,
                        apache.start_line,
                        apache.end_line,
                        apache.rule_identifier
                    );
                    eprintln!(
                        "Warranty[{}]: qspan {}-{} lines {}-{} rule={}",
                        j,
                        warranty.start_token,
                        warranty.end_token,
                        warranty.start_line,
                        warranty.end_line,
                        warranty.rule_identifier
                    );
                    eprintln!(
                        "  apache.qcontains(warranty) = {}",
                        apache.qcontains(warranty)
                    );
                    eprintln!(
                        "  warranty.qcontains(apache) = {}",
                        warranty.qcontains(apache)
                    );
                }
            }

            let (non_contained_aho, discarded_contained) = filter_contained_matches(&merged_aho);

            eprintln!(
                "After filter_contained: kept={} discarded={}",
                non_contained_aho.len(),
                discarded_contained.len()
            );

            // Check warranty matches in discarded
            let warranty_discarded: Vec<_> = discarded_contained
                .iter()
                .filter(|m| m.license_expression.contains("warranty"))
                .collect();
            eprintln!(
                "Warranty matches in discarded_contained: {}",
                warranty_discarded.len()
            );
            for m in &warranty_discarded {
                eprintln!(
                    "  WARRANTY DISCARDED: {} at lines {}-{} rule={}",
                    m.license_expression, m.start_line, m.end_line, m.rule_identifier
                );
            }

            let (filtered_aho, discarded_overlapping) =
                filter_overlapping_matches(non_contained_aho, index);

            eprintln!(
                "After filter_overlapping: kept={} discarded={}",
                filtered_aho.len(),
                discarded_overlapping.len()
            );

            // Check warranty matches in overlapping discarded
            let warranty_overlapping: Vec<_> = discarded_overlapping
                .iter()
                .filter(|m| m.license_expression.contains("warranty"))
                .collect();
            eprintln!(
                "Warranty matches in discarded_overlapping: {}",
                warranty_overlapping.len()
            );
            for m in &warranty_overlapping {
                eprintln!(
                    "  WARRANTY OVERLAPPING: {} at lines {}-{} rule={}",
                    m.license_expression, m.start_line, m.end_line, m.rule_identifier
                );
            }

            let (restored_contained, _) =
                restore_non_overlapping(&filtered_aho, discarded_contained);
            let (restored_overlapping, _) =
                restore_non_overlapping(&filtered_aho, discarded_overlapping);

            let mut final_aho = filtered_aho;
            final_aho.extend(restored_contained);
            final_aho.extend(restored_overlapping);

            eprintln!("Final Aho matches: {}", final_aho.len());
            for m in final_aho.iter().take(5) {
                eprintln!(
                    "  AHO: {} at lines {}-{} rule={} coverage={:.2} is_license_text={} rule_length={}",
                    m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.match_coverage, m.is_license_text, m.rule_length
                );
            }

            for m in &final_aho {
                if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
                if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                    let span =
                        query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
                    query.subtract(&span);
                }
            }
            all_matches.extend(final_aho);
        }

        // Check if we should skip sequence matching
        let whole_run = query.whole_query_run();
        let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
        eprintln!(
            "Skip seq matching: {} (matched_qspans: {})",
            skip_seq_matching,
            matched_qspans.len()
        );

        // Phase 2-4: Seq matching
        if !skip_seq_matching {
            use crate::license_detection::seq_match::{
                compute_candidates_with_msets, seq_match_with_candidates,
            };
            const MAX_NEAR_DUPE_CANDIDATES: usize = 50;
            const MAX_SEQ_CANDIDATES: usize = 70;
            const MAX_QUERY_RUN_CANDIDATES: usize = 70;

            let mut seq_all_matches = Vec::new();

            // Phase 2: Near-duplicate
            {
                let whole_run = query.whole_query_run();
                let near_dupe_candidates = compute_candidates_with_msets(
                    index,
                    &whole_run,
                    true,
                    MAX_NEAR_DUPE_CANDIDATES,
                );
                if !near_dupe_candidates.is_empty() {
                    let near_dupe_matches =
                        seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);
                    eprintln!("Near-dupe matches: {}", near_dupe_matches.len());
                    for m in &near_dupe_matches {
                        if m.end_token > m.start_token {
                            let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                            query.subtract(&span);
                            matched_qspans.push(span);
                        }
                    }
                    seq_all_matches.extend(near_dupe_matches);
                }
            }

            // Phase 3: Regular seq match
            {
                let whole_run = query.whole_query_run();
                let candidates =
                    compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
                if !candidates.is_empty() {
                    let matches = seq_match_with_candidates(index, &whole_run, &candidates);
                    eprintln!("Seq matches: {}", matches.len());
                    seq_all_matches.extend(matches);
                }
            }

            // Phase 4: Query runs
            {
                let whole_run = query.whole_query_run();
                for query_run in query.query_runs().iter() {
                    if query_run.start == whole_run.start && query_run.end == whole_run.end {
                        continue;
                    }
                    if !query_run.is_matchable(false, &matched_qspans) {
                        continue;
                    }
                    let candidates = compute_candidates_with_msets(
                        index,
                        query_run,
                        false,
                        MAX_QUERY_RUN_CANDIDATES,
                    );
                    if !candidates.is_empty() {
                        let matches = seq_match_with_candidates(index, query_run, &candidates);
                        seq_all_matches.extend(matches);
                    }
                }
            }

            let merged_seq = merge_overlapping_matches(&seq_all_matches);
            eprintln!("Merged seq matches: {}", merged_seq.len());
            all_matches.extend(merged_seq);
        }

        eprintln!("Total all_matches before refine: {}", all_matches.len());
        for m in all_matches.iter().take(5) {
            eprintln!(
                "  ALL: {} at lines {}-{} rule={} matcher={} coverage={:.2}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.matcher,
                m.match_coverage
            );
        }

        // Step 1: Initial refine WITHOUT false positive filtering
        let merged_matches =
            refine_matches_without_false_positive_filter(index, all_matches.clone(), &query);
        eprintln!("After refine_without_fp: {}", merged_matches.len());
        if merged_matches.is_empty() && !all_matches.is_empty() {
            eprintln!("WARNING: refine_without_fp filtered all matches!");
            for m in all_matches.iter().take(5) {
                eprintln!(
                    "  LOST: {} at lines {}-{} rule={} matcher={}",
                    m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.matcher
                );
            }

            // Step through refine to find where it's lost
            eprintln!("\n=== Step through refine_matches_internal ===");
            let merged = merge_overlapping_matches(&all_matches);
            eprintln!("After merge_overlapping: {}", merged.len());
            for m in merged.iter() {
                eprintln!(
                    "  {} at lines {}-{} rule={} matcher={} start_token={} end_token={}",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.rule_identifier,
                    m.matcher,
                    m.start_token,
                    m.end_token
                );
            }

            // Check what rule says about this match
            if let Some(m) = merged.first() {
                if let Some(rule) = index.rules_by_rid.get(m.rid) {
                    eprintln!("\n=== Rule details for {} ===", m.rule_identifier);
                    eprintln!("  license_expression: {}", rule.license_expression);
                    eprintln!("  required_phrase_spans: {:?}", rule.required_phrase_spans);
                    eprintln!("  minimum_coverage: {:?}", rule.minimum_coverage);
                    eprintln!("  is_license_text: {}", rule.is_license_text);
                    eprintln!("  is_license_notice: {}", rule.is_license_notice);
                    eprintln!("  is_license_reference: {}", rule.is_license_reference);
                    eprintln!("  is_false_positive: {}", rule.is_false_positive);
                    eprintln!("  is_continuous: {}", rule.is_continuous);
                    eprintln!("  is_required_phrase: {}", rule.is_required_phrase);
                }

                eprintln!("\n=== Match details ===");
                eprintln!("  ispan_positions: {:?}", m.ispan_positions);
                eprintln!("  ispan(): {:?}", m.ispan());
                eprintln!("  qspan_positions: {:?}", m.qspan_positions);
                eprintln!("  qspan(): {:?}", m.qspan());
                eprintln!("  rule_start_token: {}", m.rule_start_token);
                eprintln!("  start_token: {}", m.start_token);
                eprintln!("  end_token: {}", m.end_token);
                eprintln!("  matched_length: {}", m.matched_length);
                eprintln!("  rule_length: {}", m.rule_length);

                // Check required phrase span coverage
                if let Some(rule) = index.rules_by_rid.get(m.rid) {
                    let ispan = m.ispan();
                    let ispan_set: std::collections::HashSet<usize> =
                        ispan.iter().copied().collect();

                    eprintln!("\n=== Required phrase check ===");
                    for span in &rule.required_phrase_spans {
                        eprintln!("  Required span {:?}:", span);
                        let mut all_present = true;
                        for pos in span.start..span.end {
                            let present = ispan_set.contains(&pos);
                            if !present {
                                all_present = false;
                            }
                            eprintln!("    pos {} -> present={}", pos, present);
                        }
                        eprintln!("  All present: {}", all_present);
                    }

                    // Check stopwords
                    let qspan = m.qspan();
                    eprintln!("\n  rule.stopwords_by_pos: {:?}", rule.stopwords_by_pos);
                    eprintln!(
                        "  query.stopwords_by_pos (sample): {:?}",
                        query.stopwords_by_pos.iter().take(10).collect::<Vec<_>>()
                    );

                    // Check the actual text at the required phrase positions
                    eprintln!("\n=== Check tokens at required phrase positions ===");
                    let query_text_tokens =
                        crate::license_detection::tokenize::tokenize_without_stopwords(clean_text);
                    eprintln!(
                        "  Query text tokens (first 15): {:?}",
                        &query_text_tokens[..query_text_tokens.len().min(15)]
                    );

                    // What are the stopwords at rule positions 8 and 9?
                    eprintln!("  Rule text: {:?}", &rule.text[..200.min(rule.text.len())]);

                    // Check what query positions map to rule positions 8 and 9
                    // The match has qspan and ispan - we need to find the mapping
                    let qspan = m.qspan();
                    let ispan = m.ispan();
                    eprintln!("\n=== Stopword alignment check ===");
                    eprintln!("  Looking for rule positions 8 and 9 in ispan...");
                    for (i, &ipos) in ispan.iter().enumerate() {
                        if ipos == 8 || ipos == 9 {
                            let qpos = qspan.get(i).unwrap_or(&0);
                            eprintln!("    Rule pos {} -> Query pos {}", ipos, qpos);
                            let i_stop = rule.stopwords_by_pos.get(&ipos).copied().unwrap_or(0);
                            let q_stop = query
                                .stopwords_by_pos
                                .get(&Some(*qpos as i32))
                                .copied()
                                .unwrap_or(0);
                            eprintln!("      i_stop={} q_stop={}", i_stop, q_stop);
                        }
                    }
                }
            }
        }

        // Step 2: Split weak from good
        let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
        eprintln!("Good: {}, Weak: {}", good_matches.len(), weak_matches.len());

        // Step 3: Add weak back
        let mut all_matches = good_matches;
        all_matches.extend(weak_matches);

        // Step 5: Final refine WITH false positive filtering
        let refined = refine_matches(index, all_matches, &query);
        eprintln!("Final refined: {}", refined.len());

        for m in &refined {
            eprintln!(
                "  {} at lines {}-{} rule={} matcher={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.matcher
            );
        }
    }

    #[test]
    fn debug_apache_header_t2() {
        let Some(engine) = get_engine() else { return };
        let path =
            PathBuf::from("testdata/license-golden/datadriven/external/glc/Apache-2.0-Header.t2");
        let text = std::fs::read_to_string(&path).expect("Failed to read file");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("=== Apache-2.0-Header.t2 Analysis ===");
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("Detections: {}", detections.len());

        for d in &detections {
            eprintln!("Detection: {:?}", d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "  {} at lines {}-{} matcher={} score={:.1}",
                    m.license_expression, m.start_line, m.end_line, m.matcher, m.score
                );
            }
        }

        let has_apache = detections.iter().any(|d| {
            d.matches
                .iter()
                .any(|m| m.license_expression.contains("apache-2.0"))
        });
        eprintln!("Has apache-2.0: {}", has_apache);
    }

    #[test]
    fn debug_apache_header_pipeline() {
        let Some(_engine) = get_engine() else { return };
        let path =
            PathBuf::from("testdata/license-golden/datadriven/external/glc/Apache-2.0-Header.t2");
        let text = std::fs::read_to_string(&path).expect("Failed to read file");

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::refine_matches;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::seq_match;
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

        eprintln!("=== Tokenization ===");
        eprintln!("Total tokens: {}", query.tokens.len());
        eprintln!(
            "Sample tokens: {:?}",
            &query.tokens[..query.tokens.len().min(30)]
        );

        eprintln!("\n=== Hash Match ===");
        let hash_matches = hash_match(&index, &whole_run);
        eprintln!("Hash matches: {}", hash_matches.len());
        for m in &hash_matches {
            eprintln!(
                "  {} at lines {}-{} rule={} match_coverage={:.2}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.match_coverage
            );
        }

        eprintln!("\n=== SPDX LID Match ===");
        let spdx_matches = spdx_lid_match(&index, &query);
        eprintln!("SPDX matches: {}", spdx_matches.len());
        for m in &spdx_matches {
            eprintln!(
                "  {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        eprintln!("\n=== Aho Match ===");
        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("Aho matches: {}", aho_matches.len());
        for m in &aho_matches {
            eprintln!(
                "  {} at lines {}-{} rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        // Check for warranty-disclaimer specifically
        let warranty_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression.contains("warranty"))
            .collect();
        eprintln!("\nWarranty-related matches: {}", warranty_matches.len());
        for m in &warranty_matches {
            eprintln!(
                "  WARRANTY: {} at lines {}-{} rule={} coverage={:.2}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.match_coverage
            );
        }

        eprintln!("\n=== Seq Match ===");
        let seq_matches = seq_match(&index, &whole_run);
        eprintln!("Seq matches: {}", seq_matches.len());
        for m in &seq_matches {
            eprintln!(
                "  {} at lines {}-{} rule={} score={:.2}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.score
            );
        }

        let mut all_matches = Vec::new();
        all_matches.extend(hash_matches.clone());
        all_matches.extend(spdx_matches.clone());
        all_matches.extend(aho_matches.clone());
        all_matches.extend(seq_matches.clone());

        eprintln!("\n=== All Initial Matches: {} ===", all_matches.len());

        // Check warranty matches before refine
        let warranty_initial: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression.contains("warranty"))
            .collect();
        eprintln!("Warranty matches before refine: {}", warranty_initial.len());
        for m in &warranty_initial {
            eprintln!(
                "  WARRANTY: {} at lines {}-{} rule={} start_token={} end_token={} matched_len={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        // Check apache matches before refine
        let apache_initial: Vec<_> = all_matches
            .iter()
            .filter(|m| {
                m.license_expression.contains("apache-2.0") && m.rule_identifier.contains("_163")
            })
            .collect();
        eprintln!("Apache-2.0_163 matches: {}", apache_initial.len());
        for m in &apache_initial {
            eprintln!(
                "  APACHE: {} at lines {}-{} rule={} start_token={} end_token={} matched_len={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        let refined = refine_matches(&index, all_matches.clone(), &query);
        eprintln!("\n=== Refined Matches: {} ===", refined.len());
        for m in &refined {
            eprintln!(
                "  {} at lines {}-{} rule={} score={:.2} matched_len={} hilen={} matcher={} len()={} coverage={:.2}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.score,
                m.matched_length,
                m.hilen(),
                m.matcher,
                m.len(),
                m.match_coverage
            );
        }

        eprintln!("\n=== split_weak_matches test ===");
        use crate::license_detection::match_refine::split_weak_matches;
        let (good, weak) = split_weak_matches(&refined);
        eprintln!("Good matches: {}", good.len());
        eprintln!("Weak matches: {}", weak.len());
        for m in &weak {
            eprintln!(
                "  WEAK: {} at lines {}-{} rule={} matcher={} len()={} coverage={:.2}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.matcher,
                m.len(),
                m.match_coverage
            );
        }

        eprintln!("\n=== Detection Grouping ===");
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, post_process_detections, sort_matches_by_line,
        };
        use crate::license_detection::spdx_mapping::build_spdx_mapping;

        let spdx_mapping =
            build_spdx_mapping(&index.licenses_by_key.values().cloned().collect::<Vec<_>>());

        let mut sorted = good.clone();
        sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);
        eprintln!("Groups: {}", groups.len());
        for (i, group) in groups.iter().enumerate() {
            eprintln!(
                "Group {} (lines {}-{}):",
                i, group.start_line, group.end_line
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
            eprintln!("Detection {}: {:?}", i, d.license_expression);
        }

        let processed = post_process_detections(detections, 0.0);
        eprintln!(
            "\n=== Detections after post_process: {} ===",
            processed.len()
        );
        for (i, d) in processed.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i, d.license_expression);
        }

        eprintln!("\n=== Compare with engine.detect() ===");
        let engine = get_engine().unwrap();
        let engine_detections = engine.detect(&text, false).unwrap();
        eprintln!("Engine detections: {}", engine_detections.len());
        for d in &engine_detections {
            eprintln!("  {:?}", d.license_expression);
        }

        let apache_rules_in_index: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("apache-2.0"))
            .collect();
        eprintln!(
            "\n=== Apache-2.0 Rules in Index: {} ===",
            apache_rules_in_index.len()
        );
        for r in apache_rules_in_index.iter().take(10) {
            eprintln!(
                "  {} length_unique={} min_ml={} min_hml={}",
                r.identifier, r.length_unique, r.min_matched_length, r.min_high_matched_length
            );
        }

        let warranty_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("warranty-disclaimer"))
            .collect();
        eprintln!(
            "\n=== Warranty Rules in Index: {} ===",
            warranty_rules.len()
        );
        for r in warranty_rules.iter().take(5) {
            eprintln!(
                "  {} length_unique={} min_ml={}",
                r.identifier, r.length_unique, r.min_matched_length
            );
        }
    }

    #[test]
    fn debug_catosl_sep() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from("testdata/license-golden/datadriven/external/atarashi/CATOSL.sep");
        let text = std::fs::read_to_string(&path).expect("Failed to read file");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("=== CATOSL.sep Analysis ===");
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("First 200 chars: {:?}", &text[..text.len().min(200)]);
        eprintln!("Detections: {}", detections.len());

        for d in &detections {
            eprintln!("Detection: {:?}", d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "  {} at lines {}-{} matcher={}",
                    m.license_expression, m.start_line, m.end_line, m.matcher
                );
            }
        }
    }

    #[test]
    fn debug_gpl_rem_comment_xml() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from("testdata/license-golden/datadriven/external/licensecheck/devscripts/gpl-3+-with-rem-comment.xml");
        let text = std::fs::read_to_string(&path).expect("Failed to read file");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("=== gpl-3+-with-rem-comment.xml Analysis ===");
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("First 500 chars: {:?}", &text[..text.len().min(500)]);
        eprintln!("Detections: {}", detections.len());

        for d in &detections {
            eprintln!("Detection: {:?}", d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "  {} at lines {}-{} matcher={}",
                    m.license_expression, m.start_line, m.end_line, m.matcher
                );
            }
        }

        // Check if 'rem' is being tokenized as a stopword
        use crate::license_detection::tokenize::tokenize_without_stopwords;
        let tokens = tokenize_without_stopwords(&text);
        eprintln!(
            "Sample tokens (first 50): {:?}",
            &tokens[..tokens.len().min(50)]
        );

        // Check if REM lines are being captured
        let rem_lines: Vec<_> = text.lines().filter(|l| l.contains("REM")).collect();
        eprintln!("Lines with REM: {}", rem_lines.len());
        for line in rem_lines.iter().take(3) {
            eprintln!("  REM line: {:?}", line);
        }
    }

    #[test]
    fn debug_ruby_t2() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from("testdata/license-golden/datadriven/external/glc/Ruby.t2");
        let text = std::fs::read_to_string(&path).expect("Failed to read file");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("=== Ruby.t2 Analysis ===");
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("Detections: {}", detections.len());

        for d in &detections {
            eprintln!("Detection: {:?}", d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "  {} at lines {}-{} matcher={} rule={}",
                    m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
                );
            }
        }

        // Expected: single detection with "gpl-2.0 OR other-copyleft"
        // Check if we have both licenses
        let has_gpl = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map_or(false, |e| e.contains("gpl-2.0"))
        });
        let has_other = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map_or(false, |e| e.contains("other-copyleft"))
        });
        eprintln!(
            "Has gpl-2.0: {}, Has other-copyleft: {}",
            has_gpl, has_other
        );
    }

    #[test]
    fn debug_nasa_1_3_t1() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from("testdata/license-golden/datadriven/external/glc/NASA-1.3.t1");
        let text = std::fs::read_to_string(&path).expect("Failed to read file");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("=== NASA-1.3.t1 Analysis ===");
        eprintln!("Text lines: {}", text.lines().count());
        eprintln!("Detections: {}", detections.len());

        let mut nasa_count = 0;
        for d in &detections {
            eprintln!("Detection: {:?}", d.license_expression);
            for m in &d.matches {
                if m.license_expression.contains("nasa-1.3") {
                    nasa_count += 1;
                }
                eprintln!(
                    "  {} at lines {}-{}",
                    m.license_expression, m.start_line, m.end_line
                );
            }
        }
        eprintln!("Total nasa-1.3 matches: {} (expected 3)", nasa_count);
    }
}
