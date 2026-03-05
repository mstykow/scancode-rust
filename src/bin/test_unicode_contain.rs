//! Trace containment check

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
    let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);

    const MAX_SEQ_CANDIDATES: usize = 70;
    let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);

    let mut all_seq = Vec::new();
    all_seq.extend(near_dupe_matches);
    all_seq.extend(seq_matches);

    let merged_seq = merge_overlapping_matches(&all_seq);

    // Find the critical matches
    let unicode_3 = merged_seq
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE");
    let unicode_40 = refined_aho
        .iter()
        .find(|m| m.rule_identifier == "unicode_40.RULE");
    let unicode_42 = refined_aho
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE");

    if let (Some(u3), Some(u40), Some(u42)) = (unicode_3, unicode_40, unicode_42) {
        println!("=== unicode_3.RULE ===");
        println!(
            "  start={}, end={}, start_line={}, end_line={}",
            u3.start_token, u3.end_token, u3.start_line, u3.end_line
        );
        println!(
            "  qspan_positions: {:?}",
            u3.qspan_positions.as_ref().map(|p| p.len())
        );

        println!("\n=== unicode_40.RULE ===");
        println!(
            "  start={}, end={}, start_line={}, end_line={}",
            u40.start_token, u40.end_token, u40.start_line, u40.end_line
        );
        println!(
            "  qspan_positions: {:?}",
            u40.qspan_positions.as_ref().map(|p| p.len())
        );

        println!("\n=== unicode_42.RULE ===");
        println!(
            "  start={}, end={}, start_line={}, end_line={}",
            u42.start_token, u42.end_token, u42.start_line, u42.end_line
        );
        println!(
            "  qspan_positions: {:?}",
            u42.qspan_positions.as_ref().map(|p| p.len())
        );

        println!("\n=== Containment checks ===");
        println!("u3.qcontains(u40): {}", u3.qcontains(u40));
        println!("u3.qcontains(u42): {}", u3.qcontains(u42));
        println!("u40.qcontains(u42): {}", u40.qcontains(u42));
        println!("u42.qcontains(u40): {}", u42.qcontains(u40));

        // Check if there's a gap in the qspan
        println!("\n=== Gap analysis ===");
        println!("u40 end_token: {}", u40.end_token);
        println!("u42 start_token: {}", u42.start_token);
        println!(
            "Gap: {} tokens",
            u42.start_token as i64 - u40.end_token as i64
        );
    }
}
