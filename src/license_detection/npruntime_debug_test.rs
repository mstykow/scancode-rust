//! Deep investigation for npruntime.h bsd-new detection failure.

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    fn read_npruntime() -> Option<String> {
        let path =
            PathBuf::from("testdata/license-golden/datadriven/external/slic-tests/npruntime.h");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_npruntime_full_pipeline_debug() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_npruntime() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, post_process_detections, sort_matches_by_line,
        };
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
            refine_matches, restore_non_overlapping, split_weak_matches,
        };
        use crate::license_detection::query::{PositionSpan, Query};
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, seq_match_with_candidates, MAX_NEAR_DUPE_CANDIDATES,
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
        let mut matched_qspans: Vec<PositionSpan> = Vec::new();

        // Phase 1a: Hash
        let whole_run = query.whole_query_run();
        let hash_matches = hash_match(&index, &whole_run);
        if !hash_matches.is_empty() {
            eprintln!("[Phase 1a] Hash matches: {}", hash_matches.len());
            return;
        }

        // Phase 1b: SPDX-LID
        let spdx_matches = spdx_lid_match(&index, &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        for m in &merged_spdx {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        all_matches.extend(merged_spdx);

        // Phase 1c: Aho-Corasick
        let whole_run = query.whole_query_run();
        let aho_matches = aho_match(&index, &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        let (non_contained_aho, discarded_contained) = filter_contained_matches(&merged_aho);
        let (filtered_aho, discarded_overlapping) =
            filter_overlapping_matches(non_contained_aho, &index);

        let (restored_contained, _) = restore_non_overlapping(&filtered_aho, discarded_contained);
        let (restored_overlapping, _) =
            restore_non_overlapping(&filtered_aho, discarded_overlapping);

        let mut final_aho = filtered_aho;
        final_aho.extend(restored_contained);
        final_aho.extend(restored_overlapping);

        eprintln!("\n=== Phase 1c: Aho-Corasick ===");
        eprintln!("Final aho matches: {}", final_aho.len());
        for m in &final_aho {
            eprintln!(
                "  {} at lines {}-{} coverage={:.1}% rule_len={} is_license_text={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.match_coverage,
                m.rule_length,
                m.is_license_text
            );
        }

        for m in &final_aho {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
            if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                let span = PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
                query.subtract(&span);
            }
        }
        all_matches.extend(final_aho.clone());

        // Check skip condition
        let whole_run = query.whole_query_run();
        let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
        eprintln!("\nSkip sequence matching: {}", skip_seq_matching);

        let mut seq_all_matches = Vec::new();
        if !skip_seq_matching {
            // Phase 2: Near-duplicate
            let whole_run = query.whole_query_run();
            let near_dupe_candidates =
                compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
            eprintln!("\n=== Phase 2: Near-Duplicate ===");
            eprintln!("Candidates: {}", near_dupe_candidates.len());

            if !near_dupe_candidates.is_empty() {
                let near_dupe_matches =
                    seq_match_with_candidates(&index, &whole_run, &near_dupe_candidates);
                eprintln!("Matches: {}", near_dupe_matches.len());
                for m in &near_dupe_matches {
                    if m.end_token > m.start_token {
                        let span = PositionSpan::new(m.start_token, m.end_token - 1);
                        query.subtract(&span);
                        matched_qspans.push(span);
                    }
                }
                seq_all_matches.extend(near_dupe_matches);
            }

            // Phase 3: Regular sequence
            const MAX_SEQ_CANDIDATES: usize = 70;
            let whole_run = query.whole_query_run();
            let candidates =
                compute_candidates_with_msets(&index, &whole_run, false, MAX_SEQ_CANDIDATES);
            eprintln!("\n=== Phase 3: Regular Sequence ===");
            eprintln!("Candidates: {}", candidates.len());

            // Check if bsd-new_22.RULE is a candidate
            for c in &candidates {
                if let Some(rule) = index.rules_by_rid.get(c.rid) {
                    if rule.identifier == "bsd-new_22.RULE" {
                        eprintln!("FOUND bsd-new_22.RULE in candidates: rid={} containment={:.3} resemblance={:.3}",
                            c.rid, c.score_vec_full.containment, c.score_vec_full.resemblance);
                    }
                }
            }

            // Show the last candidate in the list
            if let Some(last_c) = candidates.last() {
                if let Some(rule) = index.rules_by_rid.get(last_c.rid) {
                    eprintln!(
                        "\nLast candidate (rank {}): {} containment={:.3} resemblance={:.3}",
                        candidates.len(),
                        rule.identifier,
                        last_c.score_vec_full.containment,
                        last_c.score_vec_full.resemblance
                    );
                }
            }

            // Count duplicates by license_expression
            let mut expr_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for c in &candidates {
                *expr_counts
                    .entry(c.rule.license_expression.clone())
                    .or_insert(0) += 1;
            }
            let mut sorted_counts: Vec<_> = expr_counts.into_iter().collect();
            sorted_counts.sort_by(|a, b| b.1.cmp(&a.1));
            eprintln!("\nTop license expressions by count:");
            for (expr, count) in sorted_counts.iter().take(10) {
                eprintln!("  {}: {} candidates", expr, count);
            }

            // Find why bsd-new_22.RULE isn't a candidate
            let bsd_new_22_rid = index
                .rules_by_rid
                .iter()
                .position(|r| r.identifier == "bsd-new_22.RULE");
            if let Some(rid) = bsd_new_22_rid {
                eprintln!("\n=== bsd-new_22.RULE analysis (rid={}) ===", rid);
                let is_approx_matchable = index.approx_matchable_rids.contains(&rid);
                eprintln!("approx_matchable: {}", is_approx_matchable);

                if let Some(rule_set) = index.sets_by_rid.get(&rid) {
                    let query_token_ids: Vec<u16> = whole_run
                        .matchable_tokens()
                        .iter()
                        .filter_map(|&tid| if tid >= 0 { Some(tid as u16) } else { None })
                        .collect();
                    let query_set: HashSet<u16> = query_token_ids.iter().copied().collect();
                    let intersection: HashSet<u16> =
                        query_set.intersection(rule_set).copied().collect();
                    eprintln!(
                        "query_set size: {}, rule_set size: {}, intersection size: {}",
                        query_set.len(),
                        rule_set.len(),
                        intersection.len()
                    );

                    let high_intersection = intersection
                        .iter()
                        .filter(|&&tid| (tid as usize) < index.len_legalese)
                        .copied()
                        .collect::<HashSet<u16>>();
                    eprintln!("high_intersection size: {}", high_intersection.len());

                    let rule = &index.rules_by_rid[rid];
                    eprintln!(
                        "min_matched_length_unique: {}, min_high_matched_length_unique: {}",
                        rule.min_matched_length_unique, rule.min_high_matched_length_unique
                    );

                    let matched_len = intersection.len();
                    let high_matched_len = high_intersection.len();
                    eprintln!(
                        "matched_length: {}, high_matched_length: {}",
                        matched_len, high_matched_len
                    );
                    eprintln!(
                        "passes min_matched_length: {}",
                        matched_len >= rule.min_matched_length_unique
                    );
                    eprintln!(
                        "passes min_high_matched_length: {}",
                        high_matched_len >= rule.min_high_matched_length_unique
                    );

                    // Compute actual scores
                    let query_mset: std::collections::HashMap<u16, usize> = query_token_ids
                        .iter()
                        .copied()
                        .fold(std::collections::HashMap::new(), |mut acc, tid| {
                            *acc.entry(tid).or_insert(0) += 1;
                            acc
                        });
                    let qset_len: usize = query_mset.values().sum();
                    if let Some(rule_mset) = index.msets_by_rid.get(&rid) {
                        let iset_len: usize = rule_mset.values().sum();
                        let union_len = qset_len + iset_len - matched_len;
                        let resemblance = matched_len as f32 / union_len as f32;
                        let containment = matched_len as f32 / iset_len as f32;
                        let amplified_resemblance = resemblance.powi(2);
                        eprintln!(
                            "resemblance={:.4} containment={:.4} amplified_resemblance={:.4}",
                            resemblance, containment, amplified_resemblance
                        );
                        eprintln!("qset_len={} iset_len={}", qset_len, iset_len);
                    }
                }
            }

            if !candidates.is_empty() {
                let matches = seq_match_with_candidates(&index, &whole_run, &candidates);
                eprintln!("Raw seq matches: {}", matches.len());
                seq_all_matches.extend(matches);
            }

            let merged_seq = merge_overlapping_matches(&seq_all_matches);
            eprintln!("After merge: {}", merged_seq.len());
            all_matches.extend(merged_seq);
        }

        eprintln!("\n=== Before refine_matches ===");
        eprintln!("Total all_matches: {}", all_matches.len());
        let bsd_new_22_matches: Vec<_> = all_matches
            .iter()
            .filter(|m| m.rule_identifier == "bsd-new_22.RULE")
            .collect();
        eprintln!("bsd-new_22.RULE matches: {}", bsd_new_22_matches.len());
        for m in bsd_new_22_matches.iter().take(5) {
            eprintln!(
                "  lines {}-{} score={:.1} coverage={:.1}%",
                m.start_line, m.end_line, m.score, m.match_coverage
            );
        }
        let bsd_matches_before: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression.contains("bsd"))
            .collect();
        eprintln!("BSD matches: {}", bsd_matches_before.len());
        for m in bsd_matches_before.iter().take(20) {
            eprintln!(
                "  {} at lines {}-{} matcher={} score={:.1} coverage={:.1}% rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.matcher,
                m.score,
                m.match_coverage,
                m.rule_identifier
            );
        }

        // Refine without false positive filter
        let merged_matches = refine_matches(&index, all_matches.clone(), &query);

        eprintln!("\n=== After refine_matches ===");
        eprintln!("Total merged_matches: {}", merged_matches.len());
        let bsd_matches_after: Vec<_> = merged_matches
            .iter()
            .filter(|m| m.license_expression.contains("bsd"))
            .collect();
        eprintln!("BSD matches: {}", bsd_matches_after.len());
        for m in bsd_matches_after.iter().take(20) {
            eprintln!(
                "  {} at lines {}-{} matcher={} score={:.1} coverage={:.1}% rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.matcher,
                m.score,
                m.match_coverage,
                m.rule_identifier
            );
        }

        // Split weak
        let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
        eprintln!("\n=== After split_weak_matches ===");
        eprintln!("Good: {}, Weak: {}", good_matches.len(), weak_matches.len());

        let bsd_good: Vec<_> = good_matches
            .iter()
            .filter(|m| m.license_expression.contains("bsd"))
            .collect();
        eprintln!("BSD in good: {}", bsd_good.len());
        for m in bsd_good.iter().take(20) {
            eprintln!(
                "  {} at lines {}-{} matcher={} score={:.1} coverage={:.1}% rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.matcher,
                m.score,
                m.match_coverage,
                m.rule_identifier
            );
        }

        // Final refine with false positive filter
        let mut all_final = good_matches.clone();
        all_final.extend(weak_matches.clone());
        let refined = refine_matches(&index, all_final, &query);

        eprintln!("\n=== After final refine ===");
        eprintln!("Total: {}", refined.len());

        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);
        eprintln!("\n=== Groups ===");
        eprintln!("Total groups: {}", groups.len());
        for (i, g) in groups.iter().enumerate().take(10) {
            eprintln!(
                "Group {} lines {}-{}: {} matches",
                i + 1,
                g.start_line,
                g.end_line,
                g.matches.len()
            );
            for m in g.matches.iter().take(5) {
                eprintln!(
                    "  {} at lines {}-{} matcher={}",
                    m.license_expression, m.start_line, m.end_line, m.matcher
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

        eprintln!("\n=== Before post_process ===");
        eprintln!("Detections: {}", detections.len());
        for (i, d) in detections.iter().enumerate().take(10) {
            eprintln!("Detection {}: {:?}", i + 1, d.license_expression);
        }

        let processed = post_process_detections(detections, 0.0);
        eprintln!("\n=== After post_process ===");
        eprintln!("Detections: {}", processed.len());
        for (i, d) in processed.iter().enumerate().take(10) {
            eprintln!("Detection {}: {:?}", i + 1, d.license_expression);
        }
    }
}
