// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use crate::parser_warn as warn;
use crate::parsers::utils::{CappedIterExt, MAX_FIELD_LENGTH, truncate_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pep508Requirement {
    pub name: String,
    pub extras: Vec<String>,
    pub specifiers: Option<String>,
    pub marker: Option<String>,
    pub url: Option<String>,
    pub is_name_at_url: bool,
}

pub(crate) fn parse_pep508_requirement(input: &str) -> Option<Pep508Requirement> {
    if input.len() > MAX_FIELD_LENGTH {
        warn!(
            "pep508: input exceeds MAX_FIELD_LENGTH ({} bytes), skipping",
            input.len()
        );
        return None;
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.splitn(2, ';');
    let requirement_part = parts.next().unwrap_or_default().trim();
    let marker = parts
        .next()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if requirement_part.is_empty() {
        return None;
    }

    if let Some((name_part, url)) = split_name_at_url(requirement_part) {
        let (name, extras, _rest) = parse_name_and_extras(&name_part)?;
        return Some(Pep508Requirement {
            name: truncate_field(name),
            extras,
            specifiers: None,
            marker: marker.map(truncate_field),
            url: Some(truncate_field(url)),
            is_name_at_url: true,
        });
    }

    let (name, extras, rest) = parse_name_and_extras(requirement_part)?;
    let specifiers = normalize_specifiers(rest);

    // Whatever follows the name must be a version specifier. Accepting it
    // unchecked meant any prose starting with a word-like token parsed as a
    // requirement, so `import os` became `pkg:pypi/import` and a line of licence
    // text became a dependency named after its first word.
    if specifiers
        .as_deref()
        .is_some_and(|specifiers| !is_valid_specifier_set(specifiers))
    {
        return None;
    }

    Some(Pep508Requirement {
        name: truncate_field(name),
        extras,
        specifiers: specifiers.map(truncate_field),
        marker: marker.map(truncate_field),
        url: None,
        is_name_at_url: false,
    })
}

fn split_name_at_url(input: &str) -> Option<(String, String)> {
    if let Some((left, right)) = input.split_once(" @ ") {
        let name = left.trim();
        let url = right.trim();
        if !name.is_empty() && !url.is_empty() {
            return Some((name.to_string(), url.to_string()));
        }
    }

    if let Some((left, right)) = input.split_once('@') {
        let name = left.trim();
        let url = right.trim();
        if !name.is_empty() && !url.is_empty() && (url.contains("://") || url.starts_with("file:"))
        {
            return Some((name.to_string(), url.to_string()));
        }
    }

    None
}

/// True for a name matching PEP 508's `identifier` grammar:
/// `letterOrDigit (letterOrDigit | '-' | '_' | '.')* letterOrDigit`, i.e. it must
/// start and end alphanumeric and contain only alphanumerics and `-_.` between.
///
/// Without this check any text before the first specifier character is taken as a
/// distribution name, so a line that is not a requirement at all — a
/// reStructuredText `::` literal-block marker, a table rule, prose — becomes a
/// package, and its name is then spliced straight into a PURL. Rejecting the name
/// here lets callers fall through to their link/URL handling or drop the line,
/// which is what pip's own parser does with an unparsable requirement.
pub(crate) fn is_valid_distribution_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    let Some(last) = name.chars().next_back() else {
        return false;
    };
    if !last.is_ascii_alphanumeric() {
        return false;
    }
    name.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn parse_name_and_extras(input: &str) -> Option<(String, Vec<String>, &str)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut name_end = trimmed.len();
    for (idx, ch) in trimmed.char_indices().capped("pep508 name characters") {
        if ch == '[' || ch.is_whitespace() || matches!(ch, '<' | '>' | '=' | '!' | '~' | ';') {
            name_end = idx;
            break;
        }
    }

    let name = trimmed[..name_end].trim();
    if !is_valid_distribution_name(name) {
        return None;
    }

    let mut extras = Vec::new();
    let mut rest = &trimmed[name_end..];

    let rest_trimmed = rest.trim_start();
    if rest_trimmed.starts_with('[')
        && let Some(close_idx) = rest_trimmed.find(']')
    {
        let extras_str = &rest_trimmed[1..close_idx];
        extras = extras_str
            .split(',')
            .capped("pep508 extras")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| truncate_field(value.to_string()))
            .collect();
        rest = &rest_trimmed[close_idx + 1..];
    }

    Some((name.to_string(), extras, rest))
}

/// True for a PEP 440 specifier set: comma-separated clauses, each an operator
/// followed by a version. PEP 508 also permits the whole set to be parenthesised
/// (`foo (>=1.0)`), which `pyproject.toml` dependency strings do use.
///
/// Input arrives whitespace-stripped from [`normalize_specifiers`].
fn is_valid_specifier_set(specifiers: &str) -> bool {
    const OPERATORS: [&str; 8] = ["===", "==", "!=", "<=", ">=", "~=", "<", ">"];

    let specifiers = specifiers
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(specifiers);

    if specifiers.is_empty() {
        return false;
    }

    specifiers.split(',').all(|clause| {
        OPERATORS.iter().any(|operator| {
            clause.strip_prefix(operator).is_some_and(|version| {
                // `===` is PEP 440 arbitrary-string equality, so its operand is
                // deliberately unconstrained. Every other operator takes a
                // version, and accepting arbitrary text there let `hello==world`
                // through as `pkg:pypi/hello@world`.
                if *operator == "===" {
                    !version.is_empty()
                } else {
                    is_valid_version(version)
                }
            })
        })
    })
}

/// True for a PEP 440 version as it appears in a specifier: an optional `v`
/// prefix, then a digit, then only characters the grammar can produce — digits
/// and letters for pre/post/dev segments, `.` separators, `!` for an epoch, `+`
/// for a local version, `-`/`_` for the normalising forms, and `*` for the
/// `==1.4.*` prefix match.
fn is_valid_version(version: &str) -> bool {
    let version = version.strip_prefix(['v', 'V']).unwrap_or(version);
    version.starts_with(|ch: char| ch.is_ascii_digit())
        && version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '!' | '+' | '-' | '_' | '*'))
}

fn normalize_specifiers(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized: String = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}
