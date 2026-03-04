//! Trace unicode detection step by step

use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, aho_match, merge_overlapping_matches, refine_aho_matches,
    refine_matches, refine_matches_without_false_positive_filter, sort_matches_by_line,
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

    // Create query
    let query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    // Aho matching
    println!("=== AHO MATCHING ===");
    let raw_aho = aho_match(index, &whole_run);
    println!("Raw aho matches: {}", raw_aho.len());

    let refined_aho = refine_aho_matches(index, raw_aho.clone(), &query);
    println!("Refined aho matches: {}", refined_aho.len());
    for (i, m) in refined_aho.iter().enumerate() {
        println!(
            "  [{}] {} (license: {}, lines {}-{})",
            i, m.rule_identifier, m.license_expression, m.start_line, m.end_line
        );
    }

    // Combine matches (like detect_matches does)
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());

    println!("\n=== REFINEMENT STEP 1 (without FP filter) ===");
    let merged_matches =
        refine_matches_without_false_positive_filter(index, all_matches.clone(), &query);
    println!(
        "After refine_matches_without_false_positive_filter: {}",
        merged_matches.len()
    );
    for (i, m) in merged_matches.iter().enumerate() {
        println!(
            "  [{}] {} (license: {}, lines {}-{})",
            i, m.rule_identifier, m.license_expression, m.start_line, m.end_line
        );
    }

    println!("\n=== REFINEMENT STEP 2 (with FP filter) ===");
    let refined = refine_matches(index, merged_matches.clone(), &query);
    println!("After refine_matches: {}", refined.len());
    for (i, m) in refined.iter().enumerate() {
        println!(
            "  [{}] {} (license: {}, lines {}-{})",
            i, m.rule_identifier, m.license_expression, m.start_line, m.end_line
        );
    }

    let mut sorted = refined;
    sort_matches_by_line(&mut sorted);

    let expressions: Vec<&str> = sorted
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    println!("\nFinal expressions: {:?}", expressions);

    // Now compare with detect_matches()
    println!("\n=== detect_matches() result ===");
    let matches = engine.detect_matches(&text, false).unwrap();
    println!("Match count: {}", matches.len());
    let exprs: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    println!("Expressions: {:?}", exprs);
}
