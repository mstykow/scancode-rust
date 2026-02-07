//!
//! Extracts package metadata and dependencies from Dart pubspec files.
//!

use std::fs;
use std::path::Path;

use log::warn;
use packageurl::PackageUrl;
use serde_yaml::{Mapping, Value};

use crate::models::{Dependency, PackageData, ResolvedPackage};

use super::PackageParser;

const FIELD_NAME: &str = "name";
const FIELD_VERSION: &str = "version";
const FIELD_DESCRIPTION: &str = "description";
const FIELD_HOMEPAGE: &str = "homepage";
const FIELD_LICENSE: &str = "license";
const FIELD_REPOSITORY: &str = "repository";
const FIELD_AUTHOR: &str = "author";
const FIELD_AUTHORS: &str = "authors";
const FIELD_DEPENDENCIES: &str = "dependencies";
const FIELD_DEV_DEPENDENCIES: &str = "dev_dependencies";
const FIELD_DEPENDENCY_OVERRIDES: &str = "dependency_overrides";
const FIELD_ENVIRONMENT: &str = "environment";
const FIELD_ISSUE_TRACKER: &str = "issue_tracker";
const FIELD_DOCUMENTATION: &str = "documentation";
const FIELD_EXECUTABLES: &str = "executables";
const FIELD_PUBLISH_TO: &str = "publish_to";
const FIELD_PACKAGES: &str = "packages";
const FIELD_SDKS: &str = "sdks";
const FIELD_DEPENDENCY: &str = "dependency";
const FIELD_SHA256: &str = "sha256";

/// Dart pubspec.yaml manifest parser.
pub struct PubspecYamlParser;

impl PackageParser for PubspecYamlParser {
    const PACKAGE_TYPE: &'static str = "dart";

    fn extract_package_data(path: &Path) -> PackageData {
        let yaml_content = match read_yaml_file(path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read pubspec.yaml at {:?}: {}", path, e);
                return default_package_data();
            }
        };

        parse_pubspec_yaml(&yaml_content)
    }

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "pubspec.yaml")
    }
}

/// Dart pubspec.lock lockfile parser.
pub struct PubspecLockParser;

impl PackageParser for PubspecLockParser {
    const PACKAGE_TYPE: &'static str = "pubspec";

    fn extract_package_data(path: &Path) -> PackageData {
        let yaml_content = match read_yaml_file(path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read pubspec.lock at {:?}: {}", path, e);
                return default_package_data();
            }
        };

        parse_pubspec_lock(&yaml_content)
    }

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "pubspec.lock")
    }
}

fn read_yaml_file(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))
}

fn parse_pubspec_yaml(yaml_content: &Value) -> PackageData {
    let name = extract_string_field(yaml_content, FIELD_NAME);
    let version = extract_string_field(yaml_content, FIELD_VERSION);
    let description = extract_description_field(yaml_content);
    let homepage_url = extract_string_field(yaml_content, FIELD_HOMEPAGE);
    let raw_license = extract_string_field(yaml_content, FIELD_LICENSE);
    let vcs_url = extract_string_field(yaml_content, FIELD_REPOSITORY);

    let parties = extract_authors(yaml_content);

    // Extract license statement only - detection happens in separate engine
    let declared_license_expression = None;
    let declared_license_expression_spdx = None;
    let license_detections = Vec::new();

    let dependencies = [
        collect_dependencies(
            yaml_content,
            FIELD_DEPENDENCIES,
            Some("dependencies"),
            true,
            false,
        ),
        collect_dependencies(
            yaml_content,
            FIELD_DEV_DEPENDENCIES,
            Some("dev_dependencies"),
            false,
            true,
        ),
        collect_dependencies(
            yaml_content,
            FIELD_DEPENDENCY_OVERRIDES,
            Some("dependency_overrides"),
            true,
            false,
        ),
        collect_dependencies(
            yaml_content,
            FIELD_ENVIRONMENT,
            Some("environment"),
            true,
            false,
        ),
    ]
    .concat();

    let extra_data = build_extra_data(yaml_content);

    let purl = name
        .as_ref()
        .and_then(|name| build_purl(name, version.as_deref()));

    let (api_data_url, repository_homepage_url, repository_download_url) =
        if let (Some(name_val), Some(version_val)) = (&name, &version) {
            (
                Some(format!(
                    "https://pub.dev/api/packages/{}/versions/{}",
                    name_val, version_val
                )),
                Some(format!(
                    "https://pub.dev/packages/{}/versions/{}",
                    name_val, version_val
                )),
                Some(format!(
                    "https://pub.dartlang.org/packages/{}/versions/{}.tar.gz",
                    name_val, version_val
                )),
            )
        } else {
            (None, None, None)
        };

    let download_url = repository_download_url.clone();

    PackageData {
        package_type: Some(PubspecYamlParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version,
        qualifiers: None,
        subpath: None,
        primary_language: Some("dart".to_string()),
        description,
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
        vcs_url,
        copyright: None,
        holder: None,
        declared_license_expression,
        declared_license_expression_spdx,
        license_detections,
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement: raw_license,
        notice_text: None,
        source_packages: Vec::new(),
        file_references: Vec::new(),
        is_private: false,
        is_virtual: false,
        extra_data,
        dependencies,
        repository_homepage_url,
        repository_download_url,
        api_data_url,
        datasource_id: Some("pubspec_yaml".to_string()),
        purl,
    }
}

fn parse_pubspec_lock(yaml_content: &Value) -> PackageData {
    let dependencies = extract_lock_dependencies(yaml_content);

    let mut package_data = default_package_data_with_type(PubspecLockParser::PACKAGE_TYPE);
    package_data.dependencies = dependencies;
    package_data.datasource_id = Some("pubspec_lock".to_string());
    package_data
}

fn extract_lock_dependencies(lock_data: &Value) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    if let Some(sdks) = lock_data.get(FIELD_SDKS).and_then(Value::as_mapping) {
        for (name_value, version_value) in sdks {
            if let (Some(name), Some(version_str)) = (name_value.as_str(), version_value.as_str()) {
                let purl = build_dependency_purl(name, None);
                dependencies.push(Dependency {
                    purl,
                    extracted_requirement: Some(version_str.to_string()),
                    scope: Some("sdk".to_string()),
                    is_runtime: Some(true),
                    is_optional: Some(false),
                    is_pinned: Some(false),
                    is_direct: Some(true),
                    resolved_package: None,
                    extra_data: None,
                });
            }
        }
    }

    let Some(packages) = lock_data.get(FIELD_PACKAGES).and_then(Value::as_mapping) else {
        return dependencies;
    };

    for (name_value, details_value) in packages {
        let name = match name_value.as_str() {
            Some(value) => value,
            None => continue,
        };
        let Some(details) = details_value.as_mapping() else {
            continue;
        };

        let version = mapping_get(details, FIELD_VERSION)
            .and_then(Value::as_str)
            .map(|value| value.to_string());
        let dependency_kind = mapping_get(details, FIELD_DEPENDENCY)
            .and_then(Value::as_str)
            .map(|value| value.to_string());

        let is_runtime = dependency_kind.as_deref() != Some("direct dev");

        let is_pinned = version
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty());

        let purl = build_dependency_purl(name, version.as_deref());
        let sha256 = extract_sha256(details);
        let resolved_dependencies = extract_lock_package_dependencies(details);
        let resolved_package =
            build_resolved_package(name, &version, sha256, resolved_dependencies);

        dependencies.push(Dependency {
            purl,
            extracted_requirement: version.clone(),
            scope: dependency_kind,
            is_runtime: Some(is_runtime),
            is_optional: Some(false),
            is_pinned: Some(is_pinned),
            is_direct: Some(true),
            resolved_package: Some(Box::new(resolved_package)),
            extra_data: None,
        });
    }

    dependencies
}

fn extract_lock_package_dependencies(details: &Mapping) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    let Some(dep_map) = mapping_get(details, FIELD_DEPENDENCIES).and_then(Value::as_mapping) else {
        return dependencies;
    };

    for (name_value, requirement_value) in dep_map {
        let name = match name_value.as_str() {
            Some(value) => value,
            None => continue,
        };

        let requirement = match dependency_requirement_from_value(requirement_value) {
            Some(value) => value,
            None => continue,
        };
        let is_pinned = is_pubspec_version_pinned(&requirement);
        let purl = if is_pinned {
            build_dependency_purl(name, Some(requirement.as_str()))
        } else {
            build_dependency_purl(name, None)
        };

        dependencies.push(Dependency {
            purl,
            extracted_requirement: Some(requirement),
            scope: Some(FIELD_DEPENDENCIES.to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: Some(is_pinned),
            is_direct: Some(false),
            resolved_package: None,
            extra_data: None,
        });
    }

    dependencies
}

fn extract_sha256(details: &Mapping) -> Option<String> {
    let direct = mapping_get(details, FIELD_SHA256)
        .and_then(Value::as_str)
        .map(|value| value.to_string());

    if direct.is_some() {
        return direct;
    }

    mapping_get(details, FIELD_DESCRIPTION)
        .and_then(Value::as_mapping)
        .and_then(|desc_map| mapping_get(desc_map, FIELD_SHA256))
        .and_then(Value::as_str)
        .map(|value| value.to_string())
}

fn build_resolved_package(
    name: &str,
    version: &Option<String>,
    sha256: Option<String>,
    dependencies: Vec<Dependency>,
) -> ResolvedPackage {
    ResolvedPackage {
        package_type: PubspecLockParser::PACKAGE_TYPE.to_string(),
        namespace: String::new(),
        name: name.to_string(),
        version: version.clone().unwrap_or_default(),
        primary_language: Some("dart".to_string()),
        download_url: None,
        sha1: None,
        sha256,
        sha512: None,
        md5: None,
        is_virtual: true,
        extra_data: None,
        dependencies,
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_id: None,
        purl: None,
    }
}

fn collect_dependencies(
    yaml_content: &Value,
    field: &str,
    scope: Option<&str>,
    is_runtime: bool,
    is_optional: bool,
) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    let Some(dep_map) = yaml_content.get(field).and_then(Value::as_mapping) else {
        return dependencies;
    };

    for (name_value, requirement_value) in dep_map {
        let name = match name_value.as_str() {
            Some(value) => value,
            None => continue,
        };
        let requirement = match dependency_requirement_from_value(requirement_value) {
            Some(value) => value,
            None => continue,
        };

        let is_pinned = is_pubspec_version_pinned(&requirement);
        let purl = if is_pinned {
            build_dependency_purl(name, Some(requirement.as_str()))
        } else {
            build_dependency_purl(name, None)
        };

        dependencies.push(Dependency {
            purl,
            extracted_requirement: Some(requirement),
            scope: scope.map(|value| value.to_string()),
            is_runtime: Some(is_runtime),
            is_optional: Some(is_optional),
            is_pinned: Some(is_pinned),
            is_direct: Some(true),
            resolved_package: None,
            extra_data: None,
        });
    }

    dependencies
}

fn dependency_requirement_from_value(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(trimmed.to_string());
    }

    if let Some(value) = value.as_i64() {
        return Some(value.to_string());
    }

    if let Some(value) = value.as_f64() {
        return Some(value.to_string());
    }

    if let Some(map) = value.as_mapping() {
        return format_dependency_mapping(map);
    }

    None
}

fn format_dependency_mapping(map: &Mapping) -> Option<String> {
    let mut parts = Vec::new();

    for (key, value) in map {
        let Some(key_str) = key.as_str() else {
            continue;
        };

        let value_str = if let Some(value) = value.as_str() {
            value.to_string()
        } else if let Some(value) = value.as_i64() {
            value.to_string()
        } else if let Some(value) = value.as_f64() {
            value.to_string()
        } else {
            continue;
        };

        parts.push(format!("{}: {}", key_str, value_str));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn is_pubspec_version_pinned(version: &str) -> bool {
    let trimmed = version.trim();
    if trimmed.is_empty() {
        return false;
    }

    trimmed
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
}

fn build_purl(name: &str, version: Option<&str>) -> Option<String> {
    build_purl_with_type(PubspecYamlParser::PACKAGE_TYPE, name, version)
}

fn build_dependency_purl(name: &str, version: Option<&str>) -> Option<String> {
    build_purl_with_type("pubspec", name, version)
}

fn build_purl_with_type(package_type: &str, name: &str, version: Option<&str>) -> Option<String> {
    let mut package_url = match PackageUrl::new(package_type, name) {
        Ok(purl) => purl,
        Err(e) => {
            warn!(
                "Failed to create PackageUrl for {} dependency '{}': {}",
                package_type, name, e
            );
            return None;
        }
    };

    if let Some(version) = version
        && let Err(e) = package_url.with_version(version)
    {
        warn!(
            "Failed to set version '{}' for {} dependency '{}': {}",
            version, package_type, name, e
        );
        return None;
    }

    Some(package_url.to_string())
}

fn extract_string_field(yaml_content: &Value, field: &str) -> Option<String> {
    yaml_content
        .get(field)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_description_field(yaml_content: &Value) -> Option<String> {
    // For description fields, preserve trailing newlines as they are semantically
    // significant in YAML folded/literal scalars (> or |)
    yaml_content
        .get(FIELD_DESCRIPTION)
        .and_then(Value::as_str)
        .and_then(|value| {
            // Only trim leading whitespace, preserve trailing newlines
            let trimmed = value.trim_start();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
}

fn mapping_get<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::String(key.to_string()))
}

fn default_package_data() -> PackageData {
    default_package_data_with_type(PubspecYamlParser::PACKAGE_TYPE)
}

fn default_package_data_with_type(package_type: &str) -> PackageData {
    PackageData {
        package_type: Some(package_type.to_string()),
        namespace: None,
        name: None,
        version: None,
        qualifiers: None,
        subpath: None,
        primary_language: Some("dart".to_string()),
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
        dependencies: Vec::new(),
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_id: None,
        purl: None,
    }
}

fn extract_authors(yaml_content: &Value) -> Vec<crate::models::Party> {
    use crate::models::Party;
    let mut parties = Vec::new();

    if let Some(author) = extract_string_field(yaml_content, FIELD_AUTHOR) {
        parties.push(Party {
            r#type: None,
            role: Some("author".to_string()),
            name: Some(author),
            email: None,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    if let Some(authors_value) = yaml_content.get(FIELD_AUTHORS)
        && let Some(authors_array) = authors_value.as_sequence()
    {
        for author_value in authors_array {
            if let Some(author_str) = author_value.as_str() {
                parties.push(Party {
                    r#type: None,
                    role: Some("author".to_string()),
                    name: Some(author_str.to_string()),
                    email: None,
                    url: None,
                    organization: None,
                    organization_url: None,
                    timezone: None,
                });
            }
        }
    }

    parties
}

fn build_extra_data(
    yaml_content: &Value,
) -> Option<std::collections::HashMap<String, serde_json::Value>> {
    use std::collections::HashMap;
    let mut extra_data = HashMap::new();

    if let Some(issue_tracker) = extract_string_field(yaml_content, FIELD_ISSUE_TRACKER) {
        extra_data.insert(
            FIELD_ISSUE_TRACKER.to_string(),
            serde_json::Value::String(issue_tracker),
        );
    }

    if let Some(documentation) = extract_string_field(yaml_content, FIELD_DOCUMENTATION) {
        extra_data.insert(
            FIELD_DOCUMENTATION.to_string(),
            serde_json::Value::String(documentation),
        );
    }

    if let Some(executables) = yaml_content.get(FIELD_EXECUTABLES) {
        // Convert serde_yaml::Value to serde_json::Value
        if let Ok(json_value) = serde_json::to_value(executables) {
            extra_data.insert(FIELD_EXECUTABLES.to_string(), json_value);
        }
    }

    if let Some(publish_to) = extract_string_field(yaml_content, FIELD_PUBLISH_TO) {
        extra_data.insert(
            FIELD_PUBLISH_TO.to_string(),
            serde_json::Value::String(publish_to),
        );
    }

    if extra_data.is_empty() {
        None
    } else {
        Some(extra_data)
    }
}

crate::register_parser!(
    "Dart pubspec.yaml manifest",
    &["**/pubspec.yaml", "**/pubspec.lock"],
    "pub",
    "Dart",
    Some("https://dart.dev/tools/pub/pubspec"),
);
