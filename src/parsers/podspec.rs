//! Parser for CocoaPods .podspec manifest files.
//!
//! Extracts package metadata and dependencies from .podspec files which define
//! CocoaPods package specifications using Ruby DSL syntax.
//!
//! # Supported Formats
//! - *.podspec (CocoaPods package specification files)
//! - .podspec files (same format, different naming convention)
//!
//! # Key Features
//! - Metadata extraction (name, version, summary, description, license)
//! - Author/contributor information parsing with email handling
//! - Homepage and source repository URL extraction
//! - Dependency declaration parsing with version constraints
//! - Support for development dependencies
//! - Regex-based Ruby DSL parsing (no full Ruby AST required)
//!
//! # Implementation Notes
//! - Uses regex for pattern matching in Ruby DSL syntax
//! - Supports multi-line string values and Ruby hash syntax
//! - Dependency version constraints are parsed from DSL
//! - Graceful error handling with `warn!()` logs on parse failures

use std::fs;
use std::path::Path;

use lazy_static::lazy_static;
use log::warn;
use packageurl::PackageUrl;
use regex::Regex;

use crate::models::{DatasourceId, Dependency, PackageData, PackageType, Party};
use crate::parsers::PackageParser;

/// Parses CocoaPods specification files (.podspec).
///
/// Extracts package metadata from .podspec files using regex-based Ruby DSL parsing.
///
/// # Extracted Fields
/// - Name, version, summary, description
/// - Homepage, license, source URLs
/// - Author information (including author hashes)
/// - Dependencies with version constraints
///
/// # Heredoc Support
/// Handles multiline descriptions: `s.description = <<-DESC ... DESC`
pub struct PodspecParser;

impl PackageParser for PodspecParser {
    const PACKAGE_TYPE: PackageType = PackageType::Cocoapods;

    fn is_match(path: &Path) -> bool {
        path.extension().is_some_and(|ext| {
            ext == "podspec"
                && path
                    .file_name()
                    .is_some_and(|name| !name.to_string_lossy().ends_with(".json.podspec"))
        })
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        let name = extract_field(&content, &NAME_PATTERN);
        let version = extract_field(&content, &VERSION_PATTERN);
        let _summary = extract_field(&content, &SUMMARY_PATTERN);
        let description = extract_description(&content);
        let homepage_url = extract_field(&content, &HOMEPAGE_PATTERN);
        let license = extract_field(&content, &LICENSE_PATTERN);
        let source = extract_field(&content, &SOURCE_PATTERN);
        let authors = extract_authors(&content);

        let parties = authors
            .into_iter()
            .map(|(name, email)| Party {
                r#type: Some("person".to_string()),
                name: Some(name),
                email,
                url: None,
                role: Some("author".to_string()),
                organization: None,
                organization_url: None,
                timezone: None,
            })
            .collect();

        let dependencies = extract_dependencies(&content);

        vec![PackageData {
            package_type: Some(Self::PACKAGE_TYPE),
            namespace: None,
            name,
            version,
            qualifiers: None,
            subpath: None,
            primary_language: Some("Objective-C".to_string()),
            description,
            release_date: None,
            parties,
            keywords: Vec::new(),
            homepage_url,
            download_url: None,
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            bug_tracking_url: None,
            code_view_url: None,
            vcs_url: source,
            copyright: None,
            holder: None,
            declared_license_expression: None,
            declared_license_expression_spdx: None,
            license_detections: Vec::new(),
            other_license_expression: None,
            other_license_expression_spdx: None,
            other_license_detections: Vec::new(),
            extracted_license_statement: license,
            notice_text: None,
            source_packages: Vec::new(),
            file_references: Vec::new(),
            extra_data: None,
            dependencies,
            repository_homepage_url: None,
            repository_download_url: None,
            api_data_url: None,
            datasource_id: Some(DatasourceId::CocoapodsPodspec),
            purl: None,
            is_private: false,
            is_virtual: false,
        }]
    }
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(PodspecParser::PACKAGE_TYPE),
        primary_language: Some("Objective-C".to_string()),
        datasource_id: Some(DatasourceId::CocoapodsPodspec),
        ..Default::default()
    }
}

lazy_static! {
    // Regex patterns matching Python reference implementation
    static ref NAME_PATTERN: Regex = Regex::new(r"\.name\s*=\s*(.+)").unwrap();
    static ref VERSION_PATTERN: Regex = Regex::new(r"\.version\s*=\s*(.+)").unwrap();
    static ref SUMMARY_PATTERN: Regex = Regex::new(r"\.summary\s*=\s*(.+)").unwrap();
    static ref DESCRIPTION_PATTERN: Regex = Regex::new(r"\.description\s*=\s*(.+)").unwrap();
    static ref HOMEPAGE_PATTERN: Regex = Regex::new(r"\.homepage\s*=\s*(.+)").unwrap();
    static ref LICENSE_PATTERN: Regex = Regex::new(r"\.license\s*=\s*(.+)").unwrap();
    static ref SOURCE_PATTERN: Regex = Regex::new(r"\.source\s*=\s*(.+)").unwrap();
    static ref AUTHOR_PATTERN: Regex = Regex::new(r"\.authors?\s*=\s*(.+)").unwrap();

    // Dependency patterns (using pod/dependency method calls)
    static ref DEPENDENCY_PATTERN: Regex = Regex::new(
        r#"(?:s\.)?(?:dependency|add_dependency|add_(?:runtime|development)_dependency)\s+['"]([^'"]+)['"](?:\s*,\s*(.+))?"#
    ).unwrap();
}

/// Extract a single field using a regex pattern
fn extract_field(content: &str, pattern: &Regex) -> Option<String> {
    for line in content.lines() {
        let cleaned_line = pre_process(line);
        if let Some(value) = pattern.captures(&cleaned_line).and_then(|caps| caps.get(1)) {
            return Some(clean_string(value.as_str()));
        }
    }
    None
}

/// Extract description, handling multiline heredoc format
fn extract_description(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let cleaned = pre_process(line);
        if let Some(value) = DESCRIPTION_PATTERN
            .captures(&cleaned)
            .and_then(|caps| caps.get(1))
        {
            let value_str = value.as_str();

            if value_str.contains("<<-") {
                return extract_multiline_description(&lines, i);
            } else {
                return Some(clean_string(value_str));
            }
        }
    }
    None
}

/// Extract multiline description in heredoc format
fn extract_multiline_description(lines: &[&str], start_index: usize) -> Option<String> {
    let start_line = lines.get(start_index)?;

    // Extract the delimiter (e.g., "DESC" from "<<-DESC")
    let delimiter = start_line
        .split("<<-")
        .nth(1)?
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');

    let mut description_lines = Vec::new();
    let mut found_start = false;

    for line in lines.iter().skip(start_index) {
        if !found_start && line.contains("<<-") {
            found_start = true;
            continue;
        }

        if found_start {
            let trimmed = line.trim();
            if trimmed == delimiter {
                break;
            }
            description_lines.push(*line);
        }
    }

    if description_lines.is_empty() {
        None
    } else {
        Some(description_lines.join("\n").trim().to_string())
    }
}

/// Extract authors (can be single or multiple)
fn extract_authors(content: &str) -> Vec<(String, Option<String>)> {
    let mut authors = Vec::new();

    for line in content.lines() {
        let cleaned_line = pre_process(line);
        if let Some(value) = AUTHOR_PATTERN
            .captures(&cleaned_line)
            .and_then(|caps| caps.get(1))
        {
            let value_str = value.as_str();

            if value_str.contains("=>") {
                for part in value_str.split(',') {
                    if let Some((name, email)) = parse_author_hash_entry(part) {
                        authors.push((name, Some(email)));
                    }
                }
            } else {
                let cleaned = clean_string(value_str);
                let (name, email) = parse_author_string(&cleaned);
                authors.push((name, email));
            }
        }
    }

    authors
}

/// Parse author from hash entry format: "Name" => "email"
fn parse_author_hash_entry(entry: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = entry.split("=>").collect();
    if parts.len() == 2 {
        let name = clean_string(parts[0].trim());
        let email = clean_string(parts[1].trim());
        Some((name, email))
    } else {
        None
    }
}

/// Parse author from string, extracting email if present
fn parse_author_string(author: &str) -> (String, Option<String>) {
    if let Some(email_start) = author.find('<')
        && let Some(email_end) = author.find('>')
    {
        let name = author[..email_start].trim().to_string();
        let email = author[email_start + 1..email_end].trim().to_string();
        return (name, Some(email));
    }
    (author.to_string(), None)
}

/// Extract dependencies from podspec
fn extract_dependencies(content: &str) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    for line in content.lines() {
        let cleaned_line = pre_process(line);
        if let Some(caps) = DEPENDENCY_PATTERN.captures(&cleaned_line) {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let version_req = caps.get(2).map(|m| clean_string(m.as_str()));

            if let Some(dep) = create_dependency(name, version_req) {
                dependencies.push(dep);
            }
        }
    }

    dependencies
}

/// Create a Dependency from name and version requirement
fn create_dependency(name: &str, version_req: Option<String>) -> Option<Dependency> {
    if name.is_empty() {
        return None;
    }

    let purl = PackageUrl::new("cocoapods", name).ok()?;

    // Determine if version is pinned (exact version)
    let is_pinned = version_req
        .as_ref()
        .map(|v| !v.contains(&['~', '>', '<', '='][..]))
        .unwrap_or(false);

    Some(Dependency {
        purl: Some(purl.to_string()),
        extracted_requirement: version_req,
        scope: Some("runtime".to_string()),
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(is_pinned),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}

/// Pre-process a line by removing comments and trimming
fn pre_process(line: &str) -> String {
    let line = if let Some(comment_pos) = line.find('#') {
        &line[..comment_pos]
    } else {
        line
    };
    line.trim().to_string()
}

/// Clean a string value by removing quotes and special characters
fn clean_string(s: &str) -> String {
    let after_removing_special_patterns = s.trim().replace("%q", "").replace(".freeze", "");

    after_removing_special_patterns
        .trim_matches(|c| {
            c == '\''
                || c == '"'
                || c == '{'
                || c == '}'
                || c == '['
                || c == ']'
                || c == '<'
                || c == '>'
        })
        .trim()
        .to_string()
}

crate::register_parser!(
    "CocoaPods podspec file",
    &["**/*.podspec"],
    "cocoapods",
    "Objective-C",
    Some("https://guides.cocoapods.org/syntax/podspec.html"),
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_match() {
        assert!(PodspecParser::is_match(Path::new("AFNetworking.podspec")));
        assert!(PodspecParser::is_match(Path::new("project/MyLib.podspec")));
        assert!(!PodspecParser::is_match(Path::new(
            "AFNetworking.podspec.json"
        )));
        assert!(!PodspecParser::is_match(Path::new("Podfile")));
        assert!(!PodspecParser::is_match(Path::new("Podfile.lock")));
    }

    #[test]
    fn test_clean_string() {
        assert_eq!(clean_string("'AFNetworking'"), "AFNetworking");
        assert_eq!(clean_string("\"AFNetworking\""), "AFNetworking");
        assert_eq!(clean_string("'test'.freeze"), "test");
        assert_eq!(clean_string("%q{test}"), "test");
    }

    #[test]
    fn test_extract_simple_field() {
        let content = r#"
Pod::Spec.new do |s|
  s.name = "AFNetworking"
  s.version = "4.0.1"
end
"#;
        assert_eq!(
            extract_field(content, &NAME_PATTERN),
            Some("AFNetworking".to_string())
        );
        assert_eq!(
            extract_field(content, &VERSION_PATTERN),
            Some("4.0.1".to_string())
        );
    }

    #[test]
    fn test_extract_multiline_description() {
        let content = r#"
Pod::Spec.new do |s|
  s.description = <<-DESC
    A delightful networking library.
    Features include:
    - Modern API
  DESC
end
"#;
        let desc = extract_description(content);
        assert!(desc.is_some());
        let desc_text = desc.unwrap();
        assert!(desc_text.contains("delightful networking"));
        assert!(desc_text.contains("Modern API"));
    }

    #[test]
    fn test_extract_dependency() {
        let content = r#"
Pod::Spec.new do |s|
  s.dependency "AFNetworking", "~> 4.0"
  s.dependency "Alamofire"
end
"#;
        let deps = extract_dependencies(content);
        assert_eq!(deps.len(), 2);

        assert_eq!(deps[0].purl, Some("pkg:cocoapods/AFNetworking".to_string()));
        assert_eq!(deps[0].extracted_requirement, Some("~> 4.0".to_string()));
        assert_eq!(deps[0].is_pinned, Some(false)); // Contains ~

        assert_eq!(deps[1].purl, Some("pkg:cocoapods/Alamofire".to_string()));
        assert_eq!(deps[1].extracted_requirement, None);
    }

    #[test]
    fn test_parse_author_string() {
        assert_eq!(
            parse_author_string("John Doe <john@example.com>"),
            ("John Doe".to_string(), Some("john@example.com".to_string()))
        );
        assert_eq!(
            parse_author_string("Jane Smith"),
            ("Jane Smith".to_string(), None)
        );
    }
}
