//! Lexer (tokenizer + POS tagger) for copyright detection.
//!
//! Splits prepared text lines into tokens, then assigns each token a
//! part-of-speech (POS) tag using the compiled regex patterns. This is
//! the bridge between candidate line selection and grammar parsing.
//!
//! Pipeline: numbered lines → tokenize → POS tag → tagged tokens

use std::sync::LazyLock;

use regex::Regex;

use super::patterns::COMPILED_PATTERNS;
use super::types::{PosTag, Token};

/// Splitter regex: splits on tabs, spaces, equals signs, and semicolons.
/// Matches Python's `re.compile(r'[\t =;]+').split`.
static SPLITTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\t =;]+").unwrap());

/// Tokenize and POS-tag a group of numbered lines.
///
/// Takes an iterable of `(line_number, prepared_text)` tuples (output of
/// `collect_candidate_lines`) and returns a flat list of POS-tagged tokens.
///
/// Empty lines are handled specially: if the previous line starts with
/// "copyright" or ends with continuation markers ("by", "copyright", or
/// a digit), the empty line is skipped (continuation). Otherwise an
/// `EMPTY_LINE` token is emitted.
pub fn get_tokens(numbered_lines: &[(usize, String)]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut last_line = String::new();

    for (start_line, line) in numbered_lines {
        if line.trim().is_empty() {
            let stripped = last_line
                .to_lowercase()
                .trim_matches(|c: char| c.is_ascii_punctuation())
                .to_string();

            if stripped.starts_with("copyright")
                || stripped.ends_with("by")
                || stripped.ends_with("copyright")
                || stripped.chars().last().is_some_and(|c| c.is_ascii_digit())
            {
                continue;
            } else {
                tokens.push(Token {
                    value: "\n".to_string(),
                    tag: PosTag::EmptyLine,
                    start_line: *start_line,
                });
                last_line.clear();
                continue;
            }
        }

        last_line.clone_from(line);

        for tok_str in SPLITTER.split(line) {
            let mut tok = tok_str.to_string();

            if tok.ends_with("',") {
                tok = tok.trim_end_matches(&[',', '\''][..]).to_string();
            }

            tok = tok.trim_matches(&['\'', ' '][..]).to_string();
            tok = tok.trim_end_matches(':').to_string();
            tok = tok.trim().to_string();

            if tok.is_empty() || tok == ":" || tok == "." {
                continue;
            }

            if tok.ends_with(',') {
                let base = tok.trim_end_matches(',').trim();
                if !base.is_empty() {
                    let tag = COMPILED_PATTERNS.match_token(base);
                    tokens.push(Token {
                        value: base.to_string(),
                        tag,
                        start_line: *start_line,
                    });
                    tokens.push(Token {
                        value: ",".to_string(),
                        tag: PosTag::Cc,
                        start_line: *start_line,
                    });
                    continue;
                }
            }

            let tag = COMPILED_PATTERNS.match_token(&tok);

            tokens.push(Token {
                value: tok,
                tag,
                start_line: *start_line,
            });
        }
    }

    retag_camel_case_junk_before_company_suffix_in_copyright_context(&mut tokens);

    tokens
}

fn retag_camel_case_junk_before_company_suffix_in_copyright_context(tokens: &mut [Token]) {
    if tokens.len() < 2 {
        return;
    }

    for i in 0..tokens.len().saturating_sub(1) {
        if tokens[i].tag != PosTag::Junk {
            continue;
        }
        if tokens[i + 1].tag != PosTag::Comp {
            continue;
        }
        if tokens[i].start_line != tokens[i + 1].start_line {
            continue;
        }
        if !is_camel_case_identifier_candidate(&tokens[i].value) {
            continue;
        }

        let mut has_copy_prefix = false;
        let mut j = i;
        while j > 0 {
            j -= 1;
            if tokens[j].start_line != tokens[i].start_line || tokens[j].tag == PosTag::EmptyLine {
                break;
            }
            if tokens[j].tag == PosTag::Copy {
                has_copy_prefix = true;
                break;
            }
        }

        if has_copy_prefix {
            tokens[i].tag = PosTag::Nnp;
        }
    }
}

fn is_camel_case_identifier_candidate(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }

    let mut has_lower = false;
    let mut has_inner_upper = false;
    for c in chars {
        if !c.is_ascii_alphanumeric() {
            return false;
        }
        if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_uppercase() {
            has_inner_upper = true;
        }
    }

    has_lower && has_inner_upper
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_copyright_line() {
        let lines = vec![(1, "Copyright 2024 Acme Inc.".to_string())];
        let tokens = get_tokens(&lines);
        assert!(!tokens.is_empty(), "Should produce tokens");

        assert_eq!(tokens[0].value, "Copyright");
        assert_eq!(tokens[0].tag, PosTag::Copy);
        assert_eq!(tokens[0].start_line, 1);

        assert_eq!(tokens[1].value, "2024");
        assert_eq!(tokens[1].tag, PosTag::Yr);

        assert!(tokens.len() >= 3, "tokens: {tokens:?}");
    }

    #[test]
    fn test_empty_input() {
        let lines: Vec<(usize, String)> = vec![];
        let tokens = get_tokens(&lines);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_splits_on_tabs_and_equals() {
        let lines = vec![(1, "foo\tbar=baz".to_string())];
        let tokens = get_tokens(&lines);
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        assert_eq!(values, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_strips_trailing_colon() {
        let lines = vec![(1, "Author: John".to_string())];
        let tokens = get_tokens(&lines);
        // "Author:" should have colon stripped → "Author"
        assert_eq!(tokens[0].value, "Author");
    }

    #[test]
    fn test_discards_lone_colon_and_dot() {
        let lines = vec![(1, "foo : . bar".to_string())];
        let tokens = get_tokens(&lines);
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        assert_eq!(values, vec!["foo", "bar"]);
    }

    #[test]
    fn test_strips_trailing_quote_comma() {
        let lines = vec![(1, "name',".to_string())];
        let tokens = get_tokens(&lines);
        assert_eq!(tokens[0].value, "name");
    }

    #[test]
    fn test_splits_trailing_commas_into_cc_token() {
        let lines = vec![(1, "Stearns, Michael".to_string())];
        let tokens = get_tokens(&lines);
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        let tags: Vec<PosTag> = tokens.iter().map(|t| t.tag).collect();
        assert_eq!(values, vec!["Stearns", ",", "Michael"]);
        assert_eq!(tags[1], PosTag::Cc);
    }

    #[test]
    fn test_empty_line_continuation() {
        let lines = vec![
            (1, "Copyright 2024".to_string()),
            (2, "".to_string()),
            (3, "Acme Inc.".to_string()),
        ];
        let tokens = get_tokens(&lines);
        // Empty line after "Copyright 2024" should be skipped (continuation).
        let has_empty = tokens.iter().any(|t| t.tag == PosTag::EmptyLine);
        assert!(!has_empty, "Empty line should be skipped as continuation");
    }

    #[test]
    fn test_empty_line_break() {
        let lines = vec![
            (1, "Acme Inc.".to_string()),
            (2, "".to_string()),
            (3, "Other stuff".to_string()),
        ];
        let tokens = get_tokens(&lines);
        let has_empty = tokens.iter().any(|t| t.tag == PosTag::EmptyLine);
        assert!(
            has_empty,
            "Empty line after non-continuation should emit EMPTY_LINE"
        );
    }

    #[test]
    fn test_pos_tags_assigned() {
        let lines = vec![(1, "Copyright (c) 2020-2024 Acme and Bar".to_string())];
        let tokens = get_tokens(&lines);
        assert!(tokens.len() >= 4, "tokens: {tokens:?}");

        assert_eq!(tokens[0].tag, PosTag::Copy);
        assert_eq!(tokens[1].tag, PosTag::Copy); // (c)
    }

    #[test]
    fn test_line_numbers_preserved() {
        let lines = vec![
            (10, "Copyright 2024".to_string()),
            (11, "Acme Inc.".to_string()),
        ];
        let tokens = get_tokens(&lines);
        assert_eq!(tokens[0].start_line, 10);
        // "Acme" should be on line 11
        let acme = tokens.iter().find(|t| t.value == "Acme").unwrap();
        assert_eq!(acme.start_line, 11);
    }

    #[test]
    fn test_semicolons_split() {
        let lines = vec![(1, "foo;bar;baz".to_string())];
        let tokens = get_tokens(&lines);
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn test_token_count_matches_words() {
        let lines = vec![(1, "Copyright 2024 Acme Inc.".to_string())];
        let tokens = get_tokens(&lines);
        assert!(
            tokens.len() >= 3,
            "Expected at least 3 tokens, got {}",
            tokens.len()
        );
    }

    #[test]
    fn test_retag_camelcase_junk_before_company_suffix_in_copyright_context() {
        let lines = vec![(1, "Copyright (C) OpenSharedMap Inc.".to_string())];
        let tokens = get_tokens(&lines);
        let osm = tokens
            .iter()
            .find(|t| t.value == "OpenSharedMap")
            .expect("OpenSharedMap token should exist");
        assert_eq!(
            osm.tag,
            PosTag::Nnp,
            "Expected OpenSharedMap to be retagged as Nnp in copyright context"
        );
    }

    #[test]
    fn test_does_not_retag_camelcase_junk_outside_copyright_context() {
        let lines = vec![(1, "Use OpenSharedMap Inc. APIs".to_string())];
        let tokens = get_tokens(&lines);
        let osm = tokens
            .iter()
            .find(|t| t.value == "OpenSharedMap")
            .expect("OpenSharedMap token should exist");
        assert_eq!(
            osm.tag,
            PosTag::Junk,
            "OpenSharedMap should remain Junk outside copyright context"
        );
    }

    #[test]
    fn test_does_not_retag_all_caps_token_before_company_suffix() {
        let lines = vec![(
            1,
            "Copyright Owner Inc. AS IS AND ANY EXPRESS OR IMPLIED WARRANTIES".to_string(),
        )];
        let tokens = get_tokens(&lines);
        let as_token = tokens
            .iter()
            .find(|t| t.value == "AS")
            .expect("AS token should exist");
        assert_ne!(
            as_token.tag,
            PosTag::Nnp,
            "All-caps AS token should not be retagged as a name"
        );
    }
}
