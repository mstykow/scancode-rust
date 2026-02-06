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
            name,
            extras,
            specifiers: None,
            marker,
            url: Some(url),
            is_name_at_url: true,
        });
    }

    let (name, extras, rest) = parse_name_and_extras(requirement_part)?;
    let specifiers = normalize_specifiers(rest);

    Some(Pep508Requirement {
        name,
        extras,
        specifiers,
        marker,
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

fn parse_name_and_extras(input: &str) -> Option<(String, Vec<String>, &str)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut name_end = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        if ch == '[' || ch.is_whitespace() || matches!(ch, '<' | '>' | '=' | '!' | '~' | ';') {
            name_end = idx;
            break;
        }
    }

    let name = trimmed[..name_end].trim();
    if name.is_empty() {
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
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .collect();
        rest = &rest_trimmed[close_idx + 1..];
    }

    Some((name.to_string(), extras, rest))
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
