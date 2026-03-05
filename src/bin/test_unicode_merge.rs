//! Trace merge behavior

use scancode_rust::license_detection::query::{PositionSpan, Query};
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

    // Just aho matches
    println!("=== AHO MATCHES ONLY ===");
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);
    for m in &refined_aho {
        println!(
            "  {} (license: {}, lines {}-{}, tokens {}-{}, coverage {:.1}%)",
            m.rule_identifier,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.match_coverage
        );
    }

    // Get sequence matches
    println!("\n=== SEQUENCE MATCHES (sample) ===");
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
    println!(
        "Seq matches: {}, merged: {}",
        all_seq.len(),
        merged_seq.len()
    );

    // Show merged seq matches that overlap with unicode matches
    println!("\n=== MERGED SEQ MATCHES that span unicode matches ===");
    for m in merged_seq
        .iter()
        .filter(|m| m.license_expression.contains("unicode"))
    {
        println!(
            "  {} (license: {}, lines {}-{}, tokens {}-{}, coverage {:.1}%)",
            m.rule_identifier,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.match_coverage
        );
    }

    // Combine and merge
    println!("\n=== COMBINE AND MERGE ===");
    let mut combined = Vec::new();
    combined.extend(refined_aho.clone());
    combined.extend(merged_seq.clone());

    println!("Before merge: {}", combined.len());
    let merged = merge_overlapping_matches(&combined);
    println!("After merge: {}", merged.len());

    // Show what survives
    println!("\nSurviving matches:");
    for m in &merged {
        println!(
            "  {} (license: {}, lines {}-{}, tokens {}-{})",
            m.rule_identifier,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token
        );
    }
}
