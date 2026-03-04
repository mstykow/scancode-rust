//! Check is_matchable after aho matches

use scancode_rust::license_detection::query::{PositionSpan, Query};
use scancode_rust::license_detection::{LicenseDetectionEngine, aho_match, refine_aho_matches};
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

    println!("=== Initial state ===");
    println!("Total tokens: {}", query.tokens.len());
    println!("Whole run: {}-{:?}", whole_run.start, whole_run.end);
    println!("Matchables count: {}", whole_run.matchables(false).len());

    // Get aho matches
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    println!("\n=== Aho matches ===");
    for m in &refined_aho {
        println!(
            "  {} (start={}, end={}, coverage={:.1}%)",
            m.rule_identifier, m.start_token, m.end_token, m.match_coverage
        );
    }

    // Build matched_qspans like detect_matches does
    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    println!("\n=== Matched qspans ===");
    for span in &matched_qspans {
        println!("  {}-{}", span.start, span.end);
    }

    // Check if sequence matching should be skipped
    let whole_run = query.whole_query_run();
    let is_matchable = whole_run.is_matchable(false, &matched_qspans);

    println!("\n=== After aho matches ===");
    println!("is_matchable: {}", is_matchable);

    // What tokens are still matchable?
    if is_matchable {
        let matchables = whole_run.matchables(false);
        let matched_set: std::collections::HashSet<_> = matched_qspans
            .iter()
            .flat_map(|s| s.start..=s.end)
            .collect();
        let remaining: Vec<_> = matchables
            .iter()
            .filter(|p| !matched_set.contains(p))
            .take(20)
            .collect();
        println!("Remaining matchable tokens (first 20): {:?}", remaining);
    }
}
