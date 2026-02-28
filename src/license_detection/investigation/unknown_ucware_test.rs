//! Investigation test for PLAN-017: unknown/ucware-eula.txt
//!
//! ## Issue
//! **Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "swrule"]`
//! **Actual:** `["unknown", "warranty-disclaimer", "unknown"]`
//!
//! ## Differences
//! - Position 1: Expected `unknown-license-reference`, Actual `unknown`
//! - Position 2: Expected `unknown-license-reference`, **MISSING**
//! - Position 4: Match (`warranty-disclaimer`)
//! - Position 5: Expected `unknown`, Actual `unknown` (may be different lines)
//! - Position 6: Expected `swrule`, **MISSING**
//! - **Rust is missing 3 matches total** and has wrong expression for position 1

#[cfg(test)]
mod tests {
    use crate::license_detection::aho_match::aho_match;
    use crate::license_detection::hash_match::hash_match;
    use crate::license_detection::index::build_index;
    use crate::license_detection::match_refine::refine_matches;
    use crate::license_detection::query::Query;
    use crate::license_detection::rules::{
        load_licenses_from_directory, load_rules_from_directory,
    };
    use crate::license_detection::seq_match::{
        compute_candidates_with_msets, seq_match_with_candidates,
    };
    use crate::license_detection::spdx_lid::spdx_lid_match;
    use crate::license_detection::unknown_match::unknown_match;
    use crate::utils::text::strip_utf8_bom_str;
    use std::path::PathBuf;

    fn get_engine() -> Option<crate::license_detection::index::LicenseIndex> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }

        let rules_path = data_path.join("rules");
        let licenses_path = data_path.join("licenses");

        let rules = load_rules_from_directory(&rules_path, false).ok()?;
        let licenses = load_licenses_from_directory(&licenses_path, false).ok()?;
        let index = build_index(rules, licenses);

        Some(index)
    }

    fn read_test_file() -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/unknown/ucware-eula.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_plan_017_rust_detection() {
        let Some(index) = get_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        let mut all_matches = Vec::new();
        all_matches.extend(hash_match(&index, &whole_run));
        all_matches.extend(spdx_lid_match(&index, &query));
        all_matches.extend(aho_match(&index, &whole_run));

        // Use 70 candidates like the main detection pipeline (swrule is at position 68)
        let candidates = compute_candidates_with_msets(&index, &whole_run, false, 70);
        all_matches.extend(seq_match_with_candidates(&index, &whole_run, &candidates));
        all_matches.extend(unknown_match(&index, &query, &all_matches));

        eprintln!("\n=== PHASE 1 RAW MATCHES ===");
        eprintln!("Total: {}", all_matches.len());
        for m in &all_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={}, rule={})",
                m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
            );
        }

        let refined = refine_matches(&index, all_matches.clone(), &query);
        eprintln!("\n=== AFTER refine_matches ===");
        eprintln!("Count: {}", refined.len());
        for m in &refined {
            eprintln!(
                "  {} at lines {}-{} (matcher={}, rule={})",
                m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
            );
        }

        let expressions: Vec<_> = refined
            .iter()
            .map(|m| m.license_expression.as_str())
            .collect();
        eprintln!("\n=== FINAL EXPRESSIONS ===");
        eprintln!("{:?}", expressions);

        let expected = vec![
            "unknown-license-reference",
            "unknown-license-reference",
            "unknown",
            "warranty-disclaimer",
            "unknown",
            "swrule",
        ];

        assert_eq!(
            expressions,
            expected,
            "Expected 6 matches, got {}: {:?}",
            expressions.len(),
            expressions
        );
    }

    #[test]
    fn test_plan_017_phase1_breakdown() {
        let Some(index) = get_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        let clean_text = strip_utf8_bom_str(&text);
        let query = Query::new(clean_text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        eprintln!("\n=== PHASE 1 MATCH BREAKDOWN ===");

        let hash_matches = hash_match(&index, &whole_run);
        eprintln!("\nHash matches: {}", hash_matches.len());
        for m in &hash_matches {
            eprintln!(
                "  {} at lines {}-{} (rule={})",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        let spdx_matches = spdx_lid_match(&index, &query);
        eprintln!("\nSPDX-LID matches: {}", spdx_matches.len());
        for m in &spdx_matches {
            eprintln!(
                "  {} at lines {}-{} (rule={})",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        let aho_matches = aho_match(&index, &whole_run);
        eprintln!("\nAho matches: {}", aho_matches.len());
        for m in &aho_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={}, rule={})",
                m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
            );
        }

        let seq_matches = {
            let candidates = compute_candidates_with_msets(&index, &whole_run, false, 70);
            seq_match_with_candidates(&index, &whole_run, &candidates)
        };
        eprintln!("\nSeq (approximate) matches: {}", seq_matches.len());
        for m in &seq_matches {
            eprintln!(
                "  {} at lines {}-{} (matcher={}, rule={})",
                m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
            );
        }

        let mut all_phase1 = Vec::new();
        all_phase1.extend(hash_matches.clone());
        all_phase1.extend(spdx_matches.clone());
        all_phase1.extend(aho_matches.clone());
        all_phase1.extend(seq_matches.clone());

        let unknown_matches_phase1 = unknown_match(&index, &query, &all_phase1);
        eprintln!("\nUnknown matches: {}", unknown_matches_phase1.len());
        for m in &unknown_matches_phase1 {
            eprintln!(
                "  {} at lines {}-{} (matcher={}, rule={})",
                m.license_expression, m.start_line, m.end_line, m.matcher, m.rule_identifier
            );
        }

        all_phase1.extend(unknown_matches_phase1);

        eprintln!("\n=== ALL PHASE 1 MATCHES ===");
        eprintln!("Total: {}", all_phase1.len());

        let unknown_license_ref_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression.contains("unknown-license-reference"))
            .collect();
        eprintln!(
            "\nunknown-license-reference matches: {}",
            unknown_license_ref_matches.len()
        );
        for m in &unknown_license_ref_matches {
            eprintln!(
                "  {} at lines {}-{} (rule={})",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        let swrule_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression.contains("swrule"))
            .collect();
        eprintln!("\nswrule matches: {}", swrule_matches.len());
        for m in &swrule_matches {
            eprintln!(
                "  {} at lines {}-{} (rule={})",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        let warranty_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression.contains("warranty-disclaimer"))
            .collect();
        eprintln!("\nwarranty-disclaimer matches: {}", warranty_matches.len());
        for m in &warranty_matches {
            eprintln!(
                "  {} at lines {}-{} (rule={})",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }

        let unknown_matches: Vec<_> = all_phase1
            .iter()
            .filter(|m| m.license_expression == "unknown")
            .collect();
        eprintln!("\nunknown matches: {}", unknown_matches.len());
        for m in &unknown_matches {
            eprintln!(
                "  {} at lines {}-{} (rule={})",
                m.license_expression, m.start_line, m.end_line, m.rule_identifier
            );
        }
    }

    #[test]
    fn test_plan_017_search_rules_in_index() {
        let Some(index) = get_engine() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        eprintln!("\n=== RULES IN INDEX ===");

        let unknown_license_ref_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("unknown-license-reference"))
            .collect();
        eprintln!(
            "\nunknown-license-reference rules: {}",
            unknown_license_ref_rules.len()
        );
        for r in &unknown_license_ref_rules {
            eprintln!("  {} -> {}", r.identifier, r.license_expression);
        }

        let swrule_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("swrule"))
            .collect();
        eprintln!("\nswrule rules: {}", swrule_rules.len());
        for r in &swrule_rules {
            eprintln!("  {} -> {}", r.identifier, r.license_expression);
        }

        let warranty_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("warranty-disclaimer"))
            .collect();
        eprintln!("\nwarranty-disclaimer rules: {}", warranty_rules.len());
        for r in &warranty_rules {
            eprintln!("  {} -> {}", r.identifier, r.license_expression);
        }

        let license_intro_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.identifier.contains("license-intro"))
            .collect();
        eprintln!("\nlicense-intro rules: {}", license_intro_rules.len());
        for r in &license_intro_rules {
            eprintln!("  {} -> {}", r.identifier, r.license_expression);
        }
    }

    #[test]
    fn test_plan_017_text_analysis() {
        let Some(text) = read_test_file() else {
            eprintln!("Skipping test: test file not found");
            return;
        };

        eprintln!("\n=== TEXT ANALYSIS ===");
        eprintln!("Total lines: {}", text.lines().count());
        eprintln!("Total chars: {}", text.len());

        for (i, line) in text.lines().enumerate() {
            eprintln!("Line {}: {:?}", i + 1, line);
        }

        eprintln!("\n=== LOOKING FOR KEY PHRASES ===");

        if text.contains("SOFTWARE LICENSE AGREEMENT") {
            eprintln!("Found: SOFTWARE LICENSE AGREEMENT");
        }
        if text.contains("This user license agreement") {
            eprintln!("Found: This user license agreement");
        }
        if text.contains("NOTICE TO USERS") {
            eprintln!("Found: NOTICE TO USERS");
        }
        if text.contains("THE SOFTWARE IS DISTRIBUTED \"AS IS\"") {
            eprintln!("Found: THE SOFTWARE IS DISTRIBUTED \"AS IS\"");
        }
        if text.contains("NO WARRANTY OF ANY KIND") {
            eprintln!("Found: NO WARRANTY OF ANY KIND");
        }
    }
}
