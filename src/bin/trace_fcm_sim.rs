use scancode_rust::license_detection::models::LicenseMatch;
use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, seq_match_with_candidates,
};
use std::path::PathBuf;

fn my_filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.len() < 2 {
        return (matches.to_vec(), Vec::new());
    }

    let mut matches: Vec<LicenseMatch> = matches.to_vec();
    let mut discarded = Vec::new();

    matches.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    println!("=== SIMULATING filter_contained_matches ===");
    println!("Initial matches: {}", matches.len());

    // Find key match positions
    for (i, m) in matches.iter().enumerate() {
        if m.rule_identifier == "unicode_3.RULE" && m.start_token == 985 {
            println!("[{}] unicode_3 (seq, qstart=985)", i);
        }
        if m.rule_identifier == "unicode_40.RULE" {
            println!("[{}] unicode_40 (aho)", i);
        }
        if m.rule_identifier == "unicode_42.RULE" {
            println!("[{}] {} (qstart={})", i, m.rule_identifier, m.qstart());
        }
    }

    let mut i = 0;
    while i < matches.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < matches.len() {
            let current = matches[i].clone();
            let next = matches[j].clone();

            if next.end_token > current.end_token {
                break;
            }

            if current.qspan_eq(&next) {
                println!("\n[AT i={}, j={}] qspan_eq MATCH:", i, j);
                println!(
                    "  current: {} (coverage={})",
                    current.rule_identifier, current.match_coverage
                );
                println!(
                    "  next: {} (coverage={})",
                    next.rule_identifier, next.match_coverage
                );

                if current.match_coverage >= next.match_coverage {
                    println!("  -> DISCARD next (lower or equal coverage)");
                    discarded.push(matches.remove(j));
                    continue;
                } else {
                    println!("  -> DISCARD current (lower coverage)");
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
            }

            if current.qcontains(&next) {
                println!("\n[AT i={}, j={}] qcontains MATCH:", i, j);
                println!(
                    "  current: {} qcontains next: {}",
                    current.rule_identifier, next.rule_identifier
                );
                println!("  -> DISCARD next");
                discarded.push(matches.remove(j));
                continue;
            }
            if next.qcontains(&current) {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            j += 1;
        }
        i += 1;
    }

    (matches, discarded)
}

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

    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

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
        let whole_run = query.whole_query_run();
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);
            seq_all_matches.extend(near_dupe_matches);
        }
        let whole_run = query.whole_query_run();
        let candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);
            seq_all_matches.extend(matches);
        }
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
        seq_all_matches = merged_seq;
    }

    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(seq_all_matches.clone());

    let (kept, discarded) = my_filter_contained_matches(&all_matches);

    println!("\n=== RESULT ===");
    println!("Kept: {}", kept.len());
    println!("Discarded: {}", discarded.len());

    println!("\nKept matches:");
    for m in &kept {
        println!(
            "  {} (license: {}, qstart={}, end_token={})",
            m.rule_identifier,
            m.license_expression,
            m.qstart(),
            m.end_token
        );
    }
}
