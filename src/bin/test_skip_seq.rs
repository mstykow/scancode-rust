//! Check if sequence matching should be skipped after aho

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
    println!(
        "High matchables count: {}",
        whole_run.high_matchables().len()
    );

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

    // Build matched_qspans like Python does (only for 100% coverage)
    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    for m in &refined_aho {
        if m.match_coverage == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    // Check if there are matchable tokens left
    let whole_run = query.whole_query_run();
    let is_matchable = whole_run.is_matchable(false, &matched_qspans);

    println!("\n=== After aho matches ===");
    println!("is_matchable: {}", is_matchable);

    // Check what high matchables remain
    let high_matchables = whole_run.high_matchables();
    println!("Total high matchables: {}", high_matchables.len());

    // Check if there are any high matchables not covered by the matches
    // The aho matches cover: 0-983, 985-1119, 1127-1468
    // So the gaps are: none (all covered with 100% coverage)

    // Actually, the issue is that there IS a gap: 984, 1119-1126
    // Let me check what tokens are in the gap
    println!("\n=== Checking gaps ===");
    let covered: std::collections::HashSet<_> = [(0, 983), (985, 1119), (1127, 1468)]
        .iter()
        .flat_map(|(s, e)| *s..*e)
        .collect();

    let uncovered: Vec<_> = (0..query.tokens.len())
        .filter(|p| high_matchables.contains(p) && !covered.contains(p))
        .take(20)
        .collect();

    println!("Uncovered high matchables (first 20): {:?}", uncovered);
}
