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

    let query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    // AHO matching
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    println!("=== AHO MATCHES ({}) ===", refined_aho.len());
    for m in &refined_aho {
        println!(
            "  {} (license: {}, qstart={}, end_token={}, matcher={})",
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token,
            m.matcher
        );
    }

    // Build matched_qspans like detect_matches does
    let mut matched_qspans = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(scancode_rust::license_detection::query::PositionSpan::new(
                m.start_token,
                m.end_token - 1,
            ));
        }
    }

    println!("\nmatched_qspans count: {}", matched_qspans.len());

    // Check if seq matching is skipped
    let whole_run = query.whole_query_run();
    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("\nskip_seq_matching = {}", skip_seq_matching);

    // Sequence matching
    let mut seq_all_matches = Vec::new();
    if !skip_seq_matching {
        let candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        println!("\n=== SEQUENCE CANDIDATES ({}) ===", candidates.len());
        if !candidates.is_empty() {
            let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);
            let merged_seq = merge_overlapping_matches(&seq_matches);
            println!("=== MERGED SEQUENCE MATCHES ({}) ===", merged_seq.len());
            for m in merged_seq.iter().take(5) {
                println!(
                    "  {} (license: {}, qstart={}, end_token={}, matcher={})",
                    m.rule_identifier,
                    m.license_expression,
                    m.qstart(),
                    m.end_token,
                    m.matcher
                );
            }
            seq_all_matches = merged_seq;
        }
    }

    // Combine all matches
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(seq_all_matches.clone());
    println!("\n=== ALL MATCHES ({}) ===", all_matches.len());

    // Step 1: refine without FP filter
    let merged_matches = refine_matches_without_false_positive_filter(index, all_matches, &query);
    println!(
        "=== AFTER refine_matches_without_false_positive_filter ({}) ===",
        merged_matches.len()
    );
    for m in &merged_matches {
        println!(
            "  {} (license: {}, qstart={}, end_token={}, matcher={})",
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token,
            m.matcher
        );
    }

    // Step 2: refine with FP filter
    let refined = refine_matches(index, merged_matches, &query);
    println!("\n=== AFTER refine_matches ({}) ===", refined.len());
    for m in &refined {
        println!(
            "  {} (license: {}, qstart={}, end_token={}, matcher={})",
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token,
            m.matcher
        );
    }

    let mut sorted = refined;
    sort_matches_by_line(&mut sorted);

    println!("\n=== FINAL RESULT ({}) ===", sorted.len());
    for m in &sorted {
        println!(
            "  {} (license: {})",
            m.rule_identifier, m.license_expression
        );
    }
}
