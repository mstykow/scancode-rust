//! Investigation test for PLAN-064: Wrong Detection (CPL 1.0 HTML)
//!
//! This test file investigates why Rust produces "unknown-license-reference"
//! instead of "cpl-1.0" for the test file cpl-1.0_in_html.html.

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

    fn read_test_file(name: &str) -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic1").join(name);
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_cpl_10_html_full_pipeline_debug() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cpl-1.0_in_html.html") else {
            return;
        };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::detection::{group_matches_by_region, sort_matches_by_line};
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::match_refine::{
            filter_invalid_contained_unknown_matches, merge_overlapping_matches, refine_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::seq_match::{
            MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match,
            seq_match_with_candidates,
        };
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::license_detection::unknown_match::unknown_match;

        println!("\n========================================");
        println!("FULL PIPELINE DEBUG: CPL 1.0 HTML Detection");
        println!("========================================");
        println!("Text length: {} bytes", text.len());

        let query = Query::new(&text, engine.index()).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();
        println!("Query tokens: {}", query.tokens.len());

        // Phase 1a: Hash matching
        let hash_matches = hash_match(engine.index(), &whole_run);
        println!("\n--- Phase 1a: Hash matches: {} ---", hash_matches.len());

        // Phase 1b: SPDX-LID matching
        let spdx_matches = spdx_lid_match(engine.index(), &query);
        println!("--- Phase 1b: SPDX-LID matches: {} ---", spdx_matches.len());

        // Phase 1c: Aho-Corasick matching
        let aho_matches = aho_match(engine.index(), &whole_run);
        println!("--- Phase 1c: Aho matches: {} ---", aho_matches.len());
        for m in aho_matches.iter() {
            println!(
                "  aho: {} (lines {}-{}, coverage={:.1}%)",
                m.license_expression, m.start_line, m.end_line, m.match_coverage
            );
        }

        // Phase 2: Near-duplicate detection
        let near_dupe_candidates = compute_candidates_with_msets(
            engine.index(),
            &whole_run,
            true,
            MAX_NEAR_DUPE_CANDIDATES,
        );
        println!(
            "\n--- Phase 2: Near-dupe candidates: {} ---",
            near_dupe_candidates.len()
        );
        for c in near_dupe_candidates.iter() {
            println!(
                "  candidate: {} (rid={}, resemblance={:.3}, containment={:.3})",
                c.rule.license_expression,
                c.rid,
                c.score_vec_full.resemblance,
                c.score_vec_full.containment
            );
        }

        let near_dupe_matches =
            seq_match_with_candidates(engine.index(), &whole_run, &near_dupe_candidates);
        println!(
            "--- Phase 2: Near-dupe matches: {} ---",
            near_dupe_matches.len()
        );
        for m in near_dupe_matches.iter().take(10) {
            println!(
                "  near-dupe: {} (lines {}-{}, score={:.2}, coverage={:.1}%)",
                m.license_expression, m.start_line, m.end_line, m.score, m.match_coverage
            );
        }

        // Phase 3: Regular sequence matching
        let seq_matches = seq_match(engine.index(), &whole_run);
        println!(
            "\n--- Phase 3: Regular seq matches: {} ---",
            seq_matches.len()
        );

        let cpl_seq_matches: Vec<_> = seq_matches
            .iter()
            .filter(|m| m.license_expression.contains("cpl-1.0"))
            .collect();
        println!("CPL-1.0 seq matches: {}", cpl_seq_matches.len());
        for m in cpl_seq_matches.iter().take(5) {
            println!(
                "  cpl-seq: {} (lines {}-{}, score={:.2}, coverage={:.1}%, rule_len={})",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.score,
                m.match_coverage,
                m.rule_length
            );
        }

        // Collect all matches
        let mut all_matches = Vec::new();
        all_matches.extend(spdx_matches.clone());
        all_matches.extend(merge_overlapping_matches(&aho_matches));
        all_matches.extend(merge_overlapping_matches(&near_dupe_matches));
        all_matches.extend(merge_overlapping_matches(&seq_matches));

        println!(
            "\n--- Total matches before unknown: {} ---",
            all_matches.len()
        );

        // Unknown matching
        let unknown_matches = unknown_match(engine.index(), &query, &all_matches);
        let filtered_unknown =
            filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
        println!("--- Unknown matches (raw): {} ---", unknown_matches.len());
        println!(
            "--- Unknown matches (filtered): {} ---",
            filtered_unknown.len()
        );
        all_matches.extend(filtered_unknown);

        // Refine matches
        let refined = refine_matches(engine.index(), all_matches, &query);
        println!("\n--- Refined matches: {} ---", refined.len());

        // Sort and group
        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);
        let groups = group_matches_by_region(&sorted);
        println!("--- Groups: {} ---", groups.len());

        for (i, group) in groups.iter().enumerate() {
            println!("\nGroup {}:", i + 1);
            for m in group.matches.iter().take(10) {
                println!(
                    "  {} (matcher={}, score={:.2}, coverage={:.1}%, lines={}-{})",
                    m.license_expression,
                    m.matcher,
                    m.score,
                    m.match_coverage,
                    m.start_line,
                    m.end_line
                );
            }
        }

        // Final detection
        let detections = engine.detect(&text).expect("Detection should succeed");
        println!("\n========================================");
        println!("FINAL RESULT");
        println!("========================================");
        for d in &detections {
            println!("Detection: {:?}", d.license_expression);
        }
    }

    #[test]
    fn test_cpl_10_html_check_cpl_license_in_index() {
        let Some(engine) = get_engine() else { return };

        let index = engine.index();

        println!("\n========================================");
        println!("CHECK: Is cpl-1.0 license in index?");
        println!("========================================");

        let cpl_keys: Vec<_> = index
            .licenses_by_key
            .keys()
            .filter(|k| k.starts_with("cpl"))
            .collect();
        println!("CPL license keys: {:?}", cpl_keys);

        let cpl_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("cpl-1.0"))
            .collect();
        println!("\nCPL-1.0 rules count: {}", cpl_rules.len());
        for rule in cpl_rules.iter().take(5) {
            println!(
                "  Rule: {} - is_from_license: {}, text len: {}",
                rule.identifier,
                rule.is_from_license,
                rule.text.len()
            );
        }

        assert!(!cpl_rules.is_empty(), "Should have CPL-1.0 rules in index");
    }
}
