//! Parser for FreeBSD package manifest files.
//!
//! Extracts package metadata from FreeBSD compact manifest files (+COMPACT_MANIFEST)
//! which are JSON/YAML format files containing package information.
//!
//! # Supported Formats
//! - `+COMPACT_MANIFEST` (JSON/YAML format)
//!
//! # Key Features
//! - Package metadata extraction (name, version, description, etc.)
//! - Complex license logic handling (single/and/or/dual)
//! - URL construction from origin and architecture fields
//! - Qualifier extraction (arch, origin)
//! - Maintainer information parsing
//!
//! # Implementation Notes
//! - Uses `serde_yaml` for parsing (handles both JSON and YAML)
//! - Implements FreeBSD-specific license logic combining
//! - Graceful error handling with `warn!()` logs

use std::collections::HashMap;
use std::path::Path;

use log::warn;
use serde::Deserialize;

use crate::models::{PackageData, Party};
use crate::parsers::utils::{create_default_package_data, read_file_to_string};

use super::PackageParser;

const PACKAGE_TYPE: &str = "freebsd";

/// Parser for FreeBSD +COMPACT_MANIFEST files
pub struct FreebsdCompactManifestParser;

impl PackageParser for FreebsdCompactManifestParser {
    const PACKAGE_TYPE: &'static str = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == "+COMPACT_MANIFEST")
            .unwrap_or(false)
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match read_file_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read FreeBSD manifest {:?}: {}", path, e);
                return vec![create_default_package_data(PACKAGE_TYPE, None)];
            }
        };

        vec![parse_freebsd_manifest(&content)]
    }
}

#[derive(Debug, Deserialize)]
struct FreebsdManifest {
    name: Option<String>,
    version: Option<String>,
    #[serde(rename = "desc")]
    description: Option<String>,
    categories: Option<Vec<String>>,
    www: Option<String>,
    maintainer: Option<String>,
    origin: Option<String>,
    arch: Option<String>,
    licenses: Option<Vec<String>>,
    licenselogic: Option<String>,
}

pub(crate) fn parse_freebsd_manifest(content: &str) -> PackageData {
    let manifest: FreebsdManifest = match serde_yaml::from_str(content) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to parse FreeBSD manifest: {}", e);
            return create_default_package_data(PACKAGE_TYPE, None);
        }
    };

    let name = manifest.name.clone();
    let version = manifest.version.clone();
    let description = manifest.description;
    let homepage_url = manifest.www;
    let keywords = manifest.categories.unwrap_or_default();

    // Build qualifiers from arch and origin
    let mut qualifiers = HashMap::new();
    if let Some(ref arch) = manifest.arch {
        qualifiers.insert("arch".to_string(), arch.clone());
    }
    if let Some(ref origin) = manifest.origin {
        qualifiers.insert("origin".to_string(), origin.clone());
    }

    // Build parties from maintainer (just an email address)
    let mut parties = Vec::new();
    if let Some(maintainer_email) = manifest.maintainer {
        parties.push(Party {
            r#type: Some("person".to_string()),
            role: Some("maintainer".to_string()),
            name: None,
            email: Some(maintainer_email),
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    // Build extracted_license_statement from licenses and licenselogic
    let extracted_license_statement =
        build_license_statement(&manifest.licenses, &manifest.licenselogic);

    // Build code_view_url from origin
    let code_view_url = manifest
        .origin
        .as_ref()
        .map(|origin| format!("https://svnweb.freebsd.org/ports/head/{}", origin));

    // Build download_url from arch, name, and version
    let download_url = if let (Some(arch), Some(pkg_name), Some(pkg_version)) =
        (&manifest.arch, &name, &version)
    {
        Some(format!(
            "https://pkg.freebsd.org/{}/latest/All/{}-{}.txz",
            arch, pkg_name, pkg_version
        ))
    } else {
        None
    };

    PackageData {
        datasource_id: Some("freebsd_compact_manifest".to_string()),
        package_type: Some(PACKAGE_TYPE.to_string()),
        name,
        version,
        description,
        homepage_url,
        keywords,
        parties,
        qualifiers: if qualifiers.is_empty() {
            None
        } else {
            Some(qualifiers)
        },
        extracted_license_statement,
        code_view_url,
        download_url,
        ..Default::default()
    }
}

/// Builds the extracted_license_statement string from licenses and licenselogic.
///
/// # Logic:
/// - `licenselogic: "single"` → single license string (just the first license)
/// - `licenselogic: "and"` → join licenses with " AND "
/// - `licenselogic: "or"` or `"dual"` → join licenses with " OR "
/// - If `licenselogic` is missing or unknown → join with " AND " (default)
pub(crate) fn build_license_statement(
    licenses: &Option<Vec<String>>,
    licenselogic: &Option<String>,
) -> Option<String> {
    let license_list = licenses.as_ref()?;

    if license_list.is_empty() {
        return None;
    }

    // Filter out empty licenses and trim whitespace
    let filtered_licenses: Vec<String> = license_list
        .iter()
        .filter_map(|lic| {
            let trimmed = lic.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect();

    if filtered_licenses.is_empty() {
        return None;
    }

    let logic = licenselogic.as_deref().unwrap_or("and");

    match logic {
        "single" => Some(filtered_licenses[0].clone()),
        "or" | "dual" => Some(filtered_licenses.join(" OR ")),
        _ => Some(filtered_licenses.join(" AND ")), // "and" or unknown defaults to AND
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_match() {
        assert!(FreebsdCompactManifestParser::is_match(&PathBuf::from(
            "/path/to/+COMPACT_MANIFEST"
        )));
        assert!(FreebsdCompactManifestParser::is_match(&PathBuf::from(
            "+COMPACT_MANIFEST"
        )));
        assert!(!FreebsdCompactManifestParser::is_match(&PathBuf::from(
            "+MANIFEST"
        )));
        assert!(!FreebsdCompactManifestParser::is_match(&PathBuf::from(
            "COMPACT_MANIFEST"
        )));
        assert!(!FreebsdCompactManifestParser::is_match(&PathBuf::from(
            "package.json"
        )));
    }

    #[test]
    fn test_build_license_statement_single() {
        let licenses = Some(vec!["GPLv2".to_string()]);
        let logic = Some("single".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("GPLv2".to_string()));
    }

    #[test]
    fn test_build_license_statement_and() {
        let licenses = Some(vec!["MIT".to_string(), "BSD-2-Clause".to_string()]);
        let logic = Some("and".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("MIT AND BSD-2-Clause".to_string()));
    }

    #[test]
    fn test_build_license_statement_or() {
        let licenses = Some(vec!["MIT".to_string(), "Apache-2.0".to_string()]);
        let logic = Some("or".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("MIT OR Apache-2.0".to_string()));
    }

    #[test]
    fn test_build_license_statement_dual() {
        let licenses = Some(vec!["MIT".to_string(), "Apache-2.0".to_string()]);
        let logic = Some("dual".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("MIT OR Apache-2.0".to_string()));
    }

    #[test]
    fn test_build_license_statement_default_and() {
        let licenses = Some(vec!["MIT".to_string(), "BSD".to_string()]);
        let logic = None;
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("MIT AND BSD".to_string()));
    }

    #[test]
    fn test_build_license_statement_unknown_defaults_to_and() {
        let licenses = Some(vec!["MIT".to_string(), "BSD".to_string()]);
        let logic = Some("unknown".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("MIT AND BSD".to_string()));
    }

    #[test]
    fn test_build_license_statement_empty_licenses() {
        let licenses = Some(vec![]);
        let logic = Some("and".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, None);
    }

    #[test]
    fn test_build_license_statement_no_licenses() {
        let licenses = None;
        let logic = Some("and".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, None);
    }

    #[test]
    fn test_build_license_statement_filters_empty() {
        let licenses = Some(vec!["MIT".to_string(), "".to_string(), "  ".to_string()]);
        let logic = Some("and".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("MIT".to_string()));
    }

    #[test]
    fn test_build_license_statement_trims_whitespace() {
        let licenses = Some(vec!["  MIT  ".to_string(), " Apache-2.0 ".to_string()]);
        let logic = Some("or".to_string());
        let result = build_license_statement(&licenses, &logic);
        assert_eq!(result, Some("MIT OR Apache-2.0".to_string()));
    }
}
