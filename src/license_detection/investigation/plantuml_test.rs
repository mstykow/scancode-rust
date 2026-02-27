//! Investigation test for PLAN-006: plantuml_license_notice.txt
//!
//! Issue: Expression wrapped in extra parentheses.
//!
//! Expected: `["mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus"]`
//! Actual: `["(mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)"]`
//!
//! ROOT CAUSE IDENTIFIED:
//! =====================
//! The rule file `plantuml_1.RULE` contains the expression with outer parentheses:
//!   license_expression: (mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)
//!
//! Python normalizes expressions (removes unnecessary outer parens), but Rust
//! stores the expression as-is from the rule file.
//!
//! FIX LOCATION: Expression normalization should happen either:
//! 1. During rule loading (in rules/loader.rs)
//! 2. During expression parsing (in expression.rs)

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
            PathBuf::from("testdata/license-golden/datadriven/lic4/plantuml_license_notice.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_plantuml_expression_no_extra_parens() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("=== Detection Results ===");
        eprintln!("Number of detections: {}", detections.len());

        for (i, d) in detections.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i + 1, d.license_expression);
            eprintln!("  Detection log: {:?}", d.detection_log);
            for m in &d.matches {
                eprintln!(
                    "  Match: {} at lines {}-{} matcher={} score={:.1} rule={}",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.matcher,
                    m.score,
                    m.rule_identifier
                );
            }
        }

        let actual: Vec<&str> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        let expected = vec!["mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus"];

        assert_eq!(
            actual, expected,
            "Expression mismatch:\n  Expected: {:?}\n  Actual:   {:?}",
            expected, actual
        );
    }

    #[test]
    fn test_plantuml_rule_expression_has_extra_parens() {
        use crate::license_detection::index::build_index;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };

        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        let plantuml_rule = index
            .rules_by_rid
            .iter()
            .find(|r| r.identifier == "plantuml_1.RULE")
            .expect("plantuml_1.RULE should exist");

        eprintln!("=== DIVERGENCE POINT IDENTIFIED ===");
        eprintln!("Rule identifier: {}", plantuml_rule.identifier);
        eprintln!(
            "Rule license_expression: {:?}",
            plantuml_rule.license_expression
        );

        assert_eq!(
            plantuml_rule.license_expression,
            "mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus",
            "FAIL: Rule license_expression has extra parentheses: {:?}",
            plantuml_rule.license_expression
        );
    }

    #[test]
    fn test_rule_file_has_parens() {
        let rule_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules/plantuml_1.RULE");

        let content = std::fs::read_to_string(&rule_path).expect("Should read rule file");

        eprintln!("=== Rule file content (first 500 chars) ===");
        eprintln!("{}", content.chars().take(500).collect::<String>());

        assert!(
            content.contains("license_expression: (mit OR apache-2.0"),
            "Rule file should contain expression with parens"
        );
    }

    #[test]
    fn test_expression_parse_normalizes_outer_parens() {
        use crate::license_detection::expression::{expression_to_string, parse_expression};

        let input_with_parens = "(mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)";
        let expr = parse_expression(input_with_parens).expect("Parse should succeed");
        let output = expression_to_string(&expr);

        eprintln!("Input: {:?}", input_with_parens);
        eprintln!("Parsed and rendered: {:?}", output);

        assert_eq!(
            output, "mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus",
            "FAIL: Expression parser should normalize away unnecessary outer parentheses"
        );
    }

    #[test]
    fn test_expression_parentheses_roundtrip() {
        use crate::license_detection::expression::{expression_to_string, parse_expression};

        let input = "mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus";
        let expr = parse_expression(input).expect("Parse should succeed");
        let output = expression_to_string(&expr);

        eprintln!("Input: {:?}", input);
        eprintln!("Parsed and rendered: {:?}", output);

        assert_eq!(
            output, input,
            "Expression should round-trip without extra parentheses"
        );
    }

    #[test]
    fn test_combine_expressions_single_or() {
        use crate::license_detection::expression::{combine_expressions, CombineRelation};

        let expressions = vec!["mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus"];
        let combined = combine_expressions(&expressions, CombineRelation::And, true)
            .expect("Combine should succeed");

        eprintln!("Combined single OR expression: {:?}", combined);

        assert_eq!(
            combined, "mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus",
            "Single OR expression should not be wrapped in parens"
        );
    }

    #[test]
    fn test_combine_expressions_normalizes_input_with_parens() {
        use crate::license_detection::expression::{combine_expressions, CombineRelation};

        let expressions = vec!["(mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)"];
        let combined = combine_expressions(&expressions, CombineRelation::And, true)
            .expect("Combine should succeed");

        eprintln!("Input expression with parens: {:?}", expressions[0]);
        eprintln!("Combined expression: {:?}", combined);

        assert_eq!(
            combined, "mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus",
            "FAIL: combine_expressions should normalize away unnecessary outer parentheses from input"
        );
    }

    #[test]
    fn test_sencha_expression_current_behavior() {
        use crate::license_detection::expression::{expression_to_string, parse_expression};

        let input = "(gpl-3.0 WITH sencha-app-floss-exception OR gpl-3.0 WITH sencha-dev-floss-exception OR sencha-commercial) AND (public-domain AND mit AND mit)";
        let expr = parse_expression(input).expect("Parse should succeed");
        let output = expression_to_string(&expr);

        eprintln!("Sencha input: {:?}", input);
        eprintln!("Sencha output: {:?}", output);

        assert_eq!(
            output,
            "(gpl-3.0 WITH sencha-app-floss-exception OR gpl-3.0 WITH sencha-dev-floss-exception OR sencha-commercial) AND public-domain AND mit AND mit",
            "FAIL: Current behavior loses stylistic parens on right side"
        );
    }

    #[test]
    fn test_plantuml_expression_removes_trivial_outer_parens() {
        use crate::license_detection::expression::{expression_to_string, parse_expression};

        let input_with_parens = "(mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)";
        let input_without_parens = "mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus";

        let expr = parse_expression(input_with_parens).expect("Parse should succeed");
        let output = expression_to_string(&expr);

        eprintln!("PlantUML input: {:?}", input_with_parens);
        eprintln!("PlantUML output: {:?}", output);

        assert_eq!(
            output, input_without_parens,
            "FAIL: PlantUML expression should have trivial outer parens removed"
        );
    }

    #[test]
    fn test_normalization_heuristic_trivial_outer_only_first() {
        use crate::license_detection::expression::{expression_to_string, parse_expression};

        let cases = vec![
            ("(mit)", "mit"),
            ("(mit OR apache-2.0)", "mit OR apache-2.0"),
            ("((mit OR apache-2.0))", "mit OR apache-2.0"),
            ("(mit AND apache-2.0)", "mit AND apache-2.0"),
            ("(gpl-3.0 WITH exception)", "gpl-3.0 WITH exception"),
        ];

        for (input, expected) in cases {
            let expr =
                parse_expression(input).expect(&format!("Parse should succeed for: {}", input));
            let output = expression_to_string(&expr);
            assert_eq!(
                output, expected,
                "Trivial outer parens should be removed for: {}",
                input
            );
        }
    }

    #[test]
    fn test_semantically_required_grouping_preserved() {
        use crate::license_detection::expression::{expression_to_string, parse_expression};

        let cases = vec![
            (
                "(mit OR apache-2.0) AND gpl-3.0",
                "(mit OR apache-2.0) AND gpl-3.0",
            ),
            (
                "mit AND (apache-2.0 OR gpl-3.0)",
                "mit AND (apache-2.0 OR gpl-3.0)",
            ),
            (
                "(mit AND apache-2.0) OR gpl-3.0",
                "(mit AND apache-2.0) OR gpl-3.0",
            ),
            ("(a OR b) AND (c OR d)", "(a OR b) AND (c OR d)"),
        ];

        for (input, expected) in cases {
            let expr =
                parse_expression(input).expect(&format!("Parse should succeed for: {}", input));
            let output = expression_to_string(&expr);
            assert_eq!(
                output, expected,
                "Semantic grouping parens should be preserved for: {}",
                input
            );
        }
    }

    #[test]
    fn test_stylistic_parens_lost_by_parser() {
        use crate::license_detection::expression::{expression_to_string, parse_expression};

        let cases = vec![
            (
                "(gpl-3.0 WITH exception) OR mit",
                "gpl-3.0 WITH exception OR mit",
            ),
            (
                "(public-domain AND mit AND mit)",
                "public-domain AND mit AND mit",
            ),
        ];

        for (input, expected) in cases {
            let expr =
                parse_expression(input).expect(&format!("Parse should succeed for: {}", input));
            let output = expression_to_string(&expr);
            assert_eq!(
                output, expected,
                "Stylistic parens are lost (current behavior): {}",
                input
            );
        }
    }

    #[test]
    fn test_is_trivial_outer_parens_heuristic() {
        fn has_trivial_outer_parens(s: &str) -> bool {
            let trimmed = s.trim();
            if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
                return false;
            }
            let mut depth = 0;
            let chars: Vec<char> = trimmed.chars().collect();
            for (i, c) in chars.iter().enumerate() {
                if *c == '(' {
                    depth += 1;
                } else if *c == ')' {
                    depth -= 1;
                    if depth == 0 && i < chars.len() - 1 {
                        return false;
                    }
                }
            }
            depth == 0
        }

        assert!(has_trivial_outer_parens("(mit)"));
        assert!(has_trivial_outer_parens("(mit OR apache-2.0)"));
        assert!(has_trivial_outer_parens("((mit OR apache-2.0))"));
        assert!(has_trivial_outer_parens("(mit AND apache-2.0)"));
        assert!(has_trivial_outer_parens("(gpl-3.0 WITH exception)"));
        assert!(has_trivial_outer_parens("(mit OR apache-2.0 OR epl-2.0)"));

        assert!(!has_trivial_outer_parens("(mit OR apache-2.0) AND gpl-3.0"));
        assert!(!has_trivial_outer_parens("mit AND (apache-2.0 OR gpl-3.0)"));
        assert!(!has_trivial_outer_parens("(a OR b) AND (c OR d)"));
        assert!(!has_trivial_outer_parens("mit OR apache-2.0"));
    }
}
