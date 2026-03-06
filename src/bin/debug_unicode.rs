//! Debug unicode.txt AHO match coverage

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

    println!("=== QUERY INFO ===");
    println!("Total tokens: {}", query.tokens.len());
    println!("High matchables: {}", query.high_matchables.len());
    println!("Low matchables: {}", query.low_matchables.len());

    // AHO matching
    println!("\n=== AHO MATCHES ===");
    let raw_aho = aho_match(index, &whole_run);
    println!("Raw AHO matches: {}", raw_aho.len());

    let refined_aho = refine_aho_matches(index, raw_aho, &query);
    println!("Refined AHO matches: {}", refined_aho.len());

    println!("\n=== AHO MATCH DETAILS ===");
    for (i, m) in refined_aho.iter().enumerate() {
        println!(
            "Match {}: {} coverage={:.1}%",
            i + 1,
            m.rule_identifier,
            m.match_coverage
        );
        println!("  is_license_text: {}", m.is_license_text);
        println!("  rule_length: {}", m.rule_length);
        println!("  tokens: {}-{}", m.start_token, m.end_token);
        println!(
            "  Would subtract: is_license_text && length > 120 && coverage > 98: {}",
            m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0
        );
        println!(
            "  Would add to matched_qspans (coverage == 100%): {}",
            (m.match_coverage * 100.0).round() / 100.0 == 100.0
        );
    }

    // Check what positions would be subtracted
    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    println!("\n=== MATCHED QSPANS ===");
    println!("Number of 100% coverage matches: {}", matched_qspans.len());
    for (i, span) in matched_qspans.iter().enumerate() {
        println!("  Span {}: {}-{}", i + 1, span.start, span.end);
    }

    // Check is_matchable
    let is_matchable = whole_run.is_matchable(false, &matched_qspans);
    println!("\n=== IS_MATCHABLE(include_low=false, matched_qspans) ===");
    println!("Result: {}", is_matchable);

    // Also check with an empty matched_qspans
    let is_matchable_empty = whole_run.is_matchable(false, &[]);
    println!(
        "is_matchable(include_low=false, []): {}",
        is_matchable_empty
    );

    // Check how many high matchables remain
    let high_matchables = whole_run.high_matchables();
    println!("\nHigh matchables in whole_run: {}", high_matchables.len());

    // Check what's in matched_qspans
    if !matched_qspans.is_empty() {
        let mut covered_positions: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        for span in &matched_qspans {
            covered_positions.extend(span.positions());
        }
        println!("Covered positions: {}", covered_positions.len());

        let remaining: std::collections::HashSet<usize> = high_matchables
            .difference(&covered_positions)
            .copied()
            .collect();
        println!(
            "Remaining high matchables after subtract: {}",
            remaining.len()
        );

        // Show first few remaining positions
        let mut remaining_vec: Vec<_> = remaining.iter().copied().collect();
        remaining_vec.sort();
        println!(
            "First 10 remaining positions: {:?}",
            &remaining_vec[..remaining_vec.len().min(10)]
        );
    }
}
