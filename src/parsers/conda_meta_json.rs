//! Parser for Conda metadata JSON files.
//!
//! Extracts package metadata from `conda-meta/*.json` files which contain
//! installed package information in Conda environments.
//!
//! # Supported Formats
//! - `conda-meta/*.json` - Conda installed package metadata
//!
//! # Key Features
//! - Installed package identification
//! - License extraction
//! - Download URLs and checksums
//!
//! # Implementation Notes
//! - Format: JSON with package metadata
//! - Located in conda-meta/ directory in rootfs
//! - Spec: https://docs.conda.io/

use crate::models::{DatasourceId, PackageType};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::PackageData;

use super::PackageParser;

const PACKAGE_TYPE: PackageType = PackageType::Conda;

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(PACKAGE_TYPE),
        primary_language: Some("Python".to_string()),
        datasource_id: Some(DatasourceId::CondaMetaJson),
        ..Default::default()
    }
}

/// Parser for Conda metadata JSON files
pub struct CondaMetaJsonParser;

#[derive(Debug, Deserialize, Serialize)]
struct CondaMetaJson {
    name: Option<String>,
    version: Option<String>,
    license: Option<String>,
    url: Option<String>,
    size: Option<u64>,
    md5: Option<String>,
    sha256: Option<String>,
    requested_spec: Option<String>,
    channel: Option<String>,
    extracted_package_dir: Option<String>,
    files: Option<Vec<String>>,
    package_tarball_full_path: Option<String>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

impl PackageParser for CondaMetaJsonParser {
    const PACKAGE_TYPE: PackageType = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        path.to_str()
            .is_some_and(|p| p.contains("/conda-meta/") && p.ends_with(".json"))
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read conda-meta JSON file {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        vec![parse_conda_meta_json(&content)]
    }
}

pub(crate) fn parse_conda_meta_json(content: &str) -> PackageData {
    let metadata: CondaMetaJson = match serde_json::from_str(content) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to parse conda-meta JSON: {}", e);
            return default_package_data();
        }
    };

    // Build extra_data with specific fields
    let mut extra_data = HashMap::new();
    if let Some(ref requested_spec) = metadata.requested_spec {
        extra_data.insert(
            "requested_spec".to_string(),
            Value::String(requested_spec.clone()),
        );
    }
    if let Some(ref channel) = metadata.channel {
        extra_data.insert("channel".to_string(), Value::String(channel.clone()));
    }
    if let Some(ref extracted_package_dir) = metadata.extracted_package_dir {
        extra_data.insert(
            "extracted_package_dir".to_string(),
            Value::String(extracted_package_dir.clone()),
        );
    }
    if let Some(ref files) = metadata.files {
        extra_data.insert(
            "files".to_string(),
            Value::Array(files.iter().map(|f| Value::String(f.clone())).collect()),
        );
    }
    if let Some(ref package_tarball_full_path) = metadata.package_tarball_full_path {
        extra_data.insert(
            "package_tarball_full_path".to_string(),
            Value::String(package_tarball_full_path.clone()),
        );
    }

    let extra_data_opt = if extra_data.is_empty() {
        None
    } else {
        Some(extra_data)
    };

    PackageData {
        package_type: Some(PACKAGE_TYPE),
        primary_language: Some("Python".to_string()),
        name: metadata.name,
        version: metadata.version,
        extracted_license_statement: metadata.license,
        download_url: metadata.url,
        size: metadata.size,
        md5: metadata.md5,
        sha256: metadata.sha256,
        extra_data: extra_data_opt,
        datasource_id: Some(DatasourceId::CondaMetaJson),
        ..Default::default()
    }
}

crate::register_parser!(
    "Conda installed package metadata JSON",
    &["*conda-meta/*.json"],
    "conda",
    "Python",
    Some("https://docs.conda.io/"),
);
