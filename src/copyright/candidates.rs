//! Candidate line selection and grouping for copyright detection.
//!
//! Identifies lines that may contain copyright statements and groups them
//! into contiguous blocks for further analysis. Lines are selected based on:
//! - Presence of copyright hint markers (©, "Copyright", etc.)
//! - Year patterns (1960–2099)
//! - HTTP URLs
//! - Debian-style markup (`<s>`, `</s>`)
//! - End-of-statement markers ("all rights reserved")
//!
//! Multi-line continuation logic handles cases where a copyright statement
//! spans several lines, using trailing markers (years, commas, "and", "by")
//! to determine continuation.

use std::sync::LazyLock;

use regex::Regex;

use super::hints;
use super::prepare::prepare_text_line;

/// Code-like patterns that indicate a line is minified JS/CSS, not copyright text.
static CODE_PATTERNS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "function(",
        "this.",
        ".prototype",
        "===",
        "!==",
        "var ",
        "return ",
        "typeof ",
        "undefined",
        "null)",
        "null,",
        ".apply(",
        ".call(",
        "addEventListener",
        "removeEventListener",
        "createElement",
        "appendChild",
        "innerHTML",
        "className",
        "setAttribute",
        "getAttribute",
    ]
});

/// Lines longer than this without strong copyright indicators are skipped.
///
/// Real copyright notices are never this long on a single line. Lines exceeding
/// this are invariably minified JS/CSS or binary data where regex operations
/// become pathologically slow (e.g., 624KB minified JS → 20s+ in prepare_text_line).
/// The 2000-char threshold is conservative — the longest known legitimate
/// single-line copyright notice in the golden test suite is ~3200 chars, but
/// those always contain strong indicators ("opyr"/"auth") and pass through.
const MAX_LINE_LENGTH: usize = 2_000;

/// Minimum line length to trigger code-line heuristic.
const CODE_LINE_MIN_LENGTH: usize = 200;

/// Minimum line length to trigger encoded-data detection.
///
/// Short lines are unlikely to be encoded data, and checking them would add
/// overhead without benefit. Uuencode full lines are 61 chars; base64 lines
/// are typically 76 chars. We use 40 as a conservative lower bound.
const ENCODED_LINE_MIN_LENGTH: usize = 40;

/// Minimum ratio of encoded characters (uuencode range 32–96 or base64 charset)
/// for a line to be classified as encoded data. Uuencode data lines are 100%
/// in this range; we use 90% to allow minor variations.
const ENCODED_CHAR_RATIO: f64 = 0.90;

/// Check whether a long line contains copyright-relevant content.
///
/// Returns `true` if the line has strong copyright indicators anywhere in it.
/// Strong indicators are: "opyr"/"opyl"/"auth" (case-insensitive), or "(c)"
/// followed by a digit (distinguishes copyright `(c)2024` from code `(c){var`).
///
/// Uses byte-level search to avoid allocating a lowercase copy of potentially
/// huge (100KB+) lines.
fn has_copyright_indicators(line: &str) -> bool {
    let bytes = line.as_bytes();
    contains_ascii_ci(bytes, b"opyr")
        || contains_ascii_ci(bytes, b"opyl")
        || contains_ascii_ci(bytes, b"auth")
        || has_c_sign_before_year(bytes)
}

/// Case-insensitive ASCII substring search without allocation.
fn contains_ascii_ci(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

/// Check for "(c)" or "(C)" followed by optional whitespace and a digit.
/// Distinguishes copyright signs like `(c)2024` from code like `if(c){var`.
fn has_c_sign_before_year(bytes: &[u8]) -> bool {
    for (i, window) in bytes.windows(3).enumerate() {
        if window[0] == b'(' && (window[1] == b'c' || window[1] == b'C') && window[2] == b')' {
            let rest = &bytes[i + 3..];
            for &b in rest {
                if b == b' ' || b == b'\t' {
                    continue;
                }
                if b.is_ascii_digit() {
                    return true;
                }
                break;
            }
        }
    }
    false
}

/// Detect lines that are encoded binary data (uuencode, base64, hex dumps).
///
/// These lines trigger false positives from weak hint markers like `@` but
/// never contain real copyright text. Skipping them avoids expensive regex
/// processing on thousands of data lines (e.g., 5,143 lines in a uuencode
/// file that each contain `@`).
fn is_encoded_data_line(line: &str) -> bool {
    let len = line.len();
    if len < ENCODED_LINE_MIN_LENGTH {
        return false;
    }

    // Quick check: if the line contains strong copyright indicators, never skip.
    if has_copyright_indicators(line) {
        return false;
    }

    let bytes = line.as_bytes();

    // Uuencode data lines: first byte is a length character (space to `_`, i.e.
    // ASCII 32–95), followed by encoded characters in the same range. Full lines
    // start with `M` (45 bytes = char 77) and are exactly 61 chars.
    if is_uuencode_data_line(bytes) {
        return true;
    }

    // Base64 data lines: consist entirely of [A-Za-z0-9+/] with optional `=`
    // padding and no spaces (pure data, not prose).
    if is_base64_data_line(bytes) {
        return true;
    }

    false
}

/// Check if a line looks like a uuencode data line.
///
/// Uuencode format: each line starts with a length byte (ASCII 32 + number of
/// data bytes, so `M` = 77 = 32 + 45 for a full 45-byte line), followed by
/// encoded characters all in the printable range 32–96 (space through backtick).
/// Full data lines are exactly 61 characters.
///
/// To avoid false positives on comment decorators (e.g., `/*****/`), we require
/// at least 8 distinct byte values — real uuencode data has high character
/// diversity while decorators repeat 1-3 characters.
fn is_uuencode_data_line(bytes: &[u8]) -> bool {
    let first = bytes[0];
    if !(32..=95).contains(&first) {
        return false;
    }

    let uu_count = bytes.iter().filter(|&&b| (32..=96).contains(&b)).count();
    let ratio = uu_count as f64 / bytes.len() as f64;

    if ratio < ENCODED_CHAR_RATIO {
        return false;
    }

    // Reject lines with low character diversity (comment decorators like /****/).
    let mut seen = [false; 256];
    for &b in bytes {
        seen[b as usize] = true;
    }
    let distinct_count = seen.iter().filter(|&&s| s).count();
    if distinct_count < 8 {
        return false;
    }

    let space_count = bytes.iter().filter(|&&b| b == b' ').count();
    space_count <= 1
}

/// Check if a line looks like base64-encoded data.
///
/// Base64 uses only `[A-Za-z0-9+/=]` with no spaces. We require 100% base64
/// characters — URLs and file paths also lack spaces but contain `:`, `.`, `-`
/// which are not in the base64 alphabet, so a strict check avoids false positives.
fn is_base64_data_line(bytes: &[u8]) -> bool {
    if bytes.contains(&b' ') {
        return false;
    }

    bytes
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

/// Check if a line looks like minified code where `(c)` is a false positive.
/// Returns true if the line should be skipped as a candidate.
fn is_code_line_with_false_c(line: &str) -> bool {
    if line.len() < CODE_LINE_MIN_LENGTH {
        return false;
    }

    let lower = line.to_lowercase();

    // Check if the ONLY copyright hint is `(c)` — if there's a real "copyright" word, keep it.
    if lower.contains("opyr") || lower.contains("opyl") || lower.contains("auth") {
        return false;
    }

    // Check for code-like patterns.
    let code_pattern_count = CODE_PATTERNS.iter().filter(|p| line.contains(**p)).count();

    code_pattern_count >= 2
}

/// Regex to remove all non-alphanumeric characters (for chars-only comparison).
static NON_CHARS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-zA-Z0-9]").unwrap());

/// Continuation suffixes: when the chars-only version of the previous line ends
/// with one of these, an empty/non-candidate line is treated as continuation
/// rather than a group break.
const CONTINUATION_SUFFIXES: &[&str] = &["copyright", "copyrights", "and", "by"];

/// Suffixes that indicate the end of a complete statement, causing an immediate
/// group yield.
const END_SUFFIXES: &[&str] = &["rightreserved", "rightsreserved"];

/// Remove all non-alphanumeric characters from a string.
fn chars_only(s: &str) -> String {
    NON_CHARS_RE.replace_all(&s.to_lowercase(), "").into_owned()
}

/// Check if a chars-only line marks the end of a copyright statement.
fn is_end_of_statement(chars: &str) -> bool {
    END_SUFFIXES.iter().any(|suffix| chars.ends_with(suffix))
}

/// Check if a chars-only previous line ends with a continuation marker
/// (copyright, copyrights, and, by, comma) or a trailing year.
fn ends_with_continuation(chars: &str) -> bool {
    if chars.is_empty() {
        return false;
    }
    // Check trailing comma in original (pre-chars-only) text would have been
    // stripped, but we check the original chars_only which only has alnum.
    // The Python code checks previous_chars.endswith(('copyright', 'copyrights', 'and', 'by', ',')).
    // Since chars_only strips commas, we only check the alpha suffixes here.
    // Trailing year check is separate.
    CONTINUATION_SUFFIXES
        .iter()
        .any(|suffix| chars.ends_with(suffix))
        || hints::has_trailing_year(chars)
}

/// A numbered line: (1-based line number, prepared text).
pub type NumberedLine = (usize, String);

/// Collect groups of candidate lines from numbered input lines.
///
/// Each group is a `Vec<NumberedLine>` representing a contiguous block of lines
/// that may contain copyright/author information. The caller then processes each
/// group through the lexer/parser pipeline.
///
/// This mirrors the Python `collect_candidate_lines()` function.
pub fn collect_candidate_lines(
    numbered_lines: impl IntoIterator<Item = (usize, String)>,
) -> Vec<Vec<NumberedLine>> {
    let mut groups: Vec<Vec<NumberedLine>> = Vec::new();
    let mut candidates: Vec<NumberedLine> = Vec::new();

    // `in_copyright` is a countdown: when > 0, we're inside a copyright block
    // and will include non-candidate continuation lines. Starts at 2 when a
    // candidate is found, decrements for each non-candidate continuation line.
    let mut in_copyright: u32 = 0;
    let mut previous_chars: Option<String> = None;

    for (ln, line) in numbered_lines {
        // Skip long lines without copyright indicators (minified JS, binary data).
        if line.len() > MAX_LINE_LENGTH && !has_copyright_indicators(&line) {
            if in_copyright > 0 {
                in_copyright -= 1;
                if in_copyright == 0 && !candidates.is_empty() {
                    groups.push(std::mem::take(&mut candidates));
                    previous_chars = None;
                }
            }
            continue;
        }

        // Skip encoded data lines (uuencode, base64) that trigger false
        // positives from weak hint markers like `@`.
        if is_encoded_data_line(&line) {
            if in_copyright > 0 {
                in_copyright -= 1;
                if in_copyright == 0 && !candidates.is_empty() {
                    groups.push(std::mem::take(&mut candidates));
                    previous_chars = None;
                }
            }
            continue;
        }

        let is_debian = line.contains("s>");
        let co = chars_only(&line);

        if is_end_of_statement(&co) {
            candidates.push((ln, prepare_text_line(&line)));
            groups.push(std::mem::take(&mut candidates));
            in_copyright = 0;
            previous_chars = None;
        } else if hints::is_candidate(&line) || co.contains("http") || is_debian {
            if is_code_line_with_false_c(&line) {
                continue;
            }
            in_copyright = 2;
            candidates.push((ln, prepare_text_line(&line)));
            previous_chars = Some(co);
        } else if in_copyright > 0 {
            // Inside a copyright block — check if we should continue or break.
            if co.is_empty() {
                // Empty line: continue only if previous line ends with a
                // continuation marker or a trailing year.
                if let Some(ref prev) = previous_chars {
                    if !ends_with_continuation(prev) {
                        // Break the group.
                        if !candidates.is_empty() {
                            groups.push(std::mem::take(&mut candidates));
                        }
                        in_copyright = 0;
                        previous_chars = None;
                    } else {
                        candidates.push((ln, prepare_text_line(&line)));
                        in_copyright -= 1;
                    }
                } else {
                    // No previous chars recorded — break.
                    if !candidates.is_empty() {
                        groups.push(std::mem::take(&mut candidates));
                    }
                    in_copyright = 0;
                    previous_chars = None;
                }
            } else {
                candidates.push((ln, prepare_text_line(&line)));
                in_copyright -= 1;
            }
        } else if !candidates.is_empty() {
            // Not in copyright and line is not a candidate — yield what we have.
            groups.push(std::mem::take(&mut candidates));
            in_copyright = 0;
            previous_chars = None;
        }
    }

    // Yield any remaining candidates.
    if !candidates.is_empty() {
        groups.push(candidates);
    }

    groups
}

/// Strip balanced leading and trailing parentheses from a string.
///
/// Only strips if the parentheses are truly wrapping (no inner parens).
///
/// # Examples
/// ```
/// use scancode_rust::copyright::strip_balanced_edge_parens;
/// assert_eq!(strip_balanced_edge_parens("(Hello World)"), "Hello World");
/// assert_eq!(strip_balanced_edge_parens("(Hello (World)"), "(Hello (World)");
/// ```
pub fn strip_balanced_edge_parens(s: &str) -> &str {
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1];
        if !inner.contains('(') && !inner.contains(')') {
            return inner;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── chars_only ──────────────────────────────────────────────────

    #[test]
    fn test_chars_only_basic() {
        assert_eq!(chars_only("Hello, World! 123"), "helloworld123");
    }

    #[test]
    fn test_chars_only_empty() {
        assert_eq!(chars_only(""), "");
    }

    #[test]
    fn test_chars_only_only_punct() {
        assert_eq!(chars_only("---...!!!"), "");
    }

    // ── is_end_of_statement ─────────────────────────────────────────

    #[test]
    fn test_eos_rights_reserved() {
        assert!(is_end_of_statement("allrightsreserved"));
    }

    #[test]
    fn test_eos_right_reserved() {
        assert!(is_end_of_statement("allrightreserved"));
    }

    #[test]
    fn test_eos_negative() {
        assert!(!is_end_of_statement("copyright2024"));
    }

    // ── ends_with_continuation ──────────────────────────────────────

    #[test]
    fn test_continuation_copyright() {
        assert!(ends_with_continuation("somecopyright"));
    }

    #[test]
    fn test_continuation_and() {
        assert!(ends_with_continuation("fooand"));
    }

    #[test]
    fn test_continuation_by() {
        assert!(ends_with_continuation("writtenby"));
    }

    #[test]
    fn test_continuation_year() {
        assert!(ends_with_continuation("text2024"));
    }

    #[test]
    fn test_continuation_negative() {
        assert!(!ends_with_continuation("justtext"));
    }

    #[test]
    fn test_continuation_empty() {
        assert!(!ends_with_continuation(""));
    }

    // ── strip_balanced_edge_parens ──────────────────────────────────

    #[test]
    fn test_strip_balanced_simple() {
        assert_eq!(strip_balanced_edge_parens("(Hello World)"), "Hello World");
    }

    #[test]
    fn test_strip_balanced_unmatched_start() {
        assert_eq!(strip_balanced_edge_parens("(Hello World"), "(Hello World");
    }

    #[test]
    fn test_strip_balanced_unmatched_end() {
        assert_eq!(strip_balanced_edge_parens("Hello World)"), "Hello World)");
    }

    #[test]
    fn test_strip_balanced_inner_parens() {
        assert_eq!(
            strip_balanced_edge_parens("(Hello (World)"),
            "(Hello (World)"
        );
    }

    #[test]
    fn test_strip_balanced_nested_parens() {
        // Inner contains both ( and ), so don't strip.
        assert_eq!(
            strip_balanced_edge_parens("(Hello (World))"),
            "(Hello (World))"
        );
    }

    #[test]
    fn test_strip_balanced_no_parens() {
        assert_eq!(strip_balanced_edge_parens("Hello World"), "Hello World");
    }

    // ── collect_candidate_lines ─────────────────────────────────────

    #[test]
    fn test_collect_empty_input() {
        let lines: Vec<(usize, String)> = vec![];
        let groups = collect_candidate_lines(lines);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_collect_single_copyright_line() {
        let lines = vec![(1, "Copyright 2024 Acme Inc.".to_string())];
        let groups = collect_candidate_lines(lines);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0].0, 1);
    }

    #[test]
    fn test_collect_non_candidate_lines() {
        let lines = vec![
            (1, "This is just code".to_string()),
            (2, "More code here".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_collect_two_separate_copyrights() {
        // With in_copyright countdown of 2, lines 2 and 3 are continuation
        // lines (decrementing 2→1→0), so we need 3+ gap lines to split.
        let lines = vec![
            (1, "Copyright 2020 Foo".to_string()),
            (2, "some random code".to_string()),
            (3, "not related".to_string()),
            (4, "also not related".to_string()),
            (5, "Copyright 2024 Bar".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        assert_eq!(groups.len(), 2, "groups: {groups:?}");
    }

    #[test]
    fn test_collect_end_of_statement_yields_immediately() {
        let lines = vec![
            (1, "Copyright 2024 Acme Inc.".to_string()),
            (2, "All rights reserved.".to_string()),
            (3, "Some other text".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        assert_eq!(groups.len(), 1, "groups: {groups:?}");
        // Both copyright line and "all rights reserved" should be in same group.
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_collect_continuation_with_trailing_year() {
        // A copyright line followed by an empty line should continue if the
        // previous line ends with a year.
        let lines = vec![
            (1, "Copyright 2024".to_string()),
            (2, "".to_string()),
            (3, "Acme Inc.".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        // Line 1 ends with year → empty line 2 continues → line 3 is included.
        assert_eq!(groups.len(), 1, "groups: {groups:?}");
    }

    #[test]
    fn test_collect_break_on_empty_without_continuation() {
        // A non-continuation line followed by empty → group break.
        let lines = vec![
            (1, "Copyright 2024 Acme Inc.".to_string()),
            (2, "Some additional text".to_string()),
            (3, "".to_string()),
            (4, "Copyright 2025 Bar".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        // Line 2 is continuation (in_copyright=2), line 3 is empty but
        // previous doesn't end with continuation marker → break.
        assert_eq!(groups.len(), 2, "groups: {groups:?}");
    }

    #[test]
    fn test_collect_http_as_candidate() {
        let lines = vec![(1, "http://example.com/copyright".to_string())];
        let groups = collect_candidate_lines(lines);
        assert_eq!(groups.len(), 1, "groups: {groups:?}");
    }

    #[test]
    fn test_collect_debian_markup() {
        let lines = vec![(1, "<s>John Doe</s>".to_string())];
        let groups = collect_candidate_lines(lines);
        assert_eq!(groups.len(), 1, "groups: {groups:?}");
    }

    #[test]
    fn test_collect_multiline_copyright() {
        let lines = vec![
            (1, "Copyright (C) 2020-2024".to_string()),
            (2, "  Acme Corporation".to_string()),
            (3, "  All rights reserved.".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        // Line 1 is candidate, line 2 continuation (in_copyright=2→1),
        // line 3 is end-of-statement → yields group.
        assert_eq!(groups.len(), 1, "groups: {groups:?}");
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn test_collect_remaining_candidates_at_end() {
        let lines = vec![
            (1, "Some preamble".to_string()),
            (2, "Copyright 2024 Acme".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0][0].0, 2);
    }

    // ── has_copyright_indicators ────────────────────────────────────

    #[test]
    fn test_indicators_copyright_word() {
        assert!(has_copyright_indicators("blah Copyright 2024 blah"));
    }

    #[test]
    fn test_indicators_copyleft() {
        assert!(has_copyright_indicators("Copyleft notice here"));
    }

    #[test]
    fn test_indicators_author() {
        assert!(has_copyright_indicators("@author John Doe"));
    }

    #[test]
    fn test_indicators_c_sign_with_year() {
        assert!(has_copyright_indicators("(c)2024 Acme Inc."));
        assert!(has_copyright_indicators("(C) 1996 Id Software"));
        assert!(has_copyright_indicators("(c) 2020 Foo"));
    }

    #[test]
    fn test_indicators_c_sign_code_pattern() {
        assert!(!has_copyright_indicators("if(c){var r=[]}"));
        assert!(!has_copyright_indicators("function(c){return c.length}"));
    }

    #[test]
    fn test_indicators_no_match() {
        assert!(!has_copyright_indicators("var x = 42; function foo() {}"));
        assert!(!has_copyright_indicators(
            "just some random @ text with right margin"
        ));
    }

    // ── has_c_sign_before_year ──────────────────────────────────────

    #[test]
    fn test_c_sign_year_adjacent() {
        assert!(has_c_sign_before_year(b"(c)2024"));
    }

    #[test]
    fn test_c_sign_year_with_space() {
        assert!(has_c_sign_before_year(b"(c) 1996"));
    }

    #[test]
    fn test_c_sign_year_uppercase() {
        assert!(has_c_sign_before_year(b"(C)2024"));
    }

    #[test]
    fn test_c_sign_code_brace() {
        assert!(!has_c_sign_before_year(b"(c){var}"));
    }

    #[test]
    fn test_c_sign_code_dot() {
        assert!(!has_c_sign_before_year(b"(c).length"));
    }

    #[test]
    fn test_c_sign_empty_after() {
        assert!(!has_c_sign_before_year(b"(c)"));
    }

    // ── collect_candidate_lines with long lines ─────────────────────

    #[test]
    fn test_collect_skips_long_line_without_indicators() {
        let long_line = "x".repeat(3000);
        let lines = vec![
            (1, "Copyright 2024 Acme".to_string()),
            (2, long_line),
            (3, "Copyright 2025 Bar".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        assert!(
            !groups.is_empty(),
            "Should still detect copyrights: {groups:?}"
        );
    }

    #[test]
    fn test_collect_keeps_long_line_with_copyright() {
        let mut long_line = "x".repeat(2500);
        long_line.push_str(" Copyright 2024 Acme Inc. ");
        long_line.push_str(&"y".repeat(500));
        let lines = vec![(1, long_line)];
        let groups = collect_candidate_lines(lines);
        assert_eq!(
            groups.len(),
            1,
            "Should detect copyright in long line: {groups:?}"
        );
    }

    #[test]
    fn test_collect_keeps_long_line_with_c_sign_year() {
        let mut long_line = "x".repeat(2500);
        long_line.push_str(" (c)1996 Id Software ");
        long_line.push_str(&"y".repeat(500));
        let lines = vec![(1, long_line)];
        let groups = collect_candidate_lines(lines);
        assert_eq!(
            groups.len(),
            1,
            "Should detect (c)year in long line: {groups:?}"
        );
    }

    // ── contains_ascii_ci ───────────────────────────────────────────

    #[test]
    fn test_contains_ascii_ci_found() {
        assert!(contains_ascii_ci(b"Hello World", b"world"));
        assert!(contains_ascii_ci(b"CoPyRiGhT", b"opyr"));
    }

    #[test]
    fn test_contains_ascii_ci_not_found() {
        assert!(!contains_ascii_ci(b"Hello World", b"xyz"));
    }

    #[test]
    fn test_contains_ascii_ci_needle_longer() {
        assert!(!contains_ascii_ci(b"Hi", b"Hello"));
    }

    // ── is_uuencode_data_line ───────────────────────────────────────

    #[test]
    fn test_uuencode_full_data_line() {
        // Real uuencode data line from golden test (starts with M = 45 bytes)
        let line = b"M?T5,1@$\"`0`````````````!``@````!`````````````]'0```0`0`T````";
        assert!(is_uuencode_data_line(line));
    }

    #[test]
    fn test_uuencode_short_data_line() {
        // Shorter uuencode line (last line of a block) — still matches the
        // uuencode character pattern, but is_encoded_data_line() rejects it
        // due to the minimum length check.
        let line = b"1`@``*%P```(\"```H8````@(`";
        assert!(is_uuencode_data_line(line));
        assert!(!is_encoded_data_line(std::str::from_utf8(line).unwrap()));
    }

    #[test]
    fn test_uuencode_not_natural_text() {
        let line = b"This is a normal English sentence with spaces between words here";
        assert!(!is_uuencode_data_line(line));
    }

    #[test]
    fn test_uuencode_not_copyright_line() {
        let line = b" * Copyright (c) 2002-2006 Sam Leffler, Errno Consulting, Atheros";
        assert!(!is_uuencode_data_line(line));
    }

    #[test]
    fn test_uuencode_not_comment_decorator() {
        let line = b"/************************************************************************/";
        assert!(!is_uuencode_data_line(line));
    }

    #[test]
    fn test_uuencode_not_star_line() {
        let line = b"************************************************************";
        assert!(!is_uuencode_data_line(line));
    }

    #[test]
    fn test_uuencode_not_dash_line() {
        let line = b"------------------------------------------------------------";
        assert!(!is_uuencode_data_line(line));
    }

    // ── is_base64_data_line ─────────────────────────────────────────

    #[test]
    fn test_base64_typical_line() {
        let line = b"SGVsbG8gV29ybGQhIFRoaXMgaXMgYSBiYXNlNjQgZW5jb2RlZCBzdHJpbmc=";
        assert!(is_base64_data_line(line));
    }

    #[test]
    fn test_base64_with_plus_slash() {
        let line = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        assert!(is_base64_data_line(line));
    }

    #[test]
    fn test_base64_not_text_with_spaces() {
        let line = b"This is not base64 because it has spaces in it right here";
        assert!(!is_base64_data_line(line));
    }

    #[test]
    fn test_base64_not_url() {
        let line = b"http://code.google.com/apis/protocolbuffers/";
        assert!(!is_base64_data_line(line));
    }

    #[test]
    fn test_base64_not_file_path() {
        let line = b"/usr/local/lib/python3.11/site-packages/some_package/module.py";
        assert!(!is_base64_data_line(line));
    }

    // ── is_encoded_data_line (integration) ──────────────────────────

    #[test]
    fn test_encoded_skips_short_lines() {
        assert!(!is_encoded_data_line("short"));
        assert!(!is_encoded_data_line("M`@``")); // uuencode-like but too short
    }

    #[test]
    fn test_encoded_preserves_copyright_indicators() {
        let line = "M".to_string() + &"`".repeat(20) + "Copyright" + &"`".repeat(30);
        assert!(!is_encoded_data_line(&line));
    }

    #[test]
    fn test_encoded_detects_uuencode() {
        let line = "M?T5,1@$\"`0`````````````!``@````!`````````````]'0```0`0`T````";
        assert!(is_encoded_data_line(line));
    }

    #[test]
    fn test_encoded_detects_base64() {
        let line = "SGVsbG8gV29ybGQhIFRoaXMgaXMgYSBiYXNlNjQgZW5jb2RlZCBzdHJpbmc=";
        assert!(is_encoded_data_line(line));
    }

    #[test]
    fn test_encoded_preserves_normal_text() {
        assert!(!is_encoded_data_line(
            "This is a normal line of source code with various characters"
        ));
    }

    #[test]
    fn test_encoded_preserves_email_line() {
        assert!(!is_encoded_data_line(
            "Contact us at support@example.com for more information about this"
        ));
    }

    // ── collect_candidate_lines with encoded data ───────────────────

    #[test]
    fn test_collect_skips_uuencode_data_lines() {
        let uu_line = "M?T5,1@$\"`0`````````````!``@````!`````````````]'0```0`0`T````";
        let lines = vec![
            (1, "Copyright 2024 Acme".to_string()),
            (2, uu_line.to_string()),
            (3, uu_line.to_string()),
            (4, uu_line.to_string()),
            (5, "Copyright 2025 Bar".to_string()),
        ];
        let groups = collect_candidate_lines(lines);
        assert_eq!(groups.len(), 2, "Should detect both copyrights: {groups:?}");
    }

    #[test]
    fn test_collect_preserves_copyright_in_uuencode_file() {
        // Simulates the real uuencode golden test: copyright header followed by data
        let uu_line = "M?T5,1@$\"`0`````````````!``@````!`````````````]'0```0`0`T````";
        let mut lines: Vec<(usize, String)> = vec![
            (
                1,
                " * Copyright (c) 2002-2006 Sam Leffler, Errno Consulting, Atheros".to_string(),
            ),
            (
                2,
                " * Communications, Inc.  All rights reserved.".to_string(),
            ),
        ];
        // Add 100 uuencode data lines
        for i in 3..103 {
            lines.push((i, uu_line.to_string()));
        }
        let groups = collect_candidate_lines(lines);
        assert!(!groups.is_empty(), "Should detect copyright header");
        assert_eq!(groups[0].len(), 2, "Copyright group should have 2 lines");
    }
}
