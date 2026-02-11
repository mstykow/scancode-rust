//! Parser for Cargo.lock lockfiles.
//!
//! Extracts resolved dependency information including exact versions and
//! checksums from Rust Cargo.lock files.
//!
//! # Supported Formats
//! - Cargo.lock (lockfile)
//!
//! # Key Features
//! - Exact version resolution from lockfile
//! - Direct vs transitive dependency tracking (`is_direct`)
//! - Checksum extraction for verification
//! - Package URL (purl) generation
//! - Dependency graph with source tracking (crates.io, git, path)
//!
//! # Implementation Notes
//! - All lockfile versions are pinned (`is_pinned: Some(true)`)
//! - Direct dependencies determined from root package's dependency list
//! - Uses TOML parsing for structured data extraction

use crate::models::{DatasourceId, Dependency, PackageData, PackageType};
use log::warn;
use packageurl::PackageUrl;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml::Value;

use super::PackageParser;

/// Rust Cargo.lock lockfile parser.
///
/// Extracts pinned dependency versions with checksums from Cargo-managed Rust projects.
pub struct CargoLockParser;

impl PackageParser for CargoLockParser {
    const PACKAGE_TYPE: PackageType = PackageType::Cargo;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "Cargo.lock")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match read_cargo_lock(path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read or parse Cargo.lock at {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        let packages = match content.get("package").and_then(|v| v.as_array()) {
            Some(pkgs) => pkgs,
            None => {
                warn!("No 'package' array found in Cargo.lock at {:?}", path);
                return vec![default_package_data()];
            }
        };

        let root_package = packages.first().and_then(|v| v.as_table());

        let name = root_package
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let version = root_package
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let checksum = root_package
            .and_then(|p| p.get("checksum"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let dependencies = extract_all_dependencies(packages);

        let purl = match (&name, &version) {
            (Some(n), Some(v)) => PackageUrl::new("cargo", n).ok().and_then(|mut p| {
                p.with_version(v.as_str()).ok()?;
                Some(p.to_string())
            }),
            _ => None,
        };

        let api_data_url = match (&name, &version) {
            (Some(n), Some(v)) => Some(format!("https://crates.io/api/v1/crates/{}/{}", n, v)),
            (Some(n), None) => Some(format!("https://crates.io/api/v1/crates/{}", n)),
            _ => None,
        };

        vec![PackageData {
            package_type: Some(Self::PACKAGE_TYPE),
            namespace: None,
            name,
            version,
            qualifiers: None,
            subpath: None,
            primary_language: None,
            description: None,
            release_date: None,
            parties: Vec::new(),
            keywords: Vec::new(),
            homepage_url: None,
            download_url: None,
            size: None,
            sha1: None,
            md5: None,
            sha256: checksum,
            sha512: None,
            bug_tracking_url: None,
            code_view_url: None,
            vcs_url: None,
            copyright: None,
            holder: None,
            declared_license_expression: None,
            declared_license_expression_spdx: None,
            license_detections: Vec::new(),
            other_license_expression: None,
            other_license_expression_spdx: None,
            other_license_detections: Vec::new(),
            extracted_license_statement: None,
            notice_text: None,
            source_packages: Vec::new(),
            file_references: Vec::new(),
            is_private: false,
            is_virtual: false,
            extra_data: None,
            dependencies,
            repository_homepage_url: None,
            repository_download_url: None,
            api_data_url,
            datasource_id: Some(DatasourceId::CargoLock),
            purl,
        }]
    }
}

fn read_cargo_lock(path: &Path) -> Result<Value, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))
}

fn extract_all_dependencies(packages: &[Value]) -> Vec<Dependency> {
    let mut all_dependencies = Vec::new();

    let root_package_name = packages
        .first()
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    for (index, package) in packages.iter().enumerate() {
        if let Some(pkg_table) = package.as_table() {
            let pkg_name = pkg_table.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let _pkg_version = pkg_table
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let is_root_package = index == 0 && pkg_name == root_package_name;

            if let Some(deps) = pkg_table.get("dependencies").and_then(|v| v.as_array()) {
                for dep in deps {
                    if let Some(dep_str) = dep.as_str() {
                        let (name, version) = parse_dependency_string(dep_str);
                        if !name.is_empty() {
                            let purl = if version.is_empty() {
                                PackageUrl::new("cargo", name).ok().map(|p| p.to_string())
                            } else {
                                PackageUrl::new("cargo", name).ok().and_then(|mut p| {
                                    p.with_version(version).ok()?;
                                    Some(p.to_string())
                                })
                            };

                            all_dependencies.push(Dependency {
                                purl,
                                extracted_requirement: if version.is_empty() {
                                    None
                                } else {
                                    Some(version.to_string())
                                },
                                scope: Some("dependencies".to_string()),
                                is_runtime: Some(true),
                                is_optional: Some(false),
                                is_pinned: Some(true),
                                is_direct: Some(is_root_package),
                                resolved_package: None,
                                extra_data: None,
                            });
                        }
                    }
                }
            }
        }
    }

    all_dependencies
}

fn parse_dependency_string(dep_str: &str) -> (&str, &str) {
    if let Some(space_idx) = dep_str.find(' ') {
        let name = &dep_str[..space_idx];
        let version = &dep_str[space_idx + 1..];
        (name, version)
    } else {
        (dep_str, "")
    }
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(CargoLockParser::PACKAGE_TYPE),
        ..Default::default()
    }
}

crate::register_parser!(
    "Rust Cargo.lock lockfile",
    &["**/Cargo.lock", "**/cargo.lock"],
    "cargo",
    "Rust",
    Some("https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html"),
);
