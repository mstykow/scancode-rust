//! Trace exact containment issue

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
    let unicode_42 = refined_aho
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE")
        .unwrap();

    println!("=== unicode_3.RULE ===");
    println!(
        "  start_token={}, end_token={}",
        unicode_3.start_token, unicode_3.end_token
    );
    println!(
        "  qspan_positions: {:?}",
        unicode_3.qspan_positions.as_ref().map(|p| p.len())
    );
    println!("  matcher={}", unicode_3.matcher);

    println!("\n=== unicode_42.RULE ===");
    println!(
        "  start_token={}, end_token={}",
        unicode_42.start_token, unicode_42.end_token
    );
    println!(
        "  qspan_positions: {:?}",
        unicode_42.qspan_positions.as_ref().map(|p| p.len())
    );
    println!("  matcher={}", unicode_42.matcher);

    // Check qcontains step by step
    println!("\n=== qcontains check ===");

    // The issue: unicode_3 has qspan_positions (Some), unicode_42 has qspan_positions (None)
    // So we use the branch: if let (Some(self_positions), None) = ...

    if let Some(u3_positions) = &unicode_3.qspan_positions {
        println!("u3 has qspan_positions (len={})", u3_positions.len());
        println!(
            "u42 has qspan_positions: {:?}",
            unicode_42.qspan_positions.is_some()
        );

        // The Rust code for this case:
        // if let (Some(self_positions), None) = (&self.qspan_positions, &other.qspan_positions) {
        //     let self_set: HashSet<usize> = self_positions.iter().copied().collect();
        //     return (other.start_token..other.end_token).all(|p| self_set.contains(&p));
        // }

        let u3_set: std::collections::HashSet<usize> = u3_positions.iter().copied().collect();
        let u42_range_check =
            (unicode_42.start_token..unicode_42.end_token).all(|p| u3_set.contains(&p));
        println!("u42 range all in u3 set: {}", u42_range_check);

        // Show missing
        let missing: Vec<_> = (unicode_42.start_token..unicode_42.end_token)
            .filter(|p| !u3_set.contains(p))
            .collect();
        println!(
            "Missing tokens from u42 range: {:?}",
            &missing[..missing.len().min(10)]
        );
    }

    println!("\n=== Result ===");
    println!("u3.qcontains(u42): {}", unicode_3.qcontains(unicode_42));
}
