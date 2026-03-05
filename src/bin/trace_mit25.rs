use scancode_rust::license_detection::query::PositionSpan;
use scancode_rust::license_detection::{LicenseDetector, Query};

fn main() {
    let detector = LicenseDetector::new().expect("Failed to create detector");

    let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic3/mit_25.txt")
        .expect("Failed to read mit_25.txt");

    println!("=== PHASE BY PHASE TRACE FOR mit_25.txt ===\n");

    let index = &detector.index;
    let mut query = Query::new(&text, index).expect("Failed to create query");

    let mut matched_qspans: Vec<PositionSpan> = Vec::new();
    let mut all_matches = Vec::new();

    // Phase 1b: SPDX-LID
    println!("=== Phase 1b: SPDX-LID ===");
    let spdx_matches = scancode_rust::license_detection::phases::spdx_lid_match(index, &query);
    println!("SPDX-LID matches: {}", spdx_matches.len());
    for m in &spdx_matches {
        println!(
            "  {} tokens={}-{} coverage={:.2}%",
            m.license_expression, m.start_token, m.end_token, m.match_coverage
        );
    }

    // Track 100% matches
    for m in &spdx_matches {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }
    println!("matched_qspans after SPDX-LID: {}", matched_qspans.len());

    all_matches.extend(spdx_matches);

    // Phase 1c: AHO
    println!("\n=== Phase 1c: AHO ===");
    let whole_run = query.whole_query_run();
    let aho_matches = scancode_rust::license_detection::phases::aho_match(index, &whole_run);
    println!("AHO matches before refine: {}", aho_matches.len());

    let refined_aho = scancode_rust::license_detection::match_refine::refine_aho_matches(
        index,
        aho_matches,
        &query,
    );
    println!("AHO matches after refine: {}", refined_aho.len());
    for m in &refined_aho {
        println!(
            "  {} tokens={}-{} coverage={:.2}%",
            m.license_expression, m.start_token, m.end_token, m.match_coverage
        );
    }

    // Track 100% AHO matches
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }
    println!("matched_qspans after AHO: {}", matched_qspans.len());

    all_matches.extend(refined_aho);

    // Check skip_seq_matching
    let whole_run = query.whole_query_run();
    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("\n=== Skip sequence matching? {} ===", skip_seq_matching);
    println!("matched_qspans: {:?}", matched_qspans);

    if !skip_seq_matching {
        let mut seq_all_matches = Vec::new();

        // Phase 2: Near-dupe
        println!("\n=== Phase 2: Near-Dupe ===");
        let whole_run = query.whole_query_run();
        let near_dupe_candidates =
            scancode_rust::license_detection::phases::compute_candidates_with_msets(
                index, &whole_run, true, 10,
            );
        println!("Near-dupe candidates: {}", near_dupe_candidates.len());

        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                scancode_rust::license_detection::phases::seq_match_with_candidates(
                    index,
                    &whole_run,
                    &near_dupe_candidates,
                );
            println!("Near-dupe matches: {}", near_dupe_matches.len());
            for m in &near_dupe_matches {
                println!(
                    "  {} tokens={}-{} coverage={:.2}%",
                    m.license_expression, m.start_token, m.end_token, m.match_coverage
                );
            }

            for m in &near_dupe_matches {
                if m.end_token > m.start_token {
                    let span = PositionSpan::new(m.start_token, m.end_token - 1);
                    query.subtract(&span);
                    matched_qspans.push(span);
                }
            }

            seq_all_matches.extend(near_dupe_matches);
        }

        // Phase 3: Regular seq matching
        println!("\n=== Phase 3: Regular Sequence ===");
        let whole_run = query.whole_query_run();
        let candidates = scancode_rust::license_detection::phases::compute_candidates_with_msets(
            index, &whole_run, false, 70,
        );
        println!("Regular seq candidates: {}", candidates.len());

        if !candidates.is_empty() {
            let matches = scancode_rust::license_detection::phases::seq_match_with_candidates(
                index,
                &whole_run,
                &candidates,
            );
            println!("Regular seq matches: {}", matches.len());
            for m in &matches {
                println!(
                    "  {} tokens={}-{} coverage={:.2}%",
                    m.license_expression, m.start_token, m.end_token, m.match_coverage
                );
            }

            // Track Phase 3 matches
            for m in &matches {
                if m.end_token > m.start_token {
                    let span = PositionSpan::new(m.start_token, m.end_token - 1);
                    query.subtract(&span);
                    matched_qspans.push(span);
                }
            }

            seq_all_matches.extend(matches);
        }

        println!("matched_qspans after Phase 3: {}", matched_qspans.len());

        // Phase 4: Query run matching
        println!("\n=== Phase 4: Query Runs ===");
        let whole_run = query.whole_query_run();
        let query_runs = query.query_runs();
        println!("Total query runs: {}", query_runs.len());
        for (i, qr) in query_runs.iter().enumerate() {
            println!("  Query run {}: tokens {}-{}", i, qr.start, qr.end);
        }

        for query_run in query_runs.iter() {
            if query_run.start == whole_run.start && query_run.end == whole_run.end {
                println!("  Skipping query run same as whole_run");
                continue;
            }

            let is_matchable = query_run.is_matchable(false, &matched_qspans);
            println!(
                "  Query run {}-{} is_matchable: {}",
                query_run.start, query_run.end, is_matchable
            );

            if !is_matchable {
                continue;
            }

            let candidates =
                scancode_rust::license_detection::phases::compute_candidates_with_msets(
                    index, query_run, false, 70,
                );

            if !candidates.is_empty() {
                let matches = scancode_rust::license_detection::phases::seq_match_with_candidates(
                    index,
                    query_run,
                    &candidates,
                );
                println!(
                    "    Query run {}-{} matches: {}",
                    query_run.start,
                    query_run.end,
                    matches.len()
                );
                for m in &matches {
                    println!(
                        "      {} tokens={}-{} coverage={:.2}%",
                        m.license_expression, m.start_token, m.end_token, m.match_coverage
                    );
                }

                // CRITICAL: Are we tracking these?
                println!(
                    "    ***TRACKING? matched_qspans BEFORE: {}",
                    matched_qspans.len()
                );
                // NOTE: In the current code, Phase 4 matches are NOT added to matched_qspans!
                // This is the BUG!

                seq_all_matches.extend(matches);
            }
        }

        // Merge all sequence matches
        let merged_seq = scancode_rust::license_detection::match_refine::merge_overlapping_matches(
            &seq_all_matches,
        );
        println!("\n=== Merged Sequence Matches: {} ===", merged_seq.len());
        for m in &merged_seq {
            println!(
                "  {} tokens={}-{} coverage={:.2}%",
                m.license_expression, m.start_token, m.end_token, m.match_coverage
            );
        }

        all_matches.extend(merged_seq);
    }

    println!("\n=== FINAL ALL MATCHES: {} ===", all_matches.len());
    for m in &all_matches {
        println!(
            "  {} tokens={}-{} coverage={:.2}%",
            m.license_expression, m.start_token, m.end_token, m.match_coverage
        );
    }
}
