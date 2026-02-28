//! Investigation test for PLAN-010: x11_danse.txt
//!
//! ## Issue
//! Extra `unknown-license-reference` and wrong ordering.
//!
//! **Expected:** `["x11 AND other-permissive"]`
//! **Actual:** `["unknown-license-reference", "other-permissive", "x11 AND other-permissive"]`
//!
//! ## Investigation Steps
//! 1. Run file through Python reference to get matches at each pipeline stage
//! 2. Compare Rust vs Python at: aho matching, seq matching, filtering, post-processing
//! 3. Find the exact point of divergence

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::license_detection::LicenseDetectionEngine;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    fn read_test_file() -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic4/x11_danse.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_x11_danse_full_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        println!("\n========================================");
        println!("X11_DANSE Full Detection");
        println!("========================================");
        println!("Text length: {} bytes", text.len());

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        println!("\nTotal detections: {}", detections.len());
        for (i, d) in detections.iter().enumerate() {
            println!("Detection {}: {:?}", i + 1, d.license_expression);
            for m in &d.matches {
                println!(
                    "  Match: {} | lines {}-{} | matcher={} | rule={} | coverage={:.1}%",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.matcher,
                    m.rule_identifier,
                    m.match_coverage
                );
            }
        }

        let expressions: Vec<_> = detections
            .iter()
            .filter_map(|d| d.license_expression.as_ref())
            .cloned()
            .collect();

        println!("\nExpressions: {:?}", expressions);

        // Expected: ["x11 AND other-permissive"]
        // Actual: ["unknown-license-reference AND other-permissive AND x11"]
        // We need to understand where the extra matches come from
    }

    #[test]
    fn test_x11_danse_pipeline_trace() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, post_process_detections, sort_matches_by_line,
        };
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            merge_overlapping_matches, refine_matches,
            refine_matches_without_false_positive_filter, split_weak_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match_with_candidates,
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

        println!("\n========================================");
        println!("Pipeline Trace");
        println!("========================================");
        println!("Query tokens: {}", query.tokens.len());

        let whole_run = query.whole_query_run();

        // Phase 1a: Hash matching
        println!("\n--- Phase 1a: Hash matching ---");
        let hash_matches = hash_match(&index, &whole_run);
        println!("Hash matches: {}", hash_matches.len());

        // Phase 1b: SPDX-LID matching
        println!("\n--- Phase 1b: SPDX-LID matching ---");
        let spdx_matches = spdx_lid_match(&index, &query);
        println!("SPDX-LID matches: {}", spdx_matches.len());

        // Phase 1c: Aho-Corasick matching
        println!("\n--- Phase 1c: Aho-Corasick matching ---");
        let aho_matches = aho_match(&index, &whole_run);
        println!("Aho matches (raw): {}", aho_matches.len());
        for m in aho_matches.iter().take(10) {
            println!(
                "  {} | lines {}-{} | rule={} | coverage={:.1}%",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.match_coverage
            );
        }

        let merged_aho = merge_overlapping_matches(&aho_matches);
        println!("Aho matches (merged): {}", merged_aho.len());

        // Collect all Phase 1 matches
        let mut all_matches = Vec::new();
        all_matches.extend(spdx_matches.clone());
        all_matches.extend(merged_aho.clone());

        // Phase 2-4: Sequence matching
        println!("\n--- Phase 2-4: Sequence matching ---");
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        println!("Near-dupe candidates: {}", near_dupe_candidates.len());

        let near_dupe_matches =
            seq_match_with_candidates(&index, &whole_run, &near_dupe_candidates);
        println!("Near-dupe matches: {}", near_dupe_matches.len());

        // Regular seq matching
        const MAX_SEQ_CANDIDATES: usize = 70;
        let seq_candidates =
            compute_candidates_with_msets(&index, &whole_run, false, MAX_SEQ_CANDIDATES);
        println!("Seq candidates: {}", seq_candidates.len());

        let seq_matches = seq_match_with_candidates(&index, &whole_run, &seq_candidates);
        println!("Seq matches: {}", seq_matches.len());

        // Merge all seq matches
        let mut seq_all_matches = Vec::new();
        seq_all_matches.extend(near_dupe_matches);
        seq_all_matches.extend(seq_matches);
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        println!("Seq matches (merged): {}", merged_seq.len());

        all_matches.extend(merged_seq);

        println!("\n--- All matches before refine: {} ---", all_matches.len());

        // Refine step by step
        println!("\n--- Step 1: refine_matches_without_false_positive_filter ---");
        let merged_matches =
            refine_matches_without_false_positive_filter(&index, all_matches.clone(), &query);
        println!("Merged matches: {}", merged_matches.len());

        println!("\n--- Step 2: split_weak_matches ---");
        let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
        println!("Good matches: {}", good_matches.len());
        println!("Weak matches: {}", weak_matches.len());
        for m in good_matches.iter() {
            println!(
                "  GOOD: {} | lines {}-{} | rule={} | coverage={:.1}%",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.match_coverage
            );
        }

        let mut all_matches = good_matches;
        all_matches.extend(weak_matches);

        println!("\n--- Step 5: final refine_matches ---");
        let refined = refine_matches(&index, all_matches, &query);
        println!("Refined matches: {}", refined.len());
        for m in refined.iter() {
            println!(
                "  {} | lines {}-{} | rule={} | matcher={} | coverage={:.1}%",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.matcher,
                m.match_coverage
            );
        }

        // Group and create detections
        println!("\n--- Grouping and Detection ---");
        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);
        println!("Groups: {}", groups.len());

        let detections: Vec<_> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(&mut detection, group, &spdx_mapping);
                detection
            })
            .collect();

        println!("\nDetections before post_process: {}", detections.len());
        for (i, d) in detections.iter().enumerate() {
            println!("Detection {}: {:?}", i + 1, d.license_expression);
        }

        let processed = post_process_detections(detections, 0.0);
        println!("\nDetections after post_process: {}", processed.len());
        for (i, d) in processed.iter().enumerate() {
            println!("Detection {}: {:?}", i + 1, d.license_expression);
        }
    }

    /// Test to check if seq matching is being skipped due to aho matches.
    #[test]
    fn test_x11_danse_seq_matching_skip_check() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::merge_overlapping_matches;
        use crate::license_detection::query::{self, Query};
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

        println!("\n========================================");
        println!("Seq Matching Skip Check");
        println!("========================================");

        let whole_run = query.whole_query_run();

        // Simulate Phase 1b and 1c (SPDX and Aho matching)
        let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

        // Phase 1b: SPDX-LID
        let spdx_matches = spdx_lid_match(&index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        for m in &merged_spdx {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        println!("\nAfter SPDX-LID: {} matched_qspans", matched_qspans.len());

        // Phase 1c: Aho
        let aho_matches = aho_match(&index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        println!("Aho matches: {}", merged_aho.len());
        for m in &merged_aho {
            println!(
                "  {} | lines {}-{} | tokens {}-{} | coverage={:.1}% | is_license_text={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage,
                m.is_license_text
            );
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        println!("\nAfter Aho: {} matched_qspans", matched_qspans.len());
        for span in &matched_qspans {
            let positions = span.positions();
            let min_pos = positions.iter().min().copied().unwrap_or(0);
            let max_pos = positions.iter().max().copied().unwrap_or(0);
            println!("  Span: {}-{}", min_pos, max_pos);
        }

        // Check is_matchable BEFORE seq matching
        let whole_run = query.whole_query_run();
        let is_matchable = whole_run.is_matchable(false, &matched_qspans);
        println!(
            "\nis_matchable(include_low=False, matched_qspans): {}",
            is_matchable
        );
        println!("skip_seq_matching would be: {}", !is_matchable);

        // Check high matchables
        let high_matchables = whole_run.high_matchables();
        println!("\nhigh_matchables count: {}", high_matchables.len());

        // Check if the matched_qspans cover all high_matchables
        let mut covered_positions: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        for span in &matched_qspans {
            covered_positions.extend(span.positions());
        }
        let uncovered_high: std::collections::HashSet<_> =
            high_matchables.difference(&covered_positions).collect();
        println!("Uncovered high matchables: {}", uncovered_high.len());

        if !uncovered_high.is_empty() {
            println!(
                "Uncovered positions: {:?}",
                uncovered_high.iter().take(20).collect::<Vec<_>>()
            );
        }
    }

    /// Test that compares the full detection with detection log analysis.
    /// This investigates why we get fragmented results.
    #[test]
    fn test_x11_danse_detection_log_analysis() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        println!("\n========================================");
        println!("Detection Log Analysis");
        println!("========================================");

        for (i, d) in detections.iter().enumerate() {
            println!("Detection {}", i + 1);
            println!("  Expression: {:?}", d.license_expression);
            println!("  Detection Log: {:?}", d.detection_log);

            for m in &d.matches {
                println!(
                    "  Match: {} | lines {}-{} | matcher={} | rule={} | coverage={:.1}% | is_license_text={}",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.matcher,
                    m.rule_identifier,
                    m.match_coverage,
                    m.is_license_text
                );
            }
        }
    }

    /// Test that traces the ACTUAL detect() function behavior step by step.
    /// This is a more detailed trace that mirrors the real detect() function.
    #[test]
    fn test_x11_danse_actual_detect_trace() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, post_process_detections, sort_matches_by_line,
        };
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            merge_overlapping_matches, refine_matches,
            refine_matches_without_false_positive_filter, split_weak_matches,
        };
        use crate::license_detection::query::{self, Query};
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match_with_candidates,
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
        let mut query = Query::new(clean_text, &index).expect("Query creation failed");

        let mut all_matches = Vec::new();
        let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

        println!("\n========================================");
        println!("ACTUAL DETECT TRACE (mirrors detect())");
        println!("========================================");

        // Phase 1a: Hash matching
        let whole_run = query.whole_query_run();
        let hash_matches = hash_match(&index, &whole_run);
        if !hash_matches.is_empty() {
            println!("Hash matches found - would return early");
            return;
        }

        // Phase 1b: SPDX-LID matching
        let spdx_matches = spdx_lid_match(&index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        for m in &merged_spdx {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
            }
            // Check for is_license_text subtraction
            if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
                println!(
                    "SPDX subtraction for is_license_text match: tokens {}-{}",
                    m.start_token, m.end_token
                );
                query.subtract(&span);
            }
        }
        all_matches.extend(merged_spdx);

        // Phase 1c: Aho-Corasick matching
        let whole_run = query.whole_query_run();
        let aho_matches = aho_match(&index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        println!("\n=== Aho matches after merge: {} ===", merged_aho.len());
        for m in &merged_aho {
            println!(
                "  {} | tokens {}-{} | is_license_text={} | rule_len={}",
                m.license_expression, m.start_token, m.end_token, m.is_license_text, m.rule_length
            );
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
            }
            // Check for is_license_text subtraction
            if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
                println!(
                    "AHO subtraction for is_license_text match: tokens {}-{}",
                    m.start_token, m.end_token
                );
                query.subtract(&span);
            }
        }
        all_matches.extend(merged_aho);

        // Check skip_seq_matching
        let whole_run = query.whole_query_run();
        let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
        println!("\nskip_seq_matching: {}", skip_seq_matching);
        println!("matched_qspans count: {}", matched_qspans.len());

        let mut seq_all_matches = Vec::new();
        if !skip_seq_matching {
            // Phase 2: Near-duplicate detection
            let whole_run = query.whole_query_run();
            let near_dupe_candidates =
                compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
            println!("\nNear-dupe candidates: {}", near_dupe_candidates.len());

            if !near_dupe_candidates.is_empty() {
                let near_dupe_matches =
                    seq_match_with_candidates(&index, &whole_run, &near_dupe_candidates);
                println!("Near-dupe matches: {}", near_dupe_matches.len());

                for m in &near_dupe_matches {
                    if m.end_token > m.start_token {
                        let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                        query.subtract(&span);
                        matched_qspans.push(span);
                    }
                }
                seq_all_matches.extend(near_dupe_matches);
            }

            // Phase 3: Regular sequence matching
            const MAX_SEQ_CANDIDATES: usize = 70;
            let whole_run = query.whole_query_run();
            let seq_candidates =
                compute_candidates_with_msets(&index, &whole_run, false, MAX_SEQ_CANDIDATES);
            println!("Seq candidates: {}", seq_candidates.len());

            if !seq_candidates.is_empty() {
                let seq_matches = seq_match_with_candidates(&index, &whole_run, &seq_candidates);
                println!("Seq matches: {}", seq_matches.len());
                seq_all_matches.extend(seq_matches);
            }

            let merged_seq = merge_overlapping_matches(&seq_all_matches);
            println!("Seq matches (merged): {}", merged_seq.len());
            all_matches.extend(merged_seq);
        }

        println!("\n=== All matches before refine: {} ===", all_matches.len());

        // Refine without false positive filter
        let merged_matches =
            refine_matches_without_false_positive_filter(&index, all_matches.clone(), &query);
        println!(
            "\nAfter refine_matches_without_false_positive_filter: {}",
            merged_matches.len()
        );
        for m in merged_matches.iter().take(10) {
            println!(
                "  {} | lines {}-{} | rule={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        // Split weak matches
        let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
        println!("\nGood matches: {}", good_matches.len());
        println!("Weak matches: {}", weak_matches.len());

        let mut all_matches = good_matches;
        all_matches.extend(weak_matches);

        // Final refine
        let refined = refine_matches(&index, all_matches, &query);
        println!("\nRefined matches: {}", refined.len());
        for m in refined.iter() {
            println!(
                "  {} | lines {}-{} | rule={} | matcher={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.matcher
            );
        }

        // Group and create detections
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

        let processed = post_process_detections(detections, 0.0);
        println!("\n=== FINAL DETECTIONS ===");
        for d in &processed {
            println!("Expression: {:?}", d.license_expression);
            println!("Detection log: {:?}", d.detection_log);
        }
    }

    /// Test to compare what happens with forced seq matching.
    #[test]
    fn test_x11_danse_with_forced_seq_matching() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, post_process_detections, sort_matches_by_line,
        };
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::refine_matches;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match_with_candidates,
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

        println!("\n========================================");
        println!("WITH FORCED SEQ MATCHING");
        println!("========================================");

        let whole_run = query.whole_query_run();

        // Phase 1: Exact matchers
        let mut all_matches = Vec::new();
        all_matches.extend(spdx_lid_match(&index, &query));
        all_matches.extend(aho_match(&index, &whole_run));

        // Phase 2-4: Seq matching (FORCED - ignore skip logic)
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let seq_matches = seq_match_with_candidates(&index, &whole_run, &near_dupe_candidates);
        all_matches.extend(seq_matches);

        println!("Total matches: {}", all_matches.len());

        // Refine
        let refined = refine_matches(&index, all_matches, &query);
        println!("Refined matches: {}", refined.len());
        for m in refined.iter() {
            println!(
                "  {} | lines {}-{} | rule={} | matcher={}",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier, m.matcher
            );
        }

        // Group and create detections
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

        let processed = post_process_detections(detections, 0.0);
        println!("\nFinal detections:");
        for d in &processed {
            println!("  {:?}", d.license_expression);
        }
    }

    /// FAILING TEST: Documents expected behavior once the fix is implemented.
    ///
    /// Root cause identified: The `is_license_text` subtraction in detect()
    /// removes query tokens BEFORE seq matching runs, preventing the correct
    /// `x11 AND other-permissive` rule from being found.
    ///
    /// Expected: `["x11 AND other-permissive"]`
    /// Actual: `["unknown-license-reference AND other-permissive AND x11"]`
    #[test]
    fn test_x11_danse_expected_expression() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let expressions: Vec<_> = detections
            .iter()
            .filter_map(|d| d.license_expression.as_ref())
            .cloned()
            .collect();

        // This test will FAIL until the is_license_text subtraction bug is fixed
        assert_eq!(
            expressions,
            vec!["x11 AND other-permissive"],
            "Expected 'x11 AND other-permissive' but got {:?}",
            expressions
        );
    }
}
