//! Find the unicode_3 that covers tokens 985-1468

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

    // Find unicode_3 with tokens 985-1468 (after merge)
    let unicode_3_big = merged_seq
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token >= 900);
    let unicode_40 = refined_aho
        .iter()
        .find(|m| m.rule_identifier == "unicode_40.RULE");
    let unicode_42 = refined_aho
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE");

    if let (Some(u3), Some(u40), Some(u42)) = (unicode_3_big, unicode_40, unicode_42) {
        println!("=== unicode_3.RULE (big) ===");
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

        // Check if u3 really contains u40/u42
        println!("\n=== Token range checks ===");
        println!("u3 range: {}-{}", u3.start_token, u3.end_token);
        println!("u40 range: {}-{}", u40.start_token, u40.end_token);
        println!("u42 range: {}-{}", u42.start_token, u42.end_token);

        // Check manually
        let u3_contains_u40 = u3.start_token <= u40.start_token && u3.end_token >= u40.end_token;
        let u3_contains_u42 = u3.start_token <= u42.start_token && u3.end_token >= u42.end_token;
        println!("u3 token-contains u40: {}", u3_contains_u40);
        println!("u3 token-contains u42: {}", u3_contains_u42);
    } else {
        println!("Could not find one of the matches");
        println!("\nAll unicode_3 matches:");
        for m in merged_seq
            .iter()
            .filter(|m| m.rule_identifier == "unicode_3.RULE")
        {
            println!(
                "  start={}, end={}, lines {}-{}",
                m.start_token, m.end_token, m.start_line, m.end_line
            );
        }
    }
}
