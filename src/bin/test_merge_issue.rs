//! Check merge issue

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

    // Combine aho and seq BEFORE merge
    let mut combined_before_merge = Vec::new();
    combined_before_merge.extend(refined_aho.clone());
    combined_before_merge.extend(all_seq.clone());

    println!("=== BEFORE ANY MERGE ===");
    println!("Total matches: {}", combined_before_merge.len());

    // Merge all at once (like Rust does in detect_matches)
    let merged_all = merge_overlapping_matches(&combined_before_merge);
    println!("\n=== AFTER MERGE ALL AT ONCE ===");
    println!("Total matches: {}", merged_all.len());

    // Check what survived
    let unicode_matches: Vec<_> = merged_all
        .iter()
        .filter(|m| m.license_expression == "unicode")
        .collect();
    println!("\nUnicode matches: {}", unicode_matches.len());
    for m in &unicode_matches {
        println!(
            "  {} (start={}, end={}, matcher={})",
            m.rule_identifier, m.start_token, m.end_token, m.matcher
        );
    }

    // Alternative: merge seq first, then combine (like Python does)
    let merged_seq = merge_overlapping_matches(&all_seq);
    let mut combined_py_style = Vec::new();
    combined_py_style.extend(refined_aho.clone());
    combined_py_style.extend(merged_seq.clone());

    println!("\n=== PYTHON STYLE (merge seq first, then extend) ===");
    println!("Total matches before refine: {}", combined_py_style.len());

    // Now run through refine_matches
    let merged_py = merge_overlapping_matches(&combined_py_style);
    println!("After merge: {}", merged_py.len());

    let unicode_matches_py: Vec<_> = merged_py
        .iter()
        .filter(|m| m.license_expression == "unicode")
        .collect();
    println!("\nUnicode matches: {}", unicode_matches_py.len());
    for m in &unicode_matches_py {
        println!(
            "  {} (start={}, end={}, matcher={})",
            m.rule_identifier, m.start_token, m.end_token, m.matcher
        );
    }
}
