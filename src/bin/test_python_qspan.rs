//! Simulate Python's qspan behavior

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

    // Get aho matches
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

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

    // Find unicode_3 BEFORE merge
    let u3_before_merge: Vec<_> = all_seq
        .iter()
        .filter(|m| m.rule_identifier == "unicode_3.RULE")
        .collect();

    println!("=== unicode_3.RULE matches BEFORE merge ===");
    for m in &u3_before_merge {
        println!(
            "  start={}, end={}, lines {}-{}, qspan_len={:?}",
            m.start_token,
            m.end_token,
            m.start_line,
            m.end_line,
            m.qspan_positions.as_ref().map(|p| p.len())
        );
    }

    // Find unicode_3 AFTER merge
    let u3_after_merge: Vec<_> = merged_seq
        .iter()
        .filter(|m| m.rule_identifier == "unicode_3.RULE")
        .collect();

    println!("\n=== unicode_3.RULE matches AFTER merge ===");
    for m in &u3_after_merge {
        println!(
            "  start={}, end={}, lines {}-{}, qspan_len={:?}",
            m.start_token,
            m.end_token,
            m.start_line,
            m.end_line,
            m.qspan_positions.as_ref().map(|p| p.len())
        );
    }

    // The issue: after merge, we have a unicode_3 match spanning 985-1468
    // that was merged from multiple smaller matches
    // This merged match has qspan_positions that are the union of all the matches

    // Let's check if the issue is with how merge creates the qspan_positions
}
