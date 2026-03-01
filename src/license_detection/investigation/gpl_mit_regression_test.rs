//! Investigation test for filter_overlapping_matches regression.
//!
//! ## Issue
//! After adding `filter_overlapping_matches` after `filter_contained_matches`,
//! a valid "mit" match is incorrectly filtered.
//!
//! **Expected:** `["gpl-2.0-plus", "mit", "mit", "gpl-1.0-plus"]`
//! **Actual:** `["gpl-2.0-plus", "mit", "gpl-1.0-plus"]` (missing one "mit")
//!
//! ## Investigation approach
//! Use INCREMENTAL TEST METHOD:
//! 1. Check raw aho matches (before any filtering)
//! 2. Check after `merge_overlapping_matches`
//! 3. Check after `filter_contained_matches`
//! 4. Check after `filter_overlapping_matches`
//! 5. Compare with Python at each stage

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::path::PathBuf;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    fn read_test_file() -> Option<String> {
        let path =
            PathBuf::from("testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_mit_1.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_gpl_mit_rust_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== RUST DETECTIONS ===");
        eprintln!("Number of detections: {}", detections.len());

        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            eprintln!("  detection_log: {:?}", det.detection_log);
            eprintln!("  Number of matches: {}", det.matches.len());

            for (j, m) in det.matches.iter().enumerate() {
                eprintln!("    Match {}:", j + 1);
                eprintln!("      license_expression: {}", m.license_expression);
                eprintln!("      matcher: {}", m.matcher);
                eprintln!("      lines: {}-{}", m.start_line, m.end_line);
                eprintln!("      tokens: {}-{}", m.start_token, m.end_token);
                eprintln!("      score: {:.2}", m.score);
                eprintln!("      match_coverage: {:.1}", m.match_coverage);
                eprintln!("      rule_identifier: {}", m.rule_identifier);
                eprintln!("      matched_length: {}", m.matched_length);
                eprintln!("      rule_length: {}", m.rule_length);
            }
        }

        let all_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nAll license expressions: {:?}", all_expressions);
    }

    #[test]
    fn test_gpl_mit_phase1_incremental() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;

        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };

        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        // STEP 1: Raw aho matches
        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("\n=== STEP 1: RAW AHO MATCHES ===");
        eprintln!("Count: {}", aho_matches.len());
        for m in &aho_matches {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }

        // Check for MIT matches in raw matches
        let mit_raw: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();
        eprintln!("\nMIT matches in raw aho: {}", mit_raw.len());
        for m in &mit_raw {
            eprintln!(
                "  MIT at lines {}-{} tokens {}-{}",
                m.start_line, m.end_line, m.start_token, m.end_token
            );
        }

        // STEP 2: After merge_overlapping_matches
        let merged = merge_overlapping_matches(&aho_matches);
        eprintln!("\n=== STEP 2: AFTER merge_overlapping_matches ===");
        eprintln!("Count: {}", merged.len());
        for m in &merged {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }

        let mit_merged: Vec<_> = merged
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();
        eprintln!("\nMIT matches after merge: {}", mit_merged.len());
        for m in &mit_merged {
            eprintln!(
                "  MIT at lines {}-{} tokens {}-{}",
                m.start_line, m.end_line, m.start_token, m.end_token
            );
        }

        // STEP 3: After filter_contained_matches
        let (non_contained, discarded_contained) = filter_contained_matches(&merged);
        eprintln!("\n=== STEP 3: AFTER filter_contained_matches ===");
        eprintln!(
            "Kept: {}, Discarded: {}",
            non_contained.len(),
            discarded_contained.len()
        );

        eprintln!("\nKEPT matches:");
        for m in &non_contained {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }

        eprintln!("\nDISCARDED matches:");
        for m in &discarded_contained {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }

        let mit_contained: Vec<_> = non_contained
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();
        eprintln!(
            "\nMIT matches after filter_contained: {}",
            mit_contained.len()
        );
        for m in &mit_contained {
            eprintln!(
                "  MIT at lines {}-{} tokens {}-{}",
                m.start_line, m.end_line, m.start_token, m.end_token
            );
        }

        // STEP 4: After filter_overlapping_matches
        let (filtered, discarded_overlapping) =
            filter_overlapping_matches(non_contained.clone(), &index);
        eprintln!("\n=== STEP 4: AFTER filter_overlapping_matches ===");
        eprintln!(
            "Kept: {}, Discarded: {}",
            filtered.len(),
            discarded_overlapping.len()
        );

        eprintln!("\nFINAL KEPT matches:");
        for m in &filtered {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }

        eprintln!("\nDISCARDED by overlapping:");
        for m in &discarded_overlapping {
            eprintln!(
                "  {} at lines {}-{} tokens {}-{} rule={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.rule_identifier
            );
        }

        let mit_final: Vec<_> = filtered
            .iter()
            .filter(|m| m.license_expression == "mit")
            .collect();
        eprintln!(
            "\nMIT matches after filter_overlapping: {}",
            mit_final.len()
        );
        for m in &mit_final {
            eprintln!(
                "  MIT at lines {}-{} tokens {}-{}",
                m.start_line, m.end_line, m.start_token, m.end_token
            );
        }

        // Summary
        eprintln!("\n=== SUMMARY ===");
        eprintln!("MIT count at each stage:");
        eprintln!("  Raw aho: {}", mit_raw.len());
        eprintln!("  After merge: {}", mit_merged.len());
        eprintln!("  After filter_contained: {}", mit_contained.len());
        eprintln!("  After filter_overlapping: {}", mit_final.len());
    }

    #[test]
    fn test_gpl_mit_overlapping_analysis() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        let aho_matches = aho_match(&index, &whole_run);
        let merged = merge_overlapping_matches(&aho_matches);
        let (non_contained, _) = filter_contained_matches(&merged);

        eprintln!("\n=== ANALYZING OVERLAP BETWEEN MATCHES ===");

        // Find all pairs that overlap
        for i in 0..non_contained.len() {
            for j in (i + 1)..non_contained.len() {
                let a = &non_contained[i];
                let b = &non_contained[j];

                // Check token overlap
                let a_start = a.start_token;
                let a_end = a.end_token;
                let b_start = b.start_token;
                let b_end = b.end_token;

                let overlap = if b_start < a_end && a_start < b_end {
                    let overlap_start = a_start.max(b_start);
                    let overlap_end = a_end.min(b_end);
                    Some(overlap_end - overlap_start)
                } else {
                    None
                };

                if let Some(overlap_tokens) = overlap {
                    eprintln!(
                        "\nOVERLAP: {} (tokens {}-{}) vs {} (tokens {}-{})",
                        a.license_expression, a_start, a_end, b.license_expression, b_start, b_end
                    );
                    eprintln!("  Overlap: {} tokens", overlap_tokens);
                    eprintln!(
                        "  A: rule={}, hilen={}, matched_length={}",
                        a.rule_identifier, a.hilen, a.matched_length
                    );
                    eprintln!(
                        "  B: rule={}, hilen={}, matched_length={}",
                        b.rule_identifier, b.hilen, b.matched_length
                    );

                    // Calculate overlap ratios
                    let overlap_ratio_a = overlap_tokens as f64 / a.matched_length as f64;
                    let overlap_ratio_b = overlap_tokens as f64 / b.matched_length as f64;
                    eprintln!("  Overlap ratio to A: {:.2}%", overlap_ratio_a * 100.0);
                    eprintln!("  Overlap ratio to B: {:.2}%", overlap_ratio_b * 100.0);
                }
            }
        }
    }

    #[test]
    fn test_gpl_mit_detailed_filter_overlapping_logic() {
        let Some(_engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::match_refine::{
            filter_contained_matches, merge_overlapping_matches,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };
        use crate::utils::text::strip_utf8_bom_str;

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        let aho_matches = aho_match(&index, &whole_run);
        let merged = merge_overlapping_matches(&aho_matches);
        let (non_contained, _) = filter_contained_matches(&merged);

        // Sort matches like filter_overlapping does
        let mut matches = non_contained.clone();
        matches.sort_by(|a, b| {
            a.start_token
                .cmp(&b.start_token)
                .then_with(|| b.hilen.cmp(&a.hilen))
                .then_with(|| b.matched_length.cmp(&a.matched_length))
                .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        });

        eprintln!("\n=== MATCHES SORTED FOR filter_overlapping ===");
        for (i, m) in matches.iter().enumerate() {
            eprintln!(
                "{}: {} at tokens {}-{} hilen={} len={} rule={}",
                i,
                m.license_expression,
                m.start_token,
                m.end_token,
                m.hilen,
                m.matched_length,
                m.rule_identifier
            );
        }

        // Manually trace through filter_overlapping logic
        eprintln!("\n=== TRACING filter_overlapping LOGIC ===");

        let overlap_small: f64 = 0.10;
        let overlap_medium: f64 = 0.40;
        let overlap_large: f64 = 0.70;
        let overlap_extra_large: f64 = 0.90;

        for i in 0..matches.len().saturating_sub(1) {
            for j in (i + 1)..matches.len() {
                let current_end = matches[i].end_token;
                let next_start = matches[j].start_token;

                if next_start >= current_end {
                    eprintln!("No overlap between [{}] and [{}] - breaking j loop", i, j);
                    break;
                }

                let overlap = matches[i].qoverlap(&matches[j]);
                if overlap == 0 {
                    eprintln!(
                        "Zero qoverlap between [{}] {} and [{}] {} - continuing",
                        i, matches[i].license_expression, j, matches[j].license_expression
                    );
                    continue;
                }

                let next_len = matches[j].matched_length;
                let current_len = matches[i].matched_length;

                if next_len == 0 || current_len == 0 {
                    continue;
                }

                let overlap_ratio_to_next = overlap as f64 / next_len as f64;
                let overlap_ratio_to_current = overlap as f64 / current_len as f64;

                let extra_large_next = overlap_ratio_to_next >= overlap_extra_large;
                let large_next = overlap_ratio_to_next >= overlap_large;
                let medium_next = overlap_ratio_to_next >= overlap_medium;
                let small_next = overlap_ratio_to_next >= overlap_small;

                let extra_large_current = overlap_ratio_to_current >= overlap_extra_large;
                let large_current = overlap_ratio_to_current >= overlap_large;
                let medium_current = overlap_ratio_to_current >= overlap_medium;
                let small_current = overlap_ratio_to_current >= overlap_small;

                let different_licenses =
                    matches[i].license_expression != matches[j].license_expression;

                eprintln!(
                    "\nComparing [{}] {} vs [{}] {}:",
                    i, matches[i].license_expression, j, matches[j].license_expression
                );
                eprintln!("  Token overlap: {} tokens", overlap);
                eprintln!(
                    "  Overlap ratio to current: {:.2}% (current_len={})",
                    overlap_ratio_to_current * 100.0,
                    current_len
                );
                eprintln!(
                    "  Overlap ratio to next: {:.2}% (next_len={})",
                    overlap_ratio_to_next * 100.0,
                    next_len
                );
                eprintln!(
                    "  extra_large_next={}, large_next={}, medium_next={}, small_next={}",
                    extra_large_next, large_next, medium_next, small_next
                );
                eprintln!(
                    "  extra_large_current={}, large_current={}, medium_current={}, small_current={}",
                    extra_large_current, large_current, medium_current, small_current
                );
                eprintln!("  different_licenses={}", different_licenses);

                // Check which branch would trigger
                if extra_large_next && current_len >= next_len {
                    eprintln!(
                        "  -> Would DISCARD [{}] (extra_large_next && current_len >= next_len)",
                        j
                    );
                } else if extra_large_current && current_len <= next_len {
                    eprintln!(
                        "  -> Would DISCARD [{}] (extra_large_current && current_len <= next_len)",
                        i
                    );
                } else if large_next
                    && current_len >= next_len
                    && matches[i].hilen >= matches[j].hilen
                {
                    eprintln!(
                        "  -> Would DISCARD [{}] (large_next && current_len >= next_len && hilen_check)",
                        j
                    );
                } else if large_current
                    && current_len <= next_len
                    && matches[i].hilen <= matches[j].hilen
                {
                    eprintln!(
                        "  -> Would DISCARD [{}] (large_current && current_len <= next_len && hilen_check)",
                        i
                    );
                } else {
                    eprintln!("  -> No action (checking next overlap conditions...)");
                }
            }
        }
    }
}
