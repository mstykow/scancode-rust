use crate::models::{Dependency, LicenseDetection, Match, PackageData, Party};
use log::warn;
use packageurl::PackageUrl;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml::Value as TomlValue;
use toml::map::Map as TomlMap;

use super::PackageParser;

// Field constants for pyproject.toml
const FIELD_PROJECT: &str = "project";
const FIELD_NAME: &str = "name";
const FIELD_VERSION: &str = "version";
const FIELD_LICENSE: &str = "license";
const FIELD_AUTHORS: &str = "authors";
const FIELD_MAINTAINERS: &str = "maintainers";
const FIELD_URLS: &str = "urls";
const FIELD_HOMEPAGE: &str = "homepage";
const FIELD_REPOSITORY: &str = "repository";
const FIELD_DEPENDENCIES: &str = "dependencies";
const FIELD_OPTIONAL_DEPENDENCIES: &str = "optional-dependencies";

pub struct PythonParser;

impl PackageParser for PythonParser {
    const PACKAGE_TYPE: &'static str = "pypi";

    fn extract_package_data(path: &Path) -> PackageData {
        if path.file_name().unwrap_or_default() == "pyproject.toml" {
            extract_from_pyproject_toml(path)
        } else if path.file_name().unwrap_or_default() == "setup.py" {
            extract_from_setup_py(path)
        } else {
            default_package_data()
        }
    }

    fn is_match(path: &Path) -> bool {
        if let Some(filename) = path.file_name() {
            filename == "pyproject.toml" || filename == "setup.py"
        } else {
            false
        }
    }
}

fn extract_from_pyproject_toml(path: &Path) -> PackageData {
    let toml_content = match read_toml_file(path) {
        Ok(content) => content,
        Err(e) => {
            warn!(
                "Failed to read or parse pyproject.toml at {:?}: {}",
                path, e
            );
            return default_package_data();
        }
    };

    // Handle both PEP 621 (project table) and poetry formats
    let project_table =
        if let Some(project) = toml_content.get(FIELD_PROJECT).and_then(|v| v.as_table()) {
            // Standard PEP 621 format with [project] table
            project.clone()
        } else if toml_content.get(FIELD_NAME).is_some() {
            // Poetry or other format with top-level fields
            match toml_content.as_table() {
                Some(table) => table.clone(),
                None => {
                    warn!("Failed to convert TOML content to table in {:?}", path);
                    return default_package_data();
                }
            }
        } else {
            warn!("No project data found in pyproject.toml at {:?}", path);
            return default_package_data();
        };

    let name = project_table
        .get(FIELD_NAME)
        .and_then(|v| v.as_str())
        .map(String::from);

    let version = project_table
        .get(FIELD_VERSION)
        .and_then(|v| v.as_str())
        .map(String::from);

    let license_detections = extract_license_info(&project_table);

    // URLs can be in different formats depending on the tool (poetry, flit, etc.)
    let (homepage_url, repository_url) = extract_urls(&project_table);

    let (dependencies, optional_dependencies) = extract_dependencies(&project_table);

    // Create package URL
    let purl = name.as_ref().map(|n| {
        let mut package_url =
            PackageUrl::new(PythonParser::PACKAGE_TYPE, n).expect("Failed to create PackageUrl");

        if let Some(v) = &version {
            package_url.with_version(v).expect("Failed to set version");
        }

        package_url.to_string()
    });

    PackageData {
        package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
        namespace: None, // Python doesn't typically use namespaces like npm
        name,
        version,
        homepage_url,
        download_url: repository_url,
        copyright: None,
        license_detections,
        dependencies: [dependencies, optional_dependencies].concat(),
        parties: extract_parties(&project_table),
        purl,
    }
}

fn extract_license_info(project: &TomlMap<String, TomlValue>) -> Vec<LicenseDetection> {
    let mut detections = Vec::new();

    // Different projects might specify license in various ways
    if let Some(license_value) = project.get(FIELD_LICENSE) {
        match license_value {
            TomlValue::String(license_str) => {
                detections.push(create_license_detection(license_str));
            }
            TomlValue::Table(license_table) => {
                if let Some(text) = license_table.get("text").and_then(|v| v.as_str()) {
                    detections.push(create_license_detection(text));
                }
                if let Some(expr) = license_table.get("expression").and_then(|v| v.as_str()) {
                    detections.push(create_license_detection(expr));
                }
            }
            _ => {}
        }
    }

    detections
}

fn create_license_detection(license_str: &str) -> LicenseDetection {
    LicenseDetection {
        license_expression: license_str.to_string(),
        matches: vec![Match {
            score: 100.0,
            start_line: 0, // We don't track exact line numbers with the toml parser
            end_line: 0,
            license_expression: license_str.to_string(),
            rule_identifier: None,
            matched_text: None,
        }],
    }
}

fn extract_urls(project: &TomlMap<String, TomlValue>) -> (Option<String>, Option<String>) {
    let mut homepage_url = None;
    let mut repository_url = None;

    // Check for URLs table
    if let Some(urls) = project.get(FIELD_URLS).and_then(|v| v.as_table()) {
        homepage_url = urls
            .get(FIELD_HOMEPAGE)
            .and_then(|v| v.as_str())
            .map(String::from);
        repository_url = urls
            .get(FIELD_REPOSITORY)
            .and_then(|v| v.as_str())
            .map(String::from);
    }

    // If not found in URLs table, check for top-level keys
    if homepage_url.is_none() {
        homepage_url = project
            .get(FIELD_HOMEPAGE)
            .and_then(|v| v.as_str())
            .map(String::from);
    }

    if repository_url.is_none() {
        repository_url = project
            .get(FIELD_REPOSITORY)
            .and_then(|v| v.as_str())
            .map(String::from);
    }

    (homepage_url, repository_url)
}

fn extract_parties(project: &TomlMap<String, TomlValue>) -> Vec<Party> {
    let mut parties = Vec::new();

    // Extract authors
    if let Some(authors) = project.get(FIELD_AUTHORS).and_then(|v| v.as_array()) {
        for author in authors {
            if let Some(author_str) = author.as_str()
                && let Some(email) = extract_email_from_author_string(author_str)
            {
                parties.push(Party { email })
            }
        }
    }

    // Extract maintainers
    if let Some(maintainers) = project.get(FIELD_MAINTAINERS).and_then(|v| v.as_array()) {
        for maintainer in maintainers {
            if let Some(maintainer_str) = maintainer.as_str()
                && let Some(email) = extract_email_from_author_string(maintainer_str)
            {
                parties.push(Party { email })
            }
        }
    }

    parties
}

fn extract_email_from_author_string(author_str: &str) -> Option<String> {
    // Look for email addresses in the format: "Name <email@example.com>"
    if let Some(email_start) = author_str.find('<')
        && let Some(email_end) = author_str.find('>')
        && email_start < email_end
    {
        return Some(author_str[email_start + 1..email_end].to_string());
    }

    None
}

fn extract_dependencies(
    project: &TomlMap<String, TomlValue>,
) -> (Vec<Dependency>, Vec<Dependency>) {
    let mut dependencies = Vec::new();
    let mut optional_dependencies = Vec::new();

    // Handle dependencies - can be array or table format
    if let Some(deps_value) = project.get(FIELD_DEPENDENCIES) {
        match deps_value {
            TomlValue::Array(arr) => {
                dependencies = parse_dependency_array(arr, false);
            }
            TomlValue::Table(table) => {
                dependencies = parse_dependency_table(table, false);
            }
            _ => {}
        }
    }

    // Handle optional dependencies
    if let Some(opt_deps_table) = project
        .get(FIELD_OPTIONAL_DEPENDENCIES)
        .and_then(|v| v.as_table())
    {
        for (_feature, deps) in opt_deps_table {
            match deps {
                TomlValue::Array(arr) => {
                    optional_dependencies.extend(parse_dependency_array(arr, true));
                }
                TomlValue::Table(table) => {
                    optional_dependencies.extend(parse_dependency_table(table, true));
                }
                _ => {}
            }
        }
    }

    (dependencies, optional_dependencies)
}

fn parse_dependency_table(
    table: &TomlMap<String, TomlValue>,
    is_optional: bool,
) -> Vec<Dependency> {
    table
        .iter()
        .filter_map(|(name, version)| {
            // Create version string if present
            let version_str = version.as_str().map(|s| s.to_string());
            // Create package URL with name
            let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, name).ok()?;

            // Add version if present
            if let Some(v) = &version_str {
                package_url.with_version(v).ok()?;
            }

            Some(Dependency {
                purl: Some(package_url.to_string()),
                extracted_requirement: None,
                scope: None,
                is_runtime: None,
                is_optional: Some(is_optional),
                is_pinned: None,
                is_direct: None,
                resolved_package: None,
            })
        })
        .collect()
}

fn parse_dependency_array(array: &[TomlValue], is_optional: bool) -> Vec<Dependency> {
    array
        .iter()
        .filter_map(|dep| {
            let dep_str = dep.as_str()?;

            // Basic parsing of PEP 508 dependency specifications
            // For example "requests>=2.0.0", "django==3.2.1", "flask"
            let mut parts = dep_str.split(['>', '=', '<', '~']);
            let name = parts.next()?.trim().to_string();

            // Extract version if present
            let version = parts.next().map(|v| v.trim().to_string());

            let mut package_url = match PackageUrl::new(PythonParser::PACKAGE_TYPE, &name) {
                Ok(purl) => purl,
                Err(_) => return None,
            };

            if let Some(ref v) = version {
                package_url.with_version(v).ok()?;
            }

            Some(Dependency {
                purl: Some(package_url.to_string()),
                extracted_requirement: None,
                scope: None,
                is_runtime: None,
                is_optional: Some(is_optional),
                is_pinned: None,
                is_direct: None,
                resolved_package: None,
            })
        })
        .collect()
}

fn extract_from_setup_py(path: &Path) -> PackageData {
    // For setup.py, we do a simple text-based extraction since parsing Python
    // would be much more complex. This is a basic implementation that could
    // be improved in the future.
    let content = match read_file_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            warn!("Failed to read setup.py at {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let name = extract_setup_value(&content, "name");
    let version = extract_setup_value(&content, "version");
    let license_expression = extract_setup_value(&content, "license");

    // Create license detection if we found a license
    let license_detections = license_expression.as_ref().map_or(Vec::new(), |license| {
        vec![LicenseDetection {
            license_expression: license.clone(),
            matches: vec![Match {
                score: 100.0,
                start_line: 0, // We don't track exact line numbers
                end_line: 0,
                license_expression: license.clone(),
                rule_identifier: None,
                matched_text: None,
            }],
        }]
    });

    // Create package URL
    let purl = name.as_ref().map(|n| {
        let mut package_url =
            PackageUrl::new(PythonParser::PACKAGE_TYPE, n).expect("Failed to create PackageUrl");

        if let Some(v) = &version {
            package_url.with_version(v).expect("Failed to set version");
        }

        package_url.to_string()
    });

    PackageData {
        package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version,
        homepage_url: extract_setup_value(&content, "url"),
        download_url: None,
        copyright: None,
        license_detections,
        dependencies: Vec::new(), // For setup.py, parsing dependencies reliably is challenging
        parties: Vec::new(),      // Same for authors without a proper parser
        purl,
    }
}

fn extract_setup_value(content: &str, key: &str) -> Option<String> {
    // This is a very basic parser that looks for patterns like:
    // name="package_name", or name = "package_name"
    let patterns = vec![
        format!("{}=\"", key),   // name="value"
        format!("{} =\"", key),  // name ="value"
        format!("{}= \"", key),  // name= "value"
        format!("{} = \"", key), // name = "value"
        format!("{}='", key),    // name='value'
        format!("{} ='", key),   // name ='value'
        format!("{}= '", key),   // name= 'value'
        format!("{} = '", key),  // name = 'value'
    ];

    for pattern in patterns {
        if let Some(start_idx) = content.find(&pattern) {
            let value_start = start_idx + pattern.len();
            let remaining = &content[value_start..];

            // Find closing quote
            if let Some(end_idx) = remaining.find(['"', '\'']) {
                return Some(remaining[..end_idx].to_string());
            }
        }
    }

    None
}

/// Reads and parses a TOML file
fn read_toml_file(path: &Path) -> Result<TomlValue, String> {
    let content = read_file_to_string(path)?;
    toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))
}

fn read_file_to_string(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Error reading file: {}", e))?;
    Ok(content)
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: None,
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
