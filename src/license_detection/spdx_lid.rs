//! SPDX-License-Identifier detection and parsing.
//!
//! This module handles detection of SPDX license identifier tags in source code,
//! such as "SPDX-License-Identifier: MIT" or variations with different comment
//! styles and casing.
//!
//! Based on Python implementation at:
//! reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py
//!
//! ## Signature Difference from Python
//!
//! The Rust `spdx_lid_match()` takes `(index, text)` instead of `(idx, query_run, text)`.
//!
//! **Why this differs from Python:**
//!
//! Python's `spdx_id_match()` is called per-SPDX-identifier occurrence via `Query.spdx_lines`,
//! using `query_run` for position tracking. The Rust implementation processes the entire
//! text at once via `extract_spdx_expressions_with_lines()`, computing line numbers directly
//! from the text during parsing.
//!
//! This approach is simpler (single call vs. per-line calls), produces identical output
//! (correct line numbers in matches), and avoids the complexity of tracking SPDX lines
//! during query tokenization. The functional result is equivalent to Python's behavior.

use regex::Regex;

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;

/// Matcher identifier for SPDX-License-Identifier based matching.
///
/// Corresponds to Python: `MATCH_SPDX_ID = '1-spdx-id'` (line 61)
pub const MATCH_SPDX_ID: &str = "1-spdx-id";

/// Matcher order for SPDX-License-Identifier based matching.
///
/// SPDX-LID matching runs after hash matching.
///
/// Corresponds to Python: `MATCH_SPDX_ID_ORDER = 2` (line 62)
#[allow(dead_code)]
pub const MATCH_SPDX_ID_ORDER: u8 = 1;

lazy_static::lazy_static! {
    static ref SPDX_LID_PATTERN: Regex = Regex::new(
        r"(?i)(spd[xz][\-\s]+lin?[cs]en?[sc]es?[\-\s]+identifi?er\s*:? *)"
    ).expect("Invalid SPDX-LID regex");

    static ref NUGET_SPDX_PATTERN: Regex = Regex::new(
        r"(?i)(https?://licenses\.nuget\.org/?)\s*:? *"
    ).expect("Invalid NuGet SPDX regex");
}

pub fn split_spdx_lid(text: &str) -> (Option<String>, String) {
    // Try SPDX pattern first
    if let Some(captures) = SPDX_LID_PATTERN.captures(text)
        && let Some(matched) = captures.get(1)
    {
        let prefix = matched.as_str().to_string();
        let expression = &text[matched.end()..];
        return (Some(prefix), expression.to_string());
    }

    // Try NuGet pattern
    if let Some(captures) = NUGET_SPDX_PATTERN.captures(text)
        && let Some(full_match) = captures.get(0)
    {
        let prefix = &text[..full_match.end()];
        let expression = &text[full_match.end()..];
        return (Some(prefix.to_string()), expression.to_string());
    }

    (None, text.to_string())
}

pub fn clean_spdx_text(text: &str) -> String {
    let mut text = text.to_string();

    text = text.replace("</a>", "");
    text = text.replace("</p>", "");
    text = text.replace("</div>", "");
    text = text.replace("</licenseUrl>", "");

    normalize_spaces(&mut text);

    strip_punctuation(&mut text);
    fix_unbalanced_parens(&mut text);

    if text.contains("\">") {
        let parts: Vec<&str> = text.split("\">").collect();
        if parts.len() > 1 && parts[1].contains(parts[0]) {
            text = parts[0].to_string();
        }
    }

    normalize_spaces(&mut text);

    text
}

#[allow(dead_code)]
pub fn extract_spdx_expressions(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let (prefix, expression) = split_spdx_lid(line.trim());
            prefix.as_ref()?;
            let cleaned = clean_spdx_text(&expression);
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .collect()
}

fn normalize_spaces(text: &mut String) {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    *text = normalized;
}

fn strip_punctuation(text: &mut String) {
    let punctuation = "!\"#$%&\'*,-./:;<=>?@[\\]^_`{|}~ \t\r\n ";

    while !text.is_empty()
        && text
            .chars()
            .next()
            .map(|c| punctuation.contains(c) || c == ')')
            .unwrap_or(false)
    {
        text.remove(0);
    }

    while !text.is_empty()
        && text
            .chars()
            .last()
            .map(|c| punctuation.contains(c) || c == '(')
            .unwrap_or(false)
    {
        text.pop();
    }
}

fn fix_unbalanced_parens(text: &mut String) {
    let open_count = text.matches('(').count();
    let close_count = text.matches(')').count();

    if open_count == 1 && close_count == 0 {
        *text = text.replace('(', " ");
    } else if close_count == 1 && open_count == 0 {
        *text = text.replace(')', " ");
    }
}

pub fn extract_spdx_expressions_with_lines(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            let line_num_1indexed = line_num + 1;
            let (prefix, expression) = split_spdx_lid(line.trim());
            prefix.as_ref()?;
            let cleaned = clean_spdx_text(&expression);
            if cleaned.is_empty() {
                None
            } else {
                Some((line_num_1indexed, cleaned))
            }
        })
        .collect()
}

fn normalize_spdx_key(key: &str) -> String {
    key.to_lowercase().replace("_", "-")
}

fn find_best_matching_rule(index: &LicenseIndex, spdx_key: &str) -> Option<usize> {
    let normalized_spdx = normalize_spdx_key(spdx_key);

    if let Some(&rid) = index.rid_by_spdx_key.get(&normalized_spdx) {
        return Some(rid);
    }

    let mut best_rid: Option<usize> = None;
    let mut best_relevance: u8 = 0;

    for (rid, rule) in index.rules_by_rid.iter().enumerate() {
        let license_expr = normalize_spdx_key(&rule.license_expression);

        if license_expr == normalized_spdx && rule.relevance > best_relevance {
            best_relevance = rule.relevance;
            best_rid = Some(rid);
        }
    }

    best_rid.or_else(|| {
        for (rid, rule) in index.rules_by_rid.iter().enumerate() {
            let license_expr = normalize_spdx_key(&rule.license_expression);
            if license_expr == normalized_spdx {
                return Some(rid);
            }
        }
        None
    })
}

fn split_license_expression(license_expression: &str) -> Vec<String> {
    let normalized = license_expression.replace(['(', ')'], " ");
    let mut tokens: Vec<String> = Vec::new();

    let mut current = String::new();
    for c in normalized.chars() {
        if c == ' ' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
        .into_iter()
        .filter(|t| {
            let t_lower = t.to_lowercase();
            !matches!(t_lower.as_str(), "and" | "or" | "with")
        })
        .collect()
}

fn extract_matched_text_from_lines(text: &str, start_line: usize, end_line: usize) -> String {
    if start_line == 0 || end_line == 0 || start_line > end_line {
        return String::new();
    }

    text.lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line_num = idx + 1;
            if line_num >= start_line && line_num <= end_line {
                Some(line)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn spdx_lid_match(index: &LicenseIndex, text: &str) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();

    let spdx_lines = extract_spdx_expressions_with_lines(text);

    for (line_num, spdx_expression) in spdx_lines {
        let license_keys = split_license_expression(&spdx_expression);

        for license_key in license_keys {
            if let Some(rid) = find_best_matching_rule(index, &license_key) {
                let rule = &index.rules_by_rid[rid];

                let score = rule.relevance as f32 / 100.0;
                let matched_length = spdx_expression.len();
                let match_coverage = 100.0;

                let matched_text = extract_matched_text_from_lines(text, line_num, line_num);

                let license_match = LicenseMatch {
                    license_expression: rule.license_expression.clone(),
                    license_expression_spdx: spdx_expression.clone(),
                    from_file: None,
                    start_line: line_num,
                    end_line: line_num,
                    matcher: MATCH_SPDX_ID.to_string(),
                    score,
                    matched_length,
                    match_coverage,
                    rule_relevance: rule.relevance,
                    rule_identifier: format!("#{}", rid),
                    rule_url: String::new(),
                    matched_text: Some(matched_text),
                    referenced_filenames: rule.referenced_filenames.clone(),
                    is_license_intro: rule.is_license_intro,
                    is_license_clue: rule.is_license_clue,
                };

                matches.push(license_match);
            }
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::test_utils::{create_mock_rule_simple, create_test_index};

    #[test]
    fn test_split_spdx_lid_standard() {
        let (prefix, expr) = split_spdx_lid("SPDX-License-Identifier: MIT");
        assert_eq!(prefix, Some("SPDX-License-Identifier: ".to_string()));
        assert_eq!(expr, "MIT");
    }

    #[test]
    fn test_split_spdx_lid_lowercase() {
        let (prefix, expr) = split_spdx_lid("spdx-license-identifier: MIT");
        assert!(prefix.is_some());
        assert_eq!(expr, "MIT");
    }

    #[test]
    fn test_split_spdx_lid_with_spaces() {
        let (prefix, expr) = split_spdx_lid("SPDX license identifier: Apache-2.0");
        assert!(prefix.is_some());
        assert_eq!(expr, "Apache-2.0");
    }

    #[test]
    fn test_split_spdx_lid_without_colon() {
        let (prefix, expr) = split_spdx_lid("SPDX-License-Identifier MIT");
        assert!(prefix.is_some());
        assert_eq!(expr, "MIT");
    }

    #[test]
    fn test_split_spdx_lid_nuget() {
        let (prefix, expr) = split_spdx_lid("https://licenses.nuget.org/MIT");
        assert!(prefix.is_some());
        assert_eq!(expr, "MIT");
    }

    #[test]
    fn test_split_spdx_lid_no_match() {
        let (prefix, expr) = split_spdx_lid("No SPDX here");
        assert_eq!(prefix, None);
        assert_eq!(expr, "No SPDX here");
    }

    #[test]
    fn test_split_spdx_lid_complex_expression() {
        let (prefix, expr) = split_spdx_lid(
            "SPDX-License-Identifier: GPL-2.0-or-later WITH Classpath-exception-2.0",
        );
        assert!(prefix.is_some());
        assert_eq!(expr, "GPL-2.0-or-later WITH Classpath-exception-2.0");
    }

    #[test]
    fn test_clean_spdx_text_basic() {
        let clean = clean_spdx_text("MIT");
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_clean_spdx_text_extra_spaces() {
        let clean = clean_spdx_text("  MIT   Apache-2.0  ");
        assert_eq!(clean, "MIT Apache-2.0");
    }

    #[test]
    fn test_clean_spdx_text_dangling_markup() {
        let clean = clean_spdx_text("MIT</a>");
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_clean_spdx_text_multiple_dangling_markup() {
        let clean = clean_spdx_text("MIT</a></p></div>");
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_clean_spdx_text_leading_punctuation() {
        let clean = clean_spdx_text("!MIT");
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_clean_spdx_text_trailing_punctuation() {
        let clean = clean_spdx_text("MIT.");
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_clean_spdx_text_lone_open_paren() {
        let clean = clean_spdx_text("(MIT");
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_clean_spdx_text_lone_close_paren() {
        let clean = clean_spdx_text("MIT)");
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_clean_spdx_text_balanced_parens() {
        let clean = clean_spdx_text("(MIT OR Apache-2.0)");
        assert_eq!(clean, "(MIT OR Apache-2.0)");
    }

    #[test]
    fn test_clean_spdx_text_tabs_and_newlines() {
        let clean = clean_spdx_text("MIT\tApache-2.0\nGPL-2.0");
        assert_eq!(clean, "MIT Apache-2.0 GPL-2.0");
    }

    #[test]
    fn test_extract_spdx_expressions_single() {
        let text = "# SPDX-License-Identifier: MIT";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["MIT"]);
    }

    #[test]
    fn test_extract_spdx_expressions_multiple() {
        let text = "# SPDX-License-Identifier: MIT\n# SPDX-License-Identifier: Apache-2.0";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs.len(), 2);
        assert!(exprs.contains(&"MIT".to_string()));
        assert!(exprs.contains(&"Apache-2.0".to_string()));
    }

    #[test]
    fn test_extract_spdx_expressions_complex() {
        let text = "// SPDX-License-Identifier: GPL-2.0-or-later WITH Classpath-exception-2.0";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["GPL-2.0-or-later WITH Classpath-exception-2.0"]);
    }

    #[test]
    fn test_extract_spdx_expressions_spaces_hyphens() {
        let text = "* SPDX license identifier: BSD-3-Clause";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["BSD-3-Clause"]);
    }

    #[test]
    fn test_extract_spdx_expressions_html_comment() {
        let text = "<!-- SPDX-License-Identifier: MIT -->";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["MIT"]);
    }

    #[test]
    fn test_extract_spdx_expressions_python_comment() {
        let text = "# SPDX-License-Identifier: (MIT OR Apache-2.0)";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["(MIT OR Apache-2.0)"]);
    }

    #[test]
    fn test_extract_spdx_expressions_with_whitespace() {
        let text = "  //  SPDX-License-Identifier:   MIT  ";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["MIT"]);
    }

    #[test]
    fn test_extract_spdx_expressions_no_match() {
        let text = "/* This is a regular comment */";
        let exprs = extract_spdx_expressions(text);
        assert!(exprs.is_empty());
    }

    #[test]
    fn test_extract_spdx_expressions_nuget_url() {
        let text = "<licenseUrl>https://licenses.nuget.org/MIT</licenseUrl>";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["MIT"]);
    }

    #[test]
    fn test_clean_spdx_text_json_like() {
        let clean = clean_spdx_text(r#""MIT">MIT"#);
        assert_eq!(clean, "MIT");
    }

    #[test]
    fn test_split_spdx_lid_case_variants() {
        let tests: [&str; 4] = [
            "SPDX-License-Identifier: MIT",
            "spdx-license-identifier: MIT",
            "SPDX-LICENSE-IDENTIFIER: MIT",
            "Spdx-License-Identifier: MIT",
        ];

        for test in tests {
            let (prefix, expr) = split_spdx_lid(test);
            assert!(prefix.is_some(), "Should match: {}", test);
            assert_eq!(expr, "MIT");
        }
    }

    #[test]
    fn test_extract_spdx_expressions_preserves_complex_expressions() {
        let text = "SPDX-License-Identifier: (EPL-2.0 OR Apache-2.0 OR GPL-2.0 WITH Classpath-exception-2.0)";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(
            exprs,
            vec!["(EPL-2.0 OR Apache-2.0 OR GPL-2.0 WITH Classpath-exception-2.0)"]
        );
    }

    #[test]
    fn test_extract_spdx_expressions_with_lines() {
        let text = "# SPDX-License-Identifier: MIT\n# SPDX-License-Identifier: Apache-2.0";
        let exprs = extract_spdx_expressions_with_lines(text);
        assert_eq!(exprs.len(), 2);
        assert_eq!(exprs[0], (1, "MIT".to_string()));
        assert_eq!(exprs[1], (2, "Apache-2.0".to_string()));
    }

    #[test]
    fn test_extract_spdx_expressions_with_lines_single() {
        let text = "// SPDX-License-Identifier: GPL-2.0-or-later";
        let exprs = extract_spdx_expressions_with_lines(text);
        assert_eq!(exprs.len(), 1);
        assert_eq!(exprs[0].0, 1);
        assert_eq!(exprs[0].1, "GPL-2.0-or-later");
    }

    #[test]
    fn test_extract_spdx_expressions_with_lines_no_match() {
        let text = "/* Regular comment with no SPDX identifier */";
        let exprs = extract_spdx_expressions_with_lines(text);
        assert!(exprs.is_empty());
    }

    #[test]
    fn test_normalize_spdx_key() {
        assert_eq!(normalize_spdx_key("MIT"), "mit");
        assert_eq!(normalize_spdx_key("Apache-2.0"), "apache-2.0");
        assert_eq!(normalize_spdx_key("GPL_2.0_plus"), "gpl-2.0-plus");
        assert_eq!(normalize_spdx_key("gPL-2.0-PLUS"), "gpl-2.0-plus");
    }

    #[test]
    fn test_split_license_expression_simple() {
        let expr = "MIT";
        let keys = split_license_expression(expr);
        assert_eq!(keys, vec!["MIT"]);
    }

    #[test]
    fn test_split_license_expression_with_or() {
        let expr = "MIT OR Apache-2.0";
        let keys = split_license_expression(expr);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"MIT".to_string()));
        assert!(keys.contains(&"Apache-2.0".to_string()));
    }

    #[test]
    fn test_split_license_expression_with_and() {
        let expr = "GPL-2.0 AND Classpath-exception-2.0";
        let keys = split_license_expression(expr);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"GPL-2.0".to_string()));
        assert!(keys.contains(&"Classpath-exception-2.0".to_string()));
    }

    #[test]
    fn test_split_license_expression_with_parens() {
        let expr = "(MIT OR Apache-2.0)";
        let keys = split_license_expression(expr);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"MIT".to_string()));
        assert!(keys.contains(&"Apache-2.0".to_string()));
    }

    #[test]
    fn test_split_license_expression_complex() {
        let expr = "GPL-2.0-or-later WITH Classpath-exception-2.0";
        let keys = split_license_expression(expr);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"GPL-2.0-or-later".to_string()));
        assert!(keys.contains(&"Classpath-exception-2.0".to_string()));
    }

    #[test]
    fn test_spdx_lid_match_simple() {
        let mut index = create_test_index(&[("mit", 0), ("license", 1)], 1);
        index.rules_by_rid.push(create_mock_rule_simple("mit", 100));
        index
            .rules_by_rid
            .push(create_mock_rule_simple("apache-2.0", 100));

        let text = "SPDX-License-Identifier: MIT";
        let matches = spdx_lid_match(&index, text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].license_expression, "mit");
        assert_eq!(matches[0].license_expression_spdx, "MIT");
        assert_eq!(matches[0].start_line, 1);
        assert_eq!(matches[0].end_line, 1);
        assert_eq!(matches[0].matcher, MATCH_SPDX_ID);
    }

    #[test]
    fn test_spdx_lid_match_case_insensitive() {
        let mut index = create_test_index(&[("mit", 0)], 1);
        index.rules_by_rid.push(create_mock_rule_simple("mit", 90));

        let text = "SPDX-License-Identifier: mit";
        let matches = spdx_lid_match(&index, text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].license_expression, "mit");
    }

    #[test]
    fn test_spdx_lid_match_multiple() {
        let mut index = create_test_index(&[("mit", 0), ("license", 1)], 1);
        index.rules_by_rid.push(create_mock_rule_simple("mit", 100));
        index
            .rules_by_rid
            .push(create_mock_rule_simple("apache-2.0", 100));

        let text = "SPDX-License-Identifier: OR\n# SPDX-License-Identifier: MIT\n# SPDX-License-Identifier: Apache-2.0";
        let matches = spdx_lid_match(&index, text);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_spdx_lid_match_no_match() {
        let index = create_test_index(&[("mit", 0)], 1);

        let text = "/* Regular comment */";
        let matches = spdx_lid_match(&index, text);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_spdx_lid_match_score_from_relevance() {
        let mut index = create_test_index(&[("mit", 0)], 1);
        index.rules_by_rid.push(create_mock_rule_simple("mit", 80));

        let text = "SPDX-License-Identifier: MIT";
        let matches = spdx_lid_match(&index, text);

        assert_eq!(matches.len(), 1);
        assert!((matches[0].score - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_split_license_expression_with_with() {
        let expr = "GPL-2.0 WITH Classpath-exception-2.0";
        let keys = split_license_expression(expr);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"GPL-2.0".to_string()));
        assert!(keys.contains(&"Classpath-exception-2.0".to_string()));
    }

    #[test]
    fn test_split_license_expression_with_plus() {
        let expr = "GPL-2.0+";
        let keys = split_license_expression(expr);
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"GPL-2.0+".to_string()));
    }

    #[test]
    fn test_split_license_expression_complex_with_operators() {
        let expr = "(MIT OR Apache-2.0) AND BSD-3-Clause";
        let keys = split_license_expression(expr);
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"MIT".to_string()));
        assert!(keys.contains(&"Apache-2.0".to_string()));
        assert!(keys.contains(&"BSD-3-Clause".to_string()));
    }

    #[test]
    fn test_clean_spdx_text_empty_result() {
        let clean = clean_spdx_text("");
        assert_eq!(clean, "");

        let clean = clean_spdx_text("   ");
        assert_eq!(clean, "");
    }

    #[test]
    fn test_extract_spdx_expressions_with_plus() {
        let text = "SPDX-License-Identifier: GPL-2.0+";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["GPL-2.0+"]);
    }

    #[test]
    fn test_extract_spdx_expressions_with_with_operator() {
        let text = "SPDX-License-Identifier: GPL-2.0 WITH Classpath-exception-2.0";
        let exprs = extract_spdx_expressions(text);
        assert_eq!(exprs, vec!["GPL-2.0 WITH Classpath-exception-2.0"]);
    }

    #[test]
    fn test_spdx_lid_match_with_operator() {
        let mut index = create_test_index(&[("mit", 0)], 1);
        index
            .rules_by_rid
            .push(create_mock_rule_simple("gpl-2.0", 100));
        index
            .rules_by_rid
            .push(create_mock_rule_simple("classpath-exception-2.0", 100));

        let text = "SPDX-License-Identifier: GPL-2.0 WITH Classpath-exception-2.0";
        let matches = spdx_lid_match(&index, text);

        assert!(
            !matches.is_empty(),
            "Should match WITH expression components"
        );
    }

    #[test]
    fn test_extract_spdx_expressions_multiple_on_same_line() {
        let text = "SPDX-License-Identifier: MIT  SPDX-License-Identifier: Apache-2.0";
        let exprs = extract_spdx_expressions(text);

        assert!(!exprs.is_empty(), "Should extract at least one expression");
    }

    #[test]
    fn test_clean_spdx_text_with_angle_brackets() {
        let clean = clean_spdx_text("<MIT>");
        assert!(!clean.contains('<'));
        assert!(!clean.contains('>'));
    }

    #[test]
    fn test_split_spdx_lid_typo_variants() {
        let variants = [
            "SPDX-License-Identifier: MIT",
            "SPDX-License-Identifier MIT",
            "SPDX-License-Identifier:  MIT",
            "SPDX license identifier: MIT",
            "SPDX Licence Identifier: MIT",
            "SPDZ-License-Identifier: MIT",
        ];

        for variant in variants {
            let (prefix, expr) = split_spdx_lid(variant);
            assert!(prefix.is_some(), "Should match variant: {}", variant);
            assert_eq!(expr, "MIT", "Should extract MIT from: {}", variant);
        }
    }

    #[test]
    fn test_spdx_lid_match_empty_text() {
        let index = create_test_index(&[("mit", 0)], 1);

        let text = "";
        let matches = spdx_lid_match(&index, text);

        assert!(matches.is_empty(), "Empty text should produce no matches");
    }

    #[test]
    fn test_spdx_lid_match_whitespace_only() {
        let index = create_test_index(&[("mit", 0)], 1);

        let text = "   \n\t  ";
        let matches = spdx_lid_match(&index, text);

        assert!(
            matches.is_empty(),
            "Whitespace-only text should produce no matches"
        );
    }

    #[test]
    fn test_extract_matched_text_from_lines() {
        let text = "line1\nline2\nline3\nline4\nline5";

        let matched = extract_matched_text_from_lines(text, 2, 2);
        assert_eq!(matched, "line2");

        let matched = extract_matched_text_from_lines(text, 2, 4);
        assert_eq!(matched, "line2\nline3\nline4");

        let matched = extract_matched_text_from_lines(text, 0, 2);
        assert_eq!(matched, "");

        let matched = extract_matched_text_from_lines(text, 3, 1);
        assert_eq!(matched, "");
    }

    #[test]
    fn test_normalize_spdx_key_edge_cases() {
        assert_eq!(normalize_spdx_key(""), "");
        assert_eq!(normalize_spdx_key("MIT"), "mit");
        assert_eq!(normalize_spdx_key("MIT_LICENSE"), "mit-license");
        assert_eq!(normalize_spdx_key("MIT__LICENSE"), "mit--license");
    }

    #[test]
    fn test_spdx_key_lookup_gpl_2_0_plus() {
        let path =
            std::path::Path::new("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let licenses = crate::license_detection::rules::load_licenses_from_directory(path, false)
            .expect("Failed to load licenses");
        let rules = crate::license_detection::rules::load_rules_from_directory(
            std::path::Path::new("reference/scancode-toolkit/src/licensedcode/data/rules"),
            false,
        )
        .expect("Failed to load rules");

        let index = crate::license_detection::index::build_index(rules, licenses);

        assert!(
            !index.rid_by_spdx_key.is_empty(),
            "Should have SPDX key mappings"
        );

        assert!(
            index.rid_by_spdx_key.contains_key("gpl-2.0+"),
            "Should have gpl-2.0+ in SPDX key mappings"
        );

        if let Some(&rid) = index.rid_by_spdx_key.get("gpl-2.0+") {
            let rule = &index.rules_by_rid[rid];
            assert_eq!(
                rule.license_expression, "gpl-2.0-plus",
                "GPL-2.0+ should map to gpl-2.0-plus rule"
            );
        }
    }
}
