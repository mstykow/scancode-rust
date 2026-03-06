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

    // Find ALL unicode_42 matches
    println!("=== ALL unicode_42 MATCHES ===");
    for (i, m) in all_matches.iter().enumerate() {
        if m.rule_identifier == "unicode_42.RULE" {
            println!(
                "[{}] qstart={}, end_token={}, matcher={}, hilen={}",
                i,
                m.qstart(),
                m.end_token,
                m.matcher,
                m.hilen
            );
        }
    }

    // Find ALL matches between positions 190-230 that might affect the logic
    let mut sorted: Vec<_> = all_matches.iter().collect();
    sorted.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    println!("\n=== MATCHES AT POSITIONS 220-230 ===");
    for (i, m) in sorted.iter().enumerate() {
        if i >= 220 && i <= 230 {
            println!(
                "[{}] {} (qstart={}, end_token={}, hilen={}, matcher_order={})",
                i,
                m.rule_identifier,
                m.qstart(),
                m.end_token,
                m.hilen,
                m.matcher_order()
            );
        }
    }

    // Check if unicode_42 (seq, position 225) might be contained by unicode_3
    let u3 = sorted
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token == 985)
        .unwrap();
    let u42_seq = sorted
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE" && m.matcher == "3-seq")
        .unwrap();

    println!("\n=== unicode_42 (seq) CHECK ===");
    println!(
        "unicode_42 (seq): qstart={}, end_token={}",
        u42_seq.qstart(),
        u42_seq.end_token
    );
    println!(
        "unicode_3.qcontains(unicode_42_seq): {}",
        u3.qcontains(u42_seq)
    );
}
