//! Parser for Yarn yarn.lock lockfiles.
//!
//! Extracts resolved dependency information from Yarn lockfiles supporting both
//! Yarn Classic (v1) and Yarn Berry (v2+) formats with different syntax and structures.
//!
//! # Supported Formats
//! - yarn.lock (Classic v1 format - key-value style)
//! - yarn.lock (Berry v2+ format - YAML-like structure with different key format)
//!
//! # Key Features
//! - Multi-format support for Yarn Classic and Berry versions
//! - Direct vs transitive dependency tracking (`is_direct`)
//! - Integrity hash extraction (sha1, sha512, sha256)
//! - Package URL (purl) generation for scoped and unscoped packages
//! - Workspace and monorepo dependency resolution
//!
//! # Implementation Notes
//! - v1 format: `"@scope/name@version"` keys with nested `version` and `integrity` fields
//! - v2+ format: Similar structure but different key generation with workspace awareness
//! - All lockfile versions are pinned (`is_pinned: Some(true)`)
//! - Graceful error handling with `warn!()` logs

use crate::models::{DatasourceId, Dependency, PackageData, ResolvedPackage};
use crate::parsers::utils::{npm_purl, parse_sri};
use log::warn;
use serde_yaml::Value;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::PackageParser;

/// Yarn lockfile parser supporting both Yarn Classic (v1) and Berry (v2+) formats.
///
/// Extracts pinned dependency versions with integrity hashes from yarn.lock files.
pub struct YarnLockParser;

impl PackageParser for YarnLockParser {
    const PACKAGE_TYPE: &'static str = "npm";

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == "yarn.lock")
            .unwrap_or(false)
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read yarn.lock at {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        let is_v2 = detect_yarn_version(&content);

        vec![if is_v2 {
            parse_yarn_v2(&content)
        } else {
            parse_yarn_v1(&content)
        }]
    }
}

/// Detect if yarn.lock is v2 (has __metadata) or v1 (has "yarn lockfile v1")
pub fn detect_yarn_version(content: &str) -> bool {
    content
        .lines()
        .take(10)
        .any(|line| line.contains("__metadata:"))
}

/// Parse yarn.lock v2 format (standard YAML)
fn parse_yarn_v2(content: &str) -> PackageData {
    let yaml_value: Value = match serde_yaml::from_str(content) {
        Ok(val) => val,
        Err(e) => {
            warn!("Failed to parse yarn.lock v2 YAML: {}", e);
            return default_package_data();
        }
    };

    let yaml_map = match yaml_value.as_mapping() {
        Some(map) => map,
        None => return default_package_data(),
    };

    let mut dependencies = Vec::new();

    for (spec, details) in yaml_map {
        if spec.as_str().map(|s| s == "__metadata").unwrap_or(false) {
            continue;
        }

        let _spec_str = match spec.as_str() {
            Some(s) => s,
            None => continue,
        };

        let details_map = match details.as_mapping() {
            Some(map) => map,
            None => continue,
        };

        let _version = extract_yaml_string(details_map, "version").unwrap_or_default();
        let resolution = extract_yaml_string(details_map, "resolution").unwrap_or_default();

        let (namespace_opt, name, resolved_version) = parse_yarn_v2_resolution(&resolution);
        let namespace = namespace_opt.unwrap_or_default();
        let purl = create_purl(&namespace, &name, &resolved_version);
        let checksum = extract_yaml_string(details_map, "checksum");

        let deps_yaml = details_map.get("dependencies");
        let peer_deps_yaml = details_map.get("peerDependencies");

        let nested_deps = parse_yaml_dependencies(deps_yaml);
        let peer_deps = parse_yaml_dependencies(peer_deps_yaml);

        let all_deps = if peer_deps.is_empty() {
            nested_deps
        } else {
            let mut combined = nested_deps;
            for mut dep in peer_deps {
                dep.scope = Some("peerDependencies".to_string());
                dep.is_optional = Some(true);
                dep.is_runtime = Some(false);
                combined.push(dep);
            }
            combined
        };

        let resolved_package = ResolvedPackage {
            package_type: YarnLockParser::PACKAGE_TYPE.to_string(),
            namespace: namespace.clone(),
            name: name.clone(),
            version: resolved_version.clone(),
            primary_language: Some("JavaScript".to_string()),
            download_url: None,
            sha1: None,
            sha256: None,
            sha512: checksum,
            md5: None,
            is_virtual: true,
            extra_data: None,
            dependencies: all_deps,
            repository_homepage_url: None,
            repository_download_url: None,
            api_data_url: None,
            datasource_id: Some(DatasourceId::YarnLock),
            purl: None,
        };

        // For Yarn v2+, check if this is a workspace dependency (direct)
        // Workspace dependencies use "workspace:*" resolution
        let is_direct = resolution.contains("workspace:");

        let dependency = Dependency {
            purl,
            extracted_requirement: Some(resolved_version),
            scope: Some("dependencies".to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: Some(true),
            is_direct: Some(is_direct),
            resolved_package: Some(Box::new(resolved_package)),
            extra_data: None,
        };

        dependencies.push(dependency);
    }

    PackageData {
        package_type: Some(YarnLockParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name: None,
        version: None,
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
        sha256: None,
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
        api_data_url: None,
        datasource_id: Some(DatasourceId::YarnLock),
        purl: None,
    }
}

/// Parse yarn.lock v1 format (custom YAML-like)
fn parse_yarn_v1(content: &str) -> PackageData {
    let mut dependencies = Vec::new();
    let mut seen_purls = HashSet::new();

    for block in content.split("\n\n") {
        if is_empty_or_comment_block(block) {
            continue;
        }

        if let Some(dep) = parse_yarn_v1_block(block) {
            if let Some(ref purl) = dep.purl {
                if seen_purls.insert(purl.clone()) {
                    dependencies.push(dep);
                }
            } else {
                dependencies.push(dep);
            }
        }
    }

    PackageData {
        package_type: Some(YarnLockParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name: None,
        version: None,
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
        sha256: None,
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
        api_data_url: None,
        datasource_id: Some(DatasourceId::YarnLock),
        purl: None,
    }
}

fn is_empty_or_comment_block(block: &str) -> bool {
    block
        .lines()
        .all(|line| line.trim().is_empty() || line.trim().starts_with('#'))
}

/// Parse integrity field (format: "sha512-base64string==")
fn parse_integrity_field(integrity: &str) -> Option<String> {
    parse_sri(integrity).and_then(|(algo, hex_digest)| {
        if algo == "sha512" {
            Some(hex_digest)
        } else {
            None
        }
    })
}

/// Extract namespace and name from package name ("@types/node" -> ("@types", "node"))
pub fn extract_namespace_and_name(package_name: &str) -> (String, String) {
    if package_name.starts_with('@') {
        let parts: Vec<&str> = package_name.splitn(2, '/').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (String::new(), package_name.to_string())
        }
    } else {
        (String::new(), package_name.to_string())
    }
}

fn create_purl(namespace: &str, name: &str, version: &str) -> Option<String> {
    let full_name = if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", namespace, name)
    };
    let version_opt = if version.is_empty() {
        None
    } else {
        Some(version)
    };
    npm_purl(&full_name, version_opt)
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(YarnLockParser::PACKAGE_TYPE.to_string()),
        datasource_id: Some(DatasourceId::YarnLock),
        ..Default::default()
    }
}

/// Parse a single yarn v1 dependency block
fn parse_yarn_v1_block(block: &str) -> Option<Dependency> {
    let lines: Vec<&str> = block.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let requirement_line = lines[0]
        .trim()
        .strip_suffix(':')
        .unwrap_or_else(|| lines[0].trim())
        .trim_matches('"');
    if requirement_line.is_empty() || requirement_line.starts_with('#') {
        return None;
    }

    let (namespace, name, constraint) = parse_yarn_v1_requirement(requirement_line);

    if name.is_empty() {
        return None;
    }

    let mut version = String::new();
    let mut resolved_url = String::new();
    let mut integrity = String::new();
    let mut nested_deps = Vec::new();

    for line in &lines[1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("version") {
            version = extract_quoted_value(trimmed).unwrap_or_default();
        } else if trimmed.starts_with("resolved") {
            resolved_url = extract_quoted_value(trimmed).unwrap_or_default();
        } else if trimmed.starts_with("integrity") {
            integrity = trimmed
                .strip_prefix("integrity")
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
        } else if trimmed.starts_with("dependencies") {
            // Dependencies block - parse indented lines
            continue;
        } else if trimmed.starts_with("  ") && !trimmed.starts_with("    ") {
            // Dependency line (2-space indent)
            let dep_line = trimmed.trim();
            if let Some(dep) = parse_yarn_v1_dependency_line(dep_line, &namespace, &name, &version)
            {
                nested_deps.push(dep);
            }
        }
    }

    let sha512 = if integrity.is_empty() {
        None
    } else {
        parse_integrity_field(&integrity)
    };

    let purl = create_purl(&namespace, &name, &version);

    let resolved_package = ResolvedPackage {
        package_type: YarnLockParser::PACKAGE_TYPE.to_string(),
        namespace: namespace.clone(),
        name: name.clone(),
        version: version.clone(),
        primary_language: Some("JavaScript".to_string()),
        download_url: if resolved_url.is_empty() {
            None
        } else {
            Some(resolved_url)
        },
        sha1: None,
        sha256: None,
        sha512,
        md5: None,
        is_virtual: true,
        extra_data: None,
        dependencies: nested_deps,
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_id: Some(DatasourceId::YarnLock),
        purl: None,
    };

    Some(Dependency {
        purl,
        extracted_requirement: Some(constraint),
        scope: Some("dependencies".to_string()),
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(true),
        is_direct: Some(true),
        resolved_package: Some(Box::new(resolved_package)),
        extra_data: None,
    })
}

/// Parse yarn v1 requirement line: "express@^4.0.0" or "@babel/core@^7.1.0"
pub fn parse_yarn_v1_requirement(line: &str) -> (String, String, String) {
    // Handle multiple aliases: "rimraf@2, rimraf@~2.5.1"
    if line.contains(", ") {
        // Use the first part for parsing
        let first_part = line.split(", ").next().unwrap_or(line);
        return parse_single_yarn_v1_requirement(first_part);
    }
    parse_single_yarn_v1_requirement(line)
}

/// Parse a single yarn v1 requirement
fn parse_single_yarn_v1_requirement(line: &str) -> (String, String, String) {
    if let Some(at_pos) = line.rfind('@') {
        let name_part = &line[..at_pos];
        let constraint = &line[at_pos + 1..];
        let (namespace, name) = extract_namespace_and_name(name_part);

        if !name.is_empty() {
            return (namespace, name, constraint.to_string());
        }
    }

    (String::new(), String::new(), String::new())
}

/// Parse a dependency line from yarn v1 block: "\"dep@^1.0.0\""
fn parse_yarn_v1_dependency_line(
    line: &str,
    _parent_namespace: &str,
    _parent_name: &str,
    parent_version: &str,
) -> Option<Dependency> {
    let trimmed = line.trim_matches('"');
    if !trimmed.contains('@') {
        return None;
    }

    let (namespace, name, constraint) = parse_yarn_v1_requirement(trimmed);

    let purl = create_purl(&namespace, &name, parent_version);

    Some(Dependency {
        purl,
        extracted_requirement: Some(constraint),
        scope: Some("dependencies".to_string()),
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(false),
        is_direct: Some(false),
        resolved_package: None,
        extra_data: None,
    })
}

/// Extract value from quoted line: 'version "1.0.0"' -> "1.0.0"
fn extract_quoted_value(line: &str) -> Option<String> {
    line.find('"').and_then(|start| {
        let rest = &line[start + 1..];
        rest.find('"').map(|end| rest[..end].to_string())
    })
}

/// Parse yarn v2 resolution: "@actions/core@npm:1.2.6" -> ("@actions", "core", "1.2.6")
pub fn parse_yarn_v2_resolution(resolution: &str) -> (Option<String>, String, String) {
    if resolution.contains("@npm:") {
        let parts: Vec<&str> = resolution.split("@npm:").collect();
        if parts.len() == 2 {
            let package_name = parts[0];
            let version = parts[1];
            let (namespace, name) = extract_namespace_and_name(package_name);
            let namespace_opt = if namespace.is_empty() {
                None
            } else {
                Some(namespace)
            };
            return (namespace_opt, name, version.to_string());
        }
    }

    let (namespace, name) = extract_namespace_and_name(resolution);
    let namespace_opt = if namespace.is_empty() {
        None
    } else {
        Some(namespace)
    };
    (namespace_opt, name, "*".to_string())
}

/// Extract string value from YAML mapping
fn extract_yaml_string(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Parse dependencies from YAML Value
fn parse_yaml_dependencies(yaml_value: Option<&Value>) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    if let Some(deps_value) = yaml_value
        && let Some(mapping) = deps_value.as_mapping()
    {
        for (key, value) in mapping {
            let name = match key.as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let constraint = match value.as_str() {
                Some(s) => s.to_string(),
                None => "*".to_string(),
            };

            let (namespace, dep_name) = extract_namespace_and_name(&name);
            let purl = create_purl(&namespace, &dep_name, &constraint);

            dependencies.push(Dependency {
                purl,
                extracted_requirement: Some(constraint),
                scope: Some("dependencies".to_string()),
                is_runtime: Some(true),
                is_optional: Some(false),
                is_pinned: Some(false),
                is_direct: Some(false),
                resolved_package: None,
                extra_data: None,
            });
        }
    }
    dependencies
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_match_yarn_lock() {
        let valid_path = PathBuf::from("/some/path/yarn.lock");
        assert!(YarnLockParser::is_match(&valid_path));
    }

    #[test]
    fn test_is_not_match_package_json() {
        let invalid_path = PathBuf::from("/some/path/package.json");
        assert!(!YarnLockParser::is_match(&invalid_path));
    }

    #[test]
    fn test_detect_yarn_v2() {
        let content = r#"# This file is generated by running "yarn install"
__metadata:
  version: 6
"#;
        assert!(detect_yarn_version(content));
    }

    #[test]
    fn test_detect_yarn_v1() {
        let content = r#"# THIS IS AN AUTOGENERATED FILE
# yarn lockfile v1

abbrev@1:
  version "1.0.9"
"#;
        assert!(!detect_yarn_version(content));
    }

    #[test]
    fn test_extract_namespace_and_name_scoped() {
        let (namespace, name) = extract_namespace_and_name("@types/node");
        assert_eq!(namespace, "@types");
        assert_eq!(name, "node");
    }

    #[test]
    fn test_extract_namespace_and_name_regular() {
        let (namespace, name) = extract_namespace_and_name("express");
        assert_eq!(namespace, "");
        assert_eq!(name, "express");
    }

    #[test]
    fn test_parse_yarn_v1_requirement() {
        let (namespace, name, constraint) = parse_yarn_v1_requirement("express@^4.0.0");
        assert_eq!(namespace, "");
        assert_eq!(name, "express");
        assert_eq!(constraint, "^4.0.0");
    }

    #[test]
    fn test_parse_yarn_v1_requirement_scoped() {
        let (namespace, name, constraint) = parse_yarn_v1_requirement("@types/node@^18.0.0");
        assert_eq!(namespace, "@types");
        assert_eq!(name, "node");
        assert_eq!(constraint, "^18.0.0");
    }

    #[test]
    fn test_parse_yarn_v2_resolution() {
        let (namespace, name, version) = parse_yarn_v2_resolution("@actions/core@npm:1.2.6");
        assert_eq!(namespace, Some("@actions".to_string()));
        assert_eq!(name, "core");
        assert_eq!(version, "1.2.6");
    }
}

crate::register_parser!(
    "yarn.lock lockfile (v1 and v2+)",
    &["**/yarn.lock"],
    "npm",
    "JavaScript",
    Some("https://classic.yarnpkg.com/lang/en/docs/yarn-lock/"),
);
