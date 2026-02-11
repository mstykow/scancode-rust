//! Parser for Chef cookbook metadata files (JSON and Ruby).
//!
//! Extracts package metadata, dependencies, and maintainer information from
//! Chef cookbook metadata files used by the Chef configuration management tool.
//!
//! # Supported Formats
//! - metadata.json (Chef cookbook metadata in JSON format)
//! - metadata.rb (Chef cookbook metadata in Ruby DSL format)
//!
//! # Key Features
//! - Maintainer party extraction from maintainer/maintainer_email fields
//! - Dependency extraction from both `dependencies` and `depends` fields (merged)
//! - URL construction for Chef Supermarket (download, homepage, API)
//! - dist-info guard to prevent false positives with Python wheel metadata.json
//!
//! # Implementation Notes
//! - JSON parser uses serde_json for JSON parsing
//! - Ruby parser uses line-based token extraction (not a full Ruby parser)
//! - Description from `description` or fallback to `long_description`
//! - Graceful error handling: logs warnings and returns default on parse failure
//! - IO.read(...) expressions in Ruby files are skipped (cannot evaluate Ruby code)

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use log::warn;
use packageurl::PackageUrl;
use regex::Regex;
use serde_json::Value;

use crate::models::{DatasourceId, Dependency, PackageData, PackageType, Party};

use super::PackageParser;

const FIELD_NAME: &str = "name";
const FIELD_VERSION: &str = "version";
const FIELD_DESCRIPTION: &str = "description";
const FIELD_LONG_DESCRIPTION: &str = "long_description";
const FIELD_LICENSE: &str = "license";
const FIELD_MAINTAINER: &str = "maintainer";
const FIELD_MAINTAINER_EMAIL: &str = "maintainer_email";
const FIELD_SOURCE_URL: &str = "source_url";
const FIELD_ISSUES_URL: &str = "issues_url";
const FIELD_DEPENDENCIES: &str = "dependencies";
const FIELD_DEPENDS: &str = "depends";

struct ChefPackageFields {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    extracted_license_statement: Option<String>,
    maintainer_name: Option<String>,
    maintainer_email: Option<String>,
    code_view_url: Option<String>,
    bug_tracking_url: Option<String>,
    deps: HashMap<String, Option<String>>,
}

/// Chef metadata.json parser for Chef cookbook manifests.
///
/// Extracts metadata from Chef cookbook metadata.json files, including
/// dependencies from both `dependencies` and `depends` fields.
pub struct ChefMetadataJsonParser;

impl PackageParser for ChefMetadataJsonParser {
    const PACKAGE_TYPE: PackageType = PackageType::Chef;

    fn is_match(path: &Path) -> bool {
        if path.file_name().is_some_and(|name| name == "metadata.json") {
            // Check parent directory doesn't end with "dist-info"
            // to prevent false positives with Python wheel metadata.json files
            if let Some(parent) = path.parent()
                && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str())
            {
                return !parent_name.ends_with("dist-info");
            }
            return true;
        }
        false
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let json_content = match read_json_file(path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read metadata.json at {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        let name = json_content
            .get(FIELD_NAME)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let version = json_content
            .get(FIELD_VERSION)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let description = extract_description(&json_content);

        let extracted_license_statement = json_content
            .get(FIELD_LICENSE)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let maintainer_name = json_content
            .get(FIELD_MAINTAINER)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let maintainer_email = json_content
            .get(FIELD_MAINTAINER_EMAIL)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let code_view_url = json_content
            .get(FIELD_SOURCE_URL)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let bug_tracking_url = json_content
            .get(FIELD_ISSUES_URL)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let mut deps: HashMap<String, Option<String>> = HashMap::new();

        if let Some(deps_obj) = json_content
            .get(FIELD_DEPENDENCIES)
            .and_then(|v| v.as_object())
        {
            for (dep_name, dep_version) in deps_obj {
                let version_constraint = dep_version
                    .as_str()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                deps.insert(dep_name.trim().to_string(), version_constraint);
            }
        }

        if let Some(depends_obj) = json_content.get(FIELD_DEPENDS).and_then(|v| v.as_object()) {
            for (dep_name, dep_version) in depends_obj {
                let version_constraint = dep_version
                    .as_str()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                deps.insert(dep_name.trim().to_string(), version_constraint);
            }
        }

        vec![build_package(ChefPackageFields {
            name,
            version,
            description,
            extracted_license_statement,
            maintainer_name,
            maintainer_email,
            code_view_url,
            bug_tracking_url,
            deps,
        })]
    }
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse JSON: {}", e))
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(ChefMetadataJsonParser::PACKAGE_TYPE),
        datasource_id: Some(DatasourceId::ChefCookbookMetadataJson),
        ..Default::default()
    }
}

fn extract_description(json: &Value) -> Option<String> {
    // Try description first, then long_description
    json.get(FIELD_DESCRIPTION)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            json.get(FIELD_LONG_DESCRIPTION)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

/// Chef metadata.rb parser for Chef cookbook manifests in Ruby DSL format.
///
/// Uses line-based token extraction to parse Ruby DSL without executing Ruby code.
pub struct ChefMetadataRbParser;

impl PackageParser for ChefMetadataRbParser {
    const PACKAGE_TYPE: PackageType = PackageType::Chef;

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "metadata.rb")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open metadata.rb at {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        let reader = BufReader::new(file);
        let mut fields: HashMap<String, String> = HashMap::new();
        let mut deps: HashMap<String, Option<String>> = HashMap::new();

        let field_pattern = Regex::new(r#"^\s*(\w+)\s+['"](.+?)['"]"#).unwrap();
        let depends_pattern =
            Regex::new(r#"^\s*depends\s+['"](.+?)['"](?:\s*,\s*['"](.+?)['"])?"#).unwrap();
        let io_read_pattern = Regex::new(r"IO\.read\(").unwrap();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if io_read_pattern.is_match(&line) {
                continue;
            }

            if let Some(caps) = depends_pattern.captures(&line) {
                let dep_name = caps.get(1).map(|m| m.as_str().to_string()).unwrap();
                let dep_version = caps.get(2).map(|m| m.as_str().to_string());
                deps.insert(dep_name, dep_version);
                continue;
            }

            if let Some(caps) = field_pattern.captures(&line) {
                let key = caps.get(1).map(|m| m.as_str().to_string()).unwrap();
                let value = caps.get(2).map(|m| m.as_str().to_string()).unwrap();

                match key.as_str() {
                    "name" | "version" | "description" | "long_description" | "license"
                    | "maintainer" | "maintainer_email" | "source_url" | "issues_url" => {
                        fields.insert(key, value);
                    }
                    _ => {}
                }
            }
        }

        let name = fields
            .get("name")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let version = fields
            .get("version")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let description = fields
            .get("description")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                fields
                    .get("long_description")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            });

        let extracted_license_statement = fields
            .get("license")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let maintainer_name = fields
            .get("maintainer")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let maintainer_email = fields
            .get("maintainer_email")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let code_view_url = fields
            .get("source_url")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let bug_tracking_url = fields
            .get("issues_url")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        vec![build_package(ChefPackageFields {
            name,
            version,
            description,
            extracted_license_statement,
            maintainer_name,
            maintainer_email,
            code_view_url,
            bug_tracking_url,
            deps,
        })]
    }
}

fn build_package(fields: ChefPackageFields) -> PackageData {
    let ChefPackageFields {
        name,
        version,
        description,
        extracted_license_statement,
        maintainer_name,
        maintainer_email,
        code_view_url,
        bug_tracking_url,
        deps,
    } = fields;
    let parties = if maintainer_name.is_some() || maintainer_email.is_some() {
        vec![Party {
            r#type: None,
            role: Some("maintainer".to_string()),
            name: maintainer_name,
            email: maintainer_email,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        }]
    } else {
        Vec::new()
    };

    let mut dependencies: Vec<Dependency> = deps
        .into_iter()
        .map(|(dep_name, version_constraint)| {
            let purl = PackageUrl::new("chef", &dep_name)
                .map(|p| p.to_string())
                .ok();
            Dependency {
                purl,
                extracted_requirement: version_constraint,
                scope: Some("dependencies".to_string()),
                is_runtime: Some(true),
                is_optional: Some(false),
                is_pinned: None,
                is_direct: None,
                resolved_package: None,
                extra_data: None,
            }
        })
        .collect();

    dependencies.sort_by(|a, b| {
        let name_a = a.purl.as_deref().unwrap_or("");
        let name_b = b.purl.as_deref().unwrap_or("");
        name_a.cmp(name_b)
    });

    let (download_url, repository_download_url, repository_homepage_url, api_data_url) =
        if let (Some(n), Some(v)) = (&name, &version) {
            let download = format!(
                "https://supermarket.chef.io/cookbooks/{}/versions/{}/download",
                n, v
            );
            let homepage = format!(
                "https://supermarket.chef.io/cookbooks/{}/versions/{}/",
                n, v
            );
            let api = format!(
                "https://supermarket.chef.io/api/v1/cookbooks/{}/versions/{}",
                n, v
            );
            (
                Some(download.clone()),
                Some(download),
                Some(homepage),
                Some(api),
            )
        } else {
            (None, None, None, None)
        };

    PackageData {
        package_type: Some(ChefMetadataJsonParser::PACKAGE_TYPE),
        datasource_id: Some(DatasourceId::ChefCookbookMetadataRb),
        name,
        version,
        description,
        extracted_license_statement,
        parties,
        code_view_url,
        bug_tracking_url,
        dependencies,
        download_url,
        repository_download_url,
        repository_homepage_url,
        api_data_url,
        primary_language: Some("Ruby".to_string()),
        ..Default::default()
    }
}

crate::register_parser!(
    "Chef cookbook metadata",
    &["**/metadata.json", "**/metadata.rb"],
    "chef",
    "Ruby",
    Some("https://docs.chef.io/config_rb_metadata/"),
);
