//! Parser for RPM Mariner container manifest files.
//!
//! Extracts package metadata from `container-manifest-2` files which contain
//! installed RPM package information in Mariner distroless containers.
//!
//! # Supported Formats
//! - `container-manifest-2` - RPM Mariner distroless package manifest
//!
//! # Key Features
//! - Installed package identification
//! - Version and architecture metadata
//! - Checksum information
//!
//! # Implementation Notes
//! - Format: Tab-separated text with package metadata
//! - One package per line
//! - Spec: https://github.com/microsoft/marinara/

use crate::models::{DatasourceId, PackageType};
use std::fs;
use std::path::Path;

use log::warn;

use crate::models::PackageData;

use super::PackageParser;

const PACKAGE_TYPE: PackageType = PackageType::Rpm;

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(PACKAGE_TYPE),
        namespace: Some("mariner".to_string()),
        datasource_id: Some(DatasourceId::RpmMarinerManifest),
        ..Default::default()
    }
}

/// Parser for RPM Mariner container manifest files
pub struct RpmMarinerManifestParser;

impl PackageParser for RpmMarinerManifestParser {
    const PACKAGE_TYPE: PackageType = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        path.to_str()
            .is_some_and(|p| p.ends_with("/var/lib/rpmmanifest/container-manifest-2"))
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read RPM Mariner manifest {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        parse_rpm_mariner_manifest(&content)
    }
}

pub(crate) fn parse_rpm_mariner_manifest(content: &str) -> Vec<PackageData> {
    let mut packages = Vec::new();

    for line in content.lines() {
        // Only trim whitespace, not tabs
        let line = line.trim_matches(|c: char| c.is_whitespace() && c != '\t');
        if line.is_empty() {
            continue;
        }

        // Split by tabs
        let parts: Vec<&str> = line.split('\t').collect();

        // According to Python reference, manifest_attributes are:
        // ["name", "version", "n1", "n2", "party", "n3", "n4", "arch", "checksum_algo", "filename"]
        // We only care about name, version, arch, and filename

        if parts.len() < 10 {
            warn!(
                "Invalid RPM Mariner manifest line (expected 10 fields): {}",
                line
            );
            continue;
        }

        let name = parts[0];
        let version = parts[1];
        let arch = parts[7];
        let filename = parts[9];

        let qualifiers = if arch.is_empty() {
            None
        } else {
            let mut quals = std::collections::HashMap::new();
            quals.insert("arch".to_string(), arch.to_string());
            Some(quals)
        };

        let extra_data = if filename.is_empty() {
            None
        } else {
            let mut extra = std::collections::HashMap::new();
            extra.insert(
                "filename".to_string(),
                serde_json::Value::String(filename.to_string()),
            );
            Some(extra)
        };

        packages.push(PackageData {
            package_type: Some(PACKAGE_TYPE),
            namespace: Some("mariner".to_string()),
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            },
            version: if version.is_empty() {
                None
            } else {
                Some(version.to_string())
            },
            qualifiers,
            datasource_id: Some(DatasourceId::RpmMarinerManifest),
            extra_data,
            ..Default::default()
        });
    }

    if packages.is_empty() {
        packages.push(default_package_data());
    }

    packages
}

crate::register_parser!(
    "RPM Mariner distroless package manifest",
    &["*var/lib/rpmmanifest/container-manifest-2"],
    "rpm",
    "",
    Some("https://github.com/microsoft/marinara/"),
);
