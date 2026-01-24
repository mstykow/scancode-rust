use crate::models::{Dependency, LicenseDetection, Match, PackageData, Party};
use log::warn;
use packageurl::PackageUrl;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml::Value;

use super::PackageParser;

const FIELD_PACKAGE: &str = "package";
const FIELD_NAME: &str = "name";
const FIELD_VERSION: &str = "version";
const FIELD_LICENSE: &str = "license";
const FIELD_AUTHORS: &str = "authors";
const FIELD_REPOSITORY: &str = "repository";
const FIELD_HOMEPAGE: &str = "homepage";
const FIELD_DEPENDENCIES: &str = "dependencies";
const FIELD_DEV_DEPENDENCIES: &str = "dev-dependencies";

pub struct CargoParser;

impl PackageParser for CargoParser {
    const PACKAGE_TYPE: &'static str = "cargo";

    fn extract_package_data(path: &Path) -> PackageData {
        let toml_content = match read_cargo_toml(path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read or parse Cargo.toml at {:?}: {}", path, e);
                return default_package_data();
            }
        };

        let package = toml_content.get(FIELD_PACKAGE).and_then(|v| v.as_table());

        let name = package
            .and_then(|p| p.get(FIELD_NAME))
            .and_then(|v| v.as_str())
            .map(String::from);

        let version = package
            .and_then(|p| p.get(FIELD_VERSION))
            .and_then(|v| v.as_str())
            .map(String::from);

        let license_detections = extract_license_info(&toml_content);

        let dependencies = extract_dependencies(&toml_content, false);
        let dev_dependencies = extract_dependencies(&toml_content, true);

        let purl = create_package_url(&name, &version);

        PackageData {
            package_type: Some(Self::PACKAGE_TYPE.to_string()),
            namespace: None, // Cargo doesn't use namespaces like npm
            name,
            version,
            homepage_url: package
                .and_then(|p| p.get(FIELD_HOMEPAGE))
                .and_then(|v| v.as_str())
                .map(String::from),
            download_url: package
                .and_then(|p| p.get(FIELD_REPOSITORY))
                .and_then(|v| v.as_str())
                .map(String::from),
            copyright: None,
            license_detections,
            dependencies: [dependencies, dev_dependencies].concat(),
            parties: extract_parties(&toml_content),
            purl,
        }
    }

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "Cargo.toml")
    }
}

/// Reads and parses a TOML file
fn read_cargo_toml(path: &Path) -> Result<Value, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Error reading file: {}", e))?;

    toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))
}

fn create_package_url(name: &Option<String>, version: &Option<String>) -> Option<String> {
    name.as_ref().map(|name| {
        let mut package_url =
            PackageUrl::new(CargoParser::PACKAGE_TYPE, name).expect("Failed to create PackageUrl");

        if let Some(v) = version {
            package_url.with_version(v);
        }

        package_url.to_string()
    })
}

fn extract_license_info(toml_content: &Value) -> Vec<LicenseDetection> {
    let mut detections = Vec::new();

    // Check for license field within the package table
    if let Some(package) = toml_content.get(FIELD_PACKAGE).and_then(|v| v.as_table())
        && let Some(license_str) = package.get(FIELD_LICENSE).and_then(|v| v.as_str())
    {
        detections.push(LicenseDetection {
            license_expression: license_str.to_string(),
            matches: vec![Match {
                score: 100.0,
                start_line: 0, // We don't track exact line numbers with the toml parser
                end_line: 0,
                license_expression: license_str.to_string(),
                rule_identifier: None,
                matched_text: None,
            }],
        });
    }

    detections
}

/// Extracts party information from the `authors` field
fn extract_parties(toml_content: &Value) -> Vec<Party> {
    let mut parties = Vec::new();

    if let Some(package) = toml_content.get(FIELD_PACKAGE).and_then(|v| v.as_table())
        && let Some(authors) = package.get(FIELD_AUTHORS).and_then(|v| v.as_array())
    {
        for author in authors {
            if let Some(author_str) = author.as_str() {
                // Look for email addresses in the format: "Name <email@example.com>"
                if let Some(email_start) = author_str.find('<')
                    && let Some(email_end) = author_str.find('>')
                    && email_start < email_end
                {
                    let email = &author_str[email_start + 1..email_end];
                    parties.push(Party {
                        email: email.to_string(),
                    });
                }
            }
        }
    }

    parties
}

fn extract_dependencies(toml_content: &Value, is_optional: bool) -> Vec<Dependency> {
    let field = if is_optional {
        FIELD_DEV_DEPENDENCIES
    } else {
        FIELD_DEPENDENCIES
    };

    let mut dependencies = Vec::new();

    if let Some(deps_table) = toml_content.get(field).and_then(|v| v.as_table()) {
        for (name, value) in deps_table {
            let version = match value {
                Value::String(version_str) => Some(version_str.to_string()),
                Value::Table(table) => table
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                _ => None,
            };

            if let Some(version) = version {
                let mut package_url = PackageUrl::new(CargoParser::PACKAGE_TYPE, name)
                    .expect("Failed to create PackageUrl");
                package_url.with_version(&version);

                dependencies.push(Dependency {
                    purl: Some(package_url.to_string()),
                    scope: None,
                    is_optional,
                });
            }
        }
    }

    dependencies
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
