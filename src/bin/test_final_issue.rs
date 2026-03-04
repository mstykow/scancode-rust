//! Trace the exact filter_contained_matches behavior

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

    // Sort like filter_contained_matches
    all_matches.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    // Find the indices of the critical matches
    println!("=== SORTED MATCHES (critical ones) ===");
    for (i, m) in all_matches.iter().enumerate() {
        if (m.rule_identifier == "unicode-tou_7.RULE" && m.start_token == 0)
            || (m.rule_identifier == "unicode_3.RULE" && m.start_token >= 900)
            || (m.rule_identifier == "unicode_40.RULE")
            || (m.rule_identifier == "unicode_42.RULE" && m.matcher == "2-aho")
        {
            println!(
                "[{}] {} (qstart={}, end={}, hilen={}, matched_len={}, matcher={})",
                i,
                m.rule_identifier,
                m.qstart(),
                m.end_token,
                m.hilen,
                m.matched_length,
                m.matcher_order()
            );
        }
    }

    // Simulate filter_contained_matches algorithm
    println!("\n=== SIMULATED filter_contained_matches ===");
    let mut matches = all_matches.clone();
    let mut discarded = Vec::new();

    let mut i = 0;
    while i < matches.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < matches.len() {
            let current = matches[i].clone();
            let next = matches[j].clone();

            // Break if next.end_token > current.end_token
            if next.end_token > current.end_token {
                j += 1;
                continue; // Python breaks here, but let's trace
            }

            // Check containment
            if current.qcontains(&next) {
                println!(
                    "[i={}, j={}] {} qcontains {} -> DISCARD j",
                    i, j, current.rule_identifier, next.rule_identifier
                );
                discarded.push(matches.remove(j));
                continue;
            }
            if next.qcontains(&current) {
                println!(
                    "[i={}, j={}] {} qcontains {} -> DISCARD i",
                    i, j, next.rule_identifier, current.rule_identifier
                );
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            j += 1;
        }
        i += 1;
    }

    println!("\n=== REMAINING MATCHES (unicode only) ===");
    for m in matches.iter().filter(|m| m.license_expression == "unicode") {
        println!(
            "  {} (start={}, end={}, matcher={})",
            m.rule_identifier, m.start_token, m.end_token, m.matcher
        );
    }
}
