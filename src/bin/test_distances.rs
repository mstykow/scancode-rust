//! Check distances between seq matches for unicode_3

use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets,
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

    let query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    // Get seq matches
    let near_dupe_candidates =
        compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
    let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);

    const MAX_SEQ_CANDIDATES: usize = 70;
    let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);

    let mut all_seq = Vec::new();
    all_seq.extend(near_dupe_matches);
    all_seq.extend(seq_matches);

    // Get unicode_3 matches that will be merged
    let mut u3_matches: Vec<_> = all_seq
        .iter()
        .filter(|m| m.rule_identifier == "unicode_3.RULE")
        .collect();

    // Sort by qstart
    u3_matches.sort_by_key(|m| m.qstart());

    println!("=== unicode_3.RULE matches (sorted by qstart) ===");
    for (i, m) in u3_matches.iter().enumerate() {
        println!(
            "[{}] qstart={}, qend={}, istart={}, iend={}",
            i,
            m.qstart(),
            m.end_token,
            m.rule_start_token,
            m.rule_start_token + m.matched_length
        );
    }

    // Check distances between consecutive matches
    println!("\n=== Distances between consecutive matches ===");
    let rule_length = 496; // Approximate
    let max_rule_side_dist = (rule_length / 2).clamp(1, 100);
    println!(
        "rule_length={}, max_rule_side_dist={}",
        rule_length, max_rule_side_dist
    );

    for i in 0..u3_matches.len().saturating_sub(1) {
        let current = u3_matches[i];
        let next = u3_matches[i + 1];

        let qdist = next.qstart().saturating_sub(current.end_token);
        let idist = next
            .rule_start_token
            .saturating_sub(current.rule_start_token + current.matched_length);

        println!(
            "[{}->{}] qdist={}, idist={}, within_limit={}",
            i,
            i + 1,
            qdist,
            idist,
            qdist <= max_rule_side_dist && idist <= max_rule_side_dist
        );
    }

    // Check if matches at 985 and 1122 would be merged
    let m_985 = u3_matches.iter().find(|m| m.qstart() == 985);
    let m_1122 = u3_matches.iter().find(|m| m.qstart() == 1122);

    if let (Some(m1), Some(m2)) = (m_985, m_1122) {
        let qdist = m2.qstart().saturating_sub(m1.end_token);
        let idist = m2
            .rule_start_token
            .saturating_sub(m1.rule_start_token + m1.matched_length);
        println!("\n=== Distance from 985 match to 1122 match ===");
        println!("qdist={}, idist={}", qdist, idist);
        println!(
            "Would merge: {}",
            qdist <= max_rule_side_dist && idist <= max_rule_side_dist
        );
    }
}
