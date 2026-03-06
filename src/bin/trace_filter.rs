use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    filter_contained_matches, merge_overlapping_matches, refine_aho_matches,
    seq_match_with_candidates,
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

    // AHO matching
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    // Build matched_qspans
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

    let mut seq_all_matches = Vec::new();
    if !skip_seq_matching {
        // Phase 2-4 sequence matching (simplified)
        let whole_run = query.whole_query_run();
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);
            seq_all_matches.extend(near_dupe_matches);
        }
        // Skip phases 3-4 for brevity
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        seq_all_matches = merged_seq;
    }

    // Combine all matches
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(seq_all_matches.clone());

    println!("=== KEY MATCHES (after merge_overlapping_matches) ===");

    // Find the key matches
    let unicode_3 = all_matches
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token == 985);
    let unicode_40 = all_matches
        .iter()
        .find(|m| m.rule_identifier == "unicode_40.RULE");
    let unicode_42 = all_matches
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE" && m.matcher == "2-aho");

    if let Some(m) = unicode_3 {
        println!(
            "unicode_3 (seq): qstart={}, end_token={}, hilen={}, matched_len={}, matcher_order={}",
            m.qstart(),
            m.end_token,
            m.hilen,
            m.matched_length,
            m.matcher_order()
        );
    }
    if let Some(m) = unicode_40 {
        println!(
            "unicode_40 (aho): qstart={}, end_token={}, hilen={}, matched_len={}, matcher_order={}",
            m.qstart(),
            m.end_token,
            m.hilen,
            m.matched_length,
            m.matcher_order()
        );
    }
    if let Some(m) = unicode_42 {
        println!(
            "unicode_42 (aho): qstart={}, end_token={}, hilen={}, matched_len={}, matcher_order={}",
            m.qstart(),
            m.end_token,
            m.hilen,
            m.matched_length,
            m.matcher_order()
        );
    }

    // Run filter_contained_matches
    println!("\n=== filter_contained_matches ===");
    let (kept, discarded) = filter_contained_matches(&all_matches);
    println!("Kept: {}", kept.len());
    println!("Discarded: {}", discarded.len());

    println!("\n=== KEPT MATCHES ===");
    for m in &kept {
        println!(
            "  {} (license: {}, qstart={}, end_token={}, matcher={})",
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token,
            m.matcher
        );
    }

    println!("\n=== DISCARDED MATCHES (first 10) ===");
    for m in discarded.iter().take(10) {
        println!(
            "  {} (license: {}, qstart={}, end_token={}, matcher={})",
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token,
            m.matcher
        );
    }
}
