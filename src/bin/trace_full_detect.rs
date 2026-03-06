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

    let mut query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    // AHO matching (Phase 1c)
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

    let whole_run = query.whole_query_run();
    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("skip_seq_matching = {}", skip_seq_matching);

    let mut seq_all_matches = Vec::new();
    if !skip_seq_matching {
        // Phase 2: Near-duplicate detection
        let whole_run = query.whole_query_run();
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);

        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);

            for m in &near_dupe_matches {
                if m.end_token > m.start_token {
                    let span = scancode_rust::license_detection::query::PositionSpan::new(
                        m.start_token,
                        m.end_token - 1,
                    );
                    if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                        query.subtract(&span);
                    }
                    matched_qspans.push(span);
                }
            }
            seq_all_matches.extend(near_dupe_matches);
        }

        // Phase 3: Regular sequence matching
        let whole_run = query.whole_query_run();
        let candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);
            seq_all_matches.extend(matches);
        }

        // Phase 4: Query run matching
        let whole_run = query.whole_query_run();
        for query_run in query.query_runs().iter() {
            if query_run.start == whole_run.start && query_run.end == whole_run.end {
                continue;
            }
            if !query_run.is_matchable(false, &matched_qspans) {
                continue;
            }
            let candidates = compute_candidates_with_msets(index, query_run, false, 70);
            if !candidates.is_empty() {
                let matches = seq_match_with_candidates(index, query_run, &candidates, &[]);
                seq_all_matches.extend(matches);
            }
        }

        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        println!("Merged sequence matches: {}", merged_seq.len());
        seq_all_matches = merged_seq;
    }

    // Combine all matches
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(seq_all_matches.clone());
    println!("Total matches before refinement: {}", all_matches.len());

    // Step 1: refine without FP filter
    let merged_matches = refine_matches_without_false_positive_filter(index, all_matches, &query);
    println!(
        "\nAfter refine_matches_without_false_positive_filter: {}",
        merged_matches.len()
    );

    // Step 2: refine with FP filter
    let refined = refine_matches(index, merged_matches, &query);
    println!("\nAfter refine_matches: {}", refined.len());

    let mut sorted = refined;
    sort_matches_by_line(&mut sorted);

    println!("\n=== FINAL RESULT ({}) ===", sorted.len());
    for m in &sorted {
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
