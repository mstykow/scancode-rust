//! Check for duplicates

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

    println!("=== REFINED AHO MATCHES ===");
    for m in &refined_aho {
        println!(
            "  {} (matcher={}, qstart={}, end={})",
            m.rule_identifier,
            m.matcher,
            m.qstart(),
            m.end_token
        );
    }

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

    println!("\n=== MERGED SEQ MATCHES (unicode-tou_7 and unicode_3 only) ===");
    for m in merged_seq.iter().filter(|m| {
        m.rule_identifier == "unicode-tou_7.RULE" || m.rule_identifier == "unicode_3.RULE"
    }) {
        println!(
            "  {} (matcher={}, qstart={}, end={})",
            m.rule_identifier,
            m.matcher,
            m.qstart(),
            m.end_token
        );
    }

    // Combine and check duplicates
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(merged_seq.clone());

    // Check for unicode-tou_7 duplicates
    let tou7: Vec<_> = all_matches
        .iter()
        .filter(|m| m.rule_identifier == "unicode-tou_7.RULE")
        .collect();
    println!(
        "\n=== unicode-tou_7.RULE matches (count={}) ===",
        tou7.len()
    );
    for m in &tou7 {
        println!(
            "  matcher={}, qstart={}, end={}",
            m.matcher,
            m.qstart(),
            m.end_token
        );
    }
}
