/// Shared utility functions for package parsers
///
/// This module provides common file I/O and parsing utilities
/// used across multiple parser implementations.
use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use log::warn;
use packageurl::PackageUrl;

use crate::askalono::{Store, TextData};
use crate::models::Match;

/// Reads a file's entire contents into a String.
///
/// # Arguments
///
/// * `path` - Path to the file to read
///
/// # Returns
///
/// * `Ok(String)` - File contents as UTF-8 string
/// * `Err` - I/O error or UTF-8 decoding error
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use scancode_rust::parsers::utils::read_file_to_string;
///
/// let content = read_file_to_string(Path::new("path/to/file.txt"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn read_file_to_string(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// Normalizes a raw license string using askalono with confidence threshold.
///
/// Analyzes the given license text and returns normalized SPDX identifiers
/// if confidence meets or exceeds the threshold (0.8).
///
/// Returns (None, None) if the store is empty (e.g., during tests).
///
/// # Arguments
///
/// * `raw_license` - Raw license string from manifest
/// * `store` - Askalono license store for analysis
///
/// # Returns
///
/// A tuple of `(declared_license_expression, declared_license_expression_spdx)`:
/// - Both `Some(spdx_id)` if confidence >= 0.8
/// - Both `None` if confidence < 0.8 (logs warning)
/// - Both `None` if store is empty
///
/// # Examples
///
/// ```no_run
/// use scancode_rust::askalono::Store;
/// use scancode_rust::parsers::utils::normalize_license;
///
/// let store = Store::new();
/// let (expr, spdx) = normalize_license("MIT License", &store);
/// // Returns (None, None) if store is empty
/// ```
pub fn normalize_license(raw_license: &str, store: &Store) -> (Option<String>, Option<String>) {
    if raw_license.trim().is_empty() || store.is_empty() {
        return (None, None);
    }

    let text_data = TextData::new(raw_license);
    let result = store.analyze(&text_data);

    if result.score >= 0.8 {
        let spdx_id = result.name.to_string();
        (Some(spdx_id.clone()), Some(spdx_id))
    } else {
        warn!(
            "Low confidence ({:.2}) for license normalization: {}",
            result.score, raw_license
        );
        (None, None)
    }
}

/// Creates a license detection match for a declared SPDX license identifier.
///
/// This is used when a package manifest explicitly declares a license using
/// an SPDX identifier (e.g., "MIT", "Apache-2.0"). The match is marked with
/// perfect confidence scores since it's a direct declaration, not detected text.
///
/// # Arguments
///
/// * `spdx_id` - SPDX license identifier from manifest
///
/// # Returns
///
/// A `Match` with 100% confidence scores indicating explicit license declaration.
///
/// # Examples
///
/// ```no_run
/// use scancode_rust::parsers::utils::create_spdx_license_match;
///
/// let match_data = create_spdx_license_match("MIT");
/// assert_eq!(match_data.score, 100.0);
/// assert_eq!(match_data.license_expression_spdx, "MIT");
/// ```
pub fn create_spdx_license_match(spdx_id: &str) -> Match {
    let license_lower = spdx_id.to_lowercase();
    Match {
        license_expression: license_lower.clone(),
        license_expression_spdx: spdx_id.to_string(),
        from_file: None,
        score: 100.0,
        start_line: 0,
        end_line: 0,
        matcher: Some("1-spdx-id".to_string()),
        matched_length: Some(1),
        match_coverage: Some(100.0),
        rule_relevance: Some(100),
        rule_identifier: Some(format!(
            "spdx-license-identifier-{}-5da48780aba670b0860c46d899ed42a0f243ff06",
            license_lower
        )),
        rule_url: None,
        matched_text: Some(spdx_id.to_string()),
    }
}

/// Creates a correctly-formatted npm Package URL for scoped or regular packages.
///
/// Handles namespace encoding for scoped packages (e.g., `@babel/core`) and ensures
/// the slash between namespace and package name is NOT encoded as `%2F`.
pub fn npm_purl(full_name: &str, version: Option<&str>) -> Option<String> {
    let (namespace, name) = if full_name.starts_with('@') {
        let parts: Vec<&str> = full_name.splitn(2, '/').collect();
        if parts.len() == 2 {
            (Some(parts[0]), parts[1])
        } else {
            (None, full_name)
        }
    } else {
        (None, full_name)
    };

    let mut purl = PackageUrl::new("npm", name).ok()?;

    if let Some(ns) = namespace {
        purl.with_namespace(ns).ok()?;
    }

    if let Some(ver) = version {
        purl.with_version(ver).ok()?;
    }

    Some(purl.to_string())
}

/// Parses Subresource Integrity (SRI) format and returns hash as hex string.
///
/// SRI format: "algorithm-base64string" (e.g., "sha512-9NET910DNaIPng...")
///
/// Returns the algorithm name and hex-encoded hash digest.
pub fn parse_sri(integrity: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = integrity.splitn(2, '-').collect();
    if parts.len() != 2 {
        return None;
    }

    let algorithm = parts[0];
    let base64_str = parts[1];

    let bytes = BASE64_STANDARD.decode(base64_str).ok()?;

    let hex_string = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Some((algorithm.to_string(), hex_string))
}

/// Parses "Name <email@domain.com>" format into separate components.
///
/// This utility handles common author/maintainer strings found in package manifests
/// where the format combines a human-readable name with an email address in angle brackets.
///
/// # Arguments
///
/// * `s` - A string potentially containing name and email in "Name <email>" format
///
/// # Returns
///
/// A tuple of `(Option<String>, Option<String>)` representing `(name, email)`:
/// - If `<email>` pattern found: name (trimmed, or None if empty) and email
/// - If no pattern: trimmed input as name, None for email
///
/// # Examples
///
/// ```
/// use scancode_rust::parsers::utils::split_name_email;
///
/// // Full format
/// let (name, email) = split_name_email("John Doe <john@example.com>");
/// assert_eq!(name, Some("John Doe".to_string()));
/// assert_eq!(email, Some("john@example.com".to_string()));
///
/// // Email only in angle brackets
/// let (name, email) = split_name_email("<john@example.com>");
/// assert_eq!(name, None);
/// assert_eq!(email, Some("john@example.com".to_string()));
///
/// // Name only (no angle brackets)
/// let (name, email) = split_name_email("John Doe");
/// assert_eq!(name, Some("John Doe".to_string()));
/// assert_eq!(email, None);
/// ```
pub fn split_name_email(s: &str) -> (Option<String>, Option<String>) {
    if let Some(email_start) = s.find('<')
        && let Some(email_end) = s.find('>')
        && email_start < email_end
    {
        let name = s[..email_start].trim();
        let email = &s[email_start + 1..email_end];
        (
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            },
            Some(email.to_string()),
        )
    } else {
        (Some(s.trim().to_string()), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_file_to_string_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let content = read_file_to_string(&file_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_read_file_to_string_nonexistent() {
        let path = Path::new("/nonexistent/file.txt");
        let result = read_file_to_string(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_to_string_empty() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let content = read_file_to_string(&file_path).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_npm_purl_scoped_with_version() {
        let purl = npm_purl("@babel/core", Some("7.0.0")).unwrap();
        assert_eq!(purl, "pkg:npm/%40babel/core@7.0.0");
    }

    #[test]
    fn test_npm_purl_scoped_without_version() {
        let purl = npm_purl("@babel/core", None).unwrap();
        assert_eq!(purl, "pkg:npm/%40babel/core");
    }

    #[test]
    fn test_npm_purl_unscoped_with_version() {
        let purl = npm_purl("lodash", Some("4.17.21")).unwrap();
        assert_eq!(purl, "pkg:npm/lodash@4.17.21");
    }

    #[test]
    fn test_npm_purl_unscoped_without_version() {
        let purl = npm_purl("lodash", None).unwrap();
        assert_eq!(purl, "pkg:npm/lodash");
    }

    #[test]
    fn test_npm_purl_scoped_slash_not_encoded() {
        let purl = npm_purl("@types/node", Some("18.0.0")).unwrap();
        assert!(purl.contains("/%40types/node"));
        assert!(!purl.contains("%2F"));
    }

    #[test]
    fn test_parse_sri_sha512() {
        let (algo, hash) = parse_sri("sha512-9NET910DNaIPngYnLLPeg+Ogzqsi9uM4mSboU5y6p8S5DzMTVEsJZrawi+BoDNUVBa2DhJqQYUFvMDfgU062LQ==").unwrap();
        assert_eq!(algo, "sha512");
        assert_eq!(hash.len(), 128);
    }

    #[test]
    fn test_parse_sri_sha1() {
        let (algo, hash) = parse_sri("sha1-w7M6te42DYbg5ijwRorn7yfWVN8=").unwrap();
        assert_eq!(algo, "sha1");
        assert_eq!(hash.len(), 40);
    }

    #[test]
    fn test_parse_sri_sha256() {
        let (algo, hash) =
            parse_sri("sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=").unwrap();
        assert_eq!(algo, "sha256");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_parse_sri_invalid_format() {
        assert!(parse_sri("invalid").is_none());
        assert!(parse_sri("sha512").is_none());
        assert!(parse_sri("").is_none());
    }

    #[test]
    fn test_parse_sri_invalid_base64() {
        assert!(parse_sri("sha512-!!!invalid!!!").is_none());
    }

    #[test]
    fn test_split_name_email_full_format() {
        let (name, email) = split_name_email("John Doe <john@example.com>");
        assert_eq!(name, Some("John Doe".to_string()));
        assert_eq!(email, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_split_name_email_name_only() {
        let (name, email) = split_name_email("John Doe");
        assert_eq!(name, Some("John Doe".to_string()));
        assert_eq!(email, None);
    }

    #[test]
    fn test_split_name_email_email_only_plain() {
        let (name, email) = split_name_email("john@example.com");
        assert_eq!(name, Some("john@example.com".to_string()));
        assert_eq!(email, None);
    }

    #[test]
    fn test_split_name_email_email_only_brackets() {
        let (name, email) = split_name_email("<john@example.com>");
        assert_eq!(name, None);
        assert_eq!(email, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_split_name_email_whitespace_trimming() {
        let (name, email) = split_name_email("  John Doe  <  john@example.com  >  ");
        assert_eq!(name, Some("John Doe".to_string()));
        assert_eq!(email, Some("  john@example.com  ".to_string()));
    }

    #[test]
    fn test_split_name_email_empty_string() {
        let (name, email) = split_name_email("");
        assert_eq!(name, Some("".to_string()));
        assert_eq!(email, None);
    }

    #[test]
    fn test_split_name_email_whitespace_only() {
        let (name, email) = split_name_email("   ");
        assert_eq!(name, Some("".to_string()));
        assert_eq!(email, None);
    }

    #[test]
    fn test_split_name_email_invalid_bracket_order() {
        let (name, email) = split_name_email("John >email< Doe");
        assert_eq!(name, Some("John >email< Doe".to_string()));
        assert_eq!(email, None);
    }

    #[test]
    fn test_split_name_email_missing_close_bracket() {
        let (name, email) = split_name_email("John Doe <email@example.com");
        assert_eq!(name, Some("John Doe <email@example.com".to_string()));
        assert_eq!(email, None);
    }

    #[test]
    fn test_split_name_email_missing_open_bracket() {
        let (name, email) = split_name_email("John Doe email@example.com>");
        assert_eq!(name, Some("John Doe email@example.com>".to_string()));
        assert_eq!(email, None);
    }
}
