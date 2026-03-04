//! Trace match ordering

use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, seq_match_with_candidates,
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

    // Get seq matches
    let near_dupe_candidates =
        compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
    let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);

    const MAX_SEQ_CANDIDATES: usize = 70;
    let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates);

    let mut all_seq = Vec::new();
    all_seq.extend(near_dupe_matches);
    all_seq.extend(seq_matches);

    let merged_seq = merge_overlapping_matches(&all_seq);

    // Combine all
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(merged_seq.clone());

    // Sort like filter_contained_matches does
    // sort on start, longer high, longer match, matcher type
    all_matches.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    println!("=== ALL MATCHES SORTED (sample) ===");
    for (i, m) in all_matches.iter().take(20).enumerate() {
        println!(
            "[{:3}] {} (license: {}, start={}, end={}, hilen={}, matched_len={}, matcher={}, coverage={:.1}%)",
            i,
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token,
            m.hilen(),
            m.matched_length,
            m.matcher_order(),
            m.match_coverage
        );
    }

    // Show the critical matches
    println!("\n=== CRITICAL MATCHES (in sorted order) ===");
    for m in all_matches.iter().filter(|m| {
        m.rule_identifier.contains("unicode") && (m.start_token == 0 || m.start_token >= 985)
    }) {
        println!(
            "  {} (license: {}, start={}, end={}, hilen={}, matched_len={}, matcher={}, coverage={:.1}%)",
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token,
            m.hilen(),
            m.matched_length,
            m.matcher_order(),
            m.match_coverage
        );
    }
}
