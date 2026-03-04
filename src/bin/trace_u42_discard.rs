use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::{
    LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES, aho_match, compute_candidates_with_msets,
    merge_overlapping_matches, refine_aho_matches, seq_match_with_candidates,
};
use scancode_rust::license_detection::models::LicenseMatch;
use std::path::PathBuf;

fn my_filter_contained_matches_verbose(matches: &[LicenseMatch], track_rules: &[&str]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
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

    let mut i = 0;
    while i < matches.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < matches.len() {
            let current = matches[i].clone();
            let next = matches[j].clone();

            // Only print for tracked rules
            let should_print = track_rules.contains(&current.rule_identifier.as_str()) || 
                               track_rules.contains(&next.rule_identifier.as_str());

            if next.end_token > current.end_token {
                break;
            }

            if current.qspan_eq(&next) {
                if should_print {
                    println!("[i={}, j={}] qspan_eq: {} vs {}", i, j, current.rule_identifier, next.rule_identifier);
                }
                if current.match_coverage >= next.match_coverage {
                    if should_print {
                        println!("  -> DISCARD next {} (cov {} < {})", next.rule_identifier, next.match_coverage, current.match_coverage);
                    }
                    discarded.push(matches.remove(j));
                    continue;
                } else {
                    if should_print {
                        println!("  -> DISCARD current {} (cov {} < {})", current.rule_identifier, current.match_coverage, next.match_coverage);
                    }
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
            }

            if current.qcontains(&next) {
                if should_print {
                    println!("[i={}, j={}] {} qcontains {}", i, j, current.rule_identifier, next.rule_identifier);
                }
                discarded.push(matches.remove(j));
                continue;
            }
            if next.qcontains(&current) {
                if should_print {
                    println!("[i={}, j={}] {} qcontains {}", i, j, next.rule_identifier, current.rule_identifier);
                }
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
    let path = PathBuf::from("testdata/license-golden/datadriven/external/fossology-licenses/unicode.txt");
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
                let matches = seq_match_with_candidates(index, query_run, &candidates);
                seq_all_matches.extend(matches);
            }
        }
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        seq_all_matches = merged_seq;
    }

    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(seq_all_matches.clone());

    let track_rules = ["unicode_3.RULE", "unicode_40.RULE", "unicode_42.RULE", "unicode.LICENSE"];
    
    println!("=== TRACKING RULES: {:?} ===\n", track_rules);
    
    let (kept, discarded) = my_filter_contained_matches_verbose(&all_matches, &track_rules);
    
    println!("\n=== KEPT ({}) ===", kept.len());
    for m in &kept {
        if track_rules.contains(&m.rule_identifier.as_str()) {
            println!("  {} (license: {}, qstart={}, end_token={}, matcher={})", 
                m.rule_identifier, m.license_expression, m.qstart(), m.end_token, m.matcher);
        }
    }
    
    println!("\n=== DISCARDED ({}) ===", discarded.len());
    for m in &discarded {
        if track_rules.contains(&m.rule_identifier.as_str()) {
            println!("  {} (license: {}, qstart={}, end_token={}, matcher={})", 
                m.rule_identifier, m.license_expression, m.qstart(), m.end_token, m.matcher);
        }
    }
}
