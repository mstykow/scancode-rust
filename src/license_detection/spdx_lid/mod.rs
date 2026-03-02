//! SPDX-License-Identifier detection and parsing.
//!
//! This module handles detection of SPDX license identifier tags in source code,
//! such as "SPDX-License-Identifier: MIT" or variations with different comment
//! styles and casing.
//!
//! Based on Python implementation at:
//! reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py
//!
//! ## Signature
//!
//! The `spdx_lid_match()` function takes `(index, query)` where query contains
//! pre-computed SPDX lines with token positions tracked during tokenization.
//! This enables correct `start_token` and `end_token` values in LicenseMatches.

use regex::Regex;

use crate::license_detection::expression::{
    LicenseExpression, expression_to_string, parse_expression,
};
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::Query;

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

pub(crate) fn normalize_spdx_key(key: &str) -> String {
    key.to_lowercase().replace("_", "-")
}

const DEPRECATED_SPDX_EXPRESSION_SUBS: &[(&str, &str)] = &[
    ("ecos-2.0", "gpl-2.0-plus with ecos-exception-2.0"),
    (
        "gpl-2.0-with-classpath-exception",
        "gpl-2.0-only with classpath-exception-2.0",
    ),
    (
        "gpl-2.0-with-gcc-exception",
        "gpl-2.0-only with gcc-exception-2.0",
    ),
    ("wxwindows", "lgpl-2.0-plus with wxwindows-exception-3.1"),
    (
        "gpl-2.0-with-autoconf-exception",
        "gpl-2.0-only with autoconf-exception-2.0",
    ),
    (
        "gpl-2.0-with-bison-exception",
        "gpl-2.0-only with bison-exception-2.2",
    ),
    (
        "gpl-2.0-with-font-exception",
        "gpl-2.0-only with font-exception-2.0",
    ),
    (
        "gpl-3.0-with-autoconf-exception",
        "gpl-3.0-only with autoconf-exception-3.0",
    ),
    (
        "gpl-3.0-with-gcc-exception",
        "gpl-3.0-only with gcc-exception-3.1",
    ),
];

fn get_deprecated_substitution(spdx_key: &str) -> Option<&'static str> {
    let normalized = normalize_spdx_key(spdx_key);
    for (deprecated, replacement) in DEPRECATED_SPDX_EXPRESSION_SUBS {
        if *deprecated == normalized {
            return Some(*replacement);
        }
    }
    None
}

pub(crate) fn split_license_expression(license_expression: &str) -> Vec<String> {
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

#[cfg(test)]
pub(crate) fn extract_matched_text_from_lines(text: &str, start_line: usize, end_line: usize) -> String {
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

pub fn spdx_lid_match(index: &LicenseIndex, query: &Query) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();

    for (spdx_text, start_token, end_token) in &query.spdx_lines {
        let (_, expression) = split_spdx_lid(spdx_text);
        let spdx_expression = clean_spdx_text(&expression);

        if spdx_expression.is_empty() {
            continue;
        }

        let lowered = spdx_expression.to_lowercase();
        let resolved_expression = if let Some(sub) = get_deprecated_substitution(&lowered) {
            sub.to_string()
        } else {
            lowered.clone()
        };

        if let Some(license_expression) =
            find_matching_rule_for_expression(index, &resolved_expression)
        {
            let matched_length = spdx_expression.len();
            let match_coverage = 100.0;

            let start_line = query.line_for_pos(*start_token).unwrap_or(1);
            let end_line = query.line_for_pos(*end_token).unwrap_or(start_line);

            let matched_text = query.matched_text(start_line, end_line);

            let (rid, rule_relevance, rule_identifier, rule_length, referenced_filenames) = index
                .rid_by_spdx_key
                .get(&license_expression)
                .map(|&rid| {
                    let rule = &index.rules_by_rid[rid];
                    (
                        rid,
                        rule.relevance,
                        rule.identifier.clone(),
                        rule.tokens.len(),
                        rule.referenced_filenames.clone(),
                    )
                })
                .unwrap_or_else(|| {
                    let unknown_rid = index.unknown_spdx_rid.unwrap_or(0);
                    if unknown_rid < index.rules_by_rid.len() {
                        let rule = &index.rules_by_rid[unknown_rid];
                        (
                            unknown_rid,
                            rule.relevance,
                            rule.identifier.clone(),
                            rule.tokens.len(),
                            rule.referenced_filenames.clone(),
                        )
                    } else {
                        (0, 100_u8, String::new(), 0_usize, None)
                    }
                });

            let score = rule_relevance as f32 / 100.0;

            let license_match = LicenseMatch {
                license_expression,
                license_expression_spdx: spdx_expression.clone(),
                from_file: None,
                start_line,
                end_line,
                start_token: *start_token,
                end_token: *end_token,
                matcher: MATCH_SPDX_ID.to_string(),
                score,
                matched_length,
                rule_length,
                match_coverage,
                rule_relevance,
                rid,
                rule_identifier,
                rule_url: String::new(),
                matched_text: Some(matched_text),
                referenced_filenames,
                is_license_intro: false,
                is_license_clue: false,
                is_license_reference: false,
                is_license_tag: true,
                is_license_text: false,
                is_from_license: false,
                matched_token_positions: None,
                hilen: 0,
                rule_start_token: 0,
                qspan_positions: None,
                ispan_positions: None,
                hispan_positions: None,
                candidate_resemblance: 0.0,
                candidate_containment: 0.0,
            };

            matches.push(license_match);
        }
    }

    matches
}

pub(crate) fn is_bare_license_list(expression: &str) -> bool {
    let lowered = expression.to_lowercase();
    !lowered.contains(" and ")
        && !lowered.contains(" or ")
        && !lowered.contains(" with ")
        && !expression.contains('(')
        && !expression.contains(')')
}

pub(crate) fn find_matching_rule_for_expression(index: &LicenseIndex, expression: &str) -> Option<String> {
    if let Some(&rid) = index.rid_by_spdx_key.get(expression) {
        let rule = &index.rules_by_rid[rid];
        return Some(rule.license_expression.clone());
    }

    for rule in &index.rules_by_rid {
        let normalized = normalize_spdx_key(&rule.license_expression);
        if normalized == expression {
            return Some(rule.license_expression.clone());
        }
    }

    if let Ok(parsed) = parse_expression(expression)
        && let Some(converted) = convert_spdx_expression_to_scancode(&parsed, index)
    {
        let result = expression_to_string(&converted);
        if !result.is_empty() {
            return Some(result);
        }
    }

    if is_bare_license_list(expression) {
        let license_keys = split_license_expression(expression);
        if license_keys.len() > 1 {
            let or_expression = license_keys.join(" OR ");
            if let Ok(parsed) = parse_expression(&or_expression)
                && let Some(converted) = convert_spdx_expression_to_scancode(&parsed, index)
            {
                let result = expression_to_string(&converted);
                if !result.is_empty() {
                    return Some(result);
                }
            }
        }
    }

    let license_keys = split_license_expression(expression);
    if license_keys.is_empty() {
        return index
            .unknown_spdx_rid
            .map(|rid| index.rules_by_rid[rid].license_expression.clone());
    }

    let first_key = &license_keys[0];
    if let Some(&rid) = index.rid_by_spdx_key.get(first_key) {
        return Some(index.rules_by_rid[rid].license_expression.clone());
    }

    for rule in &index.rules_by_rid {
        let normalized = normalize_spdx_key(&rule.license_expression);
        if normalized == *first_key {
            return Some(rule.license_expression.clone());
        }
    }

    index
        .unknown_spdx_rid
        .map(|rid| index.rules_by_rid[rid].license_expression.clone())
}

fn convert_spdx_expression_to_scancode(
    expr: &LicenseExpression,
    index: &LicenseIndex,
) -> Option<LicenseExpression> {
    match expr {
        LicenseExpression::License(key) => {
            let lookup_key = key.to_lowercase();
            if let Some(&rid) = index.rid_by_spdx_key.get(&lookup_key) {
                Some(LicenseExpression::License(
                    index.rules_by_rid[rid].license_expression.clone(),
                ))
            } else {
                None
            }
        }
        LicenseExpression::LicenseRef(key) => {
            let lookup_key = key.to_lowercase();
            if let Some(&rid) = index.rid_by_spdx_key.get(&lookup_key) {
                Some(LicenseExpression::License(
                    index.rules_by_rid[rid].license_expression.clone(),
                ))
            } else {
                None
            }
        }
        LicenseExpression::And { left, right } => {
            let left_converted = convert_spdx_expression_to_scancode(left, index);
            let right_converted = convert_spdx_expression_to_scancode(right, index);
            match (left_converted, right_converted) {
                (Some(l), Some(r)) => Some(LicenseExpression::And {
                    left: Box::new(l),
                    right: Box::new(r),
                }),
                _ => None,
            }
        }
        LicenseExpression::Or { left, right } => {
            let left_converted = convert_spdx_expression_to_scancode(left, index);
            let right_converted = convert_spdx_expression_to_scancode(right, index);
            match (left_converted, right_converted) {
                (Some(l), Some(r)) => Some(LicenseExpression::Or {
                    left: Box::new(l),
                    right: Box::new(r),
                }),
                _ => None,
            }
        }
        LicenseExpression::With { left, right } => {
            let left_converted = convert_spdx_expression_to_scancode(left, index);
            let right_converted = convert_spdx_expression_to_scancode(right, index);
            match (left_converted, right_converted) {
                (Some(l), Some(r)) => Some(LicenseExpression::With {
                    left: Box::new(l),
                    right: Box::new(r),
                }),
                _ => None,
            }
        }
    }
}


#[cfg(test)]
mod test;
