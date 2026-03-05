//! Trace full unicode detection pipeline

use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, refine_matches,
    refine_matches_without_false_positive_filter, seq_match_with_candidates, sort_matches_by_line,
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

    // Aho matching (Phase 1c)
    println!("=== PHASE 1c: AHO MATCHING ===");
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);
    println!("Refined aho matches: {}", refined_aho.len());

    // Check if sequence matching should run
    let mut matched_qspans = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(scancode_rust::license_detection::query::PositionSpan::new(
                m.start_token,
                m.end_token - 1,
            ));
        }
    }

    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("\nSkip sequence matching: {}", skip_seq_matching);
    println!("Matched qspans: {:?}", matched_qspans.len());

    // Build all_matches like detect_matches does
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());

    if !skip_seq_matching {
        println!("\n=== PHASE 2: NEAR-DUPE SEQUENCE MATCHING ===");
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        println!("Near-dupe candidates: {}", near_dupe_candidates.len());

        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);
            println!("Near-dupe matches: {}", near_dupe_matches.len());

            let merged_seq = merge_overlapping_matches(&near_dupe_matches);
            println!("After merge: {}", merged_seq.len());

            all_matches.extend(merged_seq);
        }
    }

    println!("\n=== BEFORE REFINEMENT ===");
    println!("Total all_matches: {}", all_matches.len());

    // Step 1: refine WITHOUT false positive filter
    println!("\n=== REFINEMENT STEP 1 (without FP filter) ===");
    let merged_matches = refine_matches_without_false_positive_filter(index, all_matches, &query);
    println!(
        "After refine_matches_without_false_positive_filter: {}",
        merged_matches.len()
    );
    for (i, m) in merged_matches.iter().enumerate() {
        println!(
            "  [{}] {} (license: {}, lines {}-{}, coverage {:.1}%)",
            i, m.rule_identifier, m.license_expression, m.start_line, m.end_line, m.match_coverage
        );
    }

    // Step 2: refine WITH false positive filter
    println!("\n=== REFINEMENT STEP 2 (with FP filter) ===");
    let refined = refine_matches(index, merged_matches, &query);
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
}
