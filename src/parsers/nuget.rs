//! Parser for NuGet package manifests and configuration files.
//!
//! Extracts package metadata and dependencies from .NET/NuGet ecosystem files:
//! - packages.config (legacy .NET Framework format)
//! - .nuspec (NuGet package specification)
//! - packages.lock.json (NuGet lock file)
//! - .nupkg (NuGet package archive — metadata extraction)
//!
//! # Supported Formats
//! - packages.config (XML)
//! - *.nuspec (XML)
//! - packages.lock.json (JSON)
//! - *.nupkg (ZIP archive with .nuspec inside)
//!
//! # Key Features
//! - Dependency extraction with targetFramework support
//! - Dependency groups by framework version
//! - Package URL (purl) generation
//!
//! # Implementation Notes
//! - Uses quick-xml for XML parsing
//! - Graceful error handling with warn!()
//! - No unwrap/expect in library code

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use log::warn;
use packageurl::PackageUrl;
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{DatasourceId, Dependency, PackageData, PackageType, Party};

use super::PackageParser;

fn build_nuget_description(
    summary: Option<&str>,
    description: Option<&str>,
    title: Option<&str>,
    name: Option<&str>,
) -> Option<String> {
    let summary = summary.map(|s| s.trim()).filter(|s| !s.is_empty());
    let description = description.map(|s| s.trim()).filter(|s| !s.is_empty());
    let title = title.map(|s| s.trim()).filter(|s| !s.is_empty());

    let mut result = match (summary, description) {
        (None, None) => return None,
        (Some(s), None) => s.to_string(),
        (None, Some(d)) => d.to_string(),
        (Some(s), Some(d)) => {
            if d.contains(s) {
                d.to_string()
            } else {
                format!("{}\n{}", s, d)
            }
        }
    };

    if let Some(t) = title {
        if let Some(n) = name {
            if t != n {
                result = format!("{}\n{}", t, result);
            }
        } else {
            result = format!("{}\n{}", t, result);
        }
    }

    Some(result)
}

/// Parser for packages.config (legacy .NET Framework format)
pub struct PackagesConfigParser;

impl PackageParser for PackagesConfigParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "packages.config")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open packages.config at {:?}: {}", path, e);
                return vec![default_package_data(Some(
                    DatasourceId::NugetPackagesConfig,
                ))];
            }
        };

        let reader = BufReader::new(file);
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

        let mut dependencies = Vec::new();
        let mut buf = Vec::new();

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e)) if e.name().as_ref() == b"package" => {
                    if let Some(dep) = parse_packages_config_package(&e) {
                        dependencies.push(dep);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!("Error parsing packages.config at {:?}: {}", path, e);
                    return vec![default_package_data(Some(
                        DatasourceId::NugetPackagesConfig,
                    ))];
                }
                _ => {}
            }
            buf.clear();
        }

        vec![PackageData {
            datasource_id: Some(DatasourceId::NugetPackagesConfig),
            package_type: Some(Self::PACKAGE_TYPE),
            dependencies,
            ..default_package_data(Some(DatasourceId::NugetPackagesConfig))
        }]
    }
}

/// Parser for .nuspec files (NuGet package specification)
pub struct NuspecParser;

impl PackageParser for NuspecParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "nuspec")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open .nuspec at {:?}: {}", path, e);
                return vec![default_package_data(Some(DatasourceId::NugetNuspec))];
            }
        };

        let reader = BufReader::new(file);
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

        let mut name = None;
        let mut version = None;
        let mut summary = None;
        let mut description = None;
        let mut title = None;
        let mut homepage_url = None;
        let mut parties = Vec::new();
        let mut dependencies = Vec::new();
        let mut extracted_license_statement = None;
        let mut copyright = None;
        let mut vcs_url = None;

        let mut buf = Vec::new();
        let mut current_element = String::new();
        let mut in_metadata = false;
        let mut in_dependencies = false;
        let mut current_group_framework = None;

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_element = tag_name.clone();

                    if tag_name == "metadata" {
                        in_metadata = true;
                    } else if tag_name == "dependencies" && in_metadata {
                        in_dependencies = true;
                    } else if tag_name == "group" && in_dependencies {
                        current_group_framework = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| attr.key.as_ref() == b"targetFramework")
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                    } else if tag_name == "repository" && in_metadata {
                        let mut repo_type = None;
                        let mut repo_url = None;

                        for attr in e.attributes().filter_map(|a| a.ok()) {
                            match attr.key.as_ref() {
                                b"type" => {
                                    repo_type = String::from_utf8(attr.value.to_vec()).ok();
                                }
                                b"url" => {
                                    repo_url = String::from_utf8(attr.value.to_vec()).ok();
                                }
                                _ => {}
                            }
                        }

                        if let Some(url) = repo_url {
                            vcs_url = if let Some(vcs_type) = repo_type {
                                Some(format!("{}+{}", vcs_type, url))
                            } else {
                                Some(url)
                            };
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "dependency" && in_dependencies {
                        if let Some(dep) =
                            parse_nuspec_dependency(&e, current_group_framework.as_deref())
                        {
                            dependencies.push(dep);
                        }
                    } else if tag_name == "repository" && in_metadata {
                        let mut repo_type = None;
                        let mut repo_url = None;

                        for attr in e.attributes().filter_map(|a| a.ok()) {
                            match attr.key.as_ref() {
                                b"type" => {
                                    repo_type = String::from_utf8(attr.value.to_vec()).ok();
                                }
                                b"url" => {
                                    repo_url = String::from_utf8(attr.value.to_vec()).ok();
                                }
                                _ => {}
                            }
                        }

                        if let Some(url) = repo_url {
                            vcs_url = if let Some(vcs_type) = repo_type {
                                Some(format!("{}+{}", vcs_type, url))
                            } else {
                                Some(url)
                            };
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    if !in_metadata {
                        continue;
                    }

                    let text = e.decode().ok().map(|s| s.trim().to_string());
                    if let Some(text) = text.filter(|s| !s.is_empty()) {
                        match current_element.as_str() {
                            "id" => name = Some(text),
                            "version" => version = Some(text),
                            "summary" => summary = Some(text),
                            "description" => description = Some(text),
                            "title" => title = Some(text),
                            "projectUrl" => homepage_url = Some(text),
                            "authors" => {
                                parties.push(Party {
                                    r#type: None,
                                    role: Some("author".to_string()),
                                    name: Some(text),
                                    email: None,
                                    url: None,
                                    organization: None,
                                    organization_url: None,
                                    timezone: None,
                                });
                            }
                            "owners" => {
                                parties.push(Party {
                                    r#type: None,
                                    role: Some("owner".to_string()),
                                    name: Some(text),
                                    email: None,
                                    url: None,
                                    organization: None,
                                    organization_url: None,
                                    timezone: None,
                                });
                            }
                            "license" => {
                                extracted_license_statement = Some(text);
                            }
                            "licenseUrl" => {
                                if extracted_license_statement.is_none() {
                                    extracted_license_statement = Some(text);
                                }
                            }
                            "copyright" => copyright = Some(text),
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "metadata" {
                        in_metadata = false;
                    } else if tag_name == "dependencies" {
                        in_dependencies = false;
                    } else if tag_name == "group" {
                        current_group_framework = None;
                    }

                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!("Error parsing .nuspec at {:?}: {}", path, e);
                    return vec![default_package_data(Some(DatasourceId::NugetNuspec))];
                }
                _ => {}
            }
            buf.clear();
        }

        // Build description from summary, description, and title fields
        // Following Python ScanCode's build_description logic
        let final_description = build_nuget_description(
            summary.as_deref(),
            description.as_deref(),
            title.as_deref(),
            name.as_deref(),
        );

        // Build repository URLs
        let repository_homepage_url = name.as_ref().and_then(|n| {
            version
                .as_ref()
                .map(|v| format!("https://www.nuget.org/packages/{}/{}", n, v))
        });

        let repository_download_url = name.as_ref().and_then(|n| {
            version
                .as_ref()
                .map(|v| format!("https://www.nuget.org/api/v2/package/{}/{}", n, v))
        });

        let api_data_url = name.as_ref().and_then(|n| {
            version.as_ref().map(|v| {
                format!(
                    "https://api.nuget.org/v3/registration3/{}/{}.json",
                    n.to_lowercase(),
                    v
                )
            })
        });

        // Generate PURL
        let purl = name.as_ref().and_then(|n| {
            let mut package_url = PackageUrl::new("nuget", n).ok()?;
            if let Some(v) = &version {
                package_url.with_version(v).ok()?;
            }
            Some(package_url.to_string())
        });

        // Extract license statement only - detection happens in separate engine
        // Do NOT populate declared_license_expression or license_detections here
        let declared_license_expression = None;
        let declared_license_expression_spdx = None;
        let license_detections = Vec::new();

        let holder = None;

        vec![PackageData {
            datasource_id: Some(DatasourceId::NugetNuspec),
            package_type: Some(Self::PACKAGE_TYPE),
            name,
            version,
            purl,
            description: final_description,
            homepage_url,
            parties,
            dependencies,
            declared_license_expression,
            declared_license_expression_spdx,
            license_detections,
            extracted_license_statement,
            copyright,
            holder,
            vcs_url,
            repository_homepage_url,
            repository_download_url,
            api_data_url,
            ..default_package_data(Some(DatasourceId::NugetNuspec))
        }]
    }
}

fn parse_packages_config_package(element: &quick_xml::events::BytesStart) -> Option<Dependency> {
    let mut id = None;
    let mut version = None;
    let mut target_framework = None;

    for attr in element.attributes().filter_map(|a| a.ok()) {
        match attr.key.as_ref() {
            b"id" => id = String::from_utf8(attr.value.to_vec()).ok(),
            b"version" => version = String::from_utf8(attr.value.to_vec()).ok(),
            b"targetFramework" => target_framework = String::from_utf8(attr.value.to_vec()).ok(),
            _ => {}
        }
    }

    let name = id?;
    let purl = PackageUrl::new("nuget", &name).ok().map(|p| p.to_string());

    Some(Dependency {
        purl,
        extracted_requirement: version,
        scope: target_framework,
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(true),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}

fn parse_nuspec_dependency(
    element: &quick_xml::events::BytesStart,
    framework: Option<&str>,
) -> Option<Dependency> {
    let mut id = None;
    let mut version = None;
    let mut include = None;
    let mut exclude = None;

    for attr in element.attributes().filter_map(|a| a.ok()) {
        match attr.key.as_ref() {
            b"id" => id = String::from_utf8(attr.value.to_vec()).ok(),
            b"version" => version = String::from_utf8(attr.value.to_vec()).ok(),
            b"include" => include = String::from_utf8(attr.value.to_vec()).ok(),
            b"exclude" => exclude = String::from_utf8(attr.value.to_vec()).ok(),
            _ => {}
        }
    }

    let name = id?;
    let purl = PackageUrl::new("nuget", &name).ok().map(|p| p.to_string());

    let mut extra_data = serde_json::Map::new();
    if let Some(fw) = framework {
        extra_data.insert(
            "framework".to_string(),
            serde_json::Value::String(fw.to_string()),
        );
    }
    if let Some(inc) = include {
        extra_data.insert("include".to_string(), serde_json::Value::String(inc));
    }
    if let Some(exc) = exclude {
        extra_data.insert("exclude".to_string(), serde_json::Value::String(exc));
    }

    Some(Dependency {
        purl,
        extracted_requirement: version,
        scope: Some("dependency".to_string()),
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(false),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: if extra_data.is_empty() {
            None
        } else {
            Some(extra_data.into_iter().collect())
        },
    })
}

fn default_package_data(datasource_id: Option<DatasourceId>) -> PackageData {
    PackageData {
        package_type: Some(PackagesConfigParser::PACKAGE_TYPE),
        datasource_id,
        ..Default::default()
    }
}

const MAX_ARCHIVE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB
const MAX_COMPRESSION_RATIO: f64 = 100.0; // 100:1

/// Parser for packages.lock.json (NuGet lock file)
pub struct PackagesLockParser;

impl PackageParser for PackagesLockParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("packages.lock.json"))
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open packages.lock.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(DatasourceId::NugetPackagesLock))];
            }
        };

        let parsed: serde_json::Value = match serde_json::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse packages.lock.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(DatasourceId::NugetPackagesLock))];
            }
        };

        let mut dependencies = Vec::new();

        if let Some(deps_obj) = parsed.get("dependencies").and_then(|v| v.as_object()) {
            for (target_framework, packages) in deps_obj {
                if let Some(packages_obj) = packages.as_object() {
                    for (package_name, package_info) in packages_obj {
                        if let Some(info_obj) = package_info.as_object() {
                            let version = info_obj
                                .get("resolved")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let requested = info_obj
                                .get("requested")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let package_type = info_obj.get("type").and_then(|v| v.as_str());

                            let is_direct = match package_type {
                                Some("Direct") => Some(true),
                                Some("Transitive") => Some(false),
                                _ => None,
                            };

                            let purl = version.as_ref().and_then(|v| {
                                PackageUrl::new("nuget", package_name).ok().map(|mut p| {
                                    let _ = p.with_version(v);
                                    p.to_string()
                                })
                            });

                            let mut extra_data = serde_json::Map::new();
                            extra_data.insert(
                                "target_framework".to_string(),
                                serde_json::Value::String(target_framework.clone()),
                            );

                            if let Some(content_hash) =
                                info_obj.get("contentHash").and_then(|v| v.as_str())
                            {
                                extra_data.insert(
                                    "content_hash".to_string(),
                                    serde_json::Value::String(content_hash.to_string()),
                                );
                            }

                            dependencies.push(Dependency {
                                purl,
                                extracted_requirement: requested.or(version),
                                scope: Some(target_framework.clone()),
                                is_runtime: Some(true),
                                is_optional: Some(false),
                                is_pinned: Some(true),
                                is_direct,
                                resolved_package: None,
                                extra_data: if extra_data.is_empty() {
                                    None
                                } else {
                                    Some(extra_data.into_iter().collect())
                                },
                            });
                        }
                    }
                }
            }
        }

        vec![PackageData {
            datasource_id: Some(DatasourceId::NugetPackagesLock),
            package_type: Some(Self::PACKAGE_TYPE),
            dependencies,
            ..default_package_data(Some(DatasourceId::NugetPackagesLock))
        }]
    }
}

/// Parser for .nupkg files (NuGet package archives)
pub struct NupkgParser;

impl PackageParser for NupkgParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "nupkg")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        vec![match extract_nupkg_archive(path) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to extract .nupkg at {:?}: {}", path, e);
                default_package_data(Some(DatasourceId::NugetNupkg))
            }
        }]
    }
}

fn extract_nupkg_archive(path: &Path) -> Result<PackageData, String> {
    use std::fs;
    use std::io::Read;
    use zip::ZipArchive;

    let file_metadata =
        fs::metadata(path).map_err(|e| format!("Failed to read file metadata: {}", e))?;
    let archive_size = file_metadata.len();

    if archive_size > MAX_ARCHIVE_SIZE {
        return Err(format!(
            "Archive too large: {} bytes (limit: {} bytes)",
            archive_size, MAX_ARCHIVE_SIZE
        ));
    }

    let file = File::open(path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP archive: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;

        let entry_name = entry.name().to_string();

        if entry_name.ends_with(".nuspec") {
            let entry_size = entry.size();
            if entry_size > MAX_FILE_SIZE {
                return Err(format!(
                    ".nuspec too large: {} bytes (limit: {} bytes)",
                    entry_size, MAX_FILE_SIZE
                ));
            }

            let compressed_size = entry.compressed_size();
            if compressed_size > 0 {
                let ratio = entry_size as f64 / compressed_size as f64;
                if ratio > MAX_COMPRESSION_RATIO {
                    return Err(format!(
                        "Suspicious compression ratio: {:.2}:1 (limit: {:.0}:1)",
                        ratio, MAX_COMPRESSION_RATIO
                    ));
                }
            }

            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .map_err(|e| format!("Failed to read .nuspec: {}", e))?;

            return parse_nuspec_content(&content);
        }
    }

    Err("No .nuspec file found in archive".to_string())
}

fn parse_nuspec_content(content: &str) -> Result<PackageData, String> {
    use quick_xml::Reader;

    let mut xml_reader = Reader::from_str(content);
    xml_reader.config_mut().trim_text(true);

    let mut name = None;
    let mut version = None;
    let mut description = None;
    let mut homepage_url = None;
    let mut parties = Vec::new();
    let mut dependencies = Vec::new();
    let mut extracted_license_statement = None;
    let mut copyright = None;
    let mut vcs_url = None;

    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_metadata = false;
    let mut in_dependencies = false;
    let mut current_group_framework = None;

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_element = tag_name.clone();

                if tag_name == "metadata" {
                    in_metadata = true;
                } else if tag_name == "dependencies" && in_metadata {
                    in_dependencies = true;
                } else if tag_name == "group" && in_dependencies {
                    current_group_framework = e
                        .attributes()
                        .filter_map(|a| a.ok())
                        .find(|attr| attr.key.as_ref() == b"targetFramework")
                        .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                } else if tag_name == "repository" && in_metadata {
                    let mut repo_type = None;
                    let mut repo_url = None;

                    for attr in e.attributes().filter_map(|a| a.ok()) {
                        match attr.key.as_ref() {
                            b"type" => repo_type = String::from_utf8(attr.value.to_vec()).ok(),
                            b"url" => repo_url = String::from_utf8(attr.value.to_vec()).ok(),
                            _ => {}
                        }
                    }

                    if let Some(url) = repo_url {
                        vcs_url = if let Some(vcs_type) = repo_type {
                            Some(format!("{}+{}", vcs_type, url))
                        } else {
                            Some(url)
                        };
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "dependency" && in_dependencies {
                    if let Some(dep) =
                        parse_nuspec_dependency(&e, current_group_framework.as_deref())
                    {
                        dependencies.push(dep);
                    }
                } else if tag_name == "repository" && in_metadata {
                    let mut repo_type = None;
                    let mut repo_url = None;

                    for attr in e.attributes().filter_map(|a| a.ok()) {
                        match attr.key.as_ref() {
                            b"type" => repo_type = String::from_utf8(attr.value.to_vec()).ok(),
                            b"url" => repo_url = String::from_utf8(attr.value.to_vec()).ok(),
                            _ => {}
                        }
                    }

                    if let Some(url) = repo_url {
                        vcs_url = if let Some(vcs_type) = repo_type {
                            Some(format!("{}+{}", vcs_type, url))
                        } else {
                            Some(url)
                        };
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if !in_metadata {
                    continue;
                }

                let text = e.decode().ok().map(|s| s.trim().to_string());
                if let Some(text) = text.filter(|s| !s.is_empty()) {
                    match current_element.as_str() {
                        "id" => name = Some(text),
                        "version" => version = Some(text),
                        "description" => description = Some(text),
                        "projectUrl" => homepage_url = Some(text),
                        "authors" => {
                            parties.push(Party {
                                r#type: None,
                                role: Some("author".to_string()),
                                name: Some(text),
                                email: None,
                                url: None,
                                organization: None,
                                organization_url: None,
                                timezone: None,
                            });
                        }
                        "owners" => {
                            parties.push(Party {
                                r#type: None,
                                role: Some("owner".to_string()),
                                name: Some(text),
                                email: None,
                                url: None,
                                organization: None,
                                organization_url: None,
                                timezone: None,
                            });
                        }
                        "license" => {
                            extracted_license_statement = Some(text);
                        }
                        "licenseUrl" => {
                            if extracted_license_statement.is_none() {
                                extracted_license_statement = Some(text);
                            }
                        }
                        "copyright" => copyright = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "metadata" {
                    in_metadata = false;
                } else if tag_name == "dependencies" {
                    in_dependencies = false;
                } else if tag_name == "group" {
                    current_group_framework = None;
                }

                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!("XML parsing error: {}", e));
            }
            _ => {}
        }
        buf.clear();
    }

    let repository_homepage_url = name.as_ref().and_then(|n| {
        version
            .as_ref()
            .map(|v| format!("https://www.nuget.org/packages/{}/{}", n, v))
    });

    let repository_download_url = name.as_ref().and_then(|n| {
        version
            .as_ref()
            .map(|v| format!("https://www.nuget.org/api/v2/package/{}/{}", n, v))
    });

    let api_data_url = name.as_ref().and_then(|n| {
        version.as_ref().map(|v| {
            format!(
                "https://api.nuget.org/v3/registration3/{}/{}.json",
                n.to_lowercase(),
                v
            )
        })
    });

    // Extract license statement only - detection happens in separate engine
    // Do NOT populate declared_license_expression or license_detections here
    let declared_license_expression = None;
    let declared_license_expression_spdx = None;
    let license_detections = Vec::new();

    let holder = None;

    Ok(PackageData {
        datasource_id: Some(DatasourceId::NugetNupkg),
        package_type: Some(NupkgParser::PACKAGE_TYPE),
        name,
        version,
        description,
        homepage_url,
        parties,
        dependencies,
        declared_license_expression,
        declared_license_expression_spdx,
        license_detections,
        extracted_license_statement,
        copyright,
        holder,
        vcs_url,
        repository_homepage_url,
        repository_download_url,
        api_data_url,
        ..default_package_data(Some(DatasourceId::NugetNupkg))
    })
}

crate::register_parser!(
    ".NET packages.config manifest",
    &["**/packages.config"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/nuget/reference/packages-config"),
);

crate::register_parser!(
    ".NET .nuspec package specification",
    &["**/*.nuspec"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/nuget/reference/nuspec"),
);

crate::register_parser!(
    ".NET packages.lock.json lockfile",
    &["**/packages.lock.json"],
    "nuget",
    "C#",
    Some(
        "https://learn.microsoft.com/en-us/nuget/consume-packages/package-references-in-project-files#locking-dependencies"
    ),
);

crate::register_parser!(
    ".NET .nupkg package archive",
    &["**/*.nupkg"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/nuget/create-packages/creating-a-package"),
);
