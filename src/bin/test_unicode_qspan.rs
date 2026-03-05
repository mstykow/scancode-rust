//! Analyze qspan positions

use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, seq_match_with_candidates,
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
    let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);

    const MAX_SEQ_CANDIDATES: usize = 70;
    let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);

    let mut all_seq = Vec::new();
    all_seq.extend(near_dupe_matches);
    all_seq.extend(seq_matches);

    let merged_seq = merge_overlapping_matches(&all_seq);

    // Find unicode_3 with tokens 985-1468
    let unicode_3_big = merged_seq
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token >= 900)
        .unwrap();

    println!("=== unicode_3.RULE (big) ===");
    println!(
        "  Token range: {}-{} ({} tokens)",
        unicode_3_big.start_token,
        unicode_3_big.end_token,
        unicode_3_big.end_token - unicode_3_big.start_token
    );
    println!(
        "  qspan_positions: {:?}",
        unicode_3_big.qspan_positions.as_ref().map(|p| p.len())
    );
    println!("  matched_length: {}", unicode_3_big.matched_length);

    if let Some(positions) = &unicode_3_big.qspan_positions {
        // Check if all tokens in range 985-1468 are in positions
        let pos_set: std::collections::HashSet<usize> = positions.iter().copied().collect();
        let all_in_set = (985..1468).all(|p| pos_set.contains(&p));
        println!("  All tokens in 985-1468 in qspan set: {}", all_in_set);

        // Find missing tokens
        let missing: Vec<_> = (985..1468).filter(|p| !pos_set.contains(p)).collect();
        println!(
            "  Missing tokens (first 20): {:?}",
            &missing[..missing.len().min(20)]
        );
        println!("  Total missing: {}", missing.len());

        // Check if tokens 1127-1468 (unicode_42 range) are all in the set
        let u42_range_all_in = (1127..1468).all(|p| pos_set.contains(&p));
        println!(
            "\n  All tokens in 1127-1468 in qspan set: {}",
            u42_range_all_in
        );

        let u42_missing: Vec<_> = (1127..1468).filter(|p| !pos_set.contains(p)).collect();
        println!(
            "  u42 range missing tokens (first 20): {:?}",
            &u42_missing[..u42_missing.len().min(20)]
        );
        println!("  u42 range total missing: {}", u42_missing.len());
    }
}
