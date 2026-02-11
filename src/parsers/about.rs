//! Parser for AboutCode .ABOUT metadata files.
//!
//! Extracts package metadata from AboutCode .ABOUT YAML files which describe
//! software components, licenses, and related information.
//!
//! # Supported Formats
//! - .ABOUT (case-sensitive uppercase extension)
//!
//! # Key Features
//! - YAML-based metadata parsing
//! - Package URL (purl) parsing for type/namespace extraction
//! - Owner party information
//! - File reference tracking (about_resource field)
//! - License expression extraction
//! - Flexible field mapping (home_url/homepage_url)
//!
//! # Implementation Notes
//! - Uses serde_yaml for YAML parsing
//! - Uses packageurl crate for purl parsing
//! - Extension is case-sensitive and must be uppercase (.ABOUT not .about)
//! - Type can be overridden by 'type' field or extracted from 'purl' field
//! - Graceful error handling: logs warnings and returns default on parse failure

use crate::models::{DatasourceId, FileReference, PackageData, Party};
use log::warn;
use packageurl::PackageUrl;
use serde_yaml::Value;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use super::PackageParser;

const FIELD_TYPE: &str = "type";
const FIELD_PURL: &str = "purl";
const FIELD_NAMESPACE: &str = "namespace";
const FIELD_NAME: &str = "name";
const FIELD_VERSION: &str = "version";
const FIELD_HOME_URL: &str = "home_url";
const FIELD_HOMEPAGE_URL: &str = "homepage_url";
const FIELD_DOWNLOAD_URL: &str = "download_url";
const FIELD_COPYRIGHT: &str = "copyright";
const FIELD_LICENSE_EXPRESSION: &str = "license_expression";
const FIELD_OWNER: &str = "owner";
const FIELD_ABOUT_RESOURCE: &str = "about_resource";

/// AboutCode .ABOUT file parser.
///
/// Parses AboutCode metadata files that contain package information,
/// licensing, and file references in YAML format.
pub struct AboutFileParser;

impl PackageParser for AboutFileParser {
    const PACKAGE_TYPE: &'static str = "about";

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let yaml = match read_and_parse_yaml(path) {
            Ok(yaml) => yaml,
            Err(e) => {
                warn!("Failed to read or parse .ABOUT file at {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        // Extract type and purl information
        let about_type = yaml
            .get(FIELD_TYPE)
            .and_then(|v| v.as_str())
            .map(String::from);

        let about_namespace = yaml
            .get(FIELD_NAMESPACE)
            .and_then(|v| v.as_str())
            .map(String::from);

        let purl_string = yaml
            .get(FIELD_PURL)
            .and_then(|v| v.as_str())
            .map(String::from);

        // Parse purl if present
        let (purl_type, purl_namespace, purl_name, purl_version) =
            if let Some(ref purl_str) = purl_string {
                match PackageUrl::from_str(purl_str) {
                    Ok(purl) => (
                        Some(purl.ty().to_string()),
                        purl.namespace().map(String::from),
                        Some(purl.name().to_string()),
                        purl.version().map(String::from),
                    ),
                    Err(e) => {
                        warn!("Failed to parse purl '{}': {}", purl_str, e);
                        (None, None, None, None)
                    }
                }
            } else {
                (None, None, None, None)
            };

        // Priority: about_type > purl_type > default
        let package_type = about_type
            .or(purl_type)
            .unwrap_or_else(|| Self::PACKAGE_TYPE.to_string());

        // Priority: about_namespace > purl_namespace
        let namespace = about_namespace.or(purl_namespace);

        // Name and version from YAML or purl
        let name = yaml
            .get(FIELD_NAME)
            .and_then(yaml_value_to_string)
            .or(purl_name);

        let version = yaml
            .get(FIELD_VERSION)
            .and_then(yaml_value_to_string)
            .or(purl_version);

        // Homepage URL (two possible field names)
        let homepage_url = yaml
            .get(FIELD_HOME_URL)
            .and_then(|v| v.as_str())
            .or_else(|| yaml.get(FIELD_HOMEPAGE_URL).and_then(|v| v.as_str()))
            .map(String::from);

        let download_url = yaml
            .get(FIELD_DOWNLOAD_URL)
            .and_then(|v| v.as_str())
            .map(String::from);

        let copyright = yaml
            .get(FIELD_COPYRIGHT)
            .and_then(|v| v.as_str())
            .map(String::from);

        let extracted_license_statement = yaml
            .get(FIELD_LICENSE_EXPRESSION)
            .and_then(|v| v.as_str())
            .map(String::from);

        // Owner party
        let parties = extract_owner_party(&yaml);

        // File references
        let file_references = extract_file_references(&yaml);

        vec![PackageData {
            package_type: Some(package_type),
            namespace,
            name,
            version,
            qualifiers: None,
            subpath: None,
            primary_language: None,
            description: None,
            release_date: None,
            parties,
            keywords: Vec::new(),
            homepage_url,
            download_url,
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            bug_tracking_url: None,
            code_view_url: None,
            vcs_url: None,
            copyright,
            holder: None,
            declared_license_expression: None,
            declared_license_expression_spdx: None,
            license_detections: Vec::new(),
            other_license_expression: None,
            other_license_expression_spdx: None,
            other_license_detections: Vec::new(),
            extracted_license_statement,
            notice_text: None,
            source_packages: Vec::new(),
            file_references,
            is_private: false,
            is_virtual: false,
            extra_data: None,
            dependencies: Vec::new(),
            repository_homepage_url: None,
            repository_download_url: None,
            api_data_url: None,
            datasource_id: Some(DatasourceId::AboutFile),
            purl: purl_string,
        }]
    }

    fn is_match(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "ABOUT")
    }
}

/// Reads and parses a YAML file.
fn read_and_parse_yaml(path: &Path) -> Result<serde_yaml::Mapping, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let value: Value =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    match value {
        Value::Mapping(map) => Ok(map),
        _ => Err("Expected YAML mapping at root".to_string()),
    }
}

/// Converts a YAML value to a string, handling strings, numbers, and booleans.
fn yaml_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Extracts owner party information from YAML.
fn extract_owner_party(yaml: &serde_yaml::Mapping) -> Vec<Party> {
    let owner = yaml
        .get(Value::String(FIELD_OWNER.to_string()))
        .map(|v| match v {
            Value::String(s) => s.clone(),
            _ => {
                // Convert non-string values to their debug representation
                format!("{:?}", v)
            }
        });

    if let Some(owner_name) = owner {
        if !owner_name.is_empty() {
            vec![Party {
                r#type: Some("person".to_string()),
                role: Some("owner".to_string()),
                name: Some(owner_name),
                email: None,
                url: None,
                organization: None,
                organization_url: None,
                timezone: None,
            }]
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Extracts file references from YAML.
fn extract_file_references(yaml: &serde_yaml::Mapping) -> Vec<FileReference> {
    let about_resource = yaml
        .get(Value::String(FIELD_ABOUT_RESOURCE.to_string()))
        .and_then(|v| v.as_str());

    if let Some(path) = about_resource {
        vec![FileReference {
            path: path.to_string(),
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            extra_data: None,
        }]
    } else {
        Vec::new()
    }
}

/// Returns a default (empty) PackageData structure.
fn default_package_data() -> PackageData {
    PackageData::default()
}

crate::register_parser!(
    "AboutCode .ABOUT metadata file",
    &["**/*.ABOUT"],
    "about",
    "",
    Some("https://aboutcode-toolkit.readthedocs.io/en/latest/specification.html"),
);
