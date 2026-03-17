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
use std::io::{BufReader, Read};
use std::path::Path;

use log::warn;
use packageurl::PackageUrl;
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{DatasourceId, Dependency, PackageData, PackageType, Party};

use super::PackageParser;

const PROJECT_FILE_EXTENSIONS: [&str; 3] = ["csproj", "vbproj", "fsproj"];

#[derive(Default)]
struct RepositoryMetadata {
    vcs_url: Option<String>,
    branch: Option<String>,
    commit: Option<String>,
}

fn build_nuget_party(role: &str, name: String) -> Party {
    Party {
        r#type: Some("person".to_string()),
        role: Some(role.to_string()),
        name: Some(name),
        email: None,
        url: None,
        organization: None,
        organization_url: None,
        timezone: None,
    }
}

fn insert_extra_string(
    extra_data: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        extra_data.insert(key.to_string(), serde_json::Value::String(value));
    }
}

fn parse_repository_metadata(element: &quick_xml::events::BytesStart) -> RepositoryMetadata {
    let mut repo_type = None;
    let mut repo_url = None;
    let mut branch = None;
    let mut commit = None;

    for attr in element.attributes().filter_map(|a| a.ok()) {
        match attr.key.as_ref() {
            b"type" => repo_type = String::from_utf8(attr.value.to_vec()).ok(),
            b"url" => repo_url = String::from_utf8(attr.value.to_vec()).ok(),
            b"branch" => branch = String::from_utf8(attr.value.to_vec()).ok(),
            b"commit" => commit = String::from_utf8(attr.value.to_vec()).ok(),
            _ => {}
        }
    }

    RepositoryMetadata {
        vcs_url: repo_url.map(|url| match repo_type {
            Some(vcs_type) if !vcs_type.trim().is_empty() => format!("{}+{}", vcs_type, url),
            _ => url,
        }),
        branch,
        commit,
    }
}

fn build_nuget_urls(
    name: Option<&str>,
    version: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    let repository_homepage_url = name.and_then(|name| {
        version.map(|version| format!("https://www.nuget.org/packages/{}/{}", name, version))
    });

    let repository_download_url = name.and_then(|name| {
        version.map(|version| format!("https://www.nuget.org/api/v2/package/{}/{}", name, version))
    });

    let api_data_url = name.and_then(|name| {
        version.map(|version| {
            format!(
                "https://api.nuget.org/v3/registration3/{}/{}.json",
                name.to_lowercase(),
                version
            )
        })
    });

    (
        repository_homepage_url,
        repository_download_url,
        api_data_url,
    )
}

fn build_nuget_purl(name: Option<&str>, version: Option<&str>) -> Option<String> {
    let name = name?;
    let mut package_url = PackageUrl::new("nuget", name).ok()?;

    if let Some(version) = version {
        package_url.with_version(version).ok()?;
    }

    Some(package_url.to_string())
}

fn project_file_datasource_id(path: &Path) -> Option<DatasourceId> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("csproj") => Some(DatasourceId::NugetCsproj),
        Some("vbproj") => Some(DatasourceId::NugetVbproj),
        Some("fsproj") => Some(DatasourceId::NugetFsproj),
        _ => None,
    }
}

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
        let mut license_type = None;
        let mut copyright = None;
        let mut vcs_url = None;
        let mut repository_branch = None;
        let mut repository_commit = None;

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
                        let repository = parse_repository_metadata(&e);
                        vcs_url = repository.vcs_url;
                        repository_branch = repository.branch;
                        repository_commit = repository.commit;
                    } else if tag_name == "license" && in_metadata {
                        license_type = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| attr.key.as_ref() == b"type")
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
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
                        let repository = parse_repository_metadata(&e);
                        vcs_url = repository.vcs_url;
                        repository_branch = repository.branch;
                        repository_commit = repository.commit;
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
                                parties.push(build_nuget_party("author", text));
                            }
                            "owners" => {
                                parties.push(build_nuget_party("owner", text));
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

        let (repository_homepage_url, repository_download_url, api_data_url) =
            build_nuget_urls(name.as_deref(), version.as_deref());

        let purl = build_nuget_purl(name.as_deref(), version.as_deref());

        // Extract license statement only - detection happens in separate engine
        // Do NOT populate declared_license_expression or license_detections here
        let declared_license_expression = None;
        let declared_license_expression_spdx = None;
        let license_detections = Vec::new();

        let holder = None;

        let mut extra_data = serde_json::Map::new();
        insert_extra_string(&mut extra_data, "license_type", license_type.clone());
        if license_type.as_deref() == Some("file") {
            insert_extra_string(
                &mut extra_data,
                "license_file",
                extracted_license_statement.clone(),
            );
        }
        insert_extra_string(&mut extra_data, "repository_branch", repository_branch);
        insert_extra_string(&mut extra_data, "repository_commit", repository_commit);

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
            extra_data: if extra_data.is_empty() {
                None
            } else {
                Some(extra_data.into_iter().collect())
            },
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

pub struct DotNetDepsJsonParser;

impl PackageParser for DotNetDepsJsonParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".deps.json"))
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to open .deps.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(DatasourceId::NugetDepsJson))];
            }
        };

        let parsed: serde_json::Value = match serde_json::from_reader(file) {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to parse .deps.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(DatasourceId::NugetDepsJson))];
            }
        };

        vec![parse_dotnet_deps_json(&parsed, path)]
    }
}

fn parse_dotnet_deps_json(parsed: &serde_json::Value, path: &Path) -> PackageData {
    let Some(libraries) = parsed.get("libraries").and_then(|value| value.as_object()) else {
        return default_package_data(Some(DatasourceId::NugetDepsJson));
    };

    let Some((selected_target_name, selected_target)) = select_deps_target(parsed) else {
        return default_package_data(Some(DatasourceId::NugetDepsJson));
    };

    let root_key = select_root_library_key(path, libraries, &selected_target);
    let root_dependencies = root_key
        .as_deref()
        .and_then(|root_key| selected_target.get(root_key))
        .and_then(|value| value.get("dependencies"))
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();

    let mut dependencies = Vec::new();
    for (library_key, target_entry) in &selected_target {
        if root_key.as_deref() == Some(library_key.as_str()) {
            continue;
        }

        let Some((name, version)) = split_library_key(library_key) else {
            continue;
        };
        let Some(library_metadata) = libraries
            .get(library_key)
            .and_then(|value| value.as_object())
        else {
            continue;
        };

        let mut extra_data = serde_json::Map::new();
        extra_data.insert(
            "target_name".to_string(),
            serde_json::Value::String(selected_target_name.clone()),
        );

        for field in [
            "type",
            "sha512",
            "path",
            "hashPath",
            "runtimeStoreManifestName",
        ] {
            if let Some(value) = library_metadata.get(field) {
                extra_data.insert(field.to_string(), value.clone());
            }
        }

        if let Some(value) = library_metadata.get("serviceable") {
            extra_data.insert("serviceable".to_string(), value.clone());
        }

        if let Some(object) = target_entry.as_object() {
            for field in ["runtime", "native", "runtimeTargets", "resources"] {
                if let Some(value) = object.get(field) {
                    extra_data.insert(field.to_string(), value.clone());
                }
            }
            if let Some(value) = object.get("compileOnly") {
                extra_data.insert("compileOnly".to_string(), value.clone());
            }
        }

        let is_direct = if root_key.is_some() {
            Some(root_dependencies.contains_key(name))
        } else {
            None
        };

        let compile_only = target_entry
            .get("compileOnly")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        dependencies.push(Dependency {
            purl: build_nuget_purl(Some(name), Some(version)),
            extracted_requirement: Some(version.to_string()),
            scope: Some(selected_target_name.clone()),
            is_runtime: Some(!compile_only),
            is_optional: Some(compile_only),
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

    let mut package_data = if let Some(root_key) = root_key {
        let (name, version) = split_library_key(&root_key).unwrap_or(("", ""));
        let mut package = default_package_data(Some(DatasourceId::NugetDepsJson));
        package.name = (!name.is_empty()).then(|| name.to_string());
        package.version = (!version.is_empty()).then(|| version.to_string());
        package.purl = build_nuget_purl(package.name.as_deref(), package.version.as_deref());
        let (repository_homepage_url, repository_download_url, api_data_url) =
            build_nuget_urls(package.name.as_deref(), package.version.as_deref());
        package.repository_homepage_url = repository_homepage_url;
        package.repository_download_url = repository_download_url;
        package.api_data_url = api_data_url;
        package
    } else {
        let mut package = default_package_data(Some(DatasourceId::NugetDepsJson));
        let file_stem = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".deps.json"))
            .filter(|name| !name.trim().is_empty())
            .map(|name| name.to_string());
        package.name = file_stem.clone();
        package.purl = build_nuget_purl(file_stem.as_deref(), None);
        package
    };

    let mut extra_data = serde_json::Map::new();
    if let Some(runtime_target) = parsed
        .get("runtimeTarget")
        .and_then(|value| value.as_object())
    {
        if let Some(name) = runtime_target.get("name").and_then(|value| value.as_str()) {
            extra_data.insert(
                "runtime_target_name".to_string(),
                serde_json::Value::String(name.to_string()),
            );
            if let Some((framework, runtime_identifier)) = name.split_once('/') {
                extra_data.insert(
                    "target_framework".to_string(),
                    serde_json::Value::String(framework.to_string()),
                );
                extra_data.insert(
                    "runtime_identifier".to_string(),
                    serde_json::Value::String(runtime_identifier.to_string()),
                );
            } else {
                extra_data.insert(
                    "target_framework".to_string(),
                    serde_json::Value::String(name.to_string()),
                );
            }
        }
        if let Some(signature) = runtime_target.get("signature") {
            extra_data.insert("runtime_signature".to_string(), signature.clone());
        }
    } else {
        extra_data.insert(
            "target_name".to_string(),
            serde_json::Value::String(selected_target_name.clone()),
        );
        if let Some((framework, runtime_identifier)) = selected_target_name.split_once('/') {
            extra_data.insert(
                "target_framework".to_string(),
                serde_json::Value::String(framework.to_string()),
            );
            extra_data.insert(
                "runtime_identifier".to_string(),
                serde_json::Value::String(runtime_identifier.to_string()),
            );
        } else {
            extra_data.insert(
                "target_framework".to_string(),
                serde_json::Value::String(selected_target_name.clone()),
            );
        }
    }

    package_data.dependencies = dependencies;
    package_data.extra_data = if extra_data.is_empty() {
        None
    } else {
        Some(extra_data.into_iter().collect())
    };
    package_data
}

fn select_deps_target(
    parsed: &serde_json::Value,
) -> Option<(String, serde_json::Map<String, serde_json::Value>)> {
    let targets = parsed.get("targets")?.as_object()?;

    if let Some(runtime_target_name) = parsed
        .get("runtimeTarget")
        .and_then(|value| value.get("name"))
        .and_then(|value| value.as_str())
        && let Some(target) = targets
            .get(runtime_target_name)
            .and_then(|value| value.as_object())
    {
        return Some((runtime_target_name.to_string(), target.clone()));
    }

    if let Some((name, value)) = targets
        .iter()
        .find(|(name, value)| name.contains('/') && value.is_object())
        && let Some(target) = value.as_object()
    {
        return Some((name.clone(), target.clone()));
    }

    targets.iter().find_map(|(name, value)| {
        value
            .as_object()
            .map(|target| (name.clone(), target.clone()))
    })
}

fn select_root_library_key(
    path: &Path,
    libraries: &serde_json::Map<String, serde_json::Value>,
    target: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    let base_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".deps.json"));

    let project_keys: Vec<String> = target
        .keys()
        .filter(|key| {
            libraries
                .get(*key)
                .and_then(|value| value.get("type"))
                .and_then(|value| value.as_str())
                == Some("project")
        })
        .cloned()
        .collect();

    if let Some(base_name) = base_name
        && let Some(matched) = project_keys.iter().find(|key| {
            split_library_key(key)
                .map(|(name, _)| name.eq_ignore_ascii_case(base_name))
                .unwrap_or(false)
        })
    {
        return Some(matched.clone());
    }

    project_keys.into_iter().next()
}

fn split_library_key(key: &str) -> Option<(&str, &str)> {
    key.rsplit_once('/')
}

#[derive(Default)]
struct ProjectReferenceData {
    name: Option<String>,
    version: Option<String>,
    version_override: Option<String>,
    condition: Option<String>,
}

pub struct ProjectJsonParser;

impl PackageParser for ProjectJsonParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "project.json")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to open project.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(DatasourceId::NugetProjectJson))];
            }
        };

        let parsed: serde_json::Value = match serde_json::from_reader(file) {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to parse project.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(DatasourceId::NugetProjectJson))];
            }
        };

        vec![parse_project_json_manifest(&parsed)]
    }
}

pub struct ProjectLockJsonParser;

impl PackageParser for ProjectLockJsonParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "project.lock.json")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to open project.lock.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(
                    DatasourceId::NugetProjectLockJson,
                ))];
            }
        };

        let parsed: serde_json::Value = match serde_json::from_reader(file) {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to parse project.lock.json at {:?}: {}", path, e);
                return vec![default_package_data(Some(
                    DatasourceId::NugetProjectLockJson,
                ))];
            }
        };

        vec![parse_project_lock_manifest(&parsed)]
    }
}

pub struct PackageReferenceProjectParser;

impl PackageParser for PackageReferenceProjectParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| PROJECT_FILE_EXTENSIONS.contains(&ext))
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let Some(datasource_id) = project_file_datasource_id(path) else {
            return vec![default_package_data(None)];
        };

        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to open project file at {:?}: {}", path, e);
                return vec![default_package_data(Some(datasource_id))];
            }
        };

        let reader = BufReader::new(file);
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

        let mut name = None;
        let mut fallback_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_string());
        let mut version = None;
        let mut description = None;
        let mut homepage_url = None;
        let mut authors = None;
        let mut repository_url = None;
        let mut repository_type = None;
        let mut repository_branch = None;
        let mut repository_commit = None;
        let mut extracted_license_statement = None;
        let mut license_type = None;
        let mut copyright = None;
        let mut readme_file = None;
        let mut icon_file = None;
        let mut dependencies = Vec::new();

        let mut buf = Vec::new();
        let mut current_element = String::new();
        let mut in_property_group = false;
        let mut current_item_group_condition = None;
        let mut current_package_reference: Option<ProjectReferenceData> = None;

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_element = tag_name.clone();

                    match tag_name.as_str() {
                        "PropertyGroup" => in_property_group = true,
                        "ItemGroup" => {
                            current_item_group_condition = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| attr.key.as_ref() == b"Condition")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                        }
                        "PackageReference" => {
                            let name = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| matches!(attr.key.as_ref(), b"Include" | b"Update"))
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                            let version = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| attr.key.as_ref() == b"Version")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                            let version_override = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| attr.key.as_ref() == b"VersionOverride")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                            let condition = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| attr.key.as_ref() == b"Condition")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .or_else(|| current_item_group_condition.clone());

                            current_package_reference = Some(ProjectReferenceData {
                                name,
                                version,
                                version_override,
                                condition,
                            });
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "PackageReference" {
                        let name = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| matches!(attr.key.as_ref(), b"Include" | b"Update"))
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                        let version = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| attr.key.as_ref() == b"Version")
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                        let version_override = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| attr.key.as_ref() == b"VersionOverride")
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                        let condition = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| attr.key.as_ref() == b"Condition")
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                            .or_else(|| current_item_group_condition.clone());

                        if let Some(dependency) = build_project_file_dependency(
                            name,
                            version,
                            version_override,
                            condition,
                        ) {
                            dependencies.push(dependency);
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.decode().ok().map(|s| s.trim().to_string());
                    let Some(text) = text.filter(|value| !value.is_empty()) else {
                        buf.clear();
                        continue;
                    };

                    if current_package_reference.is_some() {
                        if current_element.as_str() == "Version"
                            && let Some(reference) = &mut current_package_reference
                        {
                            reference.version = Some(text);
                        } else if current_element.as_str() == "VersionOverride"
                            && let Some(reference) = &mut current_package_reference
                        {
                            reference.version_override = Some(text);
                        }
                    } else if in_property_group {
                        match current_element.as_str() {
                            "PackageId" => name = Some(text),
                            "AssemblyName" if fallback_name.is_none() => fallback_name = Some(text),
                            "Version" if version.is_none() => version = Some(text),
                            "PackageVersion" => version = Some(text),
                            "Description" => description = Some(text),
                            "PackageProjectUrl" | "ProjectUrl" => homepage_url = Some(text),
                            "Authors" => authors = Some(text),
                            "RepositoryUrl" => repository_url = Some(text),
                            "RepositoryType" => repository_type = Some(text),
                            "RepositoryBranch" => repository_branch = Some(text),
                            "RepositoryCommit" => repository_commit = Some(text),
                            "PackageLicenseExpression" => {
                                extracted_license_statement = Some(text);
                                license_type = Some("expression".to_string());
                            }
                            "PackageLicenseFile" => {
                                extracted_license_statement = Some(text);
                                license_type = Some("file".to_string());
                            }
                            "PackageReadmeFile" => readme_file = Some(text),
                            "PackageIcon" => icon_file = Some(text),
                            "Copyright" => copyright = Some(text),
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "PropertyGroup" => in_property_group = false,
                        "ItemGroup" => current_item_group_condition = None,
                        "PackageReference" => {
                            if let Some(reference) = current_package_reference.take()
                                && let Some(dependency) = build_project_file_dependency(
                                    reference.name,
                                    reference.version,
                                    reference.version_override,
                                    reference.condition,
                                )
                            {
                                dependencies.push(dependency);
                            }
                        }
                        _ => {}
                    }

                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!("Error parsing project file at {:?}: {}", path, e);
                    return vec![default_package_data(Some(datasource_id))];
                }
                _ => {}
            }

            buf.clear();
        }

        let name = name.or(fallback_name);
        let vcs_url = repository_url.map(|url| match repository_type {
            Some(repo_type) if !repo_type.trim().is_empty() => format!("{}+{}", repo_type, url),
            _ => url,
        });
        let (repository_homepage_url, repository_download_url, api_data_url) =
            build_nuget_urls(name.as_deref(), version.as_deref());

        let mut parties = Vec::new();
        if let Some(authors) = authors {
            parties.push(build_nuget_party("author", authors));
        }

        let mut extra_data = serde_json::Map::new();
        insert_extra_string(&mut extra_data, "license_type", license_type.clone());
        if license_type.as_deref() == Some("file") {
            insert_extra_string(
                &mut extra_data,
                "license_file",
                extracted_license_statement.clone(),
            );
        }
        insert_extra_string(&mut extra_data, "repository_branch", repository_branch);
        insert_extra_string(&mut extra_data, "repository_commit", repository_commit);
        insert_extra_string(&mut extra_data, "readme_file", readme_file);
        insert_extra_string(&mut extra_data, "icon_file", icon_file);

        vec![PackageData {
            datasource_id: Some(datasource_id),
            package_type: Some(Self::PACKAGE_TYPE),
            name: name.clone(),
            version: version.clone(),
            purl: build_nuget_purl(name.as_deref(), version.as_deref()),
            description,
            homepage_url,
            parties,
            dependencies,
            extracted_license_statement,
            copyright,
            vcs_url,
            extra_data: if extra_data.is_empty() {
                None
            } else {
                Some(extra_data.into_iter().collect())
            },
            repository_homepage_url,
            repository_download_url,
            api_data_url,
            ..default_package_data(Some(datasource_id))
        }]
    }
}

fn parse_project_json_manifest(parsed: &serde_json::Value) -> PackageData {
    let name = parsed
        .get("name")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let version = parsed
        .get("version")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let description = parsed
        .get("description")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let homepage_url = parsed
        .get("projectUrl")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let extracted_license_statement = parsed
        .get("license")
        .or_else(|| parsed.get("licenseUrl"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    let mut parties = Vec::new();
    if let Some(authors) = parsed.get("authors") {
        let author_name = if let Some(value) = authors.as_str() {
            Some(value.to_string())
        } else {
            authors.as_array().map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        };

        if let Some(author_name) = author_name.filter(|value| !value.is_empty()) {
            parties.push(build_nuget_party("author", author_name));
        }
    }

    let mut dependencies = Vec::new();

    if let Some(root_dependencies) = parsed
        .get("dependencies")
        .and_then(|value| value.as_object())
    {
        for (dependency_name, dependency_spec) in root_dependencies {
            if let Some(dependency) =
                parse_project_json_dependency(dependency_name, dependency_spec, None)
            {
                dependencies.push(dependency);
            }
        }
    }

    if let Some(frameworks) = parsed.get("frameworks").and_then(|value| value.as_object()) {
        for (framework, framework_value) in frameworks {
            let Some(framework_dependencies) = framework_value
                .get("dependencies")
                .and_then(|value| value.as_object())
            else {
                continue;
            };

            for (dependency_name, dependency_spec) in framework_dependencies {
                if let Some(dependency) = parse_project_json_dependency(
                    dependency_name,
                    dependency_spec,
                    Some(framework.clone()),
                ) {
                    dependencies.push(dependency);
                }
            }
        }
    }

    let (repository_homepage_url, repository_download_url, api_data_url) =
        build_nuget_urls(name.as_deref(), version.as_deref());

    PackageData {
        datasource_id: Some(DatasourceId::NugetProjectJson),
        package_type: Some(PackageType::Nuget),
        name: name.clone(),
        version: version.clone(),
        purl: build_nuget_purl(name.as_deref(), version.as_deref()),
        description,
        homepage_url,
        parties,
        dependencies,
        extracted_license_statement,
        repository_homepage_url,
        repository_download_url,
        api_data_url,
        ..default_package_data(Some(DatasourceId::NugetProjectJson))
    }
}

fn parse_project_json_dependency(
    dependency_name: &str,
    dependency_spec: &serde_json::Value,
    scope: Option<String>,
) -> Option<Dependency> {
    let mut extra_data = serde_json::Map::new();

    let requirement = match dependency_spec {
        serde_json::Value::String(version) => Some(version.clone()),
        serde_json::Value::Object(object) => {
            let requirement = object
                .get("version")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            insert_extra_string(
                &mut extra_data,
                "include",
                object
                    .get("include")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string()),
            );
            insert_extra_string(
                &mut extra_data,
                "exclude",
                object
                    .get("exclude")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string()),
            );
            insert_extra_string(
                &mut extra_data,
                "type",
                object
                    .get("type")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string()),
            );
            requirement
        }
        _ => return None,
    };

    Some(Dependency {
        purl: build_nuget_purl(Some(dependency_name), None),
        extracted_requirement: requirement,
        scope,
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

fn parse_project_lock_manifest(parsed: &serde_json::Value) -> PackageData {
    let mut dependencies = Vec::new();

    if let Some(groups) = parsed
        .get("projectFileDependencyGroups")
        .and_then(|value| value.as_object())
    {
        for (framework, entries) in groups {
            let Some(entries) = entries.as_array() else {
                continue;
            };

            for entry in entries.iter().filter_map(|value| value.as_str()) {
                if let Some(dependency) = parse_project_lock_dependency(
                    entry,
                    (!framework.is_empty()).then(|| framework.clone()),
                ) {
                    dependencies.push(dependency);
                }
            }
        }
    }

    PackageData {
        datasource_id: Some(DatasourceId::NugetProjectLockJson),
        package_type: Some(PackageType::Nuget),
        dependencies,
        ..default_package_data(Some(DatasourceId::NugetProjectLockJson))
    }
}

fn parse_project_lock_dependency(entry: &str, scope: Option<String>) -> Option<Dependency> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let name = parts.next()?;
    let requirement = parts.collect::<Vec<_>>().join(" ");

    Some(Dependency {
        purl: build_nuget_purl(Some(name), None),
        extracted_requirement: (!requirement.is_empty()).then_some(requirement),
        scope,
        is_runtime: Some(true),
        is_optional: Some(false),
        is_pinned: Some(false),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}

fn build_project_file_dependency(
    name: Option<String>,
    version: Option<String>,
    version_override: Option<String>,
    condition: Option<String>,
) -> Option<Dependency> {
    let name = name?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mut extra_data = serde_json::Map::new();
    insert_extra_string(&mut extra_data, "condition", condition);
    insert_extra_string(&mut extra_data, "version_override", version_override);

    Some(Dependency {
        purl: build_nuget_purl(Some(&name), None),
        extracted_requirement: version,
        scope: None,
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

#[derive(Default)]
struct CentralPackageVersionData {
    name: Option<String>,
    version: Option<String>,
    condition: Option<String>,
}

fn build_directory_packages_dependency(
    name: Option<String>,
    version: Option<String>,
    condition: Option<String>,
) -> Option<Dependency> {
    let name = name?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mut extra_data = serde_json::Map::new();
    insert_extra_string(&mut extra_data, "condition", condition);

    Some(Dependency {
        purl: build_nuget_purl(Some(&name), None),
        extracted_requirement: version,
        scope: Some("package_version".to_string()),
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

pub struct CentralPackageManagementPropsParser;

impl PackageParser for CentralPackageManagementPropsParser {
    const PACKAGE_TYPE: PackageType = PackageType::Nuget;

    fn is_match(path: &Path) -> bool {
        path.file_name().and_then(|name| name.to_str()) == Some("Directory.Packages.props")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                warn!(
                    "Failed to open Directory.Packages.props at {:?}: {}",
                    path, e
                );
                return vec![default_package_data(Some(
                    DatasourceId::NugetDirectoryPackagesProps,
                ))];
            }
        };

        let reader = BufReader::new(file);
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

        let mut dependencies = Vec::new();
        let mut buf = Vec::new();
        let mut current_element = String::new();
        let mut current_item_group_condition = None;
        let mut current_package_version: Option<CentralPackageVersionData> = None;
        let mut manage_package_versions_centrally = None;
        let mut central_package_transitive_pinning_enabled = None;
        let mut central_package_version_override_enabled = None;

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_element = tag_name.clone();

                    match tag_name.as_str() {
                        "ItemGroup" => {
                            current_item_group_condition = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| attr.key.as_ref() == b"Condition")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                        }
                        "PackageVersion" => {
                            let name = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| matches!(attr.key.as_ref(), b"Include" | b"Update"))
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                            let version = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| attr.key.as_ref() == b"Version")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                            let condition = e
                                .attributes()
                                .filter_map(|a| a.ok())
                                .find(|attr| attr.key.as_ref() == b"Condition")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .or_else(|| current_item_group_condition.clone());

                            current_package_version = Some(CentralPackageVersionData {
                                name,
                                version,
                                condition,
                            });
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "PackageVersion" {
                        let name = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| matches!(attr.key.as_ref(), b"Include" | b"Update"))
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                        let version = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| attr.key.as_ref() == b"Version")
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                        let condition = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|attr| attr.key.as_ref() == b"Condition")
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                            .or_else(|| current_item_group_condition.clone());

                        if let Some(dependency) =
                            build_directory_packages_dependency(name, version, condition)
                        {
                            dependencies.push(dependency);
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.decode().ok().map(|s| s.trim().to_string());
                    let Some(text) = text.filter(|value| !value.is_empty()) else {
                        buf.clear();
                        continue;
                    };

                    if current_package_version.is_some() {
                        if current_element.as_str() == "Version"
                            && let Some(entry) = &mut current_package_version
                        {
                            entry.version = Some(text);
                        }
                    } else {
                        match current_element.as_str() {
                            "ManagePackageVersionsCentrally" => {
                                manage_package_versions_centrally =
                                    Some(text.eq_ignore_ascii_case("true"))
                            }
                            "CentralPackageTransitivePinningEnabled" => {
                                central_package_transitive_pinning_enabled =
                                    Some(text.eq_ignore_ascii_case("true"))
                            }
                            "CentralPackageVersionOverrideEnabled" => {
                                central_package_version_override_enabled =
                                    Some(text.eq_ignore_ascii_case("true"))
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "ItemGroup" => current_item_group_condition = None,
                        "PackageVersion" => {
                            if let Some(entry) = current_package_version.take()
                                && let Some(dependency) = build_directory_packages_dependency(
                                    entry.name,
                                    entry.version,
                                    entry.condition,
                                )
                            {
                                dependencies.push(dependency);
                            }
                        }
                        _ => {}
                    }

                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!(
                        "Error parsing Directory.Packages.props at {:?}: {}",
                        path, e
                    );
                    return vec![default_package_data(Some(
                        DatasourceId::NugetDirectoryPackagesProps,
                    ))];
                }
                _ => {}
            }

            buf.clear();
        }

        let mut extra_data = serde_json::Map::new();
        if let Some(value) = manage_package_versions_centrally {
            extra_data.insert(
                "manage_package_versions_centrally".to_string(),
                serde_json::Value::Bool(value),
            );
        }
        if let Some(value) = central_package_transitive_pinning_enabled {
            extra_data.insert(
                "central_package_transitive_pinning_enabled".to_string(),
                serde_json::Value::Bool(value),
            );
        }
        if let Some(value) = central_package_version_override_enabled {
            extra_data.insert(
                "central_package_version_override_enabled".to_string(),
                serde_json::Value::Bool(value),
            );
        }

        vec![PackageData {
            datasource_id: Some(DatasourceId::NugetDirectoryPackagesProps),
            package_type: Some(Self::PACKAGE_TYPE),
            dependencies,
            extra_data: if extra_data.is_empty() {
                None
            } else {
                Some(extra_data.into_iter().collect())
            },
            ..default_package_data(Some(DatasourceId::NugetDirectoryPackagesProps))
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
        let content = {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;

            let entry_name = entry.name().to_string();
            if !entry_name.ends_with(".nuspec") {
                continue;
            }

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
            content
        };

        let mut package_data = parse_nuspec_content(&content)?;

        let license_file = package_data.extra_data.as_ref().and_then(|extra| {
            extra
                .get("license_file")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        });

        if let Some(license_file) = license_file
            && let Some(license_text) = read_nupkg_license_file(&mut archive, &license_file)?
        {
            package_data.extracted_license_statement = Some(license_text);
        }

        return Ok(package_data);
    }

    Err("No .nuspec file found in archive".to_string())
}

fn read_nupkg_license_file(
    archive: &mut zip::ZipArchive<File>,
    license_file: &str,
) -> Result<Option<String>, String> {
    let normalized_target = license_file.replace('\\', "/");

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;
        let entry_name = entry.name().replace('\\', "/");

        if entry_name != normalized_target
            && !entry_name.ends_with(&format!("/{}", normalized_target))
        {
            continue;
        }

        let entry_size = entry.size();
        if entry_size > MAX_FILE_SIZE {
            return Err(format!(
                "License file too large: {} bytes (limit: {} bytes)",
                entry_size, MAX_FILE_SIZE
            ));
        }

        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|e| format!("Failed to read license file from archive: {}", e))?;

        return Ok(Some(String::from_utf8_lossy(&content).to_string()));
    }

    Ok(None)
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
    let mut license_type = None;
    let mut copyright = None;
    let mut vcs_url = None;
    let mut repository_branch = None;
    let mut repository_commit = None;

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
                    let repository = parse_repository_metadata(&e);
                    vcs_url = repository.vcs_url;
                    repository_branch = repository.branch;
                    repository_commit = repository.commit;
                } else if tag_name == "license" && in_metadata {
                    license_type = e
                        .attributes()
                        .filter_map(|a| a.ok())
                        .find(|attr| attr.key.as_ref() == b"type")
                        .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
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
                    let repository = parse_repository_metadata(&e);
                    vcs_url = repository.vcs_url;
                    repository_branch = repository.branch;
                    repository_commit = repository.commit;
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
                            parties.push(build_nuget_party("author", text));
                        }
                        "owners" => {
                            parties.push(build_nuget_party("owner", text));
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

    let (repository_homepage_url, repository_download_url, api_data_url) =
        build_nuget_urls(name.as_deref(), version.as_deref());

    // Extract license statement only - detection happens in separate engine
    // Do NOT populate declared_license_expression or license_detections here
    let declared_license_expression = None;
    let declared_license_expression_spdx = None;
    let license_detections = Vec::new();

    let holder = None;

    let mut extra_data = serde_json::Map::new();
    insert_extra_string(&mut extra_data, "license_type", license_type.clone());
    if license_type.as_deref() == Some("file") {
        insert_extra_string(
            &mut extra_data,
            "license_file",
            extracted_license_statement.clone(),
        );
    }
    insert_extra_string(&mut extra_data, "repository_branch", repository_branch);
    insert_extra_string(&mut extra_data, "repository_commit", repository_commit);

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
        extra_data: if extra_data.is_empty() {
            None
        } else {
            Some(extra_data.into_iter().collect())
        },
        repository_homepage_url,
        repository_download_url,
        api_data_url,
        ..default_package_data(Some(DatasourceId::NugetNupkg))
    })
}

crate::register_parser!(
    ".NET Directory.Packages.props central package management manifest",
    &["**/Directory.Packages.props"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/nuget/consume-packages/central-package-management"),
);

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
    ".NET project.json manifest",
    &["**/project.json"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/nuget/archive/project-json"),
);

crate::register_parser!(
    ".NET project.lock.json lockfile",
    &["**/project.lock.json"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/nuget/archive/project-json"),
);

crate::register_parser!(
    ".NET .deps.json runtime dependency graph",
    &["**/*.deps.json"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/dotnet/core/dependency-loading/default-probing"),
);

crate::register_parser!(
    ".NET PackageReference C# project file",
    &["**/*.csproj"],
    "nuget",
    "C#",
    Some(
        "https://learn.microsoft.com/en-us/nuget/consume-packages/package-references-in-project-files"
    ),
);

crate::register_parser!(
    ".NET PackageReference Visual Basic project file",
    &["**/*.vbproj"],
    "nuget",
    "Visual Basic .NET",
    Some(
        "https://learn.microsoft.com/en-us/nuget/consume-packages/package-references-in-project-files"
    ),
);

crate::register_parser!(
    ".NET PackageReference F# project file",
    &["**/*.fsproj"],
    "nuget",
    "F#",
    Some(
        "https://learn.microsoft.com/en-us/nuget/consume-packages/package-references-in-project-files"
    ),
);

crate::register_parser!(
    ".NET .nupkg package archive",
    &["**/*.nupkg"],
    "nuget",
    "C#",
    Some("https://learn.microsoft.com/en-us/nuget/create-packages/creating-a-package"),
);
