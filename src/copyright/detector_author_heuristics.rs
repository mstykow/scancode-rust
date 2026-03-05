use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::normalize_whitespace;
use crate::copyright::refiner::refine_author;
use crate::copyright::types::{AuthorDetection, CopyrightDetection, HolderDetection};

pub(super) fn extract_multiline_written_by_author_blocks(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }

    static WRITTEN_BY_SINGLE_LINE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*written\s+by\s+(?P<who>.+?)(?:\s+for\b|$)").unwrap());
    static AUTHOR_EMAIL_HEAD_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^(?P<head>.+?<[^>]+>)(?:\s+(?:for|to)\b.*)?$").unwrap());
    static WRITTEN_BY_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^(?:original(?:ly)?\s+)?(?:original\s+driver\s+)?(?:written|authored|created|developed)\s+by\s+(?P<who>.+)$",
        )
        .unwrap()
    });

    let lines: Vec<&str> = content.lines().collect();
    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in lines.iter().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }
        if !line.to_ascii_lowercase().starts_with("written by ") {
            continue;
        }

        if let Some(cap) = WRITTEN_BY_SINGLE_LINE_RE.captures(line) {
            let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
            if who.is_empty() {
                continue;
            }
            let who_words: Vec<&str> = who.split_whitespace().collect();
            if who_words.len() < 2 {
                continue;
            }

            let has_email = who.contains('@') || who.contains('<');
            if !has_email {
                continue;
            }

            let who = if let Some(cap) = AUTHOR_EMAIL_HEAD_RE.captures(who) {
                cap.name("head").map(|m| m.as_str()).unwrap_or(who).trim()
            } else {
                who
            };

            if let Some(author) = refine_author(who)
                && seen.insert(author.clone())
            {
                authors.push(AuthorDetection {
                    author,
                    start_line: ln,
                    end_line: ln,
                });
            }
        }
    }

    let mut i = 0;
    while i < lines.len() {
        let ln = i + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(lines[i]);
        let line = prepared.trim();
        let lower = line.to_ascii_lowercase();

        let is_start = !line.is_empty()
            && !lower.starts_with("copyright")
            && !lower.contains("copyright")
            && (lower.starts_with("written by ")
                || lower.starts_with("originally written by ")
                || lower.starts_with("original driver written by ")
                || lower.contains(" written by "));

        if !is_start {
            i += 1;
            continue;
        }

        let mut block_lines: Vec<(usize, String)> = Vec::new();
        block_lines.push((ln, line.to_string()));

        let mut j = i + 1;
        while j < lines.len() {
            let next_ln = j + 1;
            let next_prepared = crate::copyright::prepare::prepare_text_line(lines[j]);
            let next_line = next_prepared.trim();
            if next_line.is_empty() {
                break;
            }
            let next_lower = next_line.to_ascii_lowercase();
            if next_lower.starts_with("copyright") {
                break;
            }
            if !(next_lower.contains(" by ")
                || next_lower.starts_with("overhauled by ")
                || next_lower.starts_with("ported ")
                || next_lower.starts_with("updated ")
                || next_lower.starts_with("kernel ")
                || next_lower.starts_with("extensive ")
                || next_lower.starts_with("revised ")
                || next_lower.starts_with("implemented ")
                || next_lower.starts_with("copied from "))
            {
                break;
            }

            block_lines.push((next_ln, next_line.to_string()));
            j += 1;
        }

        if block_lines.len() < 2 {
            i += 1;
            continue;
        }

        let start_line = block_lines.first().map(|(l, _)| *l).unwrap_or(ln);
        let end_line = block_lines.last().map(|(l, _)| *l).unwrap_or(ln);

        let mut segments: Vec<String> = Vec::new();
        for (_l, raw_line) in &block_lines {
            let candidate = raw_line.trim();
            if let Some(cap) = WRITTEN_BY_PREFIX_RE.captures(candidate) {
                let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
                if !who.is_empty() {
                    segments.push(who.to_string());
                    continue;
                }
            }
            segments.push(candidate.to_string());
        }

        let combined_raw = segments.join(" ");
        if let Some(combined) = refine_author(&combined_raw)
            && seen.insert(combined.clone())
        {
            authors.retain(|a| a.start_line < start_line || a.end_line > end_line);
            authors.push(AuthorDetection {
                author: combined,
                start_line,
                end_line,
            });
        }

        i = j;
    }
}

pub(super) fn extract_module_author_macros(
    content: &str,
    copyrights: &[CopyrightDetection],
    holders: &[HolderDetection],
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }
    if !copyrights.is_empty() || !holders.is_empty() || !authors.is_empty() {
        return;
    }

    static MODULE_AUTHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)MODULE_AUTHOR\s*\(\s*\"(?P<who>[^\"]+)\"\s*\)"#).unwrap()
    });

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();
    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let line = raw.trim();
        if line.is_empty() || !line.contains("MODULE_AUTHOR") {
            continue;
        }

        for cap in MODULE_AUTHOR_RE.captures_iter(line) {
            let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
            if who.is_empty() {
                continue;
            }
            let who = who.replace(r#"\""#, "\"");
            let Some(author) = refine_author(&who) else {
                continue;
            };
            if seen.insert(author.clone()) {
                authors.push(AuthorDetection {
                    author,
                    start_line: ln,
                    end_line: ln,
                });
            }
        }
    }
}

pub(super) fn extract_was_developed_by_author_blocks(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }

    static WAS_DEVELOPED_BY_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bwas\s+developed\s+by\s+(?P<who>.+)$").unwrap());
    static WITH_PARTICIPATION_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bwith\s+participation\b").unwrap());

    let lines: Vec<&str> = content.lines().collect();
    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    let mut i = 0;
    while i < lines.len() {
        let ln = i + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(lines[i]);
        let line = prepared.trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        let Some(cap) = WAS_DEVELOPED_BY_RE.captures(line) else {
            i += 1;
            continue;
        };
        let mut parts: Vec<String> = Vec::new();
        let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
        if who.is_empty() {
            i += 1;
            continue;
        }
        parts.push(who.to_string());

        let mut end_ln = ln;
        let mut j = i + 1;
        while j < lines.len() {
            let next_ln = j + 1;
            let next_prepared = crate::copyright::prepare::prepare_text_line(lines[j]);
            let next_line = next_prepared.trim();
            if next_line.is_empty() {
                break;
            }

            let next_lower = next_line.to_ascii_lowercase();
            if next_lower.starts_with("copyright") {
                break;
            }

            if let Some(m) = WITH_PARTICIPATION_RE.find(next_line) {
                let prefix = next_line[..m.start()].trim_end();
                if !prefix.is_empty() {
                    parts.push(prefix.to_string());
                    end_ln = next_ln;
                }
                break;
            }

            parts.push(next_line.to_string());
            end_ln = next_ln;

            if end_ln.saturating_sub(ln) >= 3 {
                break;
            }

            j += 1;
        }

        let joined = parts.join(" ");
        let joined = joined.split_whitespace().collect::<Vec<_>>().join(" ");
        if joined.is_empty() {
            i += 1;
            continue;
        }

        let author = refine_author(&joined).unwrap_or(joined);
        if author.is_empty() {
            i += 1;
            continue;
        }

        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author,
                start_line: ln,
                end_line: end_ln,
            });
        }

        i += 1;
    }
}

pub(super) fn extract_author_colon_blocks(content: &str, authors: &mut Vec<AuthorDetection>) {
    if content.is_empty() {
        return;
    }

    static AUTHOR_COLON_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^author(?:s|\(s\)|s\(s\))?\s*:\s*(?P<tail>.+)$").unwrap()
    });
    static YEAR_ONLY_COPY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^copyright\s+\(c\)\s*(?:\d{4}(?:\s*,\s*\d{4})*|\d{4}-\d{4})\s*$").unwrap()
    });

    let lines: Vec<&str> = content.lines().collect();
    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    let mut i = 0;
    while i < lines.len() {
        let ln = i + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(lines[i]);
        let line = prepared.trim().trim_start_matches('*').trim_start();
        if line.is_empty() {
            i += 1;
            continue;
        }

        let Some(cap) = AUTHOR_COLON_RE.captures(line) else {
            i += 1;
            continue;
        };

        let mut skip = false;
        let mut prev_idx = i;
        while prev_idx > 0 {
            prev_idx -= 1;
            let prev_prepared = crate::copyright::prepare::prepare_text_line(lines[prev_idx]);
            let prev = prev_prepared.trim();
            if prev.is_empty() {
                continue;
            }
            if YEAR_ONLY_COPY_RE.is_match(prev) {
                skip = true;
            }
            break;
        }
        if skip {
            i += 1;
            continue;
        }

        let tail = cap.name("tail").map(|m| m.as_str()).unwrap_or("").trim();
        if tail.is_empty() {
            i += 1;
            continue;
        }

        let label_raw = line.split(':').next().unwrap_or("").trim();
        let label_is_all_caps = !label_raw.is_empty()
            && label_raw.chars().any(|c| c.is_ascii_uppercase())
            && !label_raw.chars().any(|c| c.is_ascii_lowercase());
        if label_is_all_caps {
            i += 1;
            continue;
        }

        let mut segments: Vec<String> = vec![tail.to_string()];
        let mut j = i + 1;
        let mut added = 0usize;
        while j < lines.len() {
            let next_prepared = crate::copyright::prepare::prepare_text_line(lines[j]);
            let next_line = next_prepared.trim().trim_start_matches('*').trim_start();
            if next_line.is_empty() {
                break;
            }
            let next_lower = next_line.to_ascii_lowercase();
            if next_lower.starts_with("copyright") {
                break;
            }
            if next_lower.starts_with("fixed") || next_lower.starts_with("software") {
                break;
            }
            if next_lower.starts_with("updated")
                || next_lower.starts_with("date")
                || next_lower.starts_with("borrows")
                || next_lower.starts_with("files")
            {
                break;
            }
            if next_lower.starts_with("et al") {
                break;
            }

            let mut include = false;
            if next_line.contains(':') {
                if next_lower.starts_with("devices")
                    || next_lower.starts_with("status")
                    || next_lower.starts_with("return")
                {
                    include = true;
                } else {
                    break;
                }
            }
            if !include {
                include = next_line.contains('@')
                    || next_line.contains('<')
                    || next_line.contains(',')
                    || next_line
                        .chars()
                        .find(|c| !c.is_whitespace())
                        .is_some_and(|c| c.is_ascii_uppercase());
            }
            if include {
                segments.push(next_line.to_string());
                added += 1;
                j += 1;
                if added >= 4 {
                    break;
                }
                let combined_len: usize = segments.iter().map(|s| s.len()).sum();
                if combined_len > 320 {
                    break;
                }
                if next_lower.starts_with("return") {
                    break;
                }
                if next_lower.starts_with("devices") {
                    let tail = next_line
                        .split_once(':')
                        .map(|(_, t)| t.trim())
                        .unwrap_or("");
                    if !tail.is_empty() {
                        break;
                    }
                }
                continue;
            }
            break;
        }

        let start_line = ln;
        let end_line = if j == i + 1 { start_line } else { j };
        let combined_raw = segments.join(" ");
        let Some(combined) = refine_author(&combined_raw) else {
            i += 1;
            continue;
        };

        if seen.insert(combined.clone()) {
            authors.retain(|a| a.start_line < start_line || a.end_line > end_line);
            authors.push(AuthorDetection {
                author: combined,
                start_line,
                end_line,
            });
        }

        i = j;
    }
}

pub(super) fn extract_code_written_by_author_blocks(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }

    static HEADER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bcode\s+written\s+by\b").unwrap());
    static BODY_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)\bwritten\s+by\s+(?P<body>.+)$").unwrap());
    static STOP_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)(?P<prefix>.+?\bDonald\s+wrote\s+the\s+SMC\s+91c92\s+code)\b").unwrap()
    });

    let lines: Vec<&str> = content.lines().collect();
    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    let mut i = 0;
    while i < lines.len() {
        let ln = i + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(lines[i]);
        let line = prepared.trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        if !HEADER_RE.is_match(line) {
            i += 1;
            continue;
        }

        let mut combined = line.to_string();
        let mut j = i + 1;
        while j < lines.len() {
            let next_prepared = crate::copyright::prepare::prepare_text_line(lines[j]);
            let next = next_prepared.trim();
            if next.is_empty() {
                break;
            }
            combined.push(' ');
            combined.push_str(next);
            if next.contains(".  ") || next.ends_with('.') {
                break;
            }
            if combined.len() > 800 {
                break;
            }
            j += 1;
        }

        let Some(cap) = BODY_RE.captures(&combined) else {
            i = j;
            continue;
        };
        let body = cap.name("body").map(|m| m.as_str()).unwrap_or("").trim();
        if body.is_empty() {
            i = j;
            continue;
        }

        let mut candidate = body.to_string();
        if let Some(cap2) = STOP_RE.captures(body) {
            let prefix = cap2.name("prefix").map(|m| m.as_str()).unwrap_or("").trim();
            if !prefix.is_empty() {
                candidate = prefix.to_string();
            }
        }

        let Some(author) = refine_author(&candidate) else {
            i = j;
            continue;
        };
        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author,
                start_line: ln,
                end_line: j,
            });
        }

        i = j;
    }
}

pub(super) fn extract_developed_and_created_by_authors(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    static PREFIX_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*developed\s+and\s+created\s+by\s+").unwrap());
    static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bhttps?://\S+").unwrap());
    static IFROSS_TAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\bon\s+free\s+and\s+open\s+source\s+software\b.*$").unwrap()
    });

    let raw_lines: Vec<&str> = content.lines().collect();
    if raw_lines.is_empty() {
        return;
    }

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for start_idx in 0..raw_lines.len() {
        let prepared0 = crate::copyright::prepare::prepare_text_line(raw_lines[start_idx]);
        if !PREFIX_RE.is_match(prepared0.trim()) {
            continue;
        }

        let mut parts: Vec<String> = Vec::new();
        let mut end_idx = start_idx;

        for (idx, raw) in raw_lines.iter().enumerate().skip(start_idx) {
            let prepared = crate::copyright::prepare::prepare_text_line(raw);
            let line = prepared.trim();
            if line.is_empty() {
                break;
            }
            if line.to_ascii_lowercase().contains("http") {
                break;
            }

            let piece = if idx == start_idx {
                PREFIX_RE.replace(line, "").to_string()
            } else {
                line.to_string()
            };
            if !piece.trim().is_empty() {
                parts.push(piece);
            }
            end_idx = idx;
        }

        if parts.is_empty() {
            continue;
        }

        let mut combined = normalize_whitespace(&parts.join(" "));
        combined = combined.replace(['(', ')'], " ");
        combined = URL_RE.replace_all(&combined, " ").into_owned();
        combined = IFROSS_TAIL_RE.replace_all(&combined, " ").into_owned();
        combined = normalize_whitespace(&combined);
        combined = combined.trim().to_string();
        if combined.is_empty() {
            continue;
        }

        let Some(author) = refine_author(&combined) else {
            continue;
        };
        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author: author.clone(),
                start_line: start_idx + 1,
                end_line: end_idx + 1,
            });
        }

        authors.retain(|a| !(author.starts_with(&a.author) && a.author.len() < author.len()));
    }
}

pub(super) fn extract_with_additional_hacking_by_authors(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^\s*with\s+additional\s+hacking\s+by\s+(?P<who>.+?)\s*$").unwrap()
    });

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }
        let Some(cap) = RE.captures(line) else {
            continue;
        };
        let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
        if who.is_empty() {
            continue;
        }
        if let Some(author) = refine_author(who)
            && seen.insert(author.clone())
        {
            authors.push(AuthorDetection {
                author,
                start_line: ln,
                end_line: ln,
            });
        }
    }
}

pub(super) fn merge_metadata_author_and_email_lines(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    let raw_lines: Vec<&str> = content.lines().collect();
    let prepared_lines: Vec<String> = raw_lines
        .iter()
        .map(|l| crate::copyright::prepare::prepare_text_line(l))
        .collect();

    if !prepared_lines
        .iter()
        .any(|l| l.trim_start().starts_with("Metadata-Version:"))
    {
        return;
    }

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for idx in 0..prepared_lines.len() {
        let author_ln = idx + 1;
        let author_line = prepared_lines[idx].trim();
        if author_line.is_empty() {
            continue;
        }
        if !author_line.to_ascii_lowercase().starts_with("author:") {
            continue;
        }
        let Some((_, name_raw)) = author_line.split_once(':') else {
            continue;
        };
        let name = name_raw.trim();
        if name.is_empty() {
            continue;
        }

        for (j, email_line_prepared) in prepared_lines.iter().enumerate().skip(idx + 1) {
            let email_ln = j + 1;
            let email_line = email_line_prepared.trim();
            if email_line.is_empty() {
                break;
            }
            if email_line.to_ascii_lowercase().starts_with("author:") {
                break;
            }

            if !email_line.to_ascii_lowercase().starts_with("author-email") {
                continue;
            }
            let Some((_, email_raw)) = email_line.split_once(':') else {
                continue;
            };
            let email = email_raw.trim();
            if email.is_empty() {
                continue;
            }

            let combined_raw = format!("{name} Author-email {email}");
            let combined = normalize_whitespace(&combined_raw);

            if seen.insert(combined.clone()) {
                authors.push(AuthorDetection {
                    author: combined,
                    start_line: author_ln,
                    end_line: email_ln,
                });
            }

            authors.retain(|a| {
                if a.start_line == author_ln && a.end_line == author_ln && a.author == name {
                    return false;
                }
                if a.start_line == email_ln
                    && a.end_line == email_ln
                    && a.author.to_ascii_lowercase() == format!("author-email {email}")
                {
                    return false;
                }
                true
            });

            break;
        }
    }
}

pub(super) fn extract_debian_maintainer_authors(content: &str, authors: &mut Vec<AuthorDetection>) {
    if content.is_empty() {
        return;
    }

    static DEBIANIZED_BY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\bdebianized\s+by\s+(?P<who>.+?)(?:\s+on\b|\s*$)").unwrap()
    });
    static CO_MAINTAINER_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^(?:debianized\s+by|new\s+co-maintainer|co-maintainer)\s+(?P<who>.+?)(?:\s+\d{4}-\d{2}-\d{1,2})?\s*$",
        )
        .unwrap()
    });
    static MAINTAINED_BY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^maintained\s+by\s+(?P<who>.+?)(?:\s+on\b|\s+since\b|\s*$)").unwrap()
    });

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }

        let who_raw = if let Some(cap) = CO_MAINTAINER_RE.captures(line) {
            cap.name("who").map(|m| m.as_str()).unwrap_or("")
        } else if let Some(cap) = DEBIANIZED_BY_RE.captures(line) {
            cap.name("who").map(|m| m.as_str()).unwrap_or("")
        } else if let Some(cap) = MAINTAINED_BY_RE.captures(line) {
            cap.name("who").map(|m| m.as_str()).unwrap_or("")
        } else {
            ""
        };

        let who = who_raw.trim();
        if who.is_empty() {
            continue;
        }

        let Some(author) = refine_author(who) else {
            continue;
        };

        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author,
                start_line: ln,
                end_line: ln,
            });
        }
    }
}

pub(super) fn extract_created_by_project_author(content: &str, authors: &mut Vec<AuthorDetection>) {
    if content.is_empty() {
        return;
    }

    static CREATED_BY_PROJECT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bcreated\s+by\s+the\s+project\b").unwrap());

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        if CREATED_BY_PROJECT_RE.is_match(prepared.trim()) {
            let author = "the Project".to_string();
            if seen.insert(author.clone()) {
                authors.push(AuthorDetection {
                    author,
                    start_line: ln,
                    end_line: ln,
                });
            }
            break;
        }
    }
}

pub(super) fn extract_created_by_authors(content: &str, authors: &mut Vec<AuthorDetection>) {
    if content.is_empty() {
        return;
    }

    static CREATED_BY_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*created\s+by\s+(?P<who>.+?)\s*$").unwrap());

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }

        let Some(cap) = CREATED_BY_RE.captures(line) else {
            continue;
        };
        let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
        if who.is_empty() {
            continue;
        }

        let who_lower = who.to_ascii_lowercase();
        let has_email_like =
            who.contains('@') || (who_lower.contains(" at ") && who_lower.contains(" dot "));
        if !has_email_like {
            continue;
        }

        let Some(author) = refine_author(who) else {
            continue;
        };
        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author: author.clone(),
                start_line: ln,
                end_line: ln,
            });
        }

        authors.retain(|a| !(author.starts_with(&a.author) && a.author.len() < author.len()));
    }
}

pub(super) fn extract_written_by_comma_and_copyright_authors(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }

    static WRITTEN_BY_AND_COPYRIGHT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\bwritten\s+by\s+(?P<who>.+?),\s+and\s+copyright\b").unwrap()
    });

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }

        let Some(cap) = WRITTEN_BY_AND_COPYRIGHT_RE.captures(line) else {
            continue;
        };
        let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
        if who.is_empty() {
            continue;
        }
        let author = format!("{who}, and");
        if seen.insert(author.clone()) {
            authors.retain(|a| !(a.start_line == ln && a.end_line == ln));
            authors.push(AuthorDetection {
                author,
                start_line: ln,
                end_line: ln,
            });
        }
    }
}

pub(super) fn extract_developed_by_sentence_authors(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }

    static DEVELOPED_BY_PREFIX_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*developed\s+by\s+(?P<rest>.+)$").unwrap());

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }

        let Some(cap) = DEVELOPED_BY_PREFIX_RE.captures(line) else {
            continue;
        };
        let rest = cap.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
        if rest.is_empty() {
            continue;
        }

        let rest_lower = rest.to_ascii_lowercase();
        let Some(is_idx) = rest_lower.find(" is ") else {
            continue;
        };
        let before_is = rest[..is_idx].trim_end();
        let Some(split_idx) = before_is.rfind(". ") else {
            continue;
        };
        let p1 = before_is[..split_idx + 1].trim();
        let p2 = before_is[split_idx + 2..].trim();
        if p1.is_empty() || p2.is_empty() {
            continue;
        }

        let candidate = format!("{p1} {p2}");
        let author = refine_author(&candidate).unwrap_or(candidate);
        if author.is_empty() {
            continue;
        }

        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author,
                start_line: ln,
                end_line: ln,
            });
        }
    }
}

pub(super) fn extract_developed_by_phrase_authors(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }

    static DEVELOPED_BY_PHRASE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\bdeveloped\s+by\s+(?P<who>.+?)\s+and\s+to\s+credit\b").unwrap()
    });

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }

        for cap in DEVELOPED_BY_PHRASE_RE.captures_iter(line) {
            let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
            if who.is_empty() {
                continue;
            }

            if who.split_whitespace().count() < 4 {
                continue;
            }

            let author = refine_author(who).unwrap_or_else(|| who.to_string());
            if author.is_empty() {
                continue;
            }

            if seen.insert(author.clone()) {
                authors.push(AuthorDetection {
                    author,
                    start_line: ln,
                    end_line: ln,
                });
            }
        }
    }
}

pub(super) fn extract_maintained_by_authors(content: &str, authors: &mut Vec<AuthorDetection>) {
    if content.is_empty() {
        return;
    }

    static MAINTAINED_BY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\bmaintained\s+by\s+(?P<who>.+?)(?:\s+(?:on|since|for)\b|$)").unwrap()
    });

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim();
        if line.is_empty() {
            continue;
        }
        for cap in MAINTAINED_BY_RE.captures_iter(line) {
            let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
            if who.is_empty() {
                continue;
            }
            if !who.to_ascii_lowercase().starts_with("the ") {
                continue;
            }
            let Some(author) = refine_author(who) else {
                continue;
            };
            if seen.insert(author.clone()) {
                authors.push(AuthorDetection {
                    author,
                    start_line: ln,
                    end_line: ln,
                });
            }
        }
    }
}

pub(super) fn extract_converted_to_by_authors(content: &str, authors: &mut Vec<AuthorDetection>) {
    if content.is_empty() {
        return;
    }

    static CONVERTED_BY_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*converted\b.*\bby\s+(?P<who>.+)$").unwrap());
    static CONVERTED_TO_THE_BY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^\s*converted\s+to\s+the\b.*\bby\s+(?P<who>.+)$").unwrap()
    });
    static CONVERTED_TO_VERSION_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bconverted\s+to\s+\d+\.\d+\b").unwrap());

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim().trim_start_matches('*').trim_start();
        if line.is_empty() {
            continue;
        }

        if CONVERTED_TO_VERSION_RE.is_match(line) {
            continue;
        }

        let mut add_converted_variant = false;
        let who_raw = if let Some(cap) = CONVERTED_TO_THE_BY_RE.captures(line) {
            add_converted_variant = true;
            cap.name("who").map(|m| m.as_str()).unwrap_or("")
        } else if let Some(cap) = CONVERTED_BY_RE.captures(line) {
            cap.name("who").map(|m| m.as_str()).unwrap_or("")
        } else {
            ""
        };

        let who = who_raw.trim();
        if who.is_empty() {
            continue;
        }

        if !who.contains('@') && !who.contains('<') {
            continue;
        }
        let Some(author) = refine_author(who) else {
            continue;
        };
        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author: author.clone(),
                start_line: ln,
                end_line: ln,
            });
        }
        if add_converted_variant {
            let converted = format!("{author} Converted");
            if seen.insert(converted.clone()) {
                authors.push(AuthorDetection {
                    author: converted,
                    start_line: ln,
                    end_line: ln,
                });
            }
        }
    }
}

pub(super) fn extract_various_bugfixes_and_enhancements_by_authors(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if content.is_empty() {
        return;
    }

    static VARIOUS_BUGFIXES_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^\s*various\s+bugfixes\s+and\s+enhancements\s+by\s+(?P<who>.+)$").unwrap()
    });

    let mut seen: HashSet<String> = authors.iter().map(|a| a.author.clone()).collect();

    for (idx, raw) in content.lines().enumerate() {
        let ln = idx + 1;
        let prepared = crate::copyright::prepare::prepare_text_line(raw);
        let line = prepared.trim().trim_start_matches('*').trim_start();
        if line.is_empty() {
            continue;
        }
        let Some(cap) = VARIOUS_BUGFIXES_RE.captures(line) else {
            continue;
        };
        let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
        if who.is_empty() {
            continue;
        }
        if !who.contains('@') && !who.contains('<') {
            continue;
        }
        let Some(author) = refine_author(who) else {
            continue;
        };
        if seen.insert(author.clone()) {
            authors.push(AuthorDetection {
                author,
                start_line: ln,
                end_line: ln,
            });
        }
    }
}

pub(super) fn drop_authors_embedded_in_copyrights(
    copyrights: &[CopyrightDetection],
    authors: &mut Vec<AuthorDetection>,
) {
    if copyrights.is_empty() || authors.is_empty() {
        return;
    }

    authors.retain(|a| {
        let a_lower = a.author.to_lowercase();
        !copyrights.iter().any(|c| {
            if a.start_line < c.start_line || a.end_line > c.end_line {
                return false;
            }
            let c_lower = c.copyright.to_lowercase();
            if c_lower.contains("authors") {
                if a.author.contains('@') {
                    return false;
                }
                return c_lower.contains(&a_lower);
            }
            if c_lower.contains("author") {
                return c_lower.contains(&a_lower);
            }
            false
        })
    });
}

pub(super) fn drop_shadowed_prefix_authors(authors: &mut Vec<AuthorDetection>) {
    if authors.len() < 2 {
        return;
    }
    let mut drop: Vec<bool> = vec![false; authors.len()];
    for i in 0..authors.len() {
        let a = authors[i].author.trim();
        if a.is_empty() {
            continue;
        }
        for (j, other) in authors.iter().enumerate() {
            if i == j {
                continue;
            }
            let b = other.author.trim();
            if b.len() <= a.len() {
                continue;
            }
            if let Some(stripped) = b.strip_prefix(a) {
                let tail = stripped.trim_start();
                let boundary = b
                    .as_bytes()
                    .get(a.len())
                    .is_some_and(|ch| ch.is_ascii_whitespace() || matches!(ch, b',' | b'/' | b'('));

                let a_has_email = a.contains('@') || a.contains('<');
                let b_has_email = b.contains('@') || b.contains('<');

                let short_word = a.split_whitespace().count() == 1;
                if short_word && boundary {
                    if a.chars().all(|c| c.is_ascii_lowercase()) {
                        continue;
                    }
                    drop[i] = true;
                    break;
                }
                if !a_has_email && b_has_email && boundary {
                    drop[i] = true;
                    break;
                }
                if boundary {
                    let tail_lower = tail.to_ascii_lowercase();
                    if a_has_email && b_has_email {
                        continue;
                    }
                    if tail.starts_with(',')
                        || tail.starts_with('<')
                        || tail_lower.starts_with("or")
                        || tail_lower.starts_with("and")
                        || tail_lower.starts_with("author-email")
                    {
                        drop[i] = true;
                        break;
                    }
                }
            }
        }
    }
    if drop.iter().all(|d| !*d) {
        return;
    }
    let mut kept = Vec::with_capacity(authors.len());
    for (i, a) in authors.iter().cloned().enumerate() {
        if !drop[i] {
            kept.push(a);
        }
    }
    *authors = kept;
}

pub(super) fn drop_comedi_ds_status_devices_authors(
    content: &str,
    copyrights: &[CopyrightDetection],
    authors: &mut Vec<AuthorDetection>,
) {
    if authors.is_empty() {
        return;
    }

    let lower = content.to_ascii_lowercase();
    if !lower.contains("author") || !lower.contains("status") {
        return;
    }
    if !content.lines().any(|l| l.contains("Author: ds")) {
        return;
    }

    let has_any_copyright = !copyrights.is_empty();
    let drop_for_national_instruments = lower.contains("national instruments");

    authors.retain(|a| {
        let s = a.author.trim();
        if !s.to_ascii_lowercase().starts_with("ds status") {
            return true;
        }
        if !has_any_copyright {
            return false;
        }
        if drop_for_national_instruments {
            return false;
        }
        true
    });
}

pub(super) fn drop_written_by_authors_preceded_by_copyright(
    content: &str,
    authors: &mut Vec<AuthorDetection>,
) {
    if authors.is_empty() {
        return;
    }

    static WRITTEN_BY_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*written\s+by\s+(?P<who>.+)$").unwrap());
    static COPYRIGHT_HINT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bcopyright\b|\(c\)").unwrap());

    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 {
        return;
    }

    let mut to_drop: HashSet<String> = HashSet::new();
    for i in 1..lines.len() {
        let prepared = crate::copyright::prepare::prepare_text_line(lines[i]);
        let line = prepared.trim();
        let Some(cap) = WRITTEN_BY_RE.captures(line) else {
            continue;
        };
        let who = cap.name("who").map(|m| m.as_str()).unwrap_or("").trim();
        if who.is_empty() {
            continue;
        }
        let prev_prepared = crate::copyright::prepare::prepare_text_line(lines[i - 1]);
        let prev = prev_prepared.trim();
        if !COPYRIGHT_HINT_RE.is_match(prev) {
            continue;
        }
        if let Some(author) = refine_author(who) {
            to_drop.insert(author);
        }
    }

    if to_drop.is_empty() {
        return;
    }
    authors.retain(|a| !to_drop.contains(&a.author));
}
