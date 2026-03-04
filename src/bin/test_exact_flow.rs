//! Trace the exact filter flow for the critical matches

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

    // Combine
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(merged_seq.clone());

    // Now simulate filter_contained_matches step by step
    // but only for the critical matches

    // Get only the three critical unicode matches (excluding unicode-tou)
    let critical: Vec<_> = all_matches
        .iter()
        .filter(|m| m.license_expression == "unicode" && m.start_token >= 900)
        .cloned()
        .collect();

    println!("=== CRITICAL UNICODE MATCHES ===");
    for m in &critical {
        println!(
            "  {} (qstart={}, qend={}, hilen={}, matcher={})",
            m.rule_identifier,
            m.qstart(),
            m.end_token,
            m.hilen(),
            m.matcher_order()
        );
    }

    // Sort like filter_contained_matches
    let mut sorted = critical.clone();
    sorted.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    println!("\n=== SORTED (by qstart, -hilen, -matched_len, matcher_order) ===");
    for (i, m) in sorted.iter().enumerate() {
        println!(
            "[{}] {} (qstart={}, qend={}, hilen={}, matcher={})",
            i,
            m.rule_identifier,
            m.qstart(),
            m.end_token,
            m.hilen(),
            m.matcher_order()
        );
    }

    // Now trace the filter algorithm
    println!("\n=== SIMULATING filter_contained_matches ===");
    let mut matches = sorted.clone();
    let mut discarded = Vec::new();

    let mut i = 0;
    while i < matches.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < matches.len() {
            let current_qend = matches[i].end_token;
            let next_qend = matches[j].end_token;
            let next_qstart = matches[j].qstart();

            // Python: if next_match.qend > current_match.qend: j += 1; break
            // Rust: if next.end_token > current.end_token: break
            if next_qend > current_qend {
                println!(
                    "[i={}, j={}] {} qend({}) > {} qend({}) -> BREAK (but should j+=1 first in Python)",
                    i,
                    j,
                    matches[j].rule_identifier,
                    next_qend,
                    matches[i].rule_identifier,
                    current_qend
                );
                // THIS IS THE BUG! Python does j += 1 BEFORE break, but Rust doesn't!
                break;
            }

            // Check containment
            if matches[i].qcontains(&matches[j]) {
                println!(
                    "[i={}, j={}] {} qcontains {} -> DISCARD j",
                    i, j, matches[i].rule_identifier, matches[j].rule_identifier
                );
                discarded.push(matches.remove(j));
                continue;
            }
            if matches[j].qcontains(&matches[i]) {
                println!(
                    "[i={}, j={}] {} qcontains {} -> DISCARD i",
                    i, j, matches[j].rule_identifier, matches[i].rule_identifier
                );
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            j += 1;
        }
        i += 1;
    }

    println!("\n=== RESULT ===");
    for m in &matches {
        println!(
            "  {} (qstart={}, qend={})",
            m.rule_identifier,
            m.qstart(),
            m.end_token
        );
    }
}
