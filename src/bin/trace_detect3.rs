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

    let mut query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    // AHO matching
    let raw_aho = aho_match(index, &whole_run);
    let refined_aho = refine_aho_matches(index, raw_aho, &query);

    // Build matched_qspans like detect_matches does
    let mut matched_qspans = Vec::new();
    for m in &refined_aho {
        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token {
            matched_qspans.push(scancode_rust::license_detection::query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }

    let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
    println!("skip_seq_matching = {}", skip_seq_matching);

    let mut seq_all_matches = Vec::new();
    if !skip_seq_matching {
        // Phase 2: Near-duplicate detection
        let whole_run = query.whole_query_run();
        let near_dupe_candidates = compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
        println!("\n=== PHASE 2: NEAR-DUPE CANDIDATES ({}) ===", near_dupe_candidates.len());
        
        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches = seq_match_with_candidates(index, &whole_run, &near_dupe_candidates, &[]);
            println!("Near-dupe matches: {}", near_dupe_matches.len());
            
            for m in &near_dupe_matches {
                if m.end_token > m.start_token {
                    let span = scancode_rust::license_detection::query::PositionSpan::new(m.start_token, m.end_token - 1);
                    matched_qspans.push(span);
                }
            }
            seq_all_matches.extend(near_dupe_matches);
        }

        // Phase 3: Regular sequence matching
        const MAX_SEQ_CANDIDATES: usize = 70;
        let whole_run = query.whole_query_run();
        let candidates = compute_candidates_with_msets(index, &whole_run, false, MAX_SEQ_CANDIDATES);
        println!("\n=== PHASE 3: REGULAR SEQ CANDIDATES ({}) ===", candidates.len());
        
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(index, &whole_run, &candidates, &[]);
            println!("Phase 3 matches: {}", matches.len());
            for m in matches.iter().take(5) {
                println!("  {} (license: {}, qstart={}, end_token={})", 
                    m.rule_identifier, m.license_expression, m.qstart(), m.end_token);
            }
            seq_all_matches.extend(matches);
        }

        // Phase 4: Query run matching
        const MAX_QUERY_RUN_CANDIDATES: usize = 70;
        let whole_run = query.whole_query_run();
        for query_run in query.query_runs().iter() {
            if query_run.start == whole_run.start && query_run.end == whole_run.end {
                continue;
            }
            if !query_run.is_matchable(false, &matched_qspans) {
                continue;
            }
            let candidates = compute_candidates_with_msets(index, query_run, false, MAX_QUERY_RUN_CANDIDATES);
            if !candidates.is_empty() {
                let matches = seq_match_with_candidates(index, query_run, &candidates, &[]);
                println!("\n=== PHASE 4: QUERY RUN MATCHES ({}) ===", matches.len());
                seq_all_matches.extend(matches);
            }
        }

        // Merge all sequence matches
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        println!("\n=== MERGED SEQUENCE MATCHES ({}) ===", merged_seq.len());
        for m in merged_seq.iter() {
            println!("  {} (license: {}, qstart={}, end_token={}, matcher={})", 
                m.rule_identifier, m.license_expression, m.qstart(), m.end_token, m.matcher);
        }
        
        // Combine with AHO matches
        let mut all_matches = Vec::new();
        all_matches.extend(refined_aho.clone());
        all_matches.extend(merged_seq);
        
        println!("\n=== ALL MATCHES COMBINED ({}) ===", all_matches.len());
        
        // Check filter_contained_matches sorting
        let mut sorted_matches = all_matches.clone();
        sorted_matches.sort_by(|a, b| {
            a.qstart()
                .cmp(&b.qstart())
                .then_with(|| b.hilen.cmp(&a.hilen))
                .then_with(|| b.matched_length.cmp(&a.matched_length))
                .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        });
        
        println!("\n=== SORTED MATCHES (by filter_contained_matches order) ===");
        for (i, m) in sorted_matches.iter().enumerate() {
            println!("  [{}] {} (license: {}, qstart={}, end_token={}, hilen={}, matched_len={}, matcher_order={})", 
                i, m.rule_identifier, m.license_expression, m.qstart(), m.end_token, m.hilen, m.matched_length, m.matcher_order());
        }
    }
}
