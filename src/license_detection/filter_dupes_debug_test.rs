// Debug test for filter_dupes investigation

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

    fn read_file(name: &str) -> Option<String> {
        std::fs::read_to_string(name).ok()
    }

    #[test]
    fn test_git_mk_filter_dupes_debug() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_file("testdata/license-golden/datadriven/external/slic-tests/git.mk")
        else {
            return;
        };

        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx, post_process_detections, sort_matches_by_line,
        };
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
            refine_matches, refine_matches_without_false_positive_filter, split_weak_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, seq_match_with_candidates, MAX_NEAR_DUPE_CANDIDATES,
        };
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

        eprintln!("\n=== git.mk ===");
        eprintln!("Query tokens: {}", whole_run.matchable_tokens().len());

        // Phase 3: seq matching with high_resemblance=false
        let seq_candidates = compute_candidates_with_msets(&index, &whole_run, false, 70);
        eprintln!("\n=== Seq candidates (high_resemblance=false) ===");
        eprintln!("Count: {}", seq_candidates.len());

        // Run seq_match_with_candidates to see what matches are produced
        eprintln!("\n=== Running seq_match_with_candidates ===");
        let matches = seq_match_with_candidates(&index, &whole_run, &seq_candidates);
        eprintln!("Matches produced: {}", matches.len());

        // Check for fsfap matches
        let fsfap_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.license_expression.contains("fsfap"))
            .collect();
        eprintln!("\n=== fsfap matches ===");
        eprintln!("Count: {}", fsfap_matches.len());
        for m in &fsfap_matches {
            eprintln!(
                "{} - score={:.2}, start={}, end={}, len={}",
                m.rule_identifier, m.match_coverage, m.start_token, m.end_token, m.matched_length
            );
        }

        // Now run through the refinement pipeline
        eprintln!("\n=== Refinement pipeline ===");

        let merged = merge_overlapping_matches(&matches);
        eprintln!("After merge_overlapping_matches: {} matches", merged.len());

        let (non_contained, _discarded_contained) = filter_contained_matches(&merged);
        eprintln!(
            "After filter_contained_matches: {} matches",
            non_contained.len()
        );

        let (filtered, _discarded_overlapping) = filter_overlapping_matches(non_contained, &index);
        eprintln!(
            "After filter_overlapping_matches: {} matches",
            filtered.len()
        );

        let refined_no_fp = refine_matches_without_false_positive_filter(&index, filtered, &query);
        eprintln!(
            "After refine_matches_without_false_positive_filter: {} matches",
            refined_no_fp.len()
        );

        let (good_matches, weak_matches) = split_weak_matches(&refined_no_fp);
        eprintln!(
            "After split_weak_matches: {} good, {} weak",
            good_matches.len(),
            weak_matches.len()
        );

        let mut all_matches = good_matches;
        all_matches.extend(weak_matches);

        let refined = refine_matches(&index, all_matches, &query);
        eprintln!("After final refine_matches: {} matches", refined.len());

        // Check for fsfap after refinement
        let fsfap_refined: Vec<_> = refined
            .iter()
            .filter(|m| m.license_expression.contains("fsfap"))
            .collect();
        eprintln!("\n=== fsfap after refinement ===");
        eprintln!("Count: {}", fsfap_refined.len());
        for m in &fsfap_refined {
            eprintln!(
                "{} - score={:.2}, start={}, end={}, len={}",
                m.rule_identifier, m.match_coverage, m.start_token, m.end_token, m.matched_length
            );
        }

        // Create detections
        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);
        let groups = group_matches_by_region(&sorted);
        eprintln!("\n=== Detection groups ===");
        eprintln!("Group count: {}", groups.len());

        let detections: Vec<_> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(&mut detection, group, &spdx_mapping);
                detection
            })
            .collect();

        let detections = post_process_detections(detections, 0.0);
        eprintln!("\n=== Final detections ===");
        eprintln!("Detection count: {}", detections.len());
        for d in &detections {
            eprintln!(
                "expr: {:?}, score: from {} matches",
                d.license_expression,
                d.matches.len()
            );
        }
    }

    #[test]
    fn test_lgpl_filter_dupes_debug() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_file("testdata/license-golden/datadriven/lic3/lgpl-2.1_14.txt")
        else {
            return;
        };

        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, seq_match_with_candidates, MAX_NEAR_DUPE_CANDIDATES,
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

        eprintln!("\n=== lgpl-2.1_14.txt ===");
        eprintln!("Query tokens: {}", whole_run.matchable_tokens().len());

        // Phase 2: near-dupe with high_resemblance=true
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("\n=== Near-dupe candidates (high_resemblance=true) ===");
        eprintln!("Count: {}", near_dupe_candidates.len());
        for (i, c) in near_dupe_candidates.iter().enumerate().take(10) {
            eprintln!(
                "{}: {} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1})",
                i,
                c.rule.identifier,
                c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant,
                c.score_vec_rounded.containment,
                c.score_vec_rounded.resemblance,
                c.score_vec_rounded.matched_length
            );
        }

        // Phase 3: seq matching with high_resemblance=false
        let seq_candidates = compute_candidates_with_msets(&index, &whole_run, false, 70);
        eprintln!("\n=== Seq candidates (high_resemblance=false) ===");
        eprintln!("Count: {}", seq_candidates.len());

        // Look for lgpl-2.1
        let lgpl_candidates: Vec<_> = seq_candidates
            .iter()
            .filter(|c| c.rule.license_expression.contains("lgpl-2.1"))
            .collect();
        eprintln!("\n=== lgpl-2.1 candidates ===");
        eprintln!("Count: {}", lgpl_candidates.len());
        for c in &lgpl_candidates {
            eprintln!(
                "{} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1})",
                c.rule.identifier,
                c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant,
                c.score_vec_rounded.containment,
                c.score_vec_rounded.resemblance,
                c.score_vec_rounded.matched_length
            );
        }

        // Show top 20 candidates
        eprintln!("\n=== Top 20 seq candidates ===");
        for (i, c) in seq_candidates.iter().enumerate().take(20) {
            eprintln!(
                "{}: {} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1})",
                i,
                c.rule.identifier,
                c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant,
                c.score_vec_rounded.containment,
                c.score_vec_rounded.resemblance,
                c.score_vec_rounded.matched_length
            );
        }

        // Run seq_match_with_candidates to see what matches are produced
        eprintln!("\n=== Running seq_match_with_candidates ===");
        let matches = seq_match_with_candidates(&index, &whole_run, &seq_candidates);
        eprintln!("Matches produced: {}", matches.len());
        for (i, m) in matches.iter().enumerate().take(20) {
            eprintln!(
                "{}: {} - score={:.2}, start={}, end={}, matched_len={}",
                i,
                m.rule_identifier,
                m.match_coverage,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        // Check for lgpl-2.1 matches
        let lgpl_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1"))
            .collect();
        eprintln!("\n=== lgpl-2.1 matches ===");
        eprintln!("Count: {}", lgpl_matches.len());
        for m in &lgpl_matches {
            eprintln!(
                "{} - score={:.2}, start={}, end={}",
                m.rule_identifier, m.match_coverage, m.start_token, m.end_token
            );
        }
    }

    #[test]
    fn test_mit_cmu_style_filter_dupes_debug() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_file(
            "testdata/license-golden/datadriven/external/fossology-tests/CMU/MIT-CMU-style.txt",
        ) else {
            return;
        };

        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, seq_match_with_candidates, MAX_NEAR_DUPE_CANDIDATES,
        };
        use crate::utils::text::strip_utf8_bom_str;
        use std::path::PathBuf;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== MIT-CMU-style.txt ===");
        eprintln!("Query tokens: {}", whole_run.matchable_tokens().len());

        // Phase 2: near-dupe with high_resemblance=true
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("\n=== Near-dupe candidates (high_resemblance=true) ===");
        eprintln!("Count: {}", near_dupe_candidates.len());
        for (i, c) in near_dupe_candidates.iter().enumerate().take(15) {
            eprintln!("{}: {} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                i, c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }

        // Look for x11-dec1 and cmu-uc
        let x11_candidates: Vec<_> = near_dupe_candidates
            .iter()
            .filter(|c| c.rule.license_expression.contains("x11-dec1"))
            .collect();
        eprintln!("\n=== x11-dec1 candidates ===");
        eprintln!("Count: {}", x11_candidates.len());
        for c in &x11_candidates {
            eprintln!("{} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }

        let cmu_candidates: Vec<_> = near_dupe_candidates
            .iter()
            .filter(|c| c.rule.license_expression.contains("cmu-uc"))
            .collect();
        eprintln!("\n=== cmu-uc candidates ===");
        eprintln!("Count: {}", cmu_candidates.len());
        for c in &cmu_candidates {
            eprintln!("{} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }

        // Now run seq_match_with_candidates to see what matches are produced
        eprintln!("\n=== Running seq_match_with_candidates ===");
        let matches = seq_match_with_candidates(&index, &whole_run, &near_dupe_candidates);
        eprintln!("Matches produced: {}", matches.len());

        // Check for x11-dec1 and cmu-uc in matches
        let x11_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.license_expression.contains("x11-dec1"))
            .collect();
        eprintln!("\n=== x11-dec1 matches ===");
        eprintln!("Count: {}", x11_matches.len());
        for m in &x11_matches {
            eprintln!(
                "{} - score={:.2}, lines={}-{}, matched_len={}",
                m.rule_identifier, m.match_coverage, m.start_line, m.end_line, m.matched_length
            );
        }

        let cmu_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.license_expression.contains("cmu-uc"))
            .collect();
        eprintln!("\n=== cmu-uc matches ===");
        eprintln!("Count: {}", cmu_matches.len());
        for m in &cmu_matches {
            eprintln!(
                "{} - score={:.2}, lines={}-{}, matched_len={}",
                m.rule_identifier, m.match_coverage, m.start_line, m.end_line, m.matched_length
            );
        }
    }

    #[test]
    fn test_mit_t21_filter_dupes_debug() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_file("testdata/license-golden/datadriven/external/glc/MIT.t21")
        else {
            return;
        };

        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, MAX_NEAR_DUPE_CANDIDATES,
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

        eprintln!("\n=== MIT.t21 ===");
        eprintln!("Query tokens: {}", whole_run.matchable_tokens().len());

        // Phase 2: near-dupe with high_resemblance=true
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("\n=== Near-dupe candidates (high_resemblance=true) ===");
        eprintln!("Count: {}", near_dupe_candidates.len());
        for (i, c) in near_dupe_candidates.iter().enumerate().take(15) {
            eprintln!("{}: {} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                i, c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }

        // Look for proprietary-license and mit
        let proprietary_candidates: Vec<_> = near_dupe_candidates
            .iter()
            .filter(|c| c.rule.license_expression.contains("proprietary-license"))
            .collect();
        eprintln!("\n=== proprietary-license candidates ===");
        eprintln!("Count: {}", proprietary_candidates.len());
        for c in &proprietary_candidates {
            eprintln!("{} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }

        let mit_candidates: Vec<_> = near_dupe_candidates
            .iter()
            .filter(|c| c.rule.license_expression == "mit")
            .collect();
        eprintln!("\n=== mit candidates ===");
        eprintln!("Count: {}", mit_candidates.len());
        for c in &mit_candidates {
            eprintln!("{} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }
    }

    #[test]
    fn test_bsd_f_filter_dupes_debug() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) =
            read_file("testdata/license-golden/datadriven/external/licensecheck/devscripts/bsd.f")
        else {
            return;
        };

        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::license_detection::seq_match::{
            compute_candidates_with_msets, MAX_NEAR_DUPE_CANDIDATES,
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

        eprintln!("\n=== bsd.f ===");
        eprintln!("Query tokens: {}", whole_run.matchable_tokens().len());

        // Phase 2: near-dupe with high_resemblance=true
        let near_dupe_candidates =
            compute_candidates_with_msets(&index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        eprintln!("\n=== Near-dupe candidates (high_resemblance=true) ===");
        eprintln!("Count: {}", near_dupe_candidates.len());
        for (i, c) in near_dupe_candidates.iter().enumerate().take(15) {
            eprintln!("{}: {} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                i, c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }

        // Look for bsd-simplified and bsd-new
        let bsd_simplified_candidates: Vec<_> = near_dupe_candidates
            .iter()
            .filter(|c| c.rule.license_expression.contains("bsd-simplified"))
            .collect();
        eprintln!("\n=== bsd-simplified candidates ===");
        eprintln!("Count: {}", bsd_simplified_candidates.len());
        for c in &bsd_simplified_candidates {
            eprintln!("{} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }

        let bsd_new_candidates: Vec<_> = near_dupe_candidates
            .iter()
            .filter(|c| c.rule.license_expression == "bsd-new")
            .collect();
        eprintln!("\n=== bsd-new candidates ===");
        eprintln!("Count: {}", bsd_new_candidates.len());
        for c in &bsd_new_candidates {
            eprintln!("{} - expr: {} - svr: (hr={}, cont={:.2}, resem={:.2}, ml={:.1}) svf: (hr={}, cont={:.3}, resem={:.3}, ml={:.1})",
                c.rule.identifier, c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant, c.score_vec_rounded.containment, c.score_vec_rounded.resemblance, c.score_vec_rounded.matched_length,
                c.score_vec_full.is_highly_resemblant, c.score_vec_full.containment, c.score_vec_full.resemblance, c.score_vec_full.matched_length);
        }
    }
}
