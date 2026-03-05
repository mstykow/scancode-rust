//! Check if the unicode_3 985-1468 match would be created in Python

use scancode_rust::license_detection::models::LicenseMatch;
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

    // Get seq matches for unicode_3
    let near_dupe_candidates =
        compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
    let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);

    const MAX_SEQ_CANDIDATES: usize = 70;
    let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
    let seq_matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);

    let mut all_seq: Vec<LicenseMatch> = Vec::new();
    all_seq.extend(near_dupe_matches);
    all_seq.extend(seq_matches);

    // Get unicode_3 matches BEFORE merge
    let mut u3_before: Vec<_> = all_seq
        .iter()
        .filter(|m| m.rule_identifier == "unicode_3.RULE")
        .cloned()
        .collect();

    // Sort like merge does
    u3_before.sort_by(|a, b| {
        a.rule_identifier
            .cmp(&b.rule_identifier)
            .then_with(|| a.qstart().cmp(&b.qstart()))
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    println!("=== unicode_3 matches BEFORE merge (sorted, qstart >= 900) ===");
    for (i, m) in u3_before
        .iter()
        .enumerate()
        .filter(|(_, m)| m.qstart() >= 900)
    {
        println!(
            "[{}] qstart={}, qend={}, istart={}, iend={}",
            i,
            m.qstart(),
            m.end_token,
            m.rule_start_token,
            m.rule_start_token + m.matched_length
        );
    }

    // Find the matches that would merge to create 985-1468
    let m_985 = u3_before
        .iter()
        .enumerate()
        .find(|(_, m)| m.qstart() == 985);
    let m_1021 = u3_before
        .iter()
        .enumerate()
        .find(|(_, m)| m.qstart() == 1021);
    let m_1122 = u3_before
        .iter()
        .enumerate()
        .find(|(_, m)| m.qstart() == 1122);

    if let (Some((i1, m1)), Some((i2, m2)), Some((i3, m3))) = (m_985, m_1021, m_1122) {
        println!("\n=== Checking merge of matches ===");
        println!("[{}] qstart={}, qend={}", i1, m1.qstart(), m1.end_token);
        println!("[{}] qstart={}, qend={}", i2, m2.qstart(), m2.end_token);
        println!("[{}] qstart={}, qend={}", i3, m3.qstart(), m3.end_token);

        // Check is_after
        println!("\nm_1021.is_after(m_985): {}", m2.is_after(m1));
        println!("m_1122.is_after(m_1021): {}", m3.is_after(m2));

        // Check distance
        let rule_length = 496;
        let max_dist = (rule_length / 2).clamp(1, 100);
        let qdist_12 = m2.qstart().saturating_sub(m1.end_token);
        let qdist_23 = m3.qstart().saturating_sub(m2.end_token);
        let idist_12 = m2
            .rule_start_token
            .saturating_sub(m1.rule_start_token + m1.matched_length);
        let idist_23 = m3
            .rule_start_token
            .saturating_sub(m2.rule_start_token + m2.matched_length);

        println!(
            "\nDistance m_985->m_1021: qdist={}, idist={}",
            qdist_12, idist_12
        );
        println!(
            "Distance m_1021->m_1122: qdist={}, idist={}",
            qdist_23, idist_23
        );
        println!("max_dist={}", max_dist);

        // Python breaks if distance > max_dist
        println!(
            "\nPython would break at m_985->m_1021: {}",
            qdist_12 > max_dist || idist_12 > max_dist
        );
        println!(
            "Python would break at m_1021->m_1122: {}",
            qdist_23 > max_dist || idist_23 > max_dist
        );

        // If Python would break, then the 985-1021 match would NOT merge with 1122-1446
        // and we'd have two separate unicode matches instead of one big one
    }
}
