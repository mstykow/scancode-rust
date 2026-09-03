// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use std::sync::LazyLock;

use regex::Regex;

use crate::copyright::prepare::prepare_text_line;
use crate::copyright::refiner::refine_author;
use crate::copyright::types::AuthorDetection;
use crate::models::LineNumber;

use super::refine_particle_name;

pub(crate) fn is_pod_author_heading(line: &str) -> bool {
    static AUTHOR_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^=head\d+\s+authors?(?:\s+(?:and|&)\s+(?:maintenance|maintainers?|modification\s+history))?\s*$",
        )
        .expect("valid POD author heading regex")
    });

    AUTHOR_HEADING_RE.is_match(line)
}

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

fn walk_author_section_paragraphs(
    raw_lines: &[&str],
    mut visit: impl FnMut(&str, LineNumber, LineNumber),
) {
    static HEADING_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^=head\d+\b").expect("valid POD heading regex"));
    static BLOCK_DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^=(?:over|back)\b").expect("valid POD block directive regex")
    });
    static ITEM_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^=item\b\s*(?:[*+-]\s*)?(?P<value>.*)$").expect("valid POD item regex")
    });

    let mut in_author_section = false;
    let mut paragraph = String::new();
    let mut paragraph_start = None;
    let mut paragraph_end = None;

    let flush_paragraph =
        |paragraph: &mut String,
         paragraph_start: &mut Option<LineNumber>,
         paragraph_end: &mut Option<LineNumber>,
         visit: &mut dyn FnMut(&str, LineNumber, LineNumber)| {
            if let (Some(start_line), Some(end_line)) = (*paragraph_start, *paragraph_end) {
                visit(paragraph, start_line, end_line);
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
                &mut visit,
            );
            in_author_section = is_pod_author_heading(prepared);
            continue;
        }
        if !in_author_section {
            continue;
        }
        if prepared.len() >= 3
            && prepared.len() <= 16
            && prepared.chars().all(|ch| !ch.is_alphanumeric())
        {
            flush_paragraph(
                &mut paragraph,
                &mut paragraph_start,
                &mut paragraph_end,
                &mut visit,
            );
            in_author_section = false;
            continue;
        }
        if prepared.is_empty()
            || prepared.eq_ignore_ascii_case("=cut")
            || BLOCK_DIRECTIVE_RE.is_match(prepared)
        {
            flush_paragraph(
                &mut paragraph,
                &mut paragraph_start,
                &mut paragraph_end,
                &mut visit,
            );
            if prepared.eq_ignore_ascii_case("=cut") {
                in_author_section = false;
            }
            continue;
        }
        let item_value = ITEM_RE
            .captures(prepared)
            .and_then(|captures| captures.name("value"))
            .map(|matched| matched.as_str().trim());
        if item_value.is_some() {
            flush_paragraph(
                &mut paragraph,
                &mut paragraph_start,
                &mut paragraph_end,
                &mut visit,
            );
        }
        let prepared = item_value.unwrap_or(prepared);
        if prepared.is_empty() {
            continue;
        }
        if paragraph.len() + prepared.len() > 4096 {
            flush_paragraph(
                &mut paragraph,
                &mut paragraph_start,
                &mut paragraph_end,
                &mut visit,
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
        &mut visit,
    );
}

fn strip_trailing_author_date(candidate: &str) -> String {
    static TRAILING_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i),?\s+(?:(?:in\s+)?(?:\d{1,2}(?:st|nd|rd|th)?\s+)?[a-z]+\s+\d{4}|(?:\d{4}-\d{2}-\d{2})|(?:\d{4}(?:-\d{2,4})?))\.?$",
        )
        .expect("valid trailing author date regex")
    });
    static TRAILING_POD_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\s+(?:L\s+)?https?://\S+.*$").expect("valid trailing POD URL regex")
    });

    let without_url = TRAILING_POD_URL_RE.replace(candidate, "");
    TRAILING_DATE_RE
        .replace(without_url.as_ref(), "")
        .trim()
        .to_string()
}

fn is_collective_noun(word: &str) -> bool {
    matches!(
        word.trim_matches(|ch: char| !ch.is_alphanumeric())
            .to_ascii_lowercase()
            .as_str(),
        "contributors" | "developers" | "gang" | "group" | "porters" | "project" | "team"
    )
}

fn refine_contactless_author(candidate: &str) -> Option<String> {
    const NON_NAME_WORDS: &[&str] = &[
        "author",
        "authors",
        "by",
        "copyright",
        "from",
        "help",
        "maintainer",
        "maintainers",
        "protocol",
        "thanks",
        "version",
        "with",
    ];

    if candidate.len() > 160 || candidate.chars().any(|ch| matches!(ch, '@' | ';' | '=')) {
        return None;
    }
    let candidate = strip_trailing_author_date(candidate);
    let candidate = candidate.trim_end_matches('.').trim();
    let author = refine_particle_name(candidate)
        .or_else(|| refine_author(candidate))
        .or_else(|| {
            let mut chars = candidate.chars();
            let starts_uppercase = chars.next().is_some_and(char::is_uppercase);
            let words: Vec<&str> = candidate.split_whitespace().collect();
            ((starts_uppercase && words.len() == 1 && candidate.chars().all(char::is_alphabetic))
                || words.iter().any(|word| is_collective_noun(word)))
            .then(|| candidate.to_string())
        })?;
    let words: Vec<&str> = author.split_whitespace().collect();
    if words.is_empty() || words.len() > 6 {
        return None;
    }
    if words.iter().any(|word| {
        let normalized = word
            .trim_matches(|ch: char| !ch.is_alphanumeric())
            .to_ascii_lowercase();
        NON_NAME_WORDS.contains(&normalized.as_str())
    }) {
        return None;
    }
    if author.chars().any(|ch| {
        !(ch.is_alphanumeric()
            || ch.is_whitespace()
            || matches!(ch, '.' | ',' | '-' | '\'' | '’' | '&'))
    }) {
        return None;
    }

    let cased_words: Vec<char> = words
        .iter()
        .filter_map(|word| word.chars().find(|ch| ch.is_alphabetic()))
        .filter(|ch| ch.is_uppercase() || ch.is_lowercase())
        .collect();
    let uppercase_words = cased_words.iter().filter(|ch| ch.is_uppercase()).count();
    let has_collective_noun = words.iter().any(|word| is_collective_noun(word));
    if cased_words.is_empty()
        || uppercase_words >= 2
        || (words.len() == 1 && uppercase_words == 1)
        || has_collective_noun
    {
        Some(author)
    } else {
        None
    }
}

fn contactless_author_values(paragraph: &str) -> Vec<String> {
    let mut normalized = strip_trailing_author_date(paragraph)
        .trim_end_matches('.')
        .trim()
        .to_string();
    for suffix in [", and others", " and others", ", et al", " et al"] {
        if normalized.to_ascii_lowercase().ends_with(suffix) {
            let end = normalized.len() - suffix.len();
            normalized.truncate(end);
            break;
        }
    }
    let normalized = normalized.replace(", and ", ", ");
    let mut comma_parts: Vec<&str> = normalized
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if comma_parts.len() >= 2
        && let Some(last) = comma_parts.pop()
    {
        if let Some((left, right)) = last.split_once(" and ") {
            comma_parts.push(left.trim());
            comma_parts.push(right.trim());
        } else {
            comma_parts.push(last);
        }
    }
    let candidates = if comma_parts.len() >= 2
        && (comma_parts.len() >= 3
            || comma_parts
                .iter()
                .all(|part| part.split_whitespace().count() >= 2))
    {
        comma_parts
    } else if let Some((left, right)) = normalized.split_once(" and ")
        && (!normalized.split_whitespace().any(is_collective_noun)
            || left.split_whitespace().count() >= 2)
    {
        vec![left.trim(), right.trim()]
    } else {
        vec![normalized.trim()]
    };
    let candidate_count = candidates.len();
    let refined: Vec<String> = candidates
        .into_iter()
        .filter_map(refine_contactless_author)
        .collect();
    if refined.len() != candidate_count {
        return Vec::new();
    }

    refined
}

fn extract_contactless_authors_from_paragraph(
    paragraph: &str,
    start_line: LineNumber,
    end_line: LineNumber,
) -> Vec<AuthorDetection> {
    contactless_author_values(paragraph)
        .into_iter()
        .map(|author| AuthorDetection {
            author,
            start_line,
            end_line,
        })
        .collect()
}

fn truncate_credit_roster(value: &str) -> &str {
    let lower = value.to_ascii_lowercase();
    let end = [
        " for ",
        ", and many ",
        " and many ",
        " and on ",
        ", based on ",
        " based on earlier ",
        ". currently ",
        ". it ",
        ". please ",
        ". the ",
        ". this ",
        " in late ",
        " over the years",
    ]
    .into_iter()
    .filter_map(|boundary| lower.find(boundary))
    .min()
    .unwrap_or(value.len());

    let roster = value[..end].trim().trim_matches(&[' ', ',', '.', ';'][..]);
    roster
        .strip_suffix(" and")
        .or_else(|| roster.strip_suffix(" And"))
        .unwrap_or(roster)
        .trim_end()
}

fn extract_narrative_credit_authors_from_paragraph(
    paragraph: &str,
    start_line: LineNumber,
    end_line: LineNumber,
) -> Vec<AuthorDetection> {
    static CREDIT_CUE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(?:(?:with\s+)?(?:contributions?|(?:invaluable|valuable)\s+help|help|advice)\s+from|with\s+(?:the\s+)?help\s+of|(?:with\s+)?thanks\s+to|supplied\s+by|attributable\s+to|(?:code|documentation|implementation|module|program|software|streamlining|work)\b[^.;]{0,80}?\s+by|(?:authored|created|developed|enhanced|introduced|maintained|modified|originated|rewritten|written)\b[^.;]{0,80}?\s+by)\b",
        )
        .expect("valid POD narrative credit cue regex")
    });
    static SUBJECT_CREDIT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^(?P<names>.+?)\s+(?:has|have)\s+(?:added|contributed|created|maintained|provided|updated|written)\b",
        )
        .expect("valid POD subject credit regex")
    });
    static LEADING_AUTHOR_BEFORE_ROLE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^(?P<names>.+?)\.\s+(?:(?:it|this\s+(?:module|package|program))\s+is\s+)?(?:currently|later|now|previously|subsequently)?\s*$",
        )
        .expect("valid leading author before role regex")
    });

    let mut rosters = Vec::new();
    let cues: Vec<_> = CREDIT_CUE_RE.find_iter(paragraph).collect();
    if let Some(first) = cues.first() {
        let prefix = paragraph[..first.start()].trim();
        if !prefix.is_empty() && (prefix.contains(',') || prefix.contains(" and ")) {
            rosters.extend(contactless_author_values(prefix));
        } else if let Some(names) = LEADING_AUTHOR_BEFORE_ROLE_RE
            .captures(prefix)
            .and_then(|captures| captures.name("names"))
        {
            rosters.extend(contactless_author_values(names.as_str()));
        }
        for (index, cue) in cues.iter().enumerate() {
            let tail_end = cues
                .get(index + 1)
                .map_or(paragraph.len(), |next| next.start());
            let tail = truncate_credit_roster(&paragraph[cue.end()..tail_end]);
            if !tail.is_empty() {
                rosters.extend(contactless_author_values(tail));
            }
        }
    }
    if let Some(captures) = SUBJECT_CREDIT_RE.captures(paragraph)
        && let Some(names) = captures.name("names")
    {
        rosters.extend(contactless_author_values(names.as_str()));
    }

    rosters
        .into_iter()
        .map(|author| AuthorDetection {
            author,
            start_line,
            end_line,
        })
        .collect()
}

/// Recover contact-backed identities from bounded POD AUTHOR(S) sections.
pub(in super::super) fn extract_pod_author_section_contact_authors(
    raw_lines: &[&str],
) -> Vec<AuthorDetection> {
    let mut authors = Vec::new();
    walk_author_section_paragraphs(raw_lines, |paragraph, start_line, end_line| {
        authors.extend(extract_contact_authors_from_paragraph(
            paragraph, start_line, end_line,
        ));
    });
    authors
}

/// Recover short name and collective credits from bounded POD AUTHOR(S) sections.
pub(in super::super) fn extract_pod_author_section_contactless_authors(
    raw_lines: &[&str],
) -> Vec<AuthorDetection> {
    let mut authors = Vec::new();
    walk_author_section_paragraphs(raw_lines, |paragraph, start_line, end_line| {
        if !paragraph.contains('@') {
            authors.extend(extract_contactless_authors_from_paragraph(
                paragraph, start_line, end_line,
            ));
        }
    });

    authors
}

/// Recover cue-backed contributor rosters from bounded POD AUTHOR(S) sections.
pub(in super::super) fn extract_pod_author_section_narrative_credit_authors(
    raw_lines: &[&str],
) -> Vec<AuthorDetection> {
    let mut authors = Vec::new();
    walk_author_section_paragraphs(raw_lines, |paragraph, start_line, end_line| {
        authors.extend(extract_narrative_credit_authors_from_paragraph(
            paragraph, start_line, end_line,
        ));
    });

    authors
}

#[cfg(test)]
mod tests {
    use super::contactless_author_values;

    #[test]
    fn contactless_rosters_keep_particle_names_and_mixed_collectives() {
        assert_eq!(
            contactless_author_values(
                "Gisle Aas, James Duncan, Hugo van der Sanden, Robin Houston, and Rafael Garcia-Suarez."
            ),
            [
                "Gisle Aas",
                "James Duncan",
                "Hugo van der Sanden",
                "Robin Houston",
                "Rafael Garcia-Suarez",
            ]
        );
        assert_eq!(
            contactless_author_values("Larry Wall and the Perl Porters."),
            ["Larry Wall", "the Perl Porters"]
        );
        assert_eq!(
            contactless_author_values("the Perl 5 Porters."),
            ["the Perl 5 Porters"]
        );
        assert_eq!(
            contactless_author_values("Tels http://example.net/"),
            ["Tels"]
        );
    }
}
