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

    let mut query = Query::new(&text, index).unwrap();
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
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        seq_all_matches = merged_seq;
    }

    // Combine all matches
    let mut all_matches = Vec::new();
    all_matches.extend(refined_aho.clone());
    all_matches.extend(seq_all_matches.clone());

    // Find the key matches
    let unicode_3 = all_matches
        .iter()
        .find(|m| m.rule_identifier == "unicode_3.RULE" && m.start_token == 985);
    let unicode_40 = all_matches
        .iter()
        .find(|m| m.rule_identifier == "unicode_40.RULE");
    let unicode_42 = all_matches
        .iter()
        .find(|m| m.rule_identifier == "unicode_42.RULE" && m.matcher == "2-aho");

    println!("=== KEY MATCHES BEFORE filter_contained_matches ===");

    if let Some(m) = unicode_3 {
        println!("\nunicode_3 (seq):");
        println!("  qstart={}, end_token={}", m.qstart(), m.end_token);
        println!("  hilen={}, matched_len={}", m.hilen, m.matched_length);
        println!("  matcher_order={}", m.matcher_order());
        println!(
            "  Sort tuple: (qstart={}, -hilen={}, -len={}, order={})",
            m.qstart(),
            -(m.hilen as i64),
            -(m.matched_length as i64),
            m.matcher_order()
        );
    }
    if let Some(m) = unicode_40 {
        println!("\nunicode_40 (aho):");
        println!("  qstart={}, end_token={}", m.qstart(), m.end_token);
        println!("  hilen={}, matched_len={}", m.hilen, m.matched_length);
        println!("  matcher_order={}", m.matcher_order());
        println!(
            "  Sort tuple: (qstart={}, -hilen={}, -len={}, order={})",
            m.qstart(),
            -(m.hilen as i64),
            -(m.matched_length as i64),
            m.matcher_order()
        );
    }
    if let Some(m) = unicode_42 {
        println!("\nunicode_42 (aho):");
        println!("  qstart={}, end_token={}", m.qstart(), m.end_token);
        println!("  hilen={}, matched_len={}", m.hilen, m.matched_length);
        println!("  matcher_order={}", m.matcher_order());
        println!(
            "  Sort tuple: (qstart={}, -hilen={}, -len={}, order={})",
            m.qstart(),
            -(m.hilen as i64),
            -(m.matched_length as i64),
            m.matcher_order()
        );
    }

    // Check qcontains
    if let (Some(u3), Some(u40), Some(u42)) = (unicode_3, unicode_40, unicode_42) {
        println!("\n=== CONTAINMENT CHECKS ===");
        println!("unicode_3.qcontains(unicode_40): {}", u3.qcontains(u40));
        println!("unicode_3.qcontains(unicode_42): {}", u3.qcontains(u42));
        println!("unicode_40.qcontains(unicode_3): {}", u40.qcontains(u3));
        println!("unicode_42.qcontains(unicode_3): {}", u42.qcontains(u3));

        // Check qspan positions
        println!("\n=== QSPAN DETAILS ===");
        let u3_qspan: Vec<usize> = u3.qspan();
        let u40_qspan: Vec<usize> = u40.qspan();
        let u42_qspan: Vec<usize> = u42.qspan();

        println!(
            "unicode_3 qspan: start={}, end={}, len={}",
            u3_qspan.first().unwrap_or(&0),
            u3_qspan.last().unwrap_or(&0),
            u3_qspan.len()
        );
        println!(
            "unicode_40 qspan: start={}, end={}, len={}",
            u40_qspan.first().unwrap_or(&0),
            u40_qspan.last().unwrap_or(&0),
            u40_qspan.len()
        );
        println!(
            "unicode_42 qspan: start={}, end={}, len={}",
            u42_qspan.first().unwrap_or(&0),
            u42_qspan.last().unwrap_or(&0),
            u42_qspan.len()
        );
    }

    // Check sorting order
    println!("\n=== SORTING ORDER (by filter_contained_matches) ===");
    let mut sorted: Vec<_> = all_matches.iter().collect();
    sorted.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    for (i, m) in sorted.iter().enumerate() {
        if m.rule_identifier == "unicode_3.RULE"
            || m.rule_identifier == "unicode_40.RULE"
            || m.rule_identifier == "unicode_42.RULE"
        {
            println!(
                "[{}] {} (qstart={}, hilen={}, len={}, order={})",
                i,
                m.rule_identifier,
                m.qstart(),
                m.hilen,
                m.matched_length,
                m.matcher_order()
            );
        }
    }
}
