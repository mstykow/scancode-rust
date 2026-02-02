use crate::models::{Dependency, LicenseDetection, Match, PackageData, Party};
use log::warn;
use packageurl::PackageUrl;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::PackageParser;

const FIELD_NAME: &str = "name";
const FIELD_VERSION: &str = "version";
const FIELD_LICENSE: &str = "license";
const FIELD_LICENSES: &str = "licenses";
const FIELD_HOMEPAGE: &str = "homepage";
const FIELD_REPOSITORY: &str = "repository";
const FIELD_AUTHOR: &str = "author";
const FIELD_CONTRIBUTORS: &str = "contributors";
const FIELD_MAINTAINERS: &str = "maintainers";
const FIELD_DEPENDENCIES: &str = "dependencies";
const FIELD_DEV_DEPENDENCIES: &str = "devDependencies";

pub struct NpmParser;

impl PackageParser for NpmParser {
    const PACKAGE_TYPE: &'static str = "npm";

    fn extract_package_data(path: &Path) -> PackageData {
        let (json, field_lines) = match read_and_parse_json_with_lines(path) {
            Ok((json, lines)) => (json, lines),
            Err(e) => {
                warn!("Failed to read or parse package.json at {:?}: {}", path, e);
                return default_package_data();
            }
        };

        let name = json
            .get(FIELD_NAME)
            .and_then(|v| v.as_str())
            .map(String::from);
        let version = json
            .get(FIELD_VERSION)
            .and_then(|v| v.as_str())
            .map(String::from);
        let namespace = extract_namespace(&name);
        let license_detections = extract_license_info(&json, &field_lines);
        let dependencies = extract_dependencies(&json, false);
        let dev_dependencies = extract_dependencies(&json, true);
        let purl = create_package_url(&name, &version, &namespace);

        PackageData {
            package_type: Some(Self::PACKAGE_TYPE.to_string()),
            namespace,
            name,
            version,
            homepage_url: json
                .get(FIELD_HOMEPAGE)
                .and_then(|v| v.as_str())
                .map(String::from),
            download_url: extract_repository_url(&json),
            copyright: None, // Not typically present in package.json
            license_detections,
            dependencies: [dependencies, dev_dependencies].concat(),
            parties: extract_parties(&json),
            purl,
        }
    }

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "package.json")
    }
}

/// Reads and parses a JSON file while tracking line numbers of fields
fn read_and_parse_json_with_lines(path: &Path) -> Result<(Value, HashMap<String, usize>), String> {
    // Read the file line by line to track line numbers
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Error reading file: {}", e))?;

    // Parse the content as JSON
    let content = lines.join("\n");
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Track line numbers for each field in the JSON
    let mut field_lines = HashMap::new();
    for (line_num, line) in lines.iter().enumerate() {
        let line = line.trim();
        // Look for field names in the format: "field": value
        if let Some(field_name) = extract_field_name(line) {
            field_lines.insert(field_name, line_num + 1); // 1-based line numbers
        }
    }

    Ok((json, field_lines))
}

/// Extracts field name from a JSON line
fn extract_field_name(line: &str) -> Option<String> {
    // Simple regex-free parsing for field names
    let line = line.trim();
    if line.is_empty() || !line.starts_with("\"") {
        return None;
    }

    // Find the closing quote of the field name
    let mut chars = line.chars();
    chars.next(); // Skip opening quote

    let mut field_name = String::new();
    for c in chars {
        if c == '"' {
            break;
        }
        field_name.push(c);
    }

    if field_name.is_empty() {
        None
    } else {
        Some(field_name)
    }
}

fn extract_namespace(name: &Option<String>) -> Option<String> {
    name.as_ref().and_then(|n| {
        if n.starts_with('@') && n.contains('/') {
            // Handle scoped package (@namespace/name)
            Some(
                n.split('/')
                    .next()
                    .unwrap()
                    .trim_start_matches('@')
                    .to_string(),
            )
        } else if n.contains('/') {
            // Handle regular namespaced package (namespace/name)
            n.split('/').next().map(String::from)
        } else {
            None
        }
    })
}

fn create_package_url(
    name: &Option<String>,
    version: &Option<String>,
    _namespace: &Option<String>,
) -> Option<String> {
    name.as_ref().map(|name| {
        // Note: We extract and store namespace in PackageData for metadata purposes,
        // but cannot use it with PackageUrl library for scoped packages.
        //
        // The PackageURL spec requires scoped npm packages to be formatted as:
        //   pkg:npm/%40scope/package@version
        // where only the @ is encoded as %40, but the / remains unencoded.
        //
        // The PackageUrl library cannot produce this format:
        // - with_namespace("scope") produces: pkg:npm/scope/package (missing %40)
        // - with_namespace("%40scope") produces: pkg:npm/%2540scope/package (double-encoded)
        // - PackageUrl::new("npm", "@scope/package") produces: pkg:npm/%40scope%2Fpackage (encodes /)
        //
        // Therefore, we must manually construct the PURL for scoped packages.

        if name.starts_with('@') && name.contains('/') {
            // Manual construction for scoped packages
            let encoded_name = name.replace('@', "%40");
            let version_part = version
                .as_ref()
                .map(|v| format!("@{}", v))
                .unwrap_or_default();
            format!("pkg:npm/{}{}", encoded_name, version_part)
        } else {
            // Use PackageUrl library for non-scoped packages
            let mut package_url = PackageUrl::new(NpmParser::PACKAGE_TYPE, name)
                .expect("Failed to create PackageUrl");
            if let Some(v) = version {
                package_url.with_version(v).expect("Failed to set version");
            }
            package_url.to_string()
        }
    })
}

fn extract_license_info(
    json: &Value,
    field_lines: &HashMap<String, usize>,
) -> Vec<LicenseDetection> {
    let mut detections = Vec::new();

    // Check for string license field
    if let Some(license_str) = json.get(FIELD_LICENSE).and_then(|v| v.as_str()) {
        let line = field_lines.get(FIELD_LICENSE).copied().unwrap_or(0);
        detections.push(LicenseDetection {
            license_expression: license_str.to_string(),
            matches: vec![Match {
                score: 100.0,
                start_line: line,
                end_line: line,
                license_expression: license_str.to_string(),
                rule_identifier: None,
                matched_text: None,
            }],
        });
        return detections;
    }

    // Check for license object
    if let Some(license_obj) = json.get(FIELD_LICENSE).and_then(|v| v.as_object())
        && let Some(license_type) = license_obj.get("type").and_then(|v| v.as_str())
    {
        let line = field_lines.get(FIELD_LICENSE).copied().unwrap_or(0);
        detections.push(LicenseDetection {
            license_expression: license_type.to_string(),
            matches: vec![Match {
                score: 100.0,
                start_line: line,
                end_line: line,
                license_expression: license_type.to_string(),
                rule_identifier: None,
                matched_text: None,
            }],
        });
        return detections;
    }

    // Check for deprecated licenses array
    if let Some(licenses) = json.get(FIELD_LICENSES).and_then(|v| v.as_array()) {
        let base_line = field_lines.get(FIELD_LICENSES).copied().unwrap_or(0);
        for (index, license) in licenses.iter().enumerate() {
            if let Some(license_type) = license.get("type").and_then(|v| v.as_str()) {
                detections.push(LicenseDetection {
                    license_expression: license_type.to_string(),
                    matches: vec![Match {
                        score: 100.0,
                        start_line: base_line + index,
                        end_line: base_line + index,
                        license_expression: license_type.to_string(),
                        rule_identifier: None,
                        matched_text: None,
                    }],
                });
            }
        }
    }

    detections
}

fn extract_repository_url(json: &Value) -> Option<String> {
    match json.get(FIELD_REPOSITORY) {
        Some(Value::String(url)) => Some(normalize_repo_url(url)),
        Some(Value::Object(obj)) => obj
            .get("url")
            .and_then(|u| u.as_str())
            .map(normalize_repo_url),
        _ => None,
    }
}

/// Normalizes repository URLs by converting various formats to a standard HTTPS URL.
fn normalize_repo_url(url: &str) -> String {
    let url = url.trim();

    if url.starts_with("git://") {
        return url.replace("git://", "https://");
    } else if url.starts_with("git+https://") {
        return url.replace("git+https://", "https://");
    } else if url.starts_with("git@github.com:") {
        return url.replace("git@github.com:", "https://github.com/");
    }

    url.to_string()
}

/// Extracts party information (emails) from the `author`, `contributors`, and `maintainers` fields.
fn extract_parties(json: &Value) -> Vec<Party> {
    let mut parties = Vec::new();

    // Extract author field
    if let Some(author) = json.get(FIELD_AUTHOR)
        && let Some(email) = extract_email_from_field(author)
    {
        parties.push(Party { email });
    }

    // Extract contributors field
    if let Some(contributors) = json.get(FIELD_CONTRIBUTORS)
        && let Some(emails) = extract_emails_from_array(contributors)
    {
        parties.extend(emails.into_iter().map(|email| Party { email }));
    }

    // Extract maintainers field
    if let Some(maintainers) = json.get(FIELD_MAINTAINERS)
        && let Some(emails) = extract_emails_from_array(maintainers)
    {
        parties.extend(emails.into_iter().map(|email| Party { email }));
    }

    parties
}

/// Extracts email from a string in the format "Name <email@example.com>".
fn extract_email_from_string(author_str: &str) -> Option<String> {
    if let Some(email_start) = author_str.find('<')
        && let Some(email_end) = author_str.find('>')
        && email_start < email_end
    {
        return Some(author_str[email_start + 1..email_end].to_string());
    }
    None
}

/// Extracts a single email from a JSON field, which can be a string or an object with an "email" field.
fn extract_email_from_field(field: &Value) -> Option<String> {
    match field {
        Value::String(s) => extract_email_from_string(s).or_else(|| Some(s.clone())),
        Value::Object(obj) => obj.get("email").and_then(|v| v.as_str()).map(String::from),
        _ => None,
    }
}

/// Extracts multiple emails from a JSON array, where each element can be a string or an object with an "email" field.
fn extract_emails_from_array(array: &Value) -> Option<Vec<String>> {
    if let Value::Array(items) = array {
        let emails = items
            .iter()
            .filter_map(extract_email_from_field)
            .collect::<Vec<_>>();
        if !emails.is_empty() {
            return Some(emails);
        }
    }
    None
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

/// Extracts dependencies from the `dependencies` or `devDependencies` field in the JSON.
fn extract_dependencies(json: &Value, is_optional: bool) -> Vec<Dependency> {
    let field = if is_optional {
        FIELD_DEV_DEPENDENCIES
    } else {
        FIELD_DEPENDENCIES
    };

    json.get(field)
        .and_then(|deps| deps.as_object())
        .map_or_else(Vec::new, |deps| {
            deps.iter()
                .filter_map(|(name, version)| {
                    let version_str = version.as_str()?;
                    let stripped_version = strip_version_modifier(version_str);
                    let encoded_version = urlencoding::encode(&stripped_version).to_string();

                    let mut package_url = PackageUrl::new(NpmParser::PACKAGE_TYPE, name).ok()?;
                    package_url.with_version(&encoded_version).ok()?;

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
        })
}

/// Strips version modifiers (e.g., ~, ^, >=) from a version string.
fn strip_version_modifier(version: &str) -> String {
    version.trim_start_matches(['~', '^', '>', '=']).to_string()
}
