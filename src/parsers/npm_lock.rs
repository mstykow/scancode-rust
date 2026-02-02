use crate::models::{Dependency, PackageData, ResolvedPackage};
use base64::prelude::*;
use log::warn;
use packageurl::PackageUrl;
use serde_json::Value;
use std::fs;
use std::path::Path;

use super::PackageParser;

// Field name constants
const FIELD_LOCKFILE_VERSION: &str = "lockfileVersion";
const FIELD_NAME: &str = "name";
const FIELD_VERSION: &str = "version";
const FIELD_DEPENDENCIES: &str = "dependencies";
const FIELD_PACKAGES: &str = "packages";
const FIELD_RESOLVED: &str = "resolved";
const FIELD_INTEGRITY: &str = "integrity";
const FIELD_DEV: &str = "dev";
const FIELD_OPTIONAL: &str = "optional";
const FIELD_DEV_OPTIONAL: &str = "devOptional";

pub struct NpmLockParser;

impl PackageParser for NpmLockParser {
    const PACKAGE_TYPE: &'static str = "npm";

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                name == "package-lock.json"
                    || name == ".package-lock.json"
                    || name == "npm-shrinkwrap.json"
                    || name == ".npm-shrinkwrap.json"
            })
            .unwrap_or(false)
    }

    fn extract_package_data(path: &Path) -> PackageData {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read package-lock.json at {:?}: {}", path, e);
                return default_package_data();
            }
        };

        let json: Value = match serde_json::from_str(&content) {
            Ok(json) => json,
            Err(e) => {
                warn!("Failed to parse package-lock.json at {:?}: {}", path, e);
                return default_package_data();
            }
        };

        let lockfile_version = json
            .get(FIELD_LOCKFILE_VERSION)
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        let root_name = json
            .get(FIELD_NAME)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let root_version = json
            .get(FIELD_VERSION)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if lockfile_version == 1 {
            parse_lockfile_v1(&json, root_name, root_version, lockfile_version)
        } else {
            parse_lockfile_v2_plus(&json, root_name, root_version, lockfile_version)
        }
    }
}

/// Returns a default empty PackageData for error cases
fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(NpmLockParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name: None,
        version: None,
        homepage_url: None,
        download_url: None,
        copyright: None,
        license_detections: Vec::new(),
        dependencies: Vec::new(),
        parties: Vec::new(),
        purl: None,
    }
}

/// Parse lockfile version 2 or 3 (flat structure with "packages" key)
fn parse_lockfile_v2_plus(
    json: &Value,
    root_name: String,
    root_version: String,
    _lockfile_version: i64,
) -> PackageData {
    let packages = match json.get(FIELD_PACKAGES).and_then(|v| v.as_object()) {
        Some(packages) => packages,
        None => {
            warn!("No 'packages' field found in lockfile v2+");
            return default_package_data();
        }
    };

    let (namespace, name) = extract_namespace_and_name(&root_name);
    let purl = create_purl(&namespace, &name, &root_version);

    let mut dependencies = Vec::new();

    for (key, value) in packages {
        // Skip the root package (empty string key)
        if key.is_empty() {
            continue;
        }

        // Extract package name from path like "node_modules/@types/node" or "node_modules/express"
        let package_name = extract_package_name_from_path(key);
        if package_name.is_empty() {
            continue;
        }

        let version = match value.get(FIELD_VERSION).and_then(|v| v.as_str()) {
            Some(v) => v.to_string(),
            None => continue, // Skip entries without version
        };

        let (dep_namespace, dep_name) = extract_namespace_and_name(&package_name);

        // Determine dependency scope and flags
        let is_dev = value
            .get(FIELD_DEV)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_optional = value
            .get(FIELD_OPTIONAL)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_dev_optional = value
            .get(FIELD_DEV_OPTIONAL)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (scope, is_runtime, is_optional_flag) =
            determine_scope(is_dev, is_dev_optional, is_optional);

        // Create PURL for dependency
        let dep_purl = create_purl(&dep_namespace, &dep_name, &version);

        // Parse integrity and resolved URL
        let resolved = value.get(FIELD_RESOLVED).and_then(|v| v.as_str());
        let integrity = value.get(FIELD_INTEGRITY).and_then(|v| v.as_str());

        let (sha1_from_integrity, sha512_from_integrity) = parse_integrity_field(integrity);
        let sha1_from_url = resolved.and_then(parse_resolved_url);

        // Prefer integrity sha1 over URL sha1
        let sha1 = sha1_from_integrity.or(sha1_from_url);

        // Create resolved package
        let resolved_package = ResolvedPackage {
            package_type: NpmLockParser::PACKAGE_TYPE.to_string(),
            namespace: dep_namespace.clone(),
            name: dep_name.clone(),
            version: version.clone(),
            primary_language: Some("JavaScript".to_string()),
            download_url: resolved.map(|s| s.to_string()),
            sha1,
            sha512: sha512_from_integrity,
            is_virtual: true,
            dependencies: Vec::new(), // v2+ doesn't have nested dependencies
        };

        let dependency = Dependency {
            purl: dep_purl,
            extracted_requirement: Some(version),
            scope: Some(scope.to_string()),
            is_runtime: Some(is_runtime),
            is_optional: Some(is_optional_flag),
            is_pinned: Some(true),  // Lock files always have pinned versions
            is_direct: Some(false), // In flat structure, we mark all as transitive
            resolved_package: Some(Box::new(resolved_package)),
        };

        dependencies.push(dependency);
    }

    PackageData {
        package_type: Some(NpmLockParser::PACKAGE_TYPE.to_string()),
        namespace: Some(namespace),
        name: Some(name),
        version: Some(root_version),
        homepage_url: None,
        download_url: None,
        copyright: None,
        license_detections: Vec::new(),
        dependencies,
        parties: Vec::new(),
        purl,
    }
}

/// Parse lockfile version 1 (nested structure with "dependencies" key)
fn parse_lockfile_v1(
    json: &Value,
    root_name: String,
    root_version: String,
    _lockfile_version: i64,
) -> PackageData {
    let dependencies_obj = match json.get(FIELD_DEPENDENCIES).and_then(|v| v.as_object()) {
        Some(deps) => deps,
        None => {
            warn!("No 'dependencies' field found in lockfile v1");
            return default_package_data();
        }
    };

    let (namespace, name) = extract_namespace_and_name(&root_name);
    let purl = create_purl(&namespace, &name, &root_version);

    let dependencies = parse_dependencies_v1(dependencies_obj);

    PackageData {
        package_type: Some(NpmLockParser::PACKAGE_TYPE.to_string()),
        namespace: Some(namespace),
        name: Some(name),
        version: Some(root_version),
        homepage_url: None,
        download_url: None,
        copyright: None,
        license_detections: Vec::new(),
        dependencies,
        parties: Vec::new(),
        purl,
    }
}

/// Recursively parse v1 dependencies object
fn parse_dependencies_v1(dependencies_obj: &serde_json::Map<String, Value>) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    for (package_name, dep_data) in dependencies_obj {
        let version = match dep_data.get(FIELD_VERSION).and_then(|v| v.as_str()) {
            Some(v) => v.to_string(),
            None => continue,
        };

        let (dep_namespace, dep_name) = extract_namespace_and_name(package_name);

        // Determine dependency scope and flags
        let is_dev = dep_data
            .get(FIELD_DEV)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_optional = dep_data
            .get(FIELD_OPTIONAL)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (scope, is_runtime, is_optional_flag) = determine_scope(is_dev, false, is_optional);

        // Create PURL for dependency
        let dep_purl = create_purl(&dep_namespace, &dep_name, &version);

        // Parse integrity and resolved URL
        let resolved = dep_data.get(FIELD_RESOLVED).and_then(|v| v.as_str());
        let integrity = dep_data.get(FIELD_INTEGRITY).and_then(|v| v.as_str());

        let (sha1_from_integrity, sha512_from_integrity) = parse_integrity_field(integrity);
        let sha1_from_url = resolved.and_then(parse_resolved_url);

        let sha1 = sha1_from_integrity.or(sha1_from_url);

        // Parse nested dependencies if present
        let nested_deps = dep_data
            .get(FIELD_DEPENDENCIES)
            .and_then(|v| v.as_object())
            .map(parse_dependencies_v1)
            .unwrap_or_default();

        // Create resolved package
        let resolved_package = ResolvedPackage {
            package_type: NpmLockParser::PACKAGE_TYPE.to_string(),
            namespace: dep_namespace.clone(),
            name: dep_name.clone(),
            version: version.clone(),
            primary_language: Some("JavaScript".to_string()),
            download_url: resolved.map(|s| s.to_string()),
            sha1,
            sha512: sha512_from_integrity,
            is_virtual: true,
            dependencies: nested_deps,
        };

        let dependency = Dependency {
            purl: dep_purl,
            extracted_requirement: Some(version),
            scope: Some(scope.to_string()),
            is_runtime: Some(is_runtime),
            is_optional: Some(is_optional_flag),
            is_pinned: Some(true),
            is_direct: Some(false),
            resolved_package: Some(Box::new(resolved_package)),
        };

        dependencies.push(dependency);
    }

    dependencies
}

/// Extract namespace and name from a package name like "@types/node" or "express"
/// Returns: (namespace, name) where namespace is empty string "" for non-scoped packages
fn extract_namespace_and_name(package_name: &str) -> (String, String) {
    if package_name.starts_with('@') {
        // Scoped package like "@types/node"
        let parts: Vec<&str> = package_name.splitn(2, '/').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            // Invalid format, treat as non-scoped
            (String::new(), package_name.to_string())
        }
    } else {
        // Regular package like "express"
        (String::new(), package_name.to_string())
    }
}

/// Extract package name from path like "node_modules/@types/node" or "node_modules/express"
fn extract_package_name_from_path(path: &str) -> String {
    // Find the last occurrence of "node_modules/"
    if let Some(pos) = path.rfind("node_modules/") {
        let after_node_modules = &path[pos + "node_modules/".len()..];

        // Handle scoped packages: "@scope/package"
        if after_node_modules.starts_with('@') {
            // Find the second slash (after @scope/)
            if let Some(slash_pos) = after_node_modules.find('/') {
                let scope_and_package = &after_node_modules[..=slash_pos];
                // Find if there's another segment after the package name
                let remaining = &after_node_modules[slash_pos + 1..];
                if let Some(next_slash) = remaining.find('/') {
                    // Return just @scope/package
                    return format!("{}{}", scope_and_package, &remaining[..next_slash]);
                } else {
                    // Return the full scoped package name
                    return after_node_modules.to_string();
                }
            }
        } else {
            // Regular package: take everything until first slash (or end of string)
            if let Some(slash_pos) = after_node_modules.find('/') {
                return after_node_modules[..slash_pos].to_string();
            } else {
                return after_node_modules.to_string();
            }
        }
    }

    path.to_string()
}

/// Create a Package URL (PURL) from namespace, name, and version
/// Namespace should be "@org" for scoped packages, "" for regular packages
fn create_purl(namespace: &str, name: &str, version: &str) -> Option<String> {
    let mut package_url = PackageUrl::new(NpmLockParser::PACKAGE_TYPE, name).ok()?;

    // Add namespace if present (for scoped packages)
    if !namespace.is_empty() {
        package_url.with_namespace(namespace).ok()?;
    }

    // Add version
    package_url.with_version(version).ok()?;

    Some(package_url.to_string())
}

/// Parse integrity field like "sha512-base64string==" or "sha1-base64string="
/// Returns: (sha1, sha512) as hex strings
fn parse_integrity_field(integrity: Option<&str>) -> (Option<String>, Option<String>) {
    let integrity = match integrity {
        Some(i) => i,
        None => return (None, None),
    };

    // Format: "algorithm-base64string"
    let parts: Vec<&str> = integrity.splitn(2, '-').collect();
    if parts.len() != 2 {
        return (None, None);
    }

    let algorithm = parts[0];
    let base64_str = parts[1];

    // Decode base64 to bytes
    let bytes = match BASE64_STANDARD.decode(base64_str) {
        Ok(bytes) => bytes,
        Err(_) => return (None, None),
    };

    // Convert bytes to hex string
    let hex_string = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    match algorithm {
        "sha1" => (Some(hex_string), None),
        "sha512" => (None, Some(hex_string)),
        _ => (None, None),
    }
}

/// Parse resolved URL to extract sha1 checksum if present
/// Example: "https://registry.npmjs.org/package/-/package-1.0.0.tgz#abc123def"
fn parse_resolved_url(url: &str) -> Option<String> {
    // Look for # followed by hex characters
    if let Some(hash_pos) = url.rfind('#') {
        let hash = &url[hash_pos + 1..];
        // Verify it's a hex string (sha1 is 40 characters)
        if hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(hash.to_string());
        }
    }
    None
}

/// Determine scope, is_runtime, and is_optional based on dev/optional flags
/// Returns: (scope, is_runtime, is_optional)
fn determine_scope(
    is_dev: bool,
    is_dev_optional: bool,
    is_optional: bool,
) -> (&'static str, bool, bool) {
    if is_dev || is_dev_optional {
        ("devDependencies", false, true)
    } else if is_optional {
        ("dependencies", true, true)
    } else {
        ("dependencies", true, false)
    }
}
