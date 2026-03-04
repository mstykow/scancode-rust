//! Trace refinement step

use scancode_rust::license_detection::query::{PositionSpan, Query};
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, refine_matches,
    refine_matches_without_false_positive_filter, seq_match_with_candidates,
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

    // Get aho matches
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);

    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());

    if !skip_seq_matching {
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = if !near_dupe_candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &near_dupe_candidates)
        } else {
            Vec::new()
        };

        const MAX_SEQ_CANDIDATES: usize = 70;
        let candidates =
            compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
        let seq_matches = if !candidates.is_empty() {
            seq_match_with_candidates(index, &whole_run, &candidates)
        } else {
            Vec::new()
        };

        let mut all_seq = Vec::new();
        all_seq.extend(near_dupe_matches);
        all_seq.extend(seq_matches);

        let merged_seq = merge_overlapping_matches(&all_seq);
        all_matches.extend(merged_seq);
    }

    println!("=== BEFORE REFINEMENT ===");
    println!("Total matches: {}", all_matches.len());

    // Check for the two unicode matches
    let unicode_matches: Vec<_> = all_matches
        .iter()
        .filter(|m| m.license_expression == "unicode")
        .collect();
    println!("\nUnicode matches: {}", unicode_matches.len());
    for m in &unicode_matches {
        println!(
            "  {} (lines {}-{}, tokens {}-{}, coverage {:.1}%)",
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.match_coverage
        );
    }

    // Run refinement step 1
    println!("\n=== REFINEMENT STEP 1 (without FP filter) ===");
    let merged = refine_matches_without_false_positive_filter(index, all_matches.clone(), &query);
    println!("After refine: {}", merged.len());

    // Check for unicode matches after step 1
    let unicode_after: Vec<_> = merged
        .iter()
        .filter(|m| m.license_expression == "unicode")
        .collect();
    println!("\nUnicode matches after step 1: {}", unicode_after.len());
    for m in &unicode_after {
        println!(
            "  {} (lines {}-{}, tokens {}-{}, coverage {:.1}%)",
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.match_coverage
        );
    }

    // Run refinement step 2
    println!("\n=== REFINEMENT STEP 2 (with FP filter) ===");
    let refined = refine_matches(index, merged.clone(), &query);
    println!("After refine: {}", refined.len());

    // Check for unicode matches after step 2
    let unicode_final: Vec<_> = refined
        .iter()
        .filter(|m| m.license_expression == "unicode")
        .collect();
    println!("\nUnicode matches after step 2: {}", unicode_final.len());
    for m in &unicode_final {
        println!(
            "  {} (lines {}-{}, tokens {}-{}, coverage {:.1}%)",
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.match_coverage
        );
    }

    // Check what got filtered
    if unicode_after.len() != unicode_final.len() {
        println!("\n=== UNICODE MATCHES FILTERED OUT ===");
        for m in unicode_after.iter() {
            if !refined
                .iter()
                .any(|r| r.rule_identifier == m.rule_identifier && r.start_token == m.start_token)
            {
                println!(
                    "  {} (lines {}-{}, tokens {}-{})",
                    m.rule_identifier, m.start_line, m.end_line, m.start_token, m.end_token
                );
            }
        }
    }
}
