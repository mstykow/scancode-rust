//! Parser for Conda/Anaconda package manifest files.
//!
//! Extracts package metadata and dependencies from Conda ecosystem manifest files
//! supporting both recipe definitions and environment specifications.
//!
//! # Supported Formats
//! - meta.yaml (Conda recipe metadata with Jinja2 templating support)
//! - conda.yaml/environment.yml (Conda environment dependency specifications)
//!
//! # Key Features
//! - YAML parsing for environment files
//! - Dependency extraction from dependencies and build_requirements sections
//! - Channel specification and platform detection
//! - Version constraint parsing for Conda version specifiers
//! - Package URL (purl) generation for conda packages
//! - Limited meta.yaml support (note: Jinja2 templating not fully resolved)
//!
//! # Implementation Notes
//! - Uses YAML parsing via `serde_yaml` crate
//! - meta.yaml: Jinja2 templates not evaluated (use rendered YAML if available)
//! - environment.yml: Full dependency specification support
//! - Graceful error handling with `warn!()` logs
//!
//! # References
//! - <https://docs.conda.io/projects/conda-build/en/latest/resources/define-metadata.html>
//! - <https://docs.conda.io/projects/conda/en/latest/user-guide/tasks/manage-environments.html>

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use log::warn;
use serde_yaml::Value;

use crate::models::{Dependency, PackageData};
use crate::parsers::utils::create_default_package_data;

use super::PackageParser;

fn default_package_data() -> PackageData {
    create_default_package_data("conda", None)
}

/// Build a PURL (Package URL) for Conda or PyPI packages
fn build_purl(
    package_type: &str,
    namespace: Option<&str>,
    name: &str,
    version: Option<&str>,
    _qualifiers: Option<&str>,
    _subpath: Option<&str>,
    _extras: Option<&str>,
) -> Option<String> {
    let purl = match package_type {
        "conda" => {
            if let Some(ns) = namespace {
                match version {
                    Some(v) => format!("pkg:conda/{}/{}@{}", ns, name, v),
                    None => format!("pkg:conda/{}/{}", ns, name),
                }
            } else {
                match version {
                    Some(v) => format!("pkg:conda/{}@{}", name, v),
                    None => format!("pkg:conda/{}", name),
                }
            }
        }
        "pypi" => match version {
            Some(v) => format!("pkg:pypi/{}@{}", name, v),
            None => format!("pkg:pypi/{}", name),
        },
        _ => format!("pkg:{}/{}", package_type, name),
    };
    Some(purl)
}

/// Conda recipe manifest (meta.yaml) parser.
///
/// Extracts package metadata and dependencies from Conda recipe files, which
/// define how to build a Conda package. Handles Jinja2 templating used in
/// recipe files for variable substitution.
pub struct CondaMetaYamlParser;

impl PackageParser for CondaMetaYamlParser {
    const PACKAGE_TYPE: &'static str = "conda";

    fn is_match(path: &Path) -> bool {
        // Match */meta.yaml following Python reference logic
        path.file_name()
            .is_some_and(|name| name == "meta.yaml" || name == "meta.yml")
    }

    fn extract_package_data(path: &Path) -> PackageData {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read {}: {}", path.display(), e);
                return default_package_data();
            }
        };

        // Extract Jinja2 variables and apply crude substitution
        let variables = extract_jinja2_variables(&contents);
        let processed_yaml = apply_jinja2_substitutions(&contents, &variables);

        // Parse YAML after Jinja2 processing
        let yaml: Value = match serde_yaml::from_str(&processed_yaml) {
            Ok(y) => y,
            Err(e) => {
                warn!("Failed to parse YAML in {}: {}", path.display(), e);
                return default_package_data();
            }
        };

        let package_element = yaml.get("package").and_then(|v| v.as_mapping());
        let name = package_element
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let version = package_element
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let source = yaml.get("source").and_then(|v| v.as_mapping());
        let download_url = source
            .and_then(|s| s.get("url"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let sha256 = source
            .and_then(|s| s.get("sha256"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let about = yaml.get("about").and_then(|v| v.as_mapping());
        let homepage_url = about
            .and_then(|a| a.get("home"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let extracted_license_statement = about
            .and_then(|a| a.get("license"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let description = about
            .and_then(|a| a.get("summary"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let vcs_url = about
            .and_then(|a| a.get("dev_url"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract dependencies from requirements sections
        let mut dependencies = Vec::new();
        let mut extra_data: HashMap<String, serde_json::Value> = HashMap::new();

        if let Some(requirements) = yaml.get("requirements").and_then(|v| v.as_mapping()) {
            for (scope_key, reqs_value) in requirements {
                let scope = scope_key.as_str().unwrap_or("unknown");
                if let Some(reqs) = reqs_value.as_sequence() {
                    for req in reqs {
                        if let Some(req_str) = req.as_str()
                            && let Some(dep) = parse_conda_requirement(req_str, scope)
                        {
                            // Filter out pip/python from dependencies, add to extra_data
                            if dep
                                .purl
                                .as_deref()
                                .is_some_and(|p| p.contains("pkg:conda/pip"))
                                || dep
                                    .purl
                                    .as_deref()
                                    .is_some_and(|p| p.contains("pkg:conda/python"))
                            {
                                if let Some(arr) = extra_data
                                    .entry(scope.to_string())
                                    .or_insert_with(|| serde_json::Value::Array(vec![]))
                                    .as_array_mut()
                                {
                                    arr.push(serde_json::Value::String(req_str.to_string()))
                                }
                            } else {
                                dependencies.push(dep);
                            }
                        }
                    }
                }
            }
        }

        let mut pkg = default_package_data();
        pkg.package_type = Some(Self::PACKAGE_TYPE.to_string());
        pkg.name = name;
        pkg.version = version;
        pkg.download_url = download_url;
        pkg.homepage_url = homepage_url;
        pkg.vcs_url = vcs_url;
        pkg.description = description;
        pkg.sha256 = sha256;
        pkg.extracted_license_statement = extracted_license_statement;
        pkg.dependencies = dependencies;
        if !extra_data.is_empty() {
            pkg.extra_data = Some(extra_data);
        }
        pkg
    }
}

/// Conda environment file (environment.yml, conda.yaml) parser.
///
/// Extracts dependencies from Conda environment files used to define reproducible
/// environments. Supports both Conda and pip dependencies, with channel specifications.
pub struct CondaEnvironmentYmlParser;

impl PackageParser for CondaEnvironmentYmlParser {
    const PACKAGE_TYPE: &'static str = "conda";

    fn is_match(path: &Path) -> bool {
        // Python reference: path_patterns = ('*conda*.yaml', '*env*.yaml', '*environment*.yaml')
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            (lower.contains("conda") || lower.contains("env") || lower.contains("environment"))
                && (lower.ends_with(".yaml") || lower.ends_with(".yml"))
        } else {
            false
        }
    }

    fn extract_package_data(path: &Path) -> PackageData {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read {}: {}", path.display(), e);
                return default_package_data();
            }
        };

        let yaml: Value = match serde_yaml::from_str(&contents) {
            Ok(y) => y,
            Err(e) => {
                warn!("Failed to parse YAML in {}: {}", path.display(), e);
                return default_package_data();
            }
        };

        let name = yaml.get("name").and_then(|v| v.as_str()).map(String::from);

        let dependencies = extract_environment_dependencies(&yaml);

        let mut extra_data = HashMap::new();
        if let Some(channels) = yaml.get("channels").and_then(|v| v.as_sequence()) {
            let channels_vec: Vec<String> = channels
                .iter()
                .filter_map(|c| c.as_str().map(String::from))
                .collect();
            if !channels_vec.is_empty() {
                extra_data.insert("channels".to_string(), serde_json::json!(channels_vec));
            }
        }

        // Environment files are private (not published packages)
        let mut pkg = default_package_data();
        pkg.package_type = Some(Self::PACKAGE_TYPE.to_string());
        pkg.name = name;
        pkg.primary_language = Some("Python".to_string());
        pkg.dependencies = dependencies;
        pkg.is_private = true;
        if !extra_data.is_empty() {
            pkg.extra_data = Some(extra_data);
        }
        pkg
    }
}

/// Extract Jinja2-style variables from a Conda meta.yaml
///
/// Example:
/// ```ignore
/// {% set version = "0.45.0" %}
/// {% set sha256 = "abc123..." %}
/// ```
pub fn extract_jinja2_variables(content: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("{%") && trimmed.ends_with("%}") && trimmed.contains('=') {
            // Strip {% and %}
            let inner = trimmed
                .trim_start_matches("{%")
                .trim_end_matches("%}")
                .trim()
                .trim_start_matches("set")
                .trim();

            // Split on '=' to get key and value
            if let Some((key, value)) = inner.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                variables.insert(key.to_string(), value.to_string());
            }
        }
    }

    variables
}

/// Apply Jinja2 variable substitutions to YAML content
///
/// Supports:
/// - `{{ variable }}` - Simple substitution
/// - `{{ variable|lower }}` - Lowercase filter
pub fn apply_jinja2_substitutions(content: &str, variables: &HashMap<String, String>) -> String {
    let mut result = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip Jinja2 set statements (already extracted)
        if trimmed.starts_with("{%") && trimmed.ends_with("%}") && trimmed.contains('=') {
            continue;
        }

        let mut processed_line = line.to_string();

        // Apply variable substitutions
        if line.contains("{{") && line.contains("}}") {
            for (var_name, var_value) in variables {
                // Handle |lower filter
                let pattern_lower = format!("{{{{ {}|lower }}}}", var_name);
                if processed_line.contains(&pattern_lower) {
                    processed_line =
                        processed_line.replace(&pattern_lower, &var_value.to_lowercase());
                }

                // Handle normal substitution
                let pattern_normal = format!("{{{{ {} }}}}", var_name);
                processed_line = processed_line.replace(&pattern_normal, var_value);
            }
        }

        // Skip lines with unresolved Jinja2 templates (complex expressions we can't handle)
        if processed_line.contains("{{") {
            continue;
        }

        result.push(processed_line);
    }

    result.join("\n")
}

/// Parse a Conda requirement string into a Dependency
///
/// Format examples:
/// - `mccortex ==1.0` - Pinned version with space before operator
/// - `python >=3.6` - Version constraint
/// - `conda-forge::numpy=1.15.4` - Namespace and pinned version (no space)
/// - `bwa` - No version specified
pub fn parse_conda_requirement(req: &str, scope: &str) -> Option<Dependency> {
    let req = req.trim();

    // Handle namespace prefix (conda-forge::package)
    let (namespace, req_without_ns) = if let Some((ns, rest)) = req.split_once("::") {
        (Some(ns), rest)
    } else {
        (None, req)
    };

    // Split on first space to separate name from version constraint
    let (name_part, version_constraint) =
        if let Some((name, constraint)) = req_without_ns.split_once(' ') {
            (name.trim(), Some(constraint.trim()))
        } else {
            (req_without_ns, None)
        };

    // Check for pinned version with `=` (no space): package=1.0
    let (name, version, is_pinned, extracted_requirement) = if name_part.contains('=') {
        let parts: Vec<&str> = name_part.splitn(2, '=').collect();
        let n = parts[0];
        let v = if parts.len() > 1 {
            Some(parts[1].to_string())
        } else {
            None
        };
        let req = v
            .as_ref()
            .map(|ver| format!("={}", ver))
            .unwrap_or_default();
        (n, v, true, Some(req))
    } else if let Some(constraint) = version_constraint {
        // Handle space-separated constraints: package >=3.6, package ==1.0
        let version_opt = if constraint.starts_with("==") {
            Some(constraint.trim_start_matches("==").trim().to_string())
        } else {
            None
        };
        (name_part, version_opt, false, Some(constraint.to_string()))
    } else {
        (name_part, None, false, None)
    };

    // Build PURL
    let purl = build_purl(
        "conda",
        namespace,
        name,
        version.as_deref(),
        None,
        None,
        None,
    );

    // Determine is_runtime and is_optional based on scope
    let (is_runtime, is_optional) = match scope {
        "run" => (true, false),
        _ => (false, true), // build, host, test are all optional
    };

    Some(Dependency {
        purl,
        extracted_requirement,
        scope: Some(scope.to_string()),
        is_runtime: Some(is_runtime),
        is_optional: Some(is_optional),
        is_pinned: Some(is_pinned),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}

fn extract_environment_dependencies(yaml: &Value) -> Vec<Dependency> {
    let dependencies = match yaml.get("dependencies").and_then(|v| v.as_sequence()) {
        Some(d) => d,
        None => return Vec::new(),
    };

    let mut deps = Vec::new();
    for dep_value in dependencies {
        if let Some(dep_str) = dep_value.as_str() {
            if let Some(dep) = parse_environment_string_dependency(dep_str) {
                deps.push(dep);
            }
        } else if let Some(pip_deps) = dep_value.get("pip").and_then(|v| v.as_sequence()) {
            deps.extend(extract_pip_dependencies(pip_deps));
        }
    }
    deps
}

fn parse_environment_string_dependency(dep_str: &str) -> Option<Dependency> {
    let (namespace, dep_without_ns) = parse_conda_namespace(dep_str);

    if namespace.is_none()
        && looks_like_pip_requirement(dep_without_ns)
        && let Ok(parsed_req) = dep_without_ns.parse::<pep508_rs::Requirement>()
    {
        return create_pip_dependency(parsed_req, "dependencies");
    }

    create_conda_dependency(namespace, dep_without_ns, "dependencies")
}

fn parse_conda_namespace(dep_str: &str) -> (Option<&str>, &str) {
    if let Some((ns, rest)) = dep_str.split_once("::") {
        if ns.contains('/') || ns.contains(':') {
            (None, dep_str)
        } else {
            (Some(ns), rest)
        }
    } else {
        (None, dep_str)
    }
}

fn looks_like_pip_requirement(dep_str: &str) -> bool {
    dep_str.contains(">=") || dep_str.contains("==") || dep_str.contains("~=")
}

fn create_conda_dependency(
    namespace: Option<&str>,
    dep_without_ns: &str,
    scope: &str,
) -> Option<Dependency> {
    let (name, version, is_pinned, extracted_requirement) =
        if let Some((n, v)) = dep_without_ns.split_once('=') {
            (
                n.trim(),
                Some(v.trim().to_string()),
                true,
                Some(format!("={}", v.trim())),
            )
        } else {
            (dep_without_ns.trim(), None, false, None)
        };

    if name == "pip" || name == "python" {
        return None;
    }

    let purl = build_purl(
        "conda",
        namespace,
        name,
        version.as_deref(),
        None,
        None,
        None,
    );
    Some(Dependency {
        purl,
        extracted_requirement,
        scope: Some(scope.to_string()),
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(is_pinned),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}

fn extract_pip_dependencies(pip_deps: &[Value]) -> Vec<Dependency> {
    pip_deps
        .iter()
        .filter_map(|pip_dep| {
            if let Some(pip_req_str) = pip_dep.as_str()
                && let Ok(parsed_req) = pip_req_str.parse::<pep508_rs::Requirement>()
            {
                create_pip_dependency(parsed_req, "dependencies")
            } else {
                None
            }
        })
        .collect()
}

fn create_pip_dependency(parsed_req: pep508_rs::Requirement, scope: &str) -> Option<Dependency> {
    let name = parsed_req.name.to_string();

    if name == "pip" || name == "python" {
        return None;
    }

    let specs = parsed_req.version_or_url.as_ref().map(|v| match v {
        pep508_rs::VersionOrUrl::VersionSpecifier(spec) => spec.to_string(),
        pep508_rs::VersionOrUrl::Url(url) => url.to_string(),
    });

    let version = specs.as_ref().and_then(|spec_str| {
        if spec_str.starts_with("==") {
            Some(spec_str.trim_start_matches("==").to_string())
        } else {
            None
        }
    });

    let is_pinned = specs.as_ref().map(|s| s.contains("==")).unwrap_or(false);
    let purl = build_purl("pypi", None, &name, version.as_deref(), None, None, None);

    Some(Dependency {
        purl,
        extracted_requirement: specs,
        scope: Some(scope.to_string()),
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(is_pinned),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}
