use scancode_rust::license_detection::query::PositionSpan;
use scancode_rust::license_detection::{
    aho_match, compute_candidates_with_msets, merge_overlapping_matches, refine_aho_matches,
    seq_match_with_candidates, LicenseDetectionEngine,
};
use std::path::PathBuf;

const MAX_SEQ_CANDIDATES: usize = 70;

fn main() {
    let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    let engine = LicenseDetectionEngine::new(&data_path).expect("Failed to create engine");
    let index = engine.index();

    let text =
        std::fs::read_to_string("testdata/license-golden/datadriven/lic1/flex-readme.txt").unwrap();
    let query = scancode_rust::license_detection::query::Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    println!("Total tokens: {}", query.tokens.len());
    println!("Matchables: {}", whole_run.matchables(false).len());
    println!(
        "Matchable positions (first 20): {:?}",
        whole_run
            .matchables(false)
            .iter()
            .take(20)
            .copied()
            .collect::<Vec<_>>()
    );

    // Phase 1: Aho matching
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    println!("\nAho matches: {}", refined_aho.len());
    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    for m in &refined_aho {
        println!(
            "  - {} (lines {}-{}, tokens {}-{}, coverage {:.1}%)",
            m.rule_identifier,
            m.start_line,
            m.end_line,
            m.start_token,
            m.end_token,
            m.match_coverage
        );
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    println!("\nMatched qspans: {:?}", matched_qspans);

    // Check is_matchable
    let is_matchable = whole_run.is_matchable(false, &matched_qspans);
    println!("\nis_matchable after Aho: {}", is_matchable);
    println!("skip_seq_matching would be: {}", !is_matchable);

    // What are the matchable positions NOT covered by matched_qspans?
    let matchables = whole_run.matchables(false);
    let mut uncovered: Vec<_> = matchables.iter().copied().collect();
    for span in &matched_qspans {
        let span_positions = span.positions();
        uncovered.retain(|p| !span_positions.contains(p));
    }
    println!(
        "\nUncovered matchable positions: {} (first 20: {:?})",
        uncovered.len(),
        uncovered.iter().take(20).copied().collect::<Vec<_>>()
    );

    // Show what lines the uncovered positions are on
    println!("\nLines of uncovered matchable positions (first 10):");
    for pos in uncovered.iter().take(10) {
        let line = query.line_by_pos.get(*pos).copied().unwrap_or(0);
        println!("  Position {} is on line {}", pos, line);
    }

    // Check regular candidates
    let regular_candidates =
        compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    println!(
        "\nRegular sequence candidates: {}",
        regular_candidates.len()
    );
    for (i, c) in regular_candidates.iter().take(5).enumerate() {
        println!(
            "  {}. {} (license: {}, score: {:.4})",
            i + 1,
            c.rule.identifier,
            c.rule.license_expression,
            c.score_vec_rounded.resemblance
        );
    }

    // Run sequence matching if candidates found
    if !regular_candidates.is_empty() {
        let seq_matches = seq_match_with_candidates(index, &whole_run, &regular_candidates);
        println!("\nSequence matches: {}", seq_matches.len());
        for m in seq_matches.iter() {
            println!(
                "  - {} (lines {}-{}, score {:.2})",
                m.rule_identifier, m.start_line, m.end_line, m.score
            );
        }

        // Now merge all matches and see containment
        let mut all_matches = refined_aho.clone();
        let merged_seq = merge_overlapping_matches(&seq_matches);
        all_matches.extend(merged_seq);

        println!("\nAll matches before merge: {}", all_matches.len());
        for m in all_matches.iter().take(20) {
            println!(
                "  - {} (lines {}-{}, qstart={}, qend={}, matcher={}, coverage={:.1}%)",
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matcher,
                m.match_coverage
            );
        }

        let merged = merge_overlapping_matches(&all_matches);

        println!("\nAfter merge: {} matches", merged.len());
        for m in &merged {
            println!(
                "  - {} (lines {}-{}, qstart={}, qend={}, matcher={})",
                m.rule_identifier, m.start_line, m.end_line, m.start_token, m.end_token, m.matcher
            );
        }
    }
}
