use crate::models::{Dependency, LicenseDetection, Match, PackageData, Party};
use log::warn;
use packageurl::PackageUrl;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::PackageParser;

const POM_NS: &str = "http://maven.apache.org/POM/4.0.0";

pub struct MavenParser;

impl PackageParser for MavenParser {
    const PACKAGE_TYPE: &'static str = "maven";

    fn extract_package_data(path: &Path) -> PackageData {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open pom.xml at {:?}: {}", path, e);
                return default_package_data();
            }
        };

        let mut reader = Reader::from_reader(BufReader::new(file));
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut package_data = default_package_data();
        package_data.package_type = Some(Self::PACKAGE_TYPE.to_string());
        
        let mut current_element = Vec::new();
        let mut in_dependencies = false;
        let mut current_dependency: Option<Dependency> = None;
        let mut license_line = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let element_name = e.name();
                    current_element.push(element_name.as_ref().to_vec());
                    
                    match element_name.as_ref() {
                        b"dependencies" => in_dependencies = true,
                        b"dependency" if in_dependencies => {
                            current_dependency = Some(Dependency {
                                purl: None,
                                scope: None,
                                is_optional: false,
                            });
                        }
                        b"license" => {
                            license_line = reader.buffer_position() as usize;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();
                    let current_path = current_element.last().map(|v| v.as_slice());

                    if let Some(dep) = &mut current_dependency {
                        match current_path {
                            Some(b"groupId") => dep.scope = Some(text),
                            Some(b"artifactId") => {
                                if let Some(group_id) = &dep.scope {
                                    dep.scope = Some(format!("{}:{}", group_id, text));
                                }
                            }
                            Some(b"version") => {
                                if let Some(scope) = &dep.scope {
                                    if let Some((group_id, artifact_id)) = scope.split_once(':') {
                                        dep.purl = Some(format!(
                                            "pkg:maven/{}/{}@{}",
                                            group_id, artifact_id, text
                                        ));
                                    }
                                }
                            }
                            Some(b"scope") => dep.is_optional = text == "test",
                            Some(b"optional") => dep.is_optional = text == "true",
                            _ => {}
                        }
                    } else {
                        match current_path {
                            Some(b"groupId") => package_data.namespace = Some(text),
                            Some(b"artifactId") => package_data.name = Some(text),
                            Some(b"version") => package_data.version = Some(text),
                            Some(b"url") => package_data.homepage_url = Some(text),
                            Some(b"name") if current_element.ends_with(&[b"license".to_vec()]) => {
                                package_data.license_detections = extract_license_info(&text, license_line);
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    if !current_element.is_empty() {
                        current_element.pop();
                    }
                    
                    match e.name().as_ref() {
                        b"dependencies" => in_dependencies = false,
                        b"dependency" => {
                            if let Some(dep) = current_dependency.take() {
                                package_data.dependencies.push(dep);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!("Error parsing pom.xml at {:?}: {}", path, e);
                    return package_data;
                }
                _ => {}
            }
            buf.clear();
        }

        // Construct PURL from parsed data
        if let (Some(group_id), Some(artifact_id), Some(version)) = (
            &package_data.namespace,
            &package_data.name,
            &package_data.version,
        ) {
            package_data.purl = PackageUrl::new("maven", &format!("{}/{}", group_id, artifact_id))
                .and_then(|mut purl| {
                    purl.with_version(version);
                    Ok(purl.to_string())
                })
                .ok();
        }

        package_data
    }

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map_or(false, |name| name == "pom.xml")
    }
}

fn extract_license_info(license_text: &str, line: usize) -> Vec<LicenseDetection> {
    let mut detections = Vec::new();
    detections.push(LicenseDetection {
        license_expression: license_text.to_string(),
        matches: vec![Match {
            score: 100.0,
            start_line: line,
            end_line: line,
            license_expression: license_text.to_string(),
            rule_identifier: None,
            matched_text: None,
        }],
    });
    detections
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