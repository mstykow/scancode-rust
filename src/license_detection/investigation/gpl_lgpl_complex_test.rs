//! Investigation test for PLAN-001: gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt
//!
//! ## Issue
//! Extra lgpl-2.1-plus detection caused by spurious seq match spanning gap.
//!
//! **Expected:** `["gpl-3.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus AND free-unknown", "mit-modern", "gpl-2.0-plus", "gpl-2.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0"]`
//!
//! **Actual:** `["gpl-3.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus AND free-unknown", "mit-modern", "gpl-2.0-plus", "gpl-2.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0"]`
//!
//! ## Root Cause
//! The seq match `lgpl-2.1-plus_419.RULE` at lines 14-25 incorrectly spans across:
//! - First LGPL block (lines 13-17)
//! - Gap with different content (lines 18-21: copyright + "Files: lib/ifd.h")
//! - Second LGPL block (lines 22-26)
//!
//! This causes the aho match at lines 13-17 to be discarded in filter_overlapping_matches.

#[cfg(test)]
mod tests {
    use crate::license_detection::aho_match::aho_match;
    use crate::license_detection::index::LicenseIndex;
    use crate::license_detection::match_refine::{
        filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
    };
    use crate::license_detection::query::Query;
    use crate::license_detection::seq_match::{
        compute_candidates_with_msets, seq_match_with_candidates,
    };
    use crate::license_detection::LicenseDetectionEngine;
    use std::path::PathBuf;

    const TEST_FILE: &str =
        "testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt";

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    fn read_test_file() -> Option<String> {
        std::fs::read_to_string(TEST_FILE).ok()
    }

    #[test]
    fn test_plan_001_divergence_point() {
        let Some(engine) = get_engine() else {
            eprintln!("Skipping test: engine not loaded");
            return;
        };
        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PLAN-001 Divergence Point Investigation ===");
        eprintln!("\nTest file structure:");
        eprintln!("  Lines 5-9: GPL-3+ license text");
        eprintln!("  Lines 13-17: LGPL-2.1+ license text (first block)");
        eprintln!("  Lines 18-21: Copyright info and 'Files: lib/ifd.h' header");
        eprintln!("  Lines 22-26: LGPL-2.1+ license text (second block)");
        eprintln!("  Lines 33-37: LGPL-2.1+ license text with MIT-style grant (third block)");

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        eprintln!("\n=== Step 1: Aho Matching ===");
        let aho_matches = aho_match(index, &query_run);
        eprintln!("Total Aho matches: {}", aho_matches.len());

        let lgpl_aho: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.rule_identifier.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("\nLGPL-2.1+ Aho matches:");
        for m in &lgpl_aho {
            eprintln!(
                "  lines={}-{} rule={} tokens={}-{} matcher={}",
                m.start_line, m.end_line, m.rule_identifier, m.start_token, m.end_token, m.matcher
            );
        }

        let merged_aho = merge_overlapping_matches(&aho_matches);
        eprintln!("\nMerged Aho matches: {}", merged_aho.len());

        eprintln!("\n=== Step 2: Seq Matching ===");
        let candidates = compute_candidates_with_msets(index, &query_run, false, 70);
        eprintln!("Seq candidates: {}", candidates.len());

        let seq_matches = seq_match_with_candidates(index, &query_run, &candidates);
        eprintln!("Seq matches: {}", seq_matches.len());

        let lgpl_seq: Vec<_> = seq_matches
            .iter()
            .filter(|m| m.rule_identifier.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("\nLGPL-2.1+ Seq matches:");
        for m in &lgpl_seq {
            eprintln!(
                "  lines={}-{} rule={} tokens={}-{} coverage={:.1}%",
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        let merged_seq = merge_overlapping_matches(&seq_matches);
        eprintln!("\nMerged Seq matches: {}", merged_seq.len());

        let lgpl_merged_seq: Vec<_> = merged_seq
            .iter()
            .filter(|m| m.rule_identifier.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("\nLGPL-2.1+ Merged Seq matches:");
        for m in &lgpl_merged_seq {
            eprintln!(
                "  lines={}-{} rule={} tokens={}-{} coverage={:.1}%",
                m.start_line,
                m.end_line,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.match_coverage
            );
        }

        eprintln!("\n=== KEY FINDING: Spurious seq match spanning gap ===");
        eprintln!("The seq match lgpl-2.1-plus_419.RULE at lines 14-25 spans across:");
        eprintln!("  - First LGPL block (lines 13-17)");
        eprintln!("  - Gap with different content (lines 18-21: copyright + 'Files: lib/ifd.h')");
        eprintln!("  - Second LGPL block (lines 22-26)");
        eprintln!("This is INCORRECT - Python does NOT produce this match.");

        eprintln!("\n=== Step 3: Combine and Filter ===");
        let mut all_matches = merged_aho.clone();
        all_matches.extend(merged_seq.clone());
        eprintln!("All matches before refine: {}", all_matches.len());

        let merged = merge_overlapping_matches(&all_matches);
        let (kept, discarded) = filter_contained_matches(&merged);

        eprintln!("\nAfter filter_contained_matches:");
        eprintln!("  Kept: {}", kept.len());
        eprintln!("  Discarded: {}", discarded.len());

        let lgpl_kept: Vec<_> = kept
            .iter()
            .filter(|m| m.rule_identifier.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("\nLGPL-2.1+ matches KEPT after filter_contained:");
        for m in &lgpl_kept {
            eprintln!(
                "  lines={}-{} rule={} tokens={}-{}",
                m.start_line, m.end_line, m.rule_identifier, m.start_token, m.end_token
            );
        }

        let (kept2, discarded2) = filter_overlapping_matches(kept.clone(), index);
        eprintln!("\nAfter filter_overlapping_matches:");
        eprintln!("  Kept: {}", kept2.len());
        eprintln!("  Discarded: {}", discarded2.len());

        let lgpl_kept2: Vec<_> = kept2
            .iter()
            .filter(|m| m.rule_identifier.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("\nLGPL-2.1+ matches KEPT after filter_overlapping:");
        for m in &lgpl_kept2 {
            eprintln!(
                "  lines={}-{} rule={} tokens={}-{}",
                m.start_line, m.end_line, m.rule_identifier, m.start_token, m.end_token
            );
        }

        let lgpl_discarded2: Vec<_> = discarded2
            .iter()
            .filter(|m| m.rule_identifier.contains("lgpl-2.1-plus"))
            .collect();
        eprintln!("\nLGPL-2.1+ matches DISCARDED after filter_overlapping:");
        for m in &lgpl_discarded2 {
            eprintln!(
                "  lines={}-{} rule={} tokens={}-{}",
                m.start_line, m.end_line, m.rule_identifier, m.start_token, m.end_token
            );
        }

        eprintln!("\n=== ANALYSIS ===");
        eprintln!("The aho match at lines 13-17 (lgpl-2.1-plus_24.RULE) is discarded because");
        eprintln!("it overlaps with the spurious seq match at lines 14-25.");
        eprintln!("\nPython behavior (expected):");
        eprintln!("  - Lines 13-17: lgpl-2.1-plus (from aho match lgpl-2.1-plus_24.RULE)");
        eprintln!("  - Lines 22-26: lgpl-2.1-plus (from aho match lgpl-2.1-plus_24.RULE)");
        eprintln!("  - Lines 33-37: lgpl-2.1-plus AND free-unknown");
        eprintln!("  Total: 3 lgpl-2.1-plus expressions");
        eprintln!("\nRust behavior (actual):");
        eprintln!(
            "  - Lines 14-25: lgpl-2.1-plus (from seq match lgpl-2.1-plus_419.RULE) <- WRONG"
        );
        eprintln!("  - Lines 22-26: lgpl-2.1-plus (from aho match lgpl-2.1-plus_24.RULE)");
        eprintln!("  - Lines 33-37: lgpl-2.1-plus AND free-unknown");
        eprintln!("  Total: 4 lgpl-2.1-plus expressions (1 extra)");
    }

    #[test]
    fn test_plan_001_expected_behavior() {
        let Some(engine) = get_engine() else {
            eprintln!("Skipping test: engine not loaded");
            return;
        };
        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== PLAN-001 Expected Behavior Test ===");

        let index = engine.index();
        let query = Query::new(&text, index).expect("Query should be created");
        let query_run = query.whole_query_run();

        let aho_matches = aho_match(index, &query_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);

        let candidates = compute_candidates_with_msets(index, &query_run, false, 70);
        let seq_matches = seq_match_with_candidates(index, &query_run, &candidates);
        let merged_seq = merge_overlapping_matches(&seq_matches);

        let lgpl_seq_lines_14_25 = merged_seq.iter().find(|m| {
            m.rule_identifier.contains("lgpl-2.1-plus") && m.start_line == 14 && m.end_line == 25
        });

        eprintln!("\nISSUE: Seq match lgpl-2.1-plus at lines 14-25 should NOT exist.");
        eprintln!("It incorrectly spans two separate LGPL blocks with a gap between them.");

        if let Some(m) = lgpl_seq_lines_14_25 {
            eprintln!(
                "\nFOUND SPURIOUS MATCH: {} at lines {}-{}",
                m.rule_identifier, m.start_line, m.end_line
            );
            eprintln!("This match should either:");
            eprintln!("  1. Not be generated by seq matching, OR");
            eprintln!("  2. Be filtered out before final results");
        } else {
            eprintln!("\nGOOD: No spurious lgpl-2.1-plus seq match at lines 14-25");
        }

        let lgpl_aho_at_13_17: Vec<_> = merged_aho
            .iter()
            .filter(|m| {
                m.rule_identifier.contains("lgpl-2.1-plus")
                    && m.start_line == 13
                    && m.end_line == 17
            })
            .collect();
        eprintln!("\nAho matches at lines 13-17 (expected: lgpl-2.1-plus):");
        for m in &lgpl_aho_at_13_17 {
            eprintln!("  {} (matcher={})", m.rule_identifier, m.matcher);
        }

        let lgpl_aho_at_22_26: Vec<_> = merged_aho
            .iter()
            .filter(|m| {
                m.rule_identifier.contains("lgpl-2.1-plus")
                    && m.start_line == 22
                    && m.end_line == 26
            })
            .collect();
        eprintln!("\nAho matches at lines 22-26 (expected: lgpl-2.1-plus):");
        for m in &lgpl_aho_at_22_26 {
            eprintln!("  {} (matcher={})", m.rule_identifier, m.matcher);
        }
    }
}
