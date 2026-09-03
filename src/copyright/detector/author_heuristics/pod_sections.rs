// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use std::sync::LazyLock;

use regex::Regex;

use crate::copyright::prepare::prepare_text_line;
use crate::copyright::refiner::refine_author;
use crate::copyright::types::AuthorDetection;
use crate::models::LineNumber;

fn extract_contact_authors_from_paragraph(
    paragraph: &str,
    start_line: LineNumber,
    end_line: LineNumber,
) -> Vec<AuthorDetection> {
    static CONTACT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)<[^<>]*(?:@|\bat\b|\[at\]|\(at\))[^<>]*>|\([^()\s]*@[^()]*\)|[\w.+-]+@[\w.-]+\.[a-z]{2,}(?:\s*\([^()]{2,80}\))?",
        )
        .unwrap()
    });
    static NON_AUTHOR_CONTACT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^(?:(?:this|the)\s+(?:module|software|library|program)\s+is\s+)?(?:copyright\b|please\s+reports?\s+bugs?\b|send\s+(?:patches|bugs?)\b)",
        )
        .unwrap()
    });
    static ROLE_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^(?:(?:based\s+on\s+ideas|with\s+contributions)\s+from|(?:(?:original|current|previous|prior)\s+)?(?:authors?(?:\s+and\s+maintainers?)?|maintainers?|external\s+protocol|stream\s+protocol|wake-on-lan|original\s+pingecho(?:\(\))?))\s*(?::|\bare\b|\bwas\b)?\s*",
        )
        .unwrap()
    });
    static EMAIL_THEN_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)(?P<email>[\w.+-]+@[\w.-]+\.[a-z]{2,})\s*\((?P<name>[\p{L}][^()]{1,80})\)\s*$",
        )
        .unwrap()
    });

    let lower = paragraph.to_ascii_lowercase();
    let mut authors = Vec::new();
    let mut previous_contact_end = 0;
    for contact in CONTACT_RE.find_iter(paragraph) {
        let prefix = &lower[previous_contact_end..contact.start()];
        let attribution_start = prefix.rfind(" by ").map_or(previous_contact_end, |offset| {
            previous_contact_end + offset + 4
        });
        let candidate = paragraph[attribution_start..contact.end()]
            .trim()
            .trim_start_matches(|ch: char| ch.is_ascii_punctuation() || ch.is_whitespace())
            .trim_start_matches("and ")
            .trim_start_matches("or ")
            .trim();
        previous_contact_end = contact.end();

        if candidate.is_empty() || NON_AUTHOR_CONTACT_RE.is_match(candidate) {
            continue;
        }
        let candidate = ROLE_PREFIX_RE.replace(candidate, "");
        let candidate = candidate.trim();
        if candidate.is_empty() || candidate.split_whitespace().count() > 8 {
            continue;
        }
        let author = if let Some(captures) = EMAIL_THEN_NAME_RE.captures(candidate) {
            let email = captures.name("email").map(|matched| matched.as_str());
            let name = captures.name("name").map(|matched| matched.as_str());
            match (name.and_then(refine_author), email) {
                (Some(name), Some(email)) => Some(format!("{name} <{email}>")),
                _ => None,
            }
        } else {
            refine_author(candidate)
        };
        let Some(author) = author else { continue };
        authors.push(AuthorDetection {
            author,
            start_line,
            end_line,
        });
    }
    authors
}

/// Recover contact-backed identities from bounded POD AUTHOR(S) sections.
pub(in super::super) fn extract_pod_author_section_contact_authors(
    raw_lines: &[&str],
) -> Vec<AuthorDetection> {
    static AUTHOR_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^=head\d+\s+authors?\s*$").expect("valid POD author heading regex")
    });
    static HEADING_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^=head\d+\b").expect("valid POD heading regex"));
    static BLOCK_DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^=(?:over|back|item|cut)\b").expect("valid POD block directive regex")
    });

    let mut authors = Vec::new();
    let mut in_author_section = false;
    let mut paragraph = String::new();
    let mut paragraph_start = None;
    let mut paragraph_end = None;

    let flush_paragraph = |paragraph: &mut String,
                           paragraph_start: &mut Option<LineNumber>,
                           paragraph_end: &mut Option<LineNumber>,
                           authors: &mut Vec<AuthorDetection>| {
        if let (Some(start_line), Some(end_line)) = (*paragraph_start, *paragraph_end) {
            authors.extend(extract_contact_authors_from_paragraph(
                paragraph, start_line, end_line,
            ));
        }
        paragraph.clear();
        *paragraph_start = None;
        *paragraph_end = None;
    };

    for (index, raw_line) in raw_lines.iter().enumerate() {
        let prepared = prepare_text_line(raw_line);
        let prepared = prepared.trim();
        if HEADING_RE.is_match(prepared) {
            flush_paragraph(
                &mut paragraph,
                &mut paragraph_start,
                &mut paragraph_end,
                &mut authors,
            );
            in_author_section = AUTHOR_HEADING_RE.is_match(prepared);
            continue;
        }
        if !in_author_section {
            continue;
        }
        if prepared.is_empty() || BLOCK_DIRECTIVE_RE.is_match(prepared) {
            flush_paragraph(
                &mut paragraph,
                &mut paragraph_start,
                &mut paragraph_end,
                &mut authors,
            );
            if prepared.eq_ignore_ascii_case("=cut") {
                in_author_section = false;
            }
            continue;
        }
        if paragraph.len() + prepared.len() > 4096 {
            flush_paragraph(
                &mut paragraph,
                &mut paragraph_start,
                &mut paragraph_end,
                &mut authors,
            );
        }
        if prepared.len() > 4096 {
            continue;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(prepared);
        let line = LineNumber::from_0_indexed(index);
        paragraph_start.get_or_insert(line);
        paragraph_end = Some(line);
    }
    flush_paragraph(
        &mut paragraph,
        &mut paragraph_start,
        &mut paragraph_end,
        &mut authors,
    );

    authors
}
