//! Investigation test for PLAN-005: kde_licenses_test.txt
//!
//! Issue: Missing `lgpl-2.1` detections, extra `lgpl-2.1-plus`, missing `bsd-simplified AND bsd-new` conjunction.
//!
//! Expected: ["gpl-2.0 OR gpl-3.0 OR kde-accepted-gpl", "lgpl-2.1 OR lgpl-3.0 OR kde-accepted-lgpl", "gpl-2.0-plus", "gpl-3.0", "gpl-3.0-plus", "gpl-3.0-plus", "gpl-3.0-plus", "lgpl-2.1", "lgpl-2.1", "lgpl-2.1-plus", "bsd-simplified AND bsd-new", "x11-xconsortium", "x11-xconsortium", "mit", "mit"]
//! Actual:   ["gpl-2.0 OR gpl-3.0 OR kde-accepted-gpl", "lgpl-2.1 OR lgpl-3.0 OR kde-accepted-lgpl", "gpl-2.0-plus", "gpl-3.0", "gpl-3.0-plus", "gpl-3.0-plus", "gpl-3.0-plus", "lgpl-2.1-plus", "bsd-simplified", "bsd-simplified AND bsd-new", "x11-xconsortium", "x11-xconsortium", "mit", "mit"]
//!
//! Python finds 15 matches:
//! 1. gpl-2.0 OR gpl-3.0 OR kde-accepted-gpl at lines 3-17 (aho)
//! 2. lgpl-2.1 OR lgpl-3.0 OR kde-accepted-lgpl at lines 22-36 (aho)
//! 3. gpl-2.0-plus at lines 41-52 (aho)
//! 4. gpl-3.0 at lines 57-71 (aho)
//! 5. gpl-3.0-plus at lines 71-73 (aho)
//! 6. gpl-3.0-plus at lines 75-75 (aho)
//! 7. gpl-3.0-plus at lines 79-90 (aho)
//! 8. lgpl-2.1 at lines 90-92 (aho)      <-- MISSING in Rust
//! 9. lgpl-2.1 at lines 94-94 (aho)      <-- MISSING in Rust
//! 10. lgpl-2.1-plus at lines 98-109 (aho)
//! 11. bsd-simplified AND bsd-new at lines 111-140 (seq)  <-- Extra bsd-simplified at 111-113 in Rust
//! 12. x11-xconsortium at lines 141-143 (aho)
//! 13. x11-xconsortium at lines 147-166 (aho)
//! 14. mit at lines 168-170 (aho)
//! 15. mit at lines 174-191 (aho)
//!
//! Divergence points:
//! - lgpl-2.1 at lines 90-92 and 94-94: Found by Python aho, missing in Rust final output
//! - bsd-simplified at lines 111-113: Extra match in Rust (not in Python final output)

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
        let path = PathBuf::from("testdata/license-golden/datadriven/lic4/kde_licenses_test.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_kde_licenses_detection_summary() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();

        eprintln!("=== KDE Licenses Detection Summary ===");
        eprintln!("Total detections: {}", detections.len());
        eprintln!("Total matches: {}", all_matches.len());
        eprintln!();

        for (i, m) in all_matches.iter().enumerate() {
            eprintln!(
                "{}. {} at lines {}-{} (matcher={}, score={:.1}, coverage={:.1}%)",
                i + 1,
                m.license_expression,
                m.start_line,
                m.end_line,
                m.matcher,
                m.score,
                m.match_coverage
            );
        }
    }

    #[test]
    fn test_kde_licenses_lgpl_21_missing() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();

        let lgpl_21_matches: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression == "lgpl-2.1")
            .collect();

        eprintln!("=== LGPL-2.1 Analysis ===");
        eprintln!("lgpl-2.1 matches: {} (expected 2)", lgpl_21_matches.len());

        for (i, m) in lgpl_21_matches.iter().enumerate() {
            eprintln!(
                "  lgpl-2.1 #{}: lines {}-{}, matcher={}, rule={}",
                i + 1,
                m.start_line,
                m.end_line,
                m.matcher,
                m.rule_identifier
            );
        }

        assert_eq!(
            lgpl_21_matches.len(),
            2,
            "Expected 2 lgpl-2.1 matches (Python finds them at lines 90-92 and 94-94)"
        );
    }

    #[test]
    fn test_kde_licenses_bsd_extra_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();

        let bsd_simplified_only: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression == "bsd-simplified")
            .collect();

        let bsd_conjunction: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression == "bsd-simplified AND bsd-new")
            .collect();

        eprintln!("=== BSD Analysis ===");
        eprintln!(
            "bsd-simplified (standalone): {} (expected 0)",
            bsd_simplified_only.len()
        );
        eprintln!(
            "bsd-simplified AND bsd-new: {} (expected 1)",
            bsd_conjunction.len()
        );

        for (i, m) in bsd_simplified_only.iter().enumerate() {
            eprintln!(
                "  bsd-simplified #{}: lines {}-{}, matcher={}",
                i + 1,
                m.start_line,
                m.end_line,
                m.matcher
            );
        }

        assert_eq!(
            bsd_simplified_only.len(),
            0,
            "Expected 0 standalone bsd-simplified matches - this is a false positive"
        );
        assert_eq!(
            bsd_conjunction.len(),
            1,
            "Expected 1 bsd-simplified AND bsd-new match"
        );
    }

    #[test]
    fn test_kde_licenses_aho_refine_trace() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::match_refine::{
            merge_overlapping_matches, refine_matches_without_false_positive_filter,
        };
        use crate::license_detection::query::Query;
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, engine.index()).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        let mut all_matches = Vec::new();
        all_matches.extend(hash_match(engine.index(), &whole_run));
        all_matches.extend(spdx_lid_match(engine.index(), &query));
        all_matches.extend(aho_match(engine.index(), &whole_run));

        let merged = merge_overlapping_matches(&all_matches);
        let refined = refine_matches_without_false_positive_filter(engine.index(), merged, &query);

        eprintln!("=== lgpl-2.1 matches after aho refine ===");
        let lgpl_refined: Vec<_> = refined
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1"))
            .collect();
        for (i, m) in lgpl_refined.iter().enumerate() {
            eprintln!(
                "  #{}: {} at lines {}-{} tokens={}-{} matcher={} coverage={:.2}%",
                i + 1,
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matcher,
                m.match_coverage
            );
        }

        let lgpl_21_matches: Vec<_> = lgpl_refined
            .iter()
            .filter(|m| m.license_expression == "lgpl-2.1")
            .collect();

        eprintln!("\n=== lgpl-2.1 exact matches ===");
        for m in &lgpl_21_matches {
            eprintln!(
                "  lgpl-2.1 at lines {}-{} tokens={}-{} coverage={:.2}%, would add to matched_qspans? {}",
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.match_coverage,
                m.match_coverage >= 99.99
            );
        }

        assert!(
            lgpl_21_matches.len() >= 2,
            "Expected at least 2 lgpl-2.1 matches after aho refine, got {}",
            lgpl_21_matches.len()
        );
    }

    #[test]
    fn test_kde_licenses_full_pipeline() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::hash_match::hash_match;
        use crate::license_detection::match_refine::merge_overlapping_matches;
        use crate::license_detection::query::{PositionSpan, Query};
        use crate::license_detection::spdx_lid::spdx_lid_match;
        use crate::utils::text::strip_utf8_bom_str;

        let clean_text = strip_utf8_bom_str(&text);
        let mut query = Query::new(clean_text, engine.index()).expect("Query creation failed");

        let whole_run = query.whole_query_run();

        let mut matched_qspans: Vec<PositionSpan> = Vec::new();
        let mut all_matches = Vec::new();

        // Phase 1b: SPDX-LID matching
        let spdx_matches = spdx_lid_match(engine.index(), &query);
        let merged_spdx = merge_overlapping_matches(&spdx_matches);
        for m in &merged_spdx {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        all_matches.extend(merged_spdx);

        // Phase 1c: Aho-Corasick matching
        let whole_run = query.whole_query_run();
        let aho_matches = aho_match(engine.index(), &whole_run);
        let merged_aho = merge_overlapping_matches(&aho_matches);
        for m in &merged_aho {
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(PositionSpan::new(m.start_token, m.end_token - 1));
            }
        }
        all_matches.extend(merged_aho);

        eprintln!("=== After aho matching ===");
        let lgpl_aho: Vec<_> = all_matches
            .iter()
            .filter(|m| m.license_expression.contains("lgpl-2.1"))
            .collect();
        for (i, m) in lgpl_aho.iter().enumerate() {
            eprintln!(
                "  #{}: {} at lines {}-{} tokens={}-{}",
                i + 1,
                m.license_expression,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token
            );
        }

        eprintln!("\n=== matched_qspans covering lgpl-2.1 region (tokens 655-665) ===");
        for span in &matched_qspans {
            let positions = span.positions();
            let covers_region = (655..=665).any(|p| positions.contains(&p));
            if covers_region {
                let min_pos = positions.iter().min().unwrap();
                let max_pos = positions.iter().max().unwrap();
                eprintln!("  span: {}-{}", min_pos, max_pos);
            }
        }

        let whole_run = query.whole_query_run();
        eprintln!(
            "\nis_matchable after aho: {}",
            whole_run.is_matchable(false, &matched_qspans)
        );
    }
}
