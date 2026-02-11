//! Parser for CPAN dist.ini files.
//!
//! Extracts Perl package metadata from `dist.ini` files used by Dist::Zilla.
//!
//! # Supported Formats
//! - `dist.ini` - CPAN Dist::Zilla configuration
//!
//! # Implementation Notes
//! - Format: INI-style configuration file
//! - Spec: https://metacpan.org/pod/Dist::Zilla::Tutorial
//! - Extracts: name, version, author, license, copyright_holder, abstract
//! - Dependencies from [Prereq] sections (beyond Python which has no parser)

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use log::warn;
use serde_json::json;

use crate::models::{DatasourceId, Dependency, PackageData, Party};

use super::PackageParser;

const PACKAGE_TYPE: &str = "cpan";

pub struct CpanDistIniParser;

impl PackageParser for CpanDistIniParser {
    const PACKAGE_TYPE: &'static str = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        path.to_str().is_some_and(|p| p.ends_with("/dist.ini"))
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read dist.ini file {:?}: {}", path, e);
                return vec![PackageData {
                    package_type: Some(PACKAGE_TYPE.to_string()),
                    primary_language: Some("Perl".to_string()),
                    datasource_id: Some(DatasourceId::CpanDistIni),
                    ..Default::default()
                }];
            }
        };

        vec![parse_dist_ini(&content)]
    }
}

pub(crate) fn parse_dist_ini(content: &str) -> PackageData {
    let (root_fields, sections) = parse_ini_structure(content);

    let name = root_fields.get("name").map(|s| s.replace('-', "::"));
    let version = root_fields.get("version").cloned();
    let description = root_fields.get("abstract").cloned();
    let declared_license_expression = root_fields.get("license").cloned();
    let copyright_holder = root_fields.get("copyright_holder").cloned();

    let parties = parse_author(&root_fields);
    let dependencies = parse_dependencies(&sections);

    let mut extra_data = HashMap::new();
    if let Some(holder) = copyright_holder {
        extra_data.insert("copyright_holder".to_string(), json!(holder));
    }
    if let Some(year) = root_fields.get("copyright_year") {
        extra_data.insert("copyright_year".to_string(), json!(year));
    }

    PackageData {
        package_type: Some(PACKAGE_TYPE.to_string()),
        namespace: Some("cpan".to_string()),
        name,
        version,
        description,
        declared_license_expression,
        parties,
        dependencies,
        extra_data: if extra_data.is_empty() {
            None
        } else {
            Some(extra_data)
        },
        datasource_id: Some(DatasourceId::CpanDistIni),
        primary_language: Some("Perl".to_string()),
        ..Default::default()
    }
}

fn parse_ini_structure(
    content: &str,
) -> (
    HashMap<String, String>,
    HashMap<String, HashMap<String, String>>,
) {
    let mut root_fields = HashMap::new();
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = Some(line[1..line.len() - 1].to_string());
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();

            if let Some(section_name) = &current_section {
                sections
                    .entry(section_name.clone())
                    .or_default()
                    .insert(key, value);
            } else {
                root_fields.insert(key, value);
            }
        }
    }

    (root_fields, sections)
}

fn parse_author(fields: &HashMap<String, String>) -> Vec<Party> {
    fields
        .get("author")
        .map(|author_str| {
            if let Some((name, email)) = parse_author_string(author_str) {
                vec![Party {
                    role: Some("author".to_string()),
                    name: Some(name),
                    email: Some(email),
                    r#type: None,
                    url: None,
                    organization: None,
                    organization_url: None,
                    timezone: None,
                }]
            } else {
                vec![Party {
                    role: Some("author".to_string()),
                    name: Some(author_str.clone()),
                    r#type: None,
                    email: None,
                    url: None,
                    organization: None,
                    organization_url: None,
                    timezone: None,
                }]
            }
        })
        .unwrap_or_default()
}

fn parse_author_string(s: &str) -> Option<(String, String)> {
    if let Some(start) = s.find('<')
        && let Some(end) = s.find('>')
    {
        let name = s[..start].trim().to_string();
        let email = s[start + 1..end].trim().to_string();
        return Some((name, email));
    }
    None
}

fn parse_dependencies(sections: &HashMap<String, HashMap<String, String>>) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    for (section_name, fields) in sections {
        let scope = if section_name.starts_with("Prereq") {
            if section_name.contains("TestRequires") || section_name.contains("Test") {
                Some("test".to_string())
            } else if section_name.contains("BuildRequires") || section_name.contains("Build") {
                Some("build".to_string())
            } else {
                Some("runtime".to_string())
            }
        } else {
            continue;
        };

        for (module_name, version_req) in fields {
            let purl = format!("pkg:cpan/{}", module_name);
            let extracted_requirement = if version_req == "0" || version_req.is_empty() {
                None
            } else {
                Some(version_req.clone())
            };

            dependencies.push(Dependency {
                purl: Some(purl),
                scope: scope.clone(),
                extracted_requirement,
                is_runtime: Some(scope.as_deref() == Some("runtime")),
                is_optional: Some(false),
                is_pinned: None,
                is_direct: None,
                resolved_package: None,
                extra_data: None,
            });
        }
    }

    dependencies
}

crate::register_parser!(
    "CPAN Perl dist.ini",
    &["*/dist.ini"],
    "cpan",
    "Perl",
    Some("https://metacpan.org/pod/Dist::Zilla::Tutorial"),
);
