//! Simulate Python's matching flow

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

    // PHASE 1: Aho matching (Python: get_exact_matches)
    // Python calls refine_matches with merge=False on aho matches
    // Then Python adds these to the matches list (without further merging)
    let raw_aho = aho_match(index, &whole_run);
    let aho_refined = refine_aho_matches(index, raw_aho, &query); // This is like Python's merge=False

    println!("=== AHO MATCHES (after refine with merge=False equivalent) ===");
    for m in &aho_refined {
        println!(
            "  {} (qstart={}, end={})",
            m.rule_identifier,
            m.qstart(),
            m.end_token
        );
    }

    // PHASE 2: Sequence matching
    // Python calls match.merge_matches() on sequence matches BEFORE extending
    let near_dupe_candidates =
        compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
    let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);

    const MAX_SEQ_CANDIDATES: usize = 70;
    let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates);

    let mut all_seq = Vec::new();
    all_seq.extend(near_dupe_matches);
    all_seq.extend(seq_matches);

    // This is what Python does: merge_matches on seq results
    let seq_merged = merge_overlapping_matches(&all_seq);

    println!("\n=== SEQ MATCHES (after merge) ===");
    let u3_matches: Vec<_> = seq_merged
        .iter()
        .filter(|m| m.rule_identifier == "unicode_3.RULE")
        .collect();
    println!("unicode_3.RULE matches: {}", u3_matches.len());
    for m in &u3_matches {
        println!("  qstart={}, end={}", m.qstart(), m.end_token);
    }

    // PHASE 3: Combine (Python style: each matcher's results already merged)
    // Python: matches.extend(matched) where matched is already merged
    let mut py_matches = Vec::new();
    py_matches.extend(aho_refined.clone());
    py_matches.extend(seq_merged.clone());

    println!("\n=== COMBINED MATCHES (Python style) ===");
    println!("Total: {}", py_matches.len());

    // PHASE 4: refine_matches with merge=True
    let py_refined = merge_overlapping_matches(&py_matches);

    // Check what we get
    println!("\n=== AFTER FINAL MERGE ===");
    let unicode_matches: Vec<_> = py_refined
        .iter()
        .filter(|m| m.license_expression == "unicode")
        .collect();
    println!("Unicode matches: {}", unicode_matches.len());
    for m in &unicode_matches {
        println!(
            "  {} (qstart={}, end={})",
            m.rule_identifier,
            m.qstart(),
            m.end_token
        );
    }
}
