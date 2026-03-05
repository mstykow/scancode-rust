//! Debug the full refine pipeline

use std::path::PathBuf;

use scancode_rust::license_detection::query::PositionSpan;
use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, refine_matches_without_false_positive_filter,
    seq_match_with_candidates,
};

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

    // Phase 1: AHO matching
    let raw_aho = aho_match(index, &whole_run);
    println!("=== RAW AHO MATCHES ({}) ===", raw_aho.len());
    for m in &raw_aho {
        if m.rule_identifier.contains("unicode") {
            println!(
                "  {}: tokens {}-{}, hilen={}, matcher={}",
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.matcher_order()
            );
        }
    }

    // Refine AHO matches
    let refined_aho = refine_aho_matches(index, raw_aho, &query);
    println!("\n=== REFINED AHO MATCHES ({}) ===", refined_aho.len());
    for m in &refined_aho {
        if m.rule_identifier.contains("unicode") {
            println!(
                "  {}: tokens {}-{}, hilen={}, matcher={}",
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.matcher_order()
            );
        }
    }

    // Build matched_qspans for sequence matching
    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    for m in &refined_aho {
        if m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    // Check if sequence matching should run
    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("\n=== SEQUENCE MATCHING ===");
    println!("skip_seq_matching: {}", skip_seq_matching);

    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());

    if !skip_seq_matching {
        // Phase 2: Near-duplicate detection
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);

        // Phase 3: Regular sequence matching
        const MAX_SEQ_CANDIDATES: usize = 70;
        let candidates =
            compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
        let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);

        let mut seq_all = Vec::new();
        seq_all.extend(near_dupe_matches);
        seq_all.extend(seq_matches);

        let merged_seq = merge_overlapping_matches(&seq_all);

        println!("\n=== MERGED SEQ MATCHES ({}) ===", merged_seq.len());
        for m in &merged_seq {
            if m.rule_identifier.contains("unicode") {
                println!(
                    "  {}: tokens {}-{}, hilen={}, matcher={}",
                    m.rule_identifier,
                    m.start_token,
                    m.end_token,
                    m.hilen(),
                    m.matcher_order()
                );
            }
        }

        all_matches.extend(merged_seq);
    }

    println!(
        "\n=== ALL MATCHES BEFORE FINAL REFINE ({}) ===",
        all_matches.len()
    );
    for m in &all_matches {
        if m.rule_identifier.contains("unicode") {
            println!(
                "  {}: tokens {}-{}, hilen={}, matcher={}",
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.matcher_order()
            );
        }
    }

    // Final refine
    let refined = refine_matches_without_false_positive_filter(index, all_matches, &query);

    println!("\n=== FINAL REFINED ({}) ===", refined.len());
    for m in &refined {
        if m.rule_identifier.contains("unicode") {
            println!(
                "  {}: tokens {}-{}, hilen={}, matcher={}",
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.matcher_order()
            );
        }
    }

    println!("\n=== LICENSE EXPRESSIONS ===");
    for m in &refined {
        println!("{}", m.license_expression);
    }
}
