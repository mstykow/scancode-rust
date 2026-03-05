//! Investigation test for duplicate license detection issue.
//!
//! Issue: Licenses that appear multiple times in a file are not being detected as separate matches.
//!
//! Test cases:
//! 1. mit_25.txt - Expected: ["mit", "mit"] Actual: ["mit"]
//! 2. libevent.LICENSE - Expected 7 matches, got 5
//! 3. flex-readme.txt - Expected: ["flex-2.5", "flex-2.5", "flex-2.5"] Actual: ["flex-2.5"]

use crate::license_detection::LicenseDetectionEngine;
use std::path::PathBuf;

fn create_engine() -> LicenseDetectionEngine {
    let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    LicenseDetectionEngine::new(&data_path).expect("Failed to create engine")
}

#[test]
fn debug_mit_25_duplicate_issue() {
    let engine = create_engine();

    let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic3/mit_25.txt")
        .expect("Failed to read test file");

    // Use the index directly to check Aho matches before refinement
    use crate::license_detection::aho_match::aho_match;
    use crate::license_detection::match_refine::{
        filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
        restore_non_overlapping,
    };
    use crate::license_detection::query::Query;

    let query = Query::new(&text, engine.index()).expect("Query failed");
    let whole_run = query.whole_query_run();

    // Get raw Aho matches
    let raw_aho_matches = aho_match(engine.index(), &whole_run);

    eprintln!(
        "\n=== RAW AHO MATCHES ({} matches) ===",
        raw_aho_matches.len()
    );
    for (i, m) in raw_aho_matches.iter().enumerate() {
        eprintln!(
            "{}: {} (rid={}) lines {}-{} tokens {}-{} ref={} text={}",
            i,
            m.license_expression,
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.is_license_reference,
            m.is_license_text
        );
    }

    // Check after merge_overlapping_matches
    let merged_matches = merge_overlapping_matches(&raw_aho_matches);

    eprintln!(
        "\n=== AFTER merge_overlapping_matches ({} matches) ===",
        merged_matches.len()
    );
    for (i, m) in merged_matches.iter().enumerate() {
        eprintln!(
            "{}: {} (rid={}) lines {}-{} tokens {}-{} ref={} text={}",
            i,
            m.license_expression,
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.is_license_reference,
            m.is_license_text
        );
    }

    // Check after filter_contained_matches
    let (kept_contained, discarded_contained) = filter_contained_matches(&merged_matches);

    eprintln!("\n=== AFTER filter_contained_matches ===");
    eprintln!("KEPT ({} matches):", kept_contained.len());
    for (i, m) in kept_contained.iter().enumerate() {
        eprintln!(
            "{}: {} (rid={}) lines {}-{} tokens {}-{} ref={} text={}",
            i,
            m.license_expression,
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.is_license_reference,
            m.is_license_text
        );
    }
    eprintln!("DISCARDED ({} matches):", discarded_contained.len());
    for (i, m) in discarded_contained.iter().enumerate() {
        eprintln!(
            "{}: {} (rid={}) lines {}-{} tokens {}-{} ref={} text={}",
            i,
            m.license_expression,
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.is_license_reference,
            m.is_license_text
        );
    }

    // Check after filter_overlapping_matches
    let (kept_overlapping, discarded_overlapping) =
        filter_overlapping_matches(kept_contained.clone(), engine.index());

    eprintln!("\n=== AFTER filter_overlapping_matches ===");
    eprintln!("KEPT ({} matches):", kept_overlapping.len());
    for (i, m) in kept_overlapping.iter().enumerate() {
        eprintln!(
            "{}: {} (rid={}) lines {}-{} tokens {}-{} ref={} text={}",
            i,
            m.license_expression,
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.is_license_reference,
            m.is_license_text
        );
    }
    eprintln!("DISCARDED ({} matches):", discarded_overlapping.len());
    for (i, m) in discarded_overlapping.iter().enumerate() {
        eprintln!(
            "{}: {} (rid={}) lines {}-{} tokens {}-{} ref={} text={}",
            i,
            m.license_expression,
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.is_license_reference,
            m.is_license_text
        );
    }

    // Check after restore_non_overlapping
    let (restored_contained, _) = restore_non_overlapping(&kept_overlapping, discarded_contained);
    let all_after_restore: Vec<_> = kept_overlapping
        .iter()
        .chain(restored_contained.iter())
        .cloned()
        .collect();

    eprintln!(
        "\n=== AFTER restore_non_overlapping ({} matches) ===",
        all_after_restore.len()
    );
    for (i, m) in all_after_restore.iter().enumerate() {
        eprintln!(
            "{}: {} (rid={}) lines {}-{} tokens {}-{} ref={} text={}",
            i,
            m.license_expression,
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.is_license_reference,
            m.is_license_text
        );
    }
}

#[test]
fn debug_libevent_duplicate_issue() {
    let engine = create_engine();

    let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic3/libevent.LICENSE")
        .expect("Failed to read test file");

    let matches = engine
        .detect_matches(&text, false)
        .expect("Detection failed");

    eprintln!("\n=== libevent.LICENSE matches ===");
    eprintln!("Total matches: {}", matches.len());
    for (i, m) in matches.iter().enumerate() {
        eprintln!("Match {}:", i);
        eprintln!("  license_expression: {}", m.license_expression);
        eprintln!("  rule_identifier: {}", m.rule_identifier);
        eprintln!("  lines: {}-{}", m.start_line, m.end_line);
        eprintln!("  tokens: {}-{}", m.start_token, m.end_token);
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    eprintln!(
        "\nExpected: [\"bsd-new\", \"bsd-new\", \"bsd-new\", \"isc\", \"isc\", \"mit\", \"mit\"]"
    );
    eprintln!("Actual: {:?}", expressions);
}

#[test]
fn debug_flex_readme_duplicate_issue() {
    let engine = create_engine();

    let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic1/flex-readme.txt")
        .expect("Failed to read test file");

    let matches = engine
        .detect_matches(&text, false)
        .expect("Detection failed");

    eprintln!("\n=== flex-readme.txt matches ===");
    eprintln!("Total matches: {}", matches.len());
    for (i, m) in matches.iter().enumerate() {
        eprintln!("Match {}:", i);
        eprintln!("  license_expression: {}", m.license_expression);
        eprintln!("  rule_identifier: {}", m.rule_identifier);
        eprintln!("  lines: {}-{}", m.start_line, m.end_line);
        eprintln!("  tokens: {}-{}", m.start_token, m.end_token);
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    eprintln!("\nExpected: [\"flex-2.5\", \"flex-2.5\", \"flex-2.5\"]");
    eprintln!("Actual: {:?}", expressions);
}
