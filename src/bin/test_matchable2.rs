//! Check is_matchable logic

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

    // Get aho matches
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    // Build matched_qspans like detect_matches does
    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    println!("=== matched_qspans ===");
    for span in &matched_qspans {
        // Can't access private fields, but we know what they are
    }
    println!("  0-982 (unicode-tou_7)");
    println!("  985-1118 (unicode_40)");
    println!("  1127-1467 (unicode_42)");

    let whole_run = query.whole_query_run();
    let is_matchable = whole_run.is_matchable(false, &matched_qspans);

    println!("\n=== is_matchable(include_low=false) ===");
    println!("Result: {}", is_matchable);

    // Check what high_matchables remain after subtracting matched_qspans
    let high = whole_run.high_matchables();
    println!("\nTotal high_matchables: {}", high.len());

    // Build a set of matched positions
    let matched_set: std::collections::HashSet<_> = [0..=982, 985..=1118, 1127..=1467]
        .iter()
        .flat_map(|r| r.clone())
        .collect();

    let remaining: Vec<_> = high.iter().filter(|p| !matched_set.contains(p)).collect();

    println!(
        "Remaining high_matchables after subtraction: {:?}",
        remaining
    );

    // The issue: there's one remaining high_matchable (983)
    // This means is_matchable returns true, and sequence matching runs
    // But the sequence match shouldn't create matches that overlap with the aho matches!
}
