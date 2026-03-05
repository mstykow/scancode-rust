//! Trace exact merge path

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

    // Get unicode_3 matches
    let mut u3_matches: Vec<_> = all_seq
        .iter()
        .filter(|m| m.rule_identifier == "unicode_3.RULE")
        .cloned()
        .collect();

    // Sort like merge does
    u3_matches.sort_by(|a, b| {
        a.rule_identifier
            .cmp(&b.rule_identifier)
            .then_with(|| a.qstart().cmp(&b.qstart()))
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    // Find matches at indices that would merge
    // Looking for matches that would create the 985-1468 merged match
    let m_985 = u3_matches.iter().find(|m| m.qstart() == 985);
    let m_1021 = u3_matches.iter().find(|m| m.qstart() == 1021);
    let m_1122 = u3_matches.iter().find(|m| m.qstart() == 1122);

    if let (Some(m1), Some(m2), Some(m3)) = (m_985, m_1021, m_1122) {
        println!("=== Three matches that merge ===");
        println!("m_985: qstart={}, qend={}", m1.qstart(), m1.end_token);
        println!("m_1021: qstart={}, qend={}", m2.qstart(), m2.end_token);
        println!("m_1122: qstart={}, qend={}", m3.qstart(), m3.end_token);

        // Check merge conditions
        println!("\n=== m_985 vs m_1021 ===");
        println!("qdist: {}", m2.qstart().saturating_sub(m1.end_token));
        println!("m1.surround(m2): {}", m1.surround(m2));
        println!("m2.is_after(m1): {}", m2.is_after(m1));

        // Check overlap
        let cur_qstart = m1.qstart();
        let cur_qend = m1.end_token;
        let next_qstart = m2.qstart();
        let next_qend = m2.end_token;
        let cur_istart = m1.rule_start_token;
        let cur_iend = m1.rule_start_token + m1.matched_length;
        let next_istart = m2.rule_start_token;
        let next_iend = m2.rule_start_token + m2.matched_length;

        println!(
            "cur_qstart <= next_qstart: {} <= {} = {}",
            cur_qstart,
            next_qstart,
            cur_qstart <= next_qstart
        );
        println!(
            "cur_qend <= next_qend: {} <= {} = {}",
            cur_qend,
            next_qend,
            cur_qend <= next_qend
        );
        println!(
            "cur_istart <= next_istart: {} <= {} = {}",
            cur_istart,
            next_istart,
            cur_istart <= next_istart
        );
        println!(
            "cur_iend <= next_iend: {} <= {} = {}",
            cur_iend,
            next_iend,
            cur_iend <= next_iend
        );

        println!("\n=== m_1021 vs m_1122 ===");
        println!("qdist: {}", m3.qstart().saturating_sub(m2.end_token));
        println!("m2.surround(m3): {}", m2.surround(m3));
        println!("m3.is_after(m2): {}", m3.is_after(m2));
    }
}
