use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, seq_match_with_candidates,
};
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("testdata/license-golden/datadriven/external/fossology-licenses/unicode.txt");
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
            matched_qspans.push(scancode_rust::license_detection::query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);

    let mut seq_all_matches = Vec::new();
    if !skip_seq_matching {
        let whole_run = query.whole_query_run();
        let near_dupe_candidates = compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);
            seq_all_matches.extend(near_dupe_matches);
        }
        let whole_run = query.whole_query_run();
        let candidates = compute_candidates_with_msets(index, &whole_run, false, 70);
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(index, &whole_run, &candidates);
            seq_all_matches.extend(matches);
        }
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        seq_all_matches = merged_seq;
    }

    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(seq_all_matches.clone());

    // Get the key matches
    let unicode_3 = all_matches.iter().find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token == 985).unwrap();
    let unicode_40 = all_matches.iter().find(|m| m.rule_identifier == "unicode_40.RULE").unwrap();
    let unicode_42 = all_matches.iter().find(|m| m.rule_identifier == "unicode_42.RULE" && m.matcher == "2-aho").unwrap();

    println!("=== SORTING ANALYSIS ===");
    println!("unicode_3 (seq): sort tuple = ({}, {}, {}, {})", 
        unicode_3.qstart(), unicode_3.hilen, unicode_3.matched_length, unicode_3.matcher_order());
    println!("unicode_40 (aho): sort tuple = ({}, {}, {}, {})", 
        unicode_40.qstart(), unicode_40.hilen, unicode_40.matched_length, unicode_40.matcher_order());
    println!("unicode_42 (aho): sort tuple = ({}, {}, {}, {})", 
        unicode_42.qstart(), unicode_42.hilen, unicode_42.matched_length, unicode_42.matcher_order());
    
    println!("\n=== CONTAINMENT CHECK ===");
    println!("unicode_3.qcontains(unicode_40): {}", unicode_3.qcontains(unicode_40));
    println!("unicode_3.qcontains(unicode_42): {}", unicode_3.qcontains(unicode_42));
    
    println!("\n=== QSPAN DETAILS ===");
    let u3_qspan: Vec<usize> = unicode_3.qspan();
    let u40_qspan: Vec<usize> = unicode_40.qspan();
    let u42_qspan: Vec<usize> = unicode_42.qspan();
    
    // Check if unicode_3's qspan contains unicode_40's qspan
    let u3_set: std::collections::HashSet<_> = u3_qspan.iter().copied().collect();
    let u40_set: std::collections::HashSet<_> = u40_qspan.iter().copied().collect();
    let u42_set: std::collections::HashSet<_> = u42_qspan.iter().copied().collect();
    
    let u3_contains_u40 = u40_set.is_subset(&u3_set);
    let u3_contains_u42 = u42_set.is_subset(&u3_set);
    
    println!("unicode_3 qspan contains unicode_40 qspan: {}", u3_contains_u40);
    println!("unicode_3 qspan contains unicode_42 qspan: {}", u3_contains_u42);
    
    println!("\nunicode_3 qspan: {} positions (first 10: {:?})", u3_qspan.len(), &u3_qspan[..u3_qspan.len().min(10)]);
    println!("unicode_40 qspan: {} positions (first 10: {:?})", u40_qspan.len(), &u40_qspan[..u40_qspan.len().min(10)]);
    println!("unicode_42 qspan: {} positions (first 10: {:?})", u42_qspan.len(), &u42_qspan[..u42_qspan.len().min(10)]);
}
