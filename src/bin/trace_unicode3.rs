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

    // AHO matching
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

    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);

    if !skip_seq_matching {
        let whole_run = query.whole_query_run();
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);

        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);

            println!("=== BEFORE merge_overlapping_matches ===");
            println!("Total sequence matches: {}", near_dupe_matches.len());

            // Find all unicode_3 matches
            let unicode_3_matches: Vec<_> = near_dupe_matches
                .iter()
                .filter(|m| m.rule_identifier == "unicode_3.RULE")
                .collect();
            println!("unicode_3.RULE matches: {}", unicode_3_matches.len());
            for m in &unicode_3_matches {
                println!("  qstart={}, end_token={}", m.qstart(), m.end_token);
            }

            // Merge
            let merged_seq = merge_overlapping_matches(&near_dupe_matches);

            println!("\n=== AFTER merge_overlapping_matches ===");
            println!("Total merged matches: {}", merged_seq.len());

            // Find all unicode_3 matches after merge
            let unicode_3_merged: Vec<_> = merged_seq
                .iter()
                .filter(|m| m.rule_identifier == "unicode_3.RULE")
                .collect();
            println!("unicode_3.RULE matches: {}", unicode_3_merged.len());
            for m in &unicode_3_merged {
                println!("  qstart={}, end_token={}", m.qstart(), m.end_token);
            }

            // Check if unicode_3 with qstart=985 exists
            let u3_985 = merged_seq
                .iter()
                .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token == 985);
            if let Some(m) = u3_985 {
                println!("\nunicode_3 (qstart=985) EXISTS after merge:");
                println!(
                    "  end_token={}, hilen={}, matched_len={}",
                    m.end_token, m.hilen, m.matched_length
                );
            } else {
                println!("\nunicode_3 (qstart=985) DOES NOT EXIST after merge!");

                // Find matches that might have absorbed it
                println!("\nMatches at qstart=985:");
                for m in merged_seq.iter().filter(|m| m.qstart() == 985) {
                    println!(
                        "  {} (end_token={}, hilen={}, matched_len={})",
                        m.rule_identifier, m.end_token, m.hilen, m.matched_length
                    );
                }
            }
        }
    }
}
