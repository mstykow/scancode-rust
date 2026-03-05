//! Trace all phases of unicode detection

use scancode_rust::license_detection::query::{PositionSpan, Query};
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

    let mut query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    let mut all_matches = Vec::new();
    let mut matched_qspans: Vec<PositionSpan> = Vec::new();

    // Phase 1c: Aho matching
    println!("=== PHASE 1c: AHO MATCHING ===");
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);
    println!("Refined aho matches: {}", refined_aho.len());

    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }
    all_matches.extend(refined_aho);

    // Check skip condition
    let whole_run = query.whole_query_run();
    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("\nSkip sequence matching: {}", skip_seq_matching);

    if !skip_seq_matching {
        // Phase 2: Near-dupe
        println!("\n=== PHASE 2: NEAR-DUPE ===");
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };
        println!("Near-dupe matches: {}", near_dupe_matches.len());

        for m in &near_dupe_matches {
            if m.end_token > m.start_token {
                let span = PositionSpan::new(m.start_token, m.end_token - 1);
                matched_qspans.push(span);
            }
        }

        // Phase 3: Regular sequence
        println!("\n=== PHASE 3: REGULAR SEQ ===");
        const MAX_SEQ_CANDIDATES: usize = 70;
        let whole_run = query.whole_query_run();
        let candidates =
            compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
        let seq_matches = if !candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &candidates)
        } else {
            Vec::new()
        };
        println!("Seq matches: {}", seq_matches.len());

        // Phase 4: Query runs
        println!("\n=== PHASE 4: QUERY RUNS ===");
        const MAX_QUERY_RUN_CANDIDATES: usize = 70;
        let whole_run = query.whole_query_run();
        let mut query_run_matches = Vec::new();
        for query_run in query.query_runs().iter() {
            if query_run.start == whole_run.start && query_run.end == whole_run.end {
                continue;
            }
            if !query_run.is_matchable(false, &matched_qspans) {
                continue;
            }
            let candidates =
                compute_candidates_with_msets(index, query_run, false, MAX_QUERY_RUN_CANDIDATES);
            if !candidates.is_empty() {
                let matches = seq_match_with_candidates(index, query_run, &candidates, &[]);
                query_run_matches.extend(matches);
            }
        }
        println!("Query run matches: {}", query_run_matches.len());

        // Merge all sequence matches
        let mut seq_all_matches = Vec::new();
        seq_all_matches.extend(near_dupe_matches);
        seq_all_matches.extend(seq_matches);
        seq_all_matches.extend(query_run_matches);

        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        println!(
            "\nTotal seq matches before merge: {}",
            seq_all_matches.len()
        );
        println!("After merge: {}", merged_seq.len());

        all_matches.extend(merged_seq);
    }

    println!("\n=== BEFORE REFINEMENT ===");
    println!("Total all_matches: {}", all_matches.len());

    // Step 1: refine WITHOUT false positive filter
    let merged_matches = refine_matches_without_false_positive_filter(index, all_matches, &query);
    println!("\nAfter refine_without_fp: {}", merged_matches.len());

    // Step 2: refine WITH false positive filter
    let refined = refine_matches(index, merged_matches, &query);
    println!("After refine_with_fp: {}", refined.len());

    let mut sorted = refined;
    sort_matches_by_line(&mut sorted);

    let expressions: Vec<&str> = sorted
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    println!("\nFinal expressions: {:?}", expressions);

    // Now compare with actual detect_matches()
    println!("\n=== ACTUAL detect_matches() ===");
    let actual = engine.detect_matches(&text, false).unwrap();
    let actual_exprs: Vec<&str> = actual
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    println!("Match count: {}", actual.len());
    println!("Expressions: {:?}", actual_exprs);
}
