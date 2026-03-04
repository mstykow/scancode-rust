//! Trace filter_contained_matches logic

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

    // Combine all matches
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(merged_seq.clone());

    println!("=== ALL MATCHES ({} total) ===", all_matches.len());
    for m in &all_matches {
        println!(
            "{}: tokens {}-{}, hilen={}, matcher={}",
            m.rule_identifier,
            m.start_token,
            m.end_token,
            m.hilen(),
            m.matcher_order()
        );
    }

    // Filter to just the critical matches for analysis
    let critical: Vec<_> = all_matches
        .iter()
        .filter(|m| {
            (m.rule_identifier == "unicode-tou_7.RULE" && m.start_token == 0)
                || (m.rule_identifier == "unicode_3.RULE" && m.start_token >= 900)
                || (m.rule_identifier == "unicode_40.RULE")
                || (m.rule_identifier == "unicode_42.RULE")
        })
        .cloned()
        .collect();

    println!("=== CRITICAL MATCHES (sorted by qstart, -hilen, -matched_len, matcher_order) ===");
    let mut sorted = critical.clone();
    sorted.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen().cmp(&a.hilen()))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    for (i, m) in sorted.iter().enumerate() {
        println!(
            "[{}] {} (qstart={}, end={}, hilen={}, matched_len={}, matcher={})",
            i,
            m.rule_identifier,
            m.qstart(),
            m.end_token,
            m.hilen(),
            m.matched_length,
            m.matcher_order()
        );
    }

    // Find the matches by rule
    let unicode_3 = sorted
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token >= 900);
    let unicode_40 = sorted
        .iter()
        .find(|m| m.rule_identifier == "unicode_40.RULE");
    let unicode_42 = sorted
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE");

    if let (Some(u3), Some(u40), Some(u42)) = (unicode_3, unicode_40, unicode_42) {
        println!("\n=== CONTAINMENT CHECKS ===");
        println!("u3 ({}) vs u40:", u3.rule_identifier);
        println!("  u3.qcontains(u40): {}", u3.qcontains(u40));
        println!(
            "  u3.range: {}-{}, u40.range: {}-{}",
            u3.start_token, u3.end_token, u40.start_token, u40.end_token
        );

        println!("\nu3 ({}) vs u42:", u3.rule_identifier);
        println!("  u3.qcontains(u42): {}", u3.qcontains(u42));
        println!(
            "  u3.range: {}-{}, u42.range: {}-{}",
            u3.start_token, u3.end_token, u42.start_token, u42.end_token
        );

        // The issue: u3 (985-1468) contains u40 (985-1119) and overlaps with u42 (1127-1468)
        // But u3.qcontains(u42) should be checked!
        // Let's see if u42's qspan positions are all in u3's qspan positions
        if let (Some(u3_positions), Some(u42_positions)) =
            (&u3.qspan_positions, &u42.qspan_positions)
        {
            let u3_set: std::collections::HashSet<usize> = u3_positions.iter().copied().collect();
            let all_u42_in_u3 = u42_positions.iter().all(|p| u3_set.contains(p));
            println!(
                "\n  All u42 qspan positions in u3 qspan set: {}",
                all_u42_in_u3
            );

            // Check if u42 positions are missing from u3
            let missing: Vec<_> = u42_positions
                .iter()
                .filter(|p| !u3_set.contains(p))
                .copied()
                .collect();
            if !missing.is_empty() {
                println!(
                    "  u42 positions NOT in u3 (first 10): {:?}",
                    &missing[..missing.len().min(10)]
                );
            }
        }
    }
}
