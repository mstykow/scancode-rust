use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, refine_matches,
    refine_matches_without_false_positive_filter, seq_match_with_candidates, sort_matches_by_line,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("testdata/license-golden/datadriven/external/glc/CC-BY-SA-1.0.t1")
    };

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
            "  {} (license: {}, lines {}-{}, coverage {:.2}%, score {:.2})",
            m.rule_identifier,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.match_coverage,
            m.score
        );
    }

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

    println!("\nmatched_qspans count: {}", matched_qspans.len());

    // Check if seq matching is skipped
    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("skip_seq_matching = {}", skip_seq_matching);

    // Sequence matching
    let mut seq_all_matches = Vec::new();
    if !skip_seq_matching {
        let candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        println!("\n=== SEQUENCE CANDIDATES ({}) ===", candidates.len());
        for c in candidates.iter().take(10) {
            println!(
                "  {} (rounded: is_high={}, cont={:.1}, resembl={:.1}, len={:.1})",
                c.rule.license_expression,
                c.score_vec_rounded.is_highly_resemblant,
                c.score_vec_rounded.containment,
                c.score_vec_rounded.resemblance,
                c.score_vec_rounded.matched_length
            );
            println!(
                "           (full:    is_high={}, cont={:.3}, resembl={:.3}, len={:.0})",
                c.score_vec_full.is_highly_resemblant,
                c.score_vec_full.containment,
                c.score_vec_full.resemblance,
                c.score_vec_full.matched_length
            );
        }

        if !candidates.is_empty() {
            let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates);
            println!("\n=== RAW SEQUENCE MATCHES ({}) ===", seq_matches.len());
            for m in seq_matches.iter().take(10) {
                println!(
                    "  {} (license: {}, lines {}-{}, coverage {:.2}%, score {:.2})",
                    m.rule_identifier,
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.match_coverage,
                    m.score
                );
            }

            let merged_seq = merge_overlapping_matches(&seq_matches);
            println!("\n=== MERGED SEQUENCE MATCHES ({}) ===", merged_seq.len());
            for m in merged_seq.iter().take(10) {
                println!(
                    "  {} (license: {}, lines {}-{}, coverage {:.2}%, score {:.2})",
                    m.rule_identifier,
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.match_coverage,
                    m.score
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
        "\n=== AFTER refine_matches_without_false_positive_filter ({}) ===",
        merged_matches.len()
    );
    for m in &merged_matches {
        println!(
            "  {} (license: {}, lines {}-{}, coverage {:.2}%, score {:.2})",
            m.rule_identifier,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.match_coverage,
            m.score
        );
    }

    // Step 2: refine with FP filter
    let refined = refine_matches(index, merged_matches, &query);
    println!("\n=== AFTER refine_matches ({}) ===", refined.len());
    for m in &refined {
        println!(
            "  {} (license: {}, lines {}-{}, coverage {:.2}%, score {:.2})",
            m.rule_identifier,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.match_coverage,
            m.score
        );
    }

    let mut sorted = refined;
    sort_matches_by_line(&mut sorted);

    println!("\n=== FINAL RESULT ({}) ===", sorted.len());
    for m in &sorted {
        println!(
            "  {} (license: {}, lines {}-{}, coverage {:.2}%, score {:.2})",
            m.rule_identifier,
            m.license_expression,
            m.start_line,
            m.end_line,
            m.match_coverage,
            m.score
        );
    }
}
