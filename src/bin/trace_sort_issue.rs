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

    // Get all matches like detect_matches does
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    let mut matched_qspans = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(scancode_rust::license_detection::query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);

    let mut seq_all_matches = Vec::new();
    if !skip_seq_matching {
        // All phases
        let whole_run = query.whole_query_run();
        let near_dupe_candidates = compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);
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

    // Now simulate filter_contained_matches sorting
    let mut sorted: Vec<_> = all_matches.iter().collect();
    sorted.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    // Find the key matches in sorted order
    println!("=== KEY MATCHES IN SORTED ORDER ===");
    for (i, m) in sorted.iter().enumerate() {
        if (m.rule_identifier == "unicode_3.RULE" && m.start_token == 985) ||
           m.rule_identifier == "unicode_40.RULE" ||
           (m.rule_identifier == "unicode_42.RULE" && m.matcher == "2-aho") {
            println!("[{}] {} (qstart={}, end_token={}, hilen={}, len={}, matcher_order={})", 
                i, m.rule_identifier, m.qstart(), m.end_token, m.hilen, m.matched_length, m.matcher_order());
        }
    }

    // Now check what happens in filter_contained_matches
    println!("\n=== SIMULATING filter_contained_matches ===");
    
    // Find positions
    let unicode_3_pos = sorted.iter().position(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token == 985).unwrap();
    let unicode_40_pos = sorted.iter().position(|m| m.rule_identifier == "unicode_40.RULE").unwrap();
    let unicode_42_pos = sorted.iter().position(|m| m.rule_identifier == "unicode_42.RULE" && m.matcher == "2-aho").unwrap();
    
    println!("unicode_3 at position {}", unicode_3_pos);
    println!("unicode_40 at position {}", unicode_40_pos);
    println!("unicode_42 at position {}", unicode_42_pos);
    
    // Check containment
    let u3 = &sorted[unicode_3_pos];
    let u40 = &sorted[unicode_40_pos];
    let u42 = &sorted[unicode_42_pos];
    
    println!("\nu3.qcontains(u40) = {}", u3.qcontains(u40));
    println!("u3.qcontains(u42) = {}", u3.qcontains(u42));
    
    // The issue: u3 comes BEFORE u40 (position 194 vs 195), so when processing u3 at i=194:
    // j=195 (u40), u3.qcontains(u40) = true, so u40 is DISCARDED
    // j=196 (next match), but u42 has qstart=1127 which is > u3.end_token=1468? No, 1127 < 1468
    // Actually wait - u42.qstart() = 1127 > u3.end_token = 1468? No, 1127 < 1468
    // But wait, the break condition is: if next.end_token > current.end_token
    // u42.end_token = 1468, u3.end_token = 1468, so 1468 > 1468 = false, so we continue
    // Then u3.qcontains(u42) = false, so u42 is NOT discarded
    // j++, continue...
    
    // But u42 is NOT being returned either! Let me check what's happening...
}
