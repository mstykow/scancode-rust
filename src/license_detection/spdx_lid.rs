//! SPDX-License-Identifier detection and parsing.
//!
//! This module handles detection of SPDX license identifier tags in source code,
//! such as "SPDX-License-Identifier: MIT" or variations with different comment
//! styles and casing.
//!
//! Based on Python implementation at:
//! reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py

use regex::Regex;

lazy_static::lazy_static! {
    static ref SPDX_LID_PATTERN: Regex = Regex::new(
        r"(?i)(spd[xz][\-\s]+lin?[cs]en?[sc]es?[\-\s]+identifi?er\s*:? *)"
    ).expect("Invalid SPDX-LID regex");

    static ref NUGET_SPDX_PATTERN: Regex = Regex::new(
        r"(?i)(https?://licenses\.nuget\.org/?)\s*:? *"
    ).expect("Invalid NuGet SPDX regex");
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn normalize_spaces(text: &mut String) {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    *text = normalized;
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn fix_unbalanced_parens(text: &mut String) {
    let open_count = text.matches('(').count();
    let close_count = text.matches(')').count();

    if open_count == 1 && close_count == 0 {
        *text = text.replace('(', " ");
    } else if close_count == 1 && open_count == 0 {
        *text = text.replace(')', " ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
