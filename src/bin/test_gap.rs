//! Check gap in merged match

use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets,
    merge_overlapping_matches, seq_match_with_candidates,
};
use std::path::PathBuf;

fn main() {
    let path =
        PathBuf::from("testdata/license-golden/datadriven/external/fossology-licenses/unicode.txt");
    let bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    let engine = LicenseDetectionEngine::new(&rules_path).unwrap();
    let index = engine.index();

    let query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    // Get seq matches
    let near_dupe_candidates =
        compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
    let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);

    const MAX_SEQ_CANDIDATES: usize = 70;
    let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates);

    let mut all_seq = Vec::new();
    all_seq.extend(near_dupe_matches);
    all_seq.extend(seq_matches);

    let merged_seq = merge_overlapping_matches(&all_seq);

    // Get the merged unicode_3 match
    let u3 = merged_seq
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token == 985)
        .unwrap();

    // Check the qspan positions vs the token range
    if let Some(positions) = &u3.qspan_positions {
        let pos_set: std::collections::HashSet<usize> = positions.iter().copied().collect();

        // Find all gaps in the range 985-1468
        let mut gaps: Vec<(usize, usize)> = Vec::new();
        let mut gap_start = None;
        let mut prev: Option<usize> = None;

        for p in 985..=1467 {
            if !pos_set.contains(&p) {
                if gap_start.is_none() {
                    gap_start = Some(p);
                }
            } else {
                if let Some(start) = gap_start {
                    gaps.push((start, p - 1));
                    gap_start = None;
                }
            }
        }
        if let Some(start) = gap_start {
            gaps.push((start, 1467));
        }

        println!("=== Gaps in merged match qspan (985-1467) ===");
        for (start, end) in &gaps {
            println!("  gap: {}-{} ({} tokens)", start, end, end - start + 1);
        }

        // Check if unicode_42 range (1127-1467) has gaps
        println!("\n=== Gaps in unicode_42 range (1127-1467) ===");
        let u42_gaps: Vec<_> = gaps.iter().filter(|(s, e)| *s >= 1127).collect();
        for (start, end) in &u42_gaps {
            println!("  gap: {}-{} ({} tokens)", start, end, end - start + 1);
        }

        // Check if the gap between 1120 and 1122 affects containment
        // The merged match was created from 985-1021, 1021-1120, 1122-1446, 1455-1456, 1460-1468
        // So there's a gap at 1120-1121 (missing) and 1446-1454 (missing)
        println!("\n=== Token coverage ===");
        println!("Total tokens in range: {}", 1468 - 985);
        println!("Tokens in qspan: {}", positions.len());
        println!(
            "Coverage: {:.1}%",
            positions.len() as f64 / (1468 - 985) as f64 * 100.0
        );
    }
}
