//! Investigation test for PLAN-064: Wrong Detection (CPL 1.0 HTML)
//!
//! This test file investigates why Rust produces "unknown-license-reference"
//! instead of "cpl-1.0" for the test file cpl-1.0_in_html.html.
//!
//! ## Root Cause Analysis
//!
//! The issue is in how Rust merges overlapping matches for the same rule.
//!
//! **Python behavior:**
//! - 61 raw CPL-1.0 seq matches → merge_matches() → 1 match with 96.7% coverage
//! - The merge_matches function in Python combines all 61 fragments into a single
//!   large match spanning lines 4-119
//!
//! **Rust behavior:**
//! - 165 CPL-1.0 seq matches → merge_overlapping_matches() → fragmented matches
//! - Rust's merge produces separate matches: lines 13-47, lines 53-99, etc.
//! - These fragments are then filtered/combined incorrectly, leading to EPL detection
//!
//! **Key difference:**
//! Python's merge_matches uses a sophisticated algorithm that combines matches
//! when they "surround" each other or are "is_after" each other, respecting
//! both qspan (query span) and ispan (index/rule span) relationships.
//!
//! Rust's merge_overlapping_matches only merges matches from the same rule together,
//! but the algorithm for determining when to combine is less aggressive.
//!
//! The CPL-1.0.LICENSE text is embedded in HTML, so the matches are fragmented
//! by HTML tags. Python's merge correctly reconstructs the full license match
//! from these fragments, while Rust's merge leaves them fragmented.

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
        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");
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

    /// Test that verifies the divergence point between Python and Rust.
    ///
    /// Python: 61 raw CPL-1.0 seq matches → 1 merged match (96.7% coverage)
    /// Rust: 165 CPL-1.0 seq matches → fragmented matches (not merged properly)
    #[test]
    fn test_cpl_10_merge_divergence() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cpl-1.0_in_html.html") else {
            return;
        };

        use crate::license_detection::match_refine::merge_overlapping_matches;
        use crate::license_detection::query::Query;
        use crate::license_detection::seq_match::seq_match;

        let query = Query::new(&text, engine.index()).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        println!("\n========================================");
        println!("MERGE DIVERGENCE TEST");
        println!("========================================");

        // Get CPL-1.0 seq matches only
        let seq_matches = seq_match(engine.index(), &whole_run);
        let cpl_matches: Vec<_> = seq_matches
            .iter()
            .filter(|m| m.license_expression.contains("cpl-1.0"))
            .cloned()
            .collect();

        println!("Raw CPL-1.0 seq matches: {}", cpl_matches.len());

        // Merge CPL matches
        let merged = merge_overlapping_matches(&cpl_matches);
        println!("After merge_overlapping_matches: {}", merged.len());

        for m in merged.iter().take(10) {
            println!(
                "  {} (lines {}-{}, score={:.2}, coverage={:.1}%, qstart={}, qend={})",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.score,
                m.match_coverage,
                m.start_token,
                m.end_token
            );
        }

        // Check if we got a single large match
        let has_large_match = merged.iter().any(|m| m.match_coverage > 90.0);
        println!("\nHas match with >90% coverage: {}", has_large_match);

        // This test documents the divergence:
        // - Python: 61 raw → 1 merged with 96.7% coverage
        // - Rust: 165 raw → multiple merged with low coverage
        // The fix should make Rust's merge produce similar results to Python
    }

    /// Test that documents the expected behavior from Python.
    ///
    /// This test compares Python and Rust behavior step by step:
    /// 1. Python with location (file path) produces: ["cpl-1.0"] with 96.65% coverage
    /// 2. Rust produces: fragmented EPL detection
    ///
    /// Root cause: Rust's merge_overlapping_matches() fails to merge fragments
    /// that should be combined because:
    /// - Python's `surround()` checks only qspan, not ispan
    /// - Python's `is_after()` checks both qspan and ispan strictly
    /// - Rust's implementation has subtle differences in these checks
    #[test]
    fn test_python_vs_rust_cpl_10_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cpl-1.0_in_html.html") else {
            return;
        };

        println!("\n========================================");
        println!("PYTHON vs RUST CPL-1.0 DETECTION");
        println!("========================================");

        // Run Rust detection
        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        println!("\nRust detections:");
        for d in &detections {
            println!("  {:?}", d.license_expression);
        }

        // Expected: Python detects "cpl-1.0" with 96.65% coverage
        // Actual: Rust detects "unknown-license-reference AND epl-1.0 AND warranty-disclaimer"
        //
        // The fix needed:
        // 1. Fix Rust's merge_overlapping_matches() to match Python's behavior
        // 2. Specifically, ensure that:
        //    - `is_after()` correctly detects when match positions are strictly increasing
        //    - `surround()` only checks qspan (not ispan) - THIS IS THE BUG
        //    - Merge combines matches when they are within max_rule_side_dist
        //
        // Key finding:
        // Python's surround() only checks qspan:
        //   return self.qstart <= other.qstart and self.qend >= other.qend
        // Rust's surround() checks BOTH qspan AND ispan (incorrect):
        //   qsurrounds && isurrounds
        //
        // This causes Rust to NOT merge matches that Python would merge.
    }

    /// Investigation test for PLAN-085: gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt
    ///
    /// Python produces 2 detections:
    /// 1. Rule 1: lines 2-13
    /// 2. Rule 6: lines 25-37
    ///
    /// Rust produces 1 detection:
    /// 1. Rule 18: lines 2-37 (broader rule that covers both sections)
    #[test]
    fn test_plan_085_gpl_lgpl_mpl_trace() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from(
            "testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt",
        );
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return,
        };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
            refine_matches,
        };
        use crate::license_detection::query::Query;

        println!("\n========================================");
        println!("PLAN-085: gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt");
        println!("========================================");

        let query = Query::new(&text, engine.index()).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        // Phase 1: Aho-Corasick matching
        let aho_matches = aho_match(engine.index(), &whole_run);
        println!("\n--- Phase 1: Aho matches: {} ---", aho_matches.len());

        // Filter for MPL/GPL/LGPL expression
        let mpl_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| {
                m.rule_identifier
                    .contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus")
            })
            .collect();
        println!("MPL/GPL/LGPL matches: {}", mpl_matches.len());
        for m in &mpl_matches {
            println!(
                "  Rule: {}, lines: {}-{}, tokens: {}-{}, len: {}",
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        // Step 2: After merge_overlapping_matches
        println!("\n--- Step 2: After merge_overlapping_matches ---");
        let merged = merge_overlapping_matches(&aho_matches);
        let merged_mpl: Vec<_> = merged
            .iter()
            .filter(|m| {
                m.rule_identifier
                    .contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus")
            })
            .collect();
        println!("MPL/GPL/LGPL matches: {}", merged_mpl.len());
        for m in &merged_mpl {
            println!(
                "  Rule: {}, lines: {}-{}, tokens: {}-{}, len: {}",
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        // Step 3: After filter_contained_matches
        println!("\n--- Step 3: After filter_contained_matches ---");
        let (kept, discarded) = filter_contained_matches(&merged);
        let kept_mpl: Vec<_> = kept
            .iter()
            .filter(|m| {
                m.rule_identifier
                    .contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus")
            })
            .collect();
        let disc_mpl: Vec<_> = discarded
            .iter()
            .filter(|m| {
                m.rule_identifier
                    .contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus")
            })
            .collect();
        println!("Kept MPL/GPL/LGPL matches: {}", kept_mpl.len());
        for m in &kept_mpl {
            println!(
                "  Rule: {}, lines: {}-{}, tokens: {}-{}, len: {}",
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }
        println!("Discarded MPL/GPL/LGPL matches: {}", disc_mpl.len());
        for m in &disc_mpl {
            println!(
                "  Rule: {}, lines: {}-{}, tokens: {}-{}, len: {}",
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        // Step 4: After filter_overlapping_matches
        println!("\n--- Step 4: After filter_overlapping_matches ---");
        let (kept2, discarded2) = filter_overlapping_matches(kept, engine.index());
        let kept2_mpl: Vec<_> = kept2
            .iter()
            .filter(|m| {
                m.rule_identifier
                    .contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus")
            })
            .collect();
        println!("Kept MPL/GPL/LGPL matches: {}", kept2_mpl.len());
        for m in &kept2_mpl {
            println!(
                "  Rule: {}, lines: {}-{}, tokens: {}-{}, len: {}",
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        // Step 5: Final refinement
        println!("\n--- Step 5: After full refine_matches ---");
        let refined = refine_matches(engine.index(), aho_matches.clone(), &query);
        let refined_mpl: Vec<_> = refined
            .iter()
            .filter(|m| {
                m.rule_identifier
                    .contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus")
            })
            .collect();
        println!("Final MPL/GPL/LGPL matches: {}", refined_mpl.len());
        for m in &refined_mpl {
            println!(
                "  Rule: {}, lines: {}-{}, tokens: {}-{}, len: {}",
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length
            );
        }

        // Expected: 2 matches (rules 1 and 6 at different locations)
        // Actual: 1 match (rule 18 covering everything)
    }

    /// Check if seq matches override the correct aho matches
    #[test]
    fn test_plan_085_full_detection() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from(
            "testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt",
        );
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return,
        };

        use crate::license_detection::LicenseDetectionEngine;

        println!("\n========================================");
        println!("PLAN-085: Full Detection");
        println!("========================================");

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");
        println!("Total detections: {}", detections.len());

        for d in &detections {
            println!("\nDetection: {:?}", d.license_expression);
            for m in &d.matches {
                println!(
                    "  Match: {} lines {}-{}, matcher={}, rule={}",
                    m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
                );
            }
        }
    }
}
