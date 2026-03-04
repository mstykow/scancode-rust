//! Check missing tokens

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

    // Find the matches
    let unicode_3 = merged_seq
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token >= 900)
        .unwrap();
    let unicode_40 = refined_aho
        .iter()
        .find(|m| m.rule_identifier == "unicode_40.RULE")
        .unwrap();
    let unicode_42 = refined_aho
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE")
        .unwrap();

    if let Some(u3_positions) = &unicode_3.qspan_positions {
        let u3_set: std::collections::HashSet<usize> = u3_positions.iter().copied().collect();

        // Check u40 range
        let u40_missing: Vec<_> = (unicode_40.start_token..unicode_40.end_token)
            .filter(|p| !u3_set.contains(p))
            .collect();
        println!("u40 missing tokens from u3: {:?}", u40_missing);

        // Check u42 range
        let u42_missing: Vec<_> = (unicode_42.start_token..unicode_42.end_token)
            .filter(|p| !u3_set.contains(p))
            .collect();
        println!(
            "u42 missing tokens from u3 (count={}): {:?}",
            u42_missing.len(),
            &u42_missing[..u42_missing.len().min(20)]
        );

        // Check what tokens are in u3 set
        println!(
            "\nu3 set contains tokens from {} to {}",
            u3_positions.iter().min().unwrap(),
            u3_positions.iter().max().unwrap()
        );

        // Check if the gap between u40 and u42 is in u3 set
        let gap_tokens: Vec<_> = (unicode_40.end_token..unicode_42.start_token).collect();
        let gap_in_u3: Vec<_> = gap_tokens
            .iter()
            .filter(|p| u3_set.contains(p))
            .copied()
            .collect();
        println!(
            "Gap tokens ({}-{}): {:?}",
            unicode_40.end_token, unicode_42.start_token, gap_tokens
        );
        println!("Gap tokens in u3 set: {:?}", gap_in_u3);
    }
}
