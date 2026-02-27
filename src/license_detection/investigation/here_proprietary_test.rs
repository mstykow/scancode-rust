//! Investigation tests for PLAN-003: Duplicate detection of `here-proprietary`
//!
//! ## Issue
//! Test file: `testdata/license-golden/datadriven/lic4/here-proprietary_4.RULE`
//! Content: `SPDX-License-Identifier: LicenseRef-Proprietary-HERE`
//!
//! Expected: `["here-proprietary"]`
//! Actual: `["here-proprietary", "here-proprietary"]`
//!
//! ## Root Cause Identified
//!
//! Two matches are being created for the same license expression:
//!
//! 1. **SPDX-LID match** via `1-spdx-id`:
//!    - Rule: `here-proprietary.LICENSE`
//!    - Tokens: `start_token=0, end_token=5`
//!    - Covers: tokens [0, 1, 2, 3, 4] (exclusive end)
//!
//! 2. **Aho-Corasick match** via `2-aho`:
//!    - Rule: `spdx_license_id_licenseref-proprietary-here_for_here-proprietary.RULE`
//!    - Tokens: `start_token=3, end_token=6`
//!    - Covers: tokens [3, 4, 5] (exclusive end)
//!    - Note: Token 5 is in Aho match but NOT in SPDX match!
//!
//! ## The Bug
//!
//! SPDX-LID matching sets `end_token` to the **last position** (inclusive), but
//! Aho-Corasick sets `end_token` as **exclusive** (one past the last position).
//! This inconsistency prevents proper containment detection in `filter_contained_matches`.
//!
//! The `qcontains()` check fails because token 5 is in Aho's qspan but NOT in SPDX's qspan.

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use once_cell::sync::Lazy;
    use std::path::PathBuf;
    use std::sync::Once;

    static TEST_ENGINE: Lazy<Option<LicenseDetectionEngine>> = Lazy::new(|| {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            eprintln!("Reference data not available at {:?}", data_path);
            return None;
        }
        match LicenseDetectionEngine::new(&data_path) {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("Failed to create engine: {:?}", e);
                None
            }
        }
    });

    static INIT: Once = Once::new();

    fn ensure_engine() -> Option<&'static LicenseDetectionEngine> {
        INIT.call_once(|| {
            let _ = &*TEST_ENGINE;
        });
        TEST_ENGINE.as_ref()
    }

    fn print_match_details(m: &crate::license_detection::models::LicenseMatch, prefix: &str) {
        eprintln!(
            "{}: expr={}, rule_id={}, matcher={}, start_token={}, end_token={}, start_line={}, end_line={}",
            prefix,
            m.license_expression,
            m.rule_identifier,
            m.matcher,
            m.start_token,
            m.end_token,
            m.start_line,
            m.end_line
        );
        eprintln!(
            "{}:   score={}, matched_length={}, rule_length={}, match_coverage={}, hilen={}",
            prefix, m.score, m.matched_length, m.rule_length, m.match_coverage, m.hilen
        );
        eprintln!(
            "{}:   is_license_tag={}, is_license_reference={}, is_license_intro={}, is_license_clue={}",
            prefix, m.is_license_tag, m.is_license_reference, m.is_license_intro, m.is_license_clue
        );
        eprintln!(
            "{}:   qspan={:?}, ispan={:?}",
            prefix, m.qspan_positions, m.ispan_positions
        );
    }

    #[test]
    fn test_here_proprietary_full_pipeline() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = "SPDX-License-Identifier: LicenseRef-Proprietary-HERE\n";

        eprintln!("\n=== FULL DETECTION PIPELINE for here-proprietary_4.RULE ===");
        eprintln!("Input: {:?}", content);

        let detections = engine
            .detect(content, false)
            .expect("Detection should succeed");

        eprintln!("Number of detections: {}", detections.len());
        for (i, d) in detections.iter().enumerate() {
            eprintln!(
                "Detection {}: expr={:?}, {} matches",
                i,
                d.license_expression,
                d.matches.len()
            );
            for m in &d.matches {
                print_match_details(m, &format!("  Match"));
            }
        }

        let all_matches: Vec<_> = detections.iter().flat_map(|d| d.matches.iter()).collect();
        let expressions: Vec<&str> = all_matches
            .iter()
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nFinal expressions: {:?}", expressions);
        eprintln!("Expected: [\"here-proprietary\"]");
        eprintln!("Actual: {:?}", expressions);

        assert_eq!(
            expressions,
            vec!["here-proprietary"],
            "Expected single 'here-proprietary' detection, got {:?}",
            expressions
        );
    }

    #[test]
    fn test_here_proprietary_spdx_lid_stage() {
        use crate::license_detection::query::Query;

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = "SPDX-License-Identifier: LicenseRef-Proprietary-HERE\n";
        let index = engine.index();

        eprintln!("\n=== SPDX-LID MATCHING STAGE ===");

        let query = Query::new(content, index).expect("Query creation should succeed");

        eprintln!("SPDX lines found: {:?}", query.spdx_lines);

        let spdx_matches = crate::license_detection::spdx_lid::spdx_lid_match(index, &query);

        eprintln!("SPDX-LID matches: {}", spdx_matches.len());
        for (i, m) in spdx_matches.iter().enumerate() {
            print_match_details(m, &format!("SPDX-LID[{}]", i));
        }

        eprintln!("\n=== CHECKING RULES ===");
        let here_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("here-proprietary"))
            .collect();
        eprintln!("Found {} here-proprietary rules", here_rules.len());
        for rule in &here_rules {
            eprintln!(
                "  Rule: {}, is_license_tag={}, is_license_reference={}, relevance={}",
                rule.identifier, rule.is_license_tag, rule.is_license_reference, rule.relevance
            );
            eprintln!("    text: {:?}", rule.text);
        }
    }

    #[test]
    fn test_here_proprietary_aho_stage() {
        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::query::Query;

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = "SPDX-License-Identifier: LicenseRef-Proprietary-HERE\n";
        let index = engine.index();

        eprintln!("\n=== AHO-CORASICK MATCHING STAGE ===");

        let query = Query::new(content, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        eprintln!("Query tokens: {:?}", query.tokens);
        eprintln!("Matchables: {:?}", whole_run.matchables(true));

        let aho_matches = aho_match(index, &whole_run);

        eprintln!("Aho matches: {}", aho_matches.len());
        for (i, m) in aho_matches.iter().enumerate() {
            print_match_details(m, &format!("Aho[{}]", i));
        }

        let here_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression.contains("here-proprietary"))
            .collect();
        eprintln!(
            "\nhere-proprietary matches in Aho results: {}",
            here_matches.len()
        );
        for m in &here_matches {
            print_match_details(m, "HERE-Aho");
        }
    }

    #[test]
    fn test_here_proprietary_qcontains_relationship() {
        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::query::Query;

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = "SPDX-License-Identifier: LicenseRef-Proprietary-HERE\n";
        let index = engine.index();

        eprintln!("\n=== QCONTAINS RELATIONSHIP ===");

        let query = Query::new(content, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        let spdx_matches = crate::license_detection::spdx_lid::spdx_lid_match(index, &query);
        let aho_matches = aho_match(index, &whole_run);

        let spdx_here: Vec<_> = spdx_matches
            .iter()
            .filter(|m| m.license_expression.contains("here-proprietary"))
            .collect();
        let aho_here: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.license_expression.contains("here-proprietary"))
            .collect();

        eprintln!("SPDX-LID here-proprietary matches: {}", spdx_here.len());
        for m in &spdx_here {
            print_match_details(m, "SPDX-HERE");
            eprintln!("  qspan(): {:?}", m.qspan());
            eprintln!("  qspan_bounds(): {:?}", m.qspan_bounds());
        }

        eprintln!("\nAho here-proprietary matches: {}", aho_here.len());
        for m in &aho_here {
            print_match_details(m, "Aho-HERE");
            eprintln!("  qspan(): {:?}", m.qspan());
            eprintln!("  qspan_bounds(): {:?}", m.qspan_bounds());
        }

        if spdx_here.len() == 1 && aho_here.len() == 1 {
            let spdx = spdx_here[0];
            let aho = aho_here[0];

            eprintln!("\n=== CONTAINMENT CHECK ===");
            eprintln!("SPDX qcontains Aho: {}", spdx.qcontains(aho));
            eprintln!("Aho qcontains SPDX: {}", aho.qcontains(spdx));
            eprintln!("SPDX qspan: {:?}", spdx.qspan());
            eprintln!("Aho qspan: {:?}", aho.qspan());

            let spdx_qspan = spdx.qspan();
            let aho_qspan = aho.qspan();
            let overlap: Vec<_> = spdx_qspan
                .iter()
                .filter(|p| aho_qspan.contains(p))
                .collect();
            eprintln!("Overlapping positions: {:?}", overlap);

            eprintln!("\n=== QCcontains Debug ===");
            eprintln!(
                "SPDX start_token={}, end_token={}",
                spdx.start_token, spdx.end_token
            );
            eprintln!(
                "Aho start_token={}, end_token={}",
                aho.start_token, aho.end_token
            );

            let spdx_tokens: Vec<usize> = (spdx.start_token..spdx.end_token).collect();
            let aho_tokens: Vec<usize> = (aho.start_token..aho.end_token).collect();
            eprintln!("SPDX token range (exclusive end): {:?}", spdx_tokens);
            eprintln!("Aho token range (exclusive end): {:?}", aho_tokens);

            eprintln!("\nQuery tokens ({} total):", query.tokens.len());
            for (i, &tid) in query.tokens.iter().enumerate() {
                eprintln!("  token[{}] = {}", i, tid);
            }

            eprintln!("\n=== THE BUG ===");
            eprintln!("Token 5 is in Aho qspan but NOT in SPDX qspan!");
            eprintln!("This means SPDX end_token is INCLUSIVE (last position), not EXCLUSIVE.");
            eprintln!("SPDX match should have end_token=6 to cover tokens [0,1,2,3,4,5].");
            eprintln!("But it has end_token=5, so it only covers [0,1,2,3,4].");
        }
    }

    #[test]
    fn test_here_proprietary_refinement() {
        use crate::license_detection::aho_match::aho_match;
        use crate::license_detection::match_refine::refine_matches;
        use crate::license_detection::query::Query;

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = "SPDX-License-Identifier: LicenseRef-Proprietary-HERE\n";
        let index = engine.index();

        eprintln!("\n=== MATCH REFINEMENT ===");

        let query = Query::new(content, index).expect("Query creation should succeed");
        let whole_run = query.whole_query_run();

        let hash_matches = crate::license_detection::hash_match::hash_match(index, &whole_run);
        let spdx_matches = crate::license_detection::spdx_lid::spdx_lid_match(index, &query);
        let aho_matches = aho_match(index, &whole_run);

        let mut all_matches = Vec::new();
        all_matches.extend(hash_matches);
        all_matches.extend(spdx_matches);
        all_matches.extend(aho_matches);

        eprintln!("Before refinement: {} matches", all_matches.len());
        for m in &all_matches {
            print_match_details(m, "Before-refine");
        }

        let refined = refine_matches(index, all_matches.clone(), &query);
        eprintln!("\nAfter full refine_matches: {} matches", refined.len());
        for m in &refined {
            print_match_details(m, "Refined");
        }

        let here_matches: Vec<_> = refined
            .iter()
            .filter(|m| m.license_expression.contains("here-proprietary"))
            .collect();
        eprintln!(
            "\nhere-proprietary matches after refinement: {}",
            here_matches.len()
        );

        assert_eq!(
            here_matches.len(),
            1,
            "Expected 1 here-proprietary match after refinement, got {}",
            here_matches.len()
        );
    }

    #[test]
    fn test_here_proprietary_token_position_bug() {
        use crate::license_detection::query::Query;

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let content = "SPDX-License-Identifier: LicenseRef-Proprietary-HERE\n";
        let index = engine.index();

        eprintln!("\n=== TOKEN POSITION BUG INVESTIGATION ===");

        let query = Query::new(content, index).expect("Query creation should succeed");

        eprintln!(
            "Query tokens ({} total): {:?}",
            query.tokens.len(),
            query.tokens
        );
        eprintln!("SPDX lines: {:?}", query.spdx_lines);

        if let Some((spdx_text, start_token, end_token)) = query.spdx_lines.first() {
            eprintln!("\nSPDX line analysis:");
            eprintln!("  spdx_text: {:?}", spdx_text);
            eprintln!("  start_token: {}", start_token);
            eprintln!("  end_token: {}", end_token);
            eprintln!(
                "  Tokens covered: {:?}",
                (*start_token..*end_token).collect::<Vec<_>>()
            );
            eprintln!();
            eprintln!(
                "BUG: end_token={} is the LAST token position (inclusive)",
                end_token
            );
            eprintln!(
                "But it should be {} (exclusive, one past the last)",
                end_token + 1
            );
            eprintln!("See: src/license_detection/query.rs:416-417");
            eprintln!("     spdx_end = line_last_known_pos as usize");
            eprintln!("This sets end_token to the last known position, not exclusive end.");
        }
    }
}
