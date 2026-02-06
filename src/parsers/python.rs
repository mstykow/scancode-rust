//! Parser for Python package manifests and metadata files.
//!
//! Comprehensive parser supporting multiple Python packaging formats including
//! modern (pyproject.toml) and legacy (setup.py, setup.cfg) standards.
//!
//! # Supported Formats
//! - pyproject.toml (PEP 621)
//! - setup.py (AST parsing, no code execution)
//! - setup.cfg (INI format)
//! - PKG-INFO / METADATA (RFC 822 format)
//! - .whl archives (wheel format)
//! - .egg archives (legacy egg format)
//! - requirements.txt
//!
//! # Key Features
//! - License declaration normalization using askalono
//! - Archive safety checks (size limits, compression ratio validation)
//! - AST-based setup.py parsing (no code execution)
//! - RFC 822 metadata parsing for wheels/eggs
//! - Dependency extraction with PEP 508 markers
//! - Party information (authors, maintainers)
//!
//! # Security Features
//! - Archive size limit: 100MB
//! - Per-file size limit: 50MB
//! - Compression ratio limit: 100:1
//! - Total extracted size tracking
//! - No code execution from setup.py or .egg files
//!
//! # Implementation Notes
//! - Uses multiple parsers for different formats
//! - Direct dependencies: all manifest dependencies are direct
//! - Graceful fallback on parse errors with warning logs

use crate::askalono::Store;
use crate::models::{Dependency, FileReference, LicenseDetection, PackageData, Party};
use crate::parsers::utils::{
    create_spdx_license_match, normalize_license, read_file_to_string, split_name_email,
};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use csv::ReaderBuilder;
use log::warn;
use packageurl::PackageUrl;
use regex::Regex;
use rustpython_parser::{Parse, ast};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml::Value as TomlValue;
use toml::map::Map as TomlMap;
use zip::ZipArchive;

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
const MAX_SETUP_PY_BYTES: usize = 1_048_576;
const MAX_SETUP_PY_AST_NODES: usize = 10_000;
const MAX_SETUP_PY_AST_DEPTH: usize = 50;
const MAX_ARCHIVE_SIZE: u64 = 100 * 1024 * 1024; // 100MB uncompressed
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB per file
const MAX_COMPRESSION_RATIO: f64 = 100.0; // 100:1 ratio

/// Python package parser supporting 11 manifest formats.
///
/// Extracts metadata from Python package files including pyproject.toml, setup.py,
/// setup.cfg, PKG-INFO, METADATA, pip-inspect lockfiles, and .whl/.egg archives.
///
/// # Security
///
/// setup.py files are parsed using AST analysis rather than code execution to prevent
/// arbitrary code execution during scanning. See `extract_from_setup_py_ast` for details.
pub struct PythonParser;

impl PackageParser for PythonParser {
    const PACKAGE_TYPE: &'static str = "pypi";

    fn extract_package_data(path: &Path) -> PackageData {
        if path.file_name().unwrap_or_default() == "pyproject.toml" {
            extract_from_pyproject_toml(path)
        } else if path.file_name().unwrap_or_default() == "setup.cfg" {
            extract_from_setup_cfg(path)
        } else if path.file_name().unwrap_or_default() == "setup.py" {
            extract_from_setup_py(path)
        } else if path.file_name().unwrap_or_default() == "PKG-INFO" {
            extract_from_rfc822_metadata(path, "pypi_sdist_pkginfo")
        } else if path.file_name().unwrap_or_default() == "METADATA" {
            extract_from_rfc822_metadata(path, "pypi_wheel_metadata")
        } else if path.file_name().unwrap_or_default() == "pip-inspect.deplock" {
            extract_from_pip_inspect(path)
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("whl"))
        {
            extract_from_wheel_archive(path)
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("egg"))
        {
            extract_from_egg_archive(path)
        } else {
            default_package_data()
        }
    }

    fn is_match(path: &Path) -> bool {
        if let Some(filename) = path.file_name()
            && (filename == "pyproject.toml"
                || filename == "setup.cfg"
                || filename == "setup.py"
                || filename == "PKG-INFO"
                || filename == "METADATA"
                || filename == "pip-inspect.deplock")
        {
            return true;
        }

        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            if ext == "whl" || ext == "egg" {
                return true;
            }
        }

        false
    }
}

fn extract_from_rfc822_metadata(path: &Path, datasource_id: &str) -> PackageData {
    let content = match read_file_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            warn!("Failed to read metadata at {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let metadata = parse_rfc822_metadata(&content);
    build_package_data_from_rfc822(&metadata, datasource_id)
}

fn validate_zip_archive<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &Path,
    archive_type: &str,
) -> Result<u64, String> {
    let mut total_extracted = 0u64;

    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let compressed_size = file.compressed_size();
            let uncompressed_size = file.size();

            if compressed_size > 0 {
                let ratio = uncompressed_size as f64 / compressed_size as f64;
                if ratio > MAX_COMPRESSION_RATIO {
                    warn!(
                        "Suspicious compression ratio in {} {:?}: {:.2}:1",
                        archive_type, path, ratio
                    );
                    continue;
                }
            }

            if uncompressed_size > MAX_FILE_SIZE {
                warn!(
                    "File too large in {} {:?}: {} bytes (limit: {} bytes)",
                    archive_type, path, uncompressed_size, MAX_FILE_SIZE
                );
                continue;
            }

            total_extracted += uncompressed_size;
            if total_extracted > MAX_ARCHIVE_SIZE {
                let msg = format!(
                    "Total extracted size exceeds limit for {} {:?}",
                    archive_type, path
                );
                warn!("{}", msg);
                return Err(msg);
            }
        }
    }

    Ok(total_extracted)
}

fn extract_from_wheel_archive(path: &Path) -> PackageData {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            warn!(
                "Failed to read metadata for wheel archive {:?}: {}",
                path, e
            );
            return default_package_data();
        }
    };

    if metadata.len() > MAX_ARCHIVE_SIZE {
        warn!(
            "Wheel archive too large: {} bytes (limit: {} bytes)",
            metadata.len(),
            MAX_ARCHIVE_SIZE
        );
        return default_package_data();
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to open wheel archive {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            warn!("Failed to read wheel archive {:?}: {}", path, e);
            return default_package_data();
        }
    };

    if validate_zip_archive(&mut archive, path, "wheel").is_err() {
        return default_package_data();
    }

    let metadata_path = find_wheel_metadata_path(&mut archive);
    let metadata_path = match metadata_path {
        Some(p) => p,
        None => {
            warn!("No METADATA file found in wheel archive {:?}", path);
            return default_package_data();
        }
    };

    let content = match read_zip_entry(&mut archive, &metadata_path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read METADATA from {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let mut package_data = parse_rfc822_content(&content, "pypi_wheel");

    let (size, sha256) = calculate_file_checksums(path);
    package_data.size = size;
    package_data.sha256 = sha256;

    if let Some(record_path) = find_wheel_record_path(&mut archive)
        && let Ok(record_content) = read_zip_entry(&mut archive, &record_path)
    {
        package_data.file_references = parse_record_csv(&record_content);
    }

    if let Some(wheel_info) = parse_wheel_filename(path) {
        if package_data.name.is_none() {
            package_data.name = Some(wheel_info.name.clone());
        }
        if package_data.version.is_none() {
            package_data.version = Some(wheel_info.version.clone());
        }

        package_data.purl = build_wheel_purl(
            package_data.name.as_deref(),
            package_data.version.as_deref(),
            &wheel_info,
        );

        let mut extra_data = package_data.extra_data.unwrap_or_default();
        extra_data.insert(
            "python_requires".to_string(),
            serde_json::Value::String(wheel_info.python_tag.clone()),
        );
        extra_data.insert(
            "abi_tag".to_string(),
            serde_json::Value::String(wheel_info.abi_tag.clone()),
        );
        extra_data.insert(
            "platform_tag".to_string(),
            serde_json::Value::String(wheel_info.platform_tag.clone()),
        );
        package_data.extra_data = Some(extra_data);
    }

    package_data
}

fn extract_from_egg_archive(path: &Path) -> PackageData {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to read metadata for egg archive {:?}: {}", path, e);
            return default_package_data();
        }
    };

    if metadata.len() > MAX_ARCHIVE_SIZE {
        warn!(
            "Egg archive too large: {} bytes (limit: {} bytes)",
            metadata.len(),
            MAX_ARCHIVE_SIZE
        );
        return default_package_data();
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to open egg archive {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            warn!("Failed to read egg archive {:?}: {}", path, e);
            return default_package_data();
        }
    };

    if validate_zip_archive(&mut archive, path, "egg").is_err() {
        return default_package_data();
    }

    let pkginfo_path = find_egg_pkginfo_path(&mut archive);
    let pkginfo_path = match pkginfo_path {
        Some(p) => p,
        None => {
            warn!("No PKG-INFO file found in egg archive {:?}", path);
            return default_package_data();
        }
    };

    let content = match read_zip_entry(&mut archive, &pkginfo_path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read PKG-INFO from {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let mut package_data = parse_rfc822_content(&content, "pypi_egg");

    let (size, sha256) = calculate_file_checksums(path);
    package_data.size = size;
    package_data.sha256 = sha256;

    if let Some(installed_files_path) = find_egg_installed_files_path(&mut archive)
        && let Ok(installed_files_content) = read_zip_entry(&mut archive, &installed_files_path)
    {
        package_data.file_references = parse_installed_files_txt(&installed_files_content);
    }

    if let Some(egg_info) = parse_egg_filename(path) {
        if package_data.name.is_none() {
            package_data.name = Some(egg_info.name.clone());
        }
        if package_data.version.is_none() {
            package_data.version = Some(egg_info.version.clone());
        }

        if let Some(python_version) = &egg_info.python_version {
            let mut extra_data = package_data.extra_data.unwrap_or_default();
            extra_data.insert(
                "python_version".to_string(),
                serde_json::Value::String(python_version.clone()),
            );
            package_data.extra_data = Some(extra_data);
        }
    }

    package_data.purl = build_egg_purl(
        package_data.name.as_deref(),
        package_data.version.as_deref(),
    );

    package_data
}

fn find_wheel_metadata_path<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Option<String> {
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index_raw(i) {
            let name = file.name();
            if name.ends_with(".dist-info/METADATA") {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn find_egg_pkginfo_path<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index_raw(i) {
            let name = file.name();
            if name.ends_with("EGG-INFO/PKG-INFO") || name.ends_with(".egg-info/PKG-INFO") {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn read_zip_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Result<String, String> {
    let mut file = archive
        .by_name(path)
        .map_err(|e| format!("Failed to find entry {}: {}", path, e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;
    Ok(content)
}

fn find_wheel_record_path<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index_raw(i) {
            let name = file.name();
            if name.ends_with(".dist-info/RECORD") {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn find_egg_installed_files_path<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Option<String> {
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index_raw(i) {
            let name = file.name();
            if name.ends_with("EGG-INFO/installed-files.txt")
                || name.ends_with(".egg-info/installed-files.txt")
            {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Parses RECORD CSV format from wheel archives (PEP 427).
/// Format: path,hash,size (3 columns, no header)
/// Hash format: sha256=urlsafe_base64_hash or empty
/// Size: bytes as u64 or empty
pub fn parse_record_csv(content: &str) -> Vec<FileReference> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(content.as_bytes());

    let mut file_references = Vec::new();

    for result in reader.records() {
        match result {
            Ok(record) => {
                if record.len() < 3 {
                    continue;
                }

                let path = record.get(0).unwrap_or("").trim().to_string();
                if path.is_empty() {
                    continue;
                }

                let hash_field = record.get(1).unwrap_or("").trim();
                let size_field = record.get(2).unwrap_or("").trim();

                // Parse hash: format is "algorithm=value"
                let sha256 = if !hash_field.is_empty() && hash_field.contains('=') {
                    let parts: Vec<&str> = hash_field.split('=').collect();
                    if parts.len() == 2 && parts[0] == "sha256" {
                        // Decode base64 to hex
                        match URL_SAFE_NO_PAD.decode(parts[1]) {
                            Ok(decoded) => {
                                let hex = decoded
                                    .iter()
                                    .map(|b| format!("{:02x}", b))
                                    .collect::<String>();
                                Some(hex)
                            }
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Parse size
                let size = if !size_field.is_empty() && size_field != "-" {
                    size_field.parse::<u64>().ok()
                } else {
                    None
                };

                file_references.push(FileReference {
                    path,
                    size,
                    sha1: None,
                    md5: None,
                    sha256,
                    sha512: None,
                    extra_data: None,
                });
            }
            Err(e) => {
                warn!("Failed to parse RECORD CSV row: {}", e);
                continue;
            }
        }
    }

    file_references
}

/// Parses installed-files.txt format from egg archives (PEP 376).
/// Format: one file path per line, no headers, no hash, no size
pub fn parse_installed_files_txt(content: &str) -> Vec<FileReference> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|path| FileReference {
            path: path.to_string(),
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            extra_data: None,
        })
        .collect()
}

struct WheelInfo {
    name: String,
    version: String,
    python_tag: String,
    abi_tag: String,
    platform_tag: String,
}

fn parse_wheel_filename(path: &Path) -> Option<WheelInfo> {
    let stem = path.file_stem()?.to_string_lossy();
    let parts: Vec<&str> = stem.split('-').collect();

    if parts.len() >= 5 {
        Some(WheelInfo {
            name: parts[0].replace('_', "-"),
            version: parts[1].to_string(),
            python_tag: parts[2].to_string(),
            abi_tag: parts[3].to_string(),
            platform_tag: parts[4..].join("-"),
        })
    } else {
        None
    }
}

struct EggInfo {
    name: String,
    version: String,
    python_version: Option<String>,
}

fn parse_egg_filename(path: &Path) -> Option<EggInfo> {
    let stem = path.file_stem()?.to_string_lossy();
    let parts: Vec<&str> = stem.split('-').collect();

    if parts.len() >= 2 {
        Some(EggInfo {
            name: parts[0].replace('_', "-"),
            version: parts[1].to_string(),
            python_version: parts.get(2).map(|s| s.to_string()),
        })
    } else {
        None
    }
}

fn build_wheel_purl(
    name: Option<&str>,
    version: Option<&str>,
    wheel_info: &WheelInfo,
) -> Option<String> {
    let name = name?;
    let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, name).ok()?;

    if let Some(ver) = version {
        package_url.with_version(ver).ok()?;
    }

    let extension = format!(
        "{}-{}-{}",
        wheel_info.python_tag, wheel_info.abi_tag, wheel_info.platform_tag
    );
    package_url.add_qualifier("extension", extension).ok()?;

    Some(package_url.to_string())
}

fn build_egg_purl(name: Option<&str>, version: Option<&str>) -> Option<String> {
    let name = name?;
    let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, name).ok()?;

    if let Some(ver) = version {
        package_url.with_version(ver).ok()?;
    }

    package_url.add_qualifier("type", "egg").ok()?;

    Some(package_url.to_string())
}

fn parse_rfc822_content(content: &str, datasource_id: &str) -> PackageData {
    let metadata = parse_rfc822_metadata(content);
    build_package_data_from_rfc822(&metadata, datasource_id)
}

struct Rfc822Metadata {
    headers: HashMap<String, Vec<String>>,
    body: String,
}

fn parse_rfc822_metadata(content: &str) -> Rfc822Metadata {
    let mut headers: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current_value = String::new();
    let mut body_lines: Vec<String> = Vec::new();
    let mut in_headers = true;

    for line in content.lines() {
        if in_headers {
            if line.is_empty() {
                if let Some(name) = current_name.take() {
                    add_header_value(&mut headers, &name, &current_value);
                    current_value.clear();
                }
                in_headers = false;
                continue;
            }

            if line.starts_with(' ') || line.starts_with('\t') {
                if !current_value.is_empty() {
                    current_value.push(' ');
                }
                current_value.push_str(line.trim_start());
                continue;
            }

            if let Some(name) = current_name.take() {
                add_header_value(&mut headers, &name, &current_value);
                current_value.clear();
            }

            if let Some((name, value)) = line.split_once(':') {
                current_name = Some(name.trim().to_ascii_lowercase());
                current_value = value.trim_start().to_string();
            }
        } else {
            body_lines.push(line.to_string());
        }
    }

    if let Some(name) = current_name.take() {
        add_header_value(&mut headers, &name, &current_value);
    }

    let mut body = body_lines.join("\n");
    body = body.trim_end_matches(['\n', '\r']).to_string();

    Rfc822Metadata { headers, body }
}

/// Builds PackageData from parsed RFC822 metadata.
///
/// This is the shared implementation for both `extract_from_rfc822_metadata` (file-based)
/// and `parse_rfc822_content` (content-based) functions.
fn build_package_data_from_rfc822(metadata: &Rfc822Metadata, datasource_id: &str) -> PackageData {
    let name = get_header_first(&metadata.headers, "name");
    let version = get_header_first(&metadata.headers, "version");
    let summary = get_header_first(&metadata.headers, "summary");
    let mut homepage_url = get_header_first(&metadata.headers, "home-page");
    let author = get_header_first(&metadata.headers, "author");
    let author_email = get_header_first(&metadata.headers, "author-email");
    let license = get_header_first(&metadata.headers, "license");
    let download_url = get_header_first(&metadata.headers, "download-url");
    let platform = get_header_first(&metadata.headers, "platform");
    let requires_python = get_header_first(&metadata.headers, "requires-python");
    let classifiers = get_header_all(&metadata.headers, "classifier");
    let license_files = get_header_all(&metadata.headers, "license-file");

    let description_body = if metadata.body.is_empty() {
        get_header_first(&metadata.headers, "description").unwrap_or_default()
    } else {
        metadata.body.clone()
    };

    let description = build_description(summary.as_deref(), &description_body);

    let mut parties = Vec::new();
    if author.is_some() || author_email.is_some() {
        parties.push(Party {
            r#type: Some("person".to_string()),
            role: Some("author".to_string()),
            name: author,
            email: author_email,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    let (keywords, license_classifiers) = split_classifiers(&classifiers);
    let license_detections = build_license_detections(license.as_deref(), &license_classifiers);

    let declared_license_expression = license.as_ref().map(|value| value.to_lowercase());
    let declared_license_expression_spdx = license.clone();

    let extracted_license_statement =
        build_extracted_license_statement(license.as_deref(), &license_classifiers);

    let mut extra_data = HashMap::new();
    if let Some(platform_value) = platform
        && !platform_value.eq_ignore_ascii_case("unknown")
        && !platform_value.is_empty()
    {
        extra_data.insert(
            "platform".to_string(),
            serde_json::Value::String(platform_value),
        );
    }

    if let Some(requires_python_value) = requires_python
        && !requires_python_value.is_empty()
    {
        extra_data.insert(
            "requires_python".to_string(),
            serde_json::Value::String(requires_python_value),
        );
    }

    if !license_files.is_empty() {
        extra_data.insert(
            "license_files".to_string(),
            serde_json::Value::Array(
                license_files
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }

    let project_urls = get_header_all(&metadata.headers, "project-url");
    let (mut bug_tracking_url, mut code_view_url, mut vcs_url) = (None, None, None);

    if !project_urls.is_empty() {
        let parsed_urls = parse_project_urls(&project_urls);

        for (label, url) in &parsed_urls {
            let label_lower = label.to_lowercase();

            if bug_tracking_url.is_none()
                && matches!(
                    label_lower.as_str(),
                    "tracker"
                        | "bug reports"
                        | "bug tracker"
                        | "issues"
                        | "issue tracker"
                        | "github: issues"
                )
            {
                bug_tracking_url = Some(url.clone());
            } else if code_view_url.is_none()
                && matches!(label_lower.as_str(), "source" | "source code" | "code")
            {
                code_view_url = Some(url.clone());
            } else if vcs_url.is_none()
                && matches!(
                    label_lower.as_str(),
                    "github" | "gitlab" | "github: repo" | "repository"
                )
            {
                vcs_url = Some(url.clone());
            } else if homepage_url.is_none()
                && matches!(label_lower.as_str(), "website" | "homepage" | "home")
            {
                homepage_url = Some(url.clone());
            } else if label_lower == "changelog" {
                extra_data.insert(
                    "changelog_url".to_string(),
                    serde_json::Value::String(url.clone()),
                );
            }
        }

        let project_urls_json: serde_json::Map<String, serde_json::Value> = parsed_urls
            .iter()
            .map(|(label, url)| (label.clone(), serde_json::Value::String(url.clone())))
            .collect();

        if !project_urls_json.is_empty() {
            extra_data.insert(
                "project_urls".to_string(),
                serde_json::Value::Object(project_urls_json),
            );
        }
    }

    let extra_data = if extra_data.is_empty() {
        None
    } else {
        Some(extra_data)
    };

    let (repository_homepage_url, repository_download_url, api_data_url, purl) =
        build_pypi_urls(name.as_deref(), version.as_deref());

    PackageData {
        package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version,
        qualifiers: None,
        subpath: None,
        primary_language: Some("Python".to_string()),
        description,
        release_date: None,
        parties,
        keywords,
        homepage_url,
        download_url,
        size: None,
        sha1: None,
        md5: None,
        sha256: None,
        sha512: None,
        bug_tracking_url,
        code_view_url,
        vcs_url,
        copyright: None,
        holder: None,
        declared_license_expression,
        declared_license_expression_spdx,
        license_detections,
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement,
        notice_text: None,
        source_packages: Vec::new(),
        file_references: Vec::new(),
        is_private: false,
        is_virtual: false,
        extra_data,
        dependencies: Vec::new(),
        repository_homepage_url,
        repository_download_url,
        api_data_url,
        datasource_id: Some(datasource_id.to_string()),
        purl,
    }
}

fn add_header_value(headers: &mut HashMap<String, Vec<String>>, name: &str, value: &str) {
    let entry = headers.entry(name.to_string()).or_default();
    let trimmed = value.trim_end();
    if !trimmed.is_empty() {
        entry.push(trimmed.to_string());
    }
}

fn get_header_first(headers: &HashMap<String, Vec<String>>, name: &str) -> Option<String> {
    headers
        .get(&name.to_ascii_lowercase())
        .and_then(|values| values.first())
        .map(|value| value.trim().to_string())
}

fn get_header_all(headers: &HashMap<String, Vec<String>>, name: &str) -> Vec<String> {
    headers
        .get(&name.to_ascii_lowercase())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn parse_project_urls(project_urls: &[String]) -> Vec<(String, String)> {
    project_urls
        .iter()
        .filter_map(|url_entry| {
            if let Some((label, url)) = url_entry.split_once(", ") {
                let label_trimmed = label.trim();
                let url_trimmed = url.trim();
                if !label_trimmed.is_empty() && !url_trimmed.is_empty() {
                    return Some((label_trimmed.to_string(), url_trimmed.to_string()));
                }
            }
            None
        })
        .collect()
}

fn build_description(summary: Option<&str>, body: &str) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(summary_value) = summary
        && !summary_value.trim().is_empty()
    {
        parts.push(summary_value.trim().to_string());
    }

    if !body.trim().is_empty() {
        parts.push(body.trim().to_string());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn split_classifiers(classifiers: &[String]) -> (Vec<String>, Vec<String>) {
    let mut keywords = Vec::new();
    let mut license_classifiers = Vec::new();

    for classifier in classifiers {
        if classifier.starts_with("License ::") {
            license_classifiers.push(classifier.to_string());
        } else {
            keywords.push(classifier.to_string());
        }
    }

    (keywords, license_classifiers)
}

fn build_license_detections(
    license: Option<&str>,
    license_classifiers: &[String],
) -> Vec<LicenseDetection> {
    let mut detections = Vec::new();

    if let Some(value) = license
        && !value.trim().is_empty()
    {
        detections.push(create_license_detection(value.trim()));
    }

    for classifier in license_classifiers {
        if let Some(normalized) = normalize_license_classifier(classifier) {
            detections.push(create_license_detection(&normalized));
        }
    }

    detections
}

fn normalize_license_classifier(classifier: &str) -> Option<String> {
    let last_segment = classifier.split("::").last()?.trim();
    if last_segment.is_empty() {
        return None;
    }

    let mut cleaned = last_segment.to_string();
    for suffix in [
        "Software License",
        "Public License",
        "Open Source License",
        "License",
    ] {
        if cleaned.ends_with(suffix) {
            cleaned = cleaned.trim_end_matches(suffix).trim().to_string();
            break;
        }
    }

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn build_extracted_license_statement(
    license: Option<&str>,
    license_classifiers: &[String],
) -> Option<String> {
    let mut lines = Vec::new();

    if let Some(value) = license
        && !value.trim().is_empty()
    {
        lines.push(format!("license: {}", value.trim()));
    }

    if !license_classifiers.is_empty() {
        lines.push("classifiers:".to_string());
        for classifier in license_classifiers {
            lines.push(format!("  - '{}'", classifier));
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(format!("{}\n", lines.join("\n")))
    }
}

pub(crate) fn build_pypi_urls(
    name: Option<&str>,
    version: Option<&str>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let repository_homepage_url = name.map(|value| format!("https://pypi.org/project/{}", value));

    let repository_download_url = name.and_then(|value| {
        version.map(|ver| {
            format!(
                "https://pypi.org/packages/source/{}/{}/{}-{}.tar.gz",
                &value[..1.min(value.len())],
                value,
                value,
                ver
            )
        })
    });

    let api_data_url = name.map(|value| {
        if let Some(ver) = version {
            format!("https://pypi.org/pypi/{}/{}/json", value, ver)
        } else {
            format!("https://pypi.org/pypi/{}/json", value)
        }
    });

    let purl = name.and_then(|value| {
        let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, value).ok()?;
        if let Some(ver) = version {
            package_url.with_version(ver).ok()?;
        }
        Some(package_url.to_string())
    });

    (
        repository_homepage_url,
        repository_download_url,
        api_data_url,
        purl,
    )
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

    let extracted_license_statement = extract_raw_license_string(&project_table);
    let store = Store::new();
    let (declared_license_expression, declared_license_expression_spdx) =
        if let Some(raw) = &extracted_license_statement {
            let (expr, spdx) = normalize_license(raw, &store);
            // Fallback to raw license string if store is empty or normalization fails
            if store.is_empty() {
                (Some(raw.to_lowercase()), Some(raw.clone()))
            } else {
                (expr, spdx)
            }
        } else {
            (None, None)
        };

    // URLs can be in different formats depending on the tool (poetry, flit, etc.)
    let (homepage_url, repository_url) = extract_urls(&project_table);

    let (dependencies, optional_dependencies) = extract_dependencies(&project_table);

    // Create package URL
    let purl = name.as_ref().and_then(|n| {
        let mut package_url = match PackageUrl::new(PythonParser::PACKAGE_TYPE, n) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Failed to create PackageUrl for Python package '{}': {}",
                    n, e
                );
                return None;
            }
        };

        if let Some(v) = &version
            && let Err(e) = package_url.with_version(v)
        {
            warn!(
                "Failed to set version '{}' for Python package '{}': {}",
                v, n, e
            );
            return None;
        }

        Some(package_url.to_string())
    });

    let api_data_url = name.as_ref().map(|n| {
        if let Some(v) = &version {
            format!("https://pypi.org/pypi/{}/{}/json", n, v)
        } else {
            format!("https://pypi.org/pypi/{}/json", n)
        }
    });

    let pypi_homepage_url = name
        .as_ref()
        .map(|n| format!("https://pypi.org/project/{}", n));

    let pypi_download_url = name.as_ref().and_then(|n| {
        version.as_ref().map(|v| {
            format!(
                "https://pypi.org/packages/source/{}/{}/{}-{}.tar.gz",
                &n[..1.min(n.len())],
                n,
                n,
                v
            )
        })
    });

    PackageData {
        package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version,
        qualifiers: None,
        subpath: None,
        primary_language: None,
        description: None,
        release_date: None,
        parties: extract_parties(&project_table),
        keywords: Vec::new(),
        homepage_url: homepage_url.or(pypi_homepage_url),
        download_url: repository_url.clone().or(pypi_download_url),
        size: None,
        sha1: None,
        md5: None,
        sha256: None,
        sha512: None,
        bug_tracking_url: None,
        code_view_url: None,
        vcs_url: repository_url,
        copyright: None,
        holder: None,
        declared_license_expression,
        declared_license_expression_spdx,
        license_detections,
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement,
        notice_text: None,
        source_packages: Vec::new(),
        file_references: Vec::new(),
        is_private: false,
        is_virtual: false,
        extra_data: None,
        dependencies: [dependencies, optional_dependencies].concat(),
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url,
        datasource_id: None,
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
    let license_lower = license_str.to_lowercase();
    LicenseDetection {
        license_expression: license_lower.clone(),
        license_expression_spdx: license_str.to_string(),
        matches: vec![create_spdx_license_match(license_str)],
        identifier: Some(format!(
            "{}-a822f434-d61f-f2b1-c792-8b8cb9e7b9bf",
            license_lower
        )),
    }
}

fn extract_raw_license_string(project: &TomlMap<String, TomlValue>) -> Option<String> {
    project
        .get(FIELD_LICENSE)
        .and_then(|license_value| match license_value {
            TomlValue::String(license_str) => Some(license_str.clone()),
            TomlValue::Table(license_table) => license_table
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    license_table
                        .get("expression")
                        .and_then(|v| v.as_str())
                        .map(|expr| expr.to_string())
                }),
            _ => None,
        })
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

    if let Some(authors) = project.get(FIELD_AUTHORS).and_then(|v| v.as_array()) {
        for author in authors {
            if let Some(author_str) = author.as_str() {
                let (name, email) = split_name_email(author_str);
                parties.push(Party {
                    r#type: None,
                    role: Some("author".to_string()),
                    name,
                    email,
                    url: None,
                    organization: None,
                    organization_url: None,
                    timezone: None,
                });
            }
        }
    }

    if let Some(maintainers) = project.get(FIELD_MAINTAINERS).and_then(|v| v.as_array()) {
        for maintainer in maintainers {
            if let Some(maintainer_str) = maintainer.as_str() {
                let (name, email) = split_name_email(maintainer_str);
                parties.push(Party {
                    r#type: None,
                    role: Some("maintainer".to_string()),
                    name,
                    email,
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

fn extract_dependencies(
    project: &TomlMap<String, TomlValue>,
) -> (Vec<Dependency>, Vec<Dependency>) {
    let mut dependencies = Vec::new();
    let mut optional_dependencies = Vec::new();

    // Handle dependencies - can be array or table format
    if let Some(deps_value) = project.get(FIELD_DEPENDENCIES) {
        match deps_value {
            TomlValue::Array(arr) => {
                dependencies = parse_dependency_array(arr, false, None);
            }
            TomlValue::Table(table) => {
                dependencies = parse_dependency_table(table, false, None);
            }
            _ => {}
        }
    }

    // Handle optional dependencies with scope
    if let Some(opt_deps_table) = project
        .get(FIELD_OPTIONAL_DEPENDENCIES)
        .and_then(|v| v.as_table())
    {
        for (extra_name, deps) in opt_deps_table {
            match deps {
                TomlValue::Array(arr) => {
                    optional_dependencies.extend(parse_dependency_array(
                        arr,
                        true,
                        Some(extra_name),
                    ));
                }
                TomlValue::Table(table) => {
                    optional_dependencies.extend(parse_dependency_table(
                        table,
                        true,
                        Some(extra_name),
                    ));
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
    scope: Option<&str>,
) -> Vec<Dependency> {
    table
        .iter()
        .filter_map(|(name, version)| {
            let version_str = version.as_str().map(|s| s.to_string());
            let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, name).ok()?;

            if let Some(v) = &version_str {
                package_url.with_version(v).ok()?;
            }

            Some(Dependency {
                purl: Some(package_url.to_string()),
                extracted_requirement: None,
                scope: scope.map(|s| s.to_string()),
                is_runtime: Some(!is_optional),
                is_optional: Some(is_optional),
                is_pinned: None,
                is_direct: Some(true),
                resolved_package: None,
                extra_data: None,
            })
        })
        .collect()
}

fn parse_dependency_array(
    array: &[TomlValue],
    is_optional: bool,
    scope: Option<&str>,
) -> Vec<Dependency> {
    array
        .iter()
        .filter_map(|dep| {
            let dep_str = dep.as_str()?;

            let mut parts = dep_str.split(['>', '=', '<', '~']);
            let name = parts.next()?.trim().to_string();

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
                scope: scope.map(|s| s.to_string()),
                is_runtime: Some(!is_optional),
                is_optional: Some(is_optional),
                is_pinned: None,
                is_direct: Some(true),
                resolved_package: None,
                extra_data: None,
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    None,
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Dict(HashMap<String, Value>),
}

struct LiteralEvaluator {
    constants: HashMap<String, Value>,
    max_depth: usize,
    max_nodes: usize,
    nodes_visited: usize,
}

impl LiteralEvaluator {
    fn new(constants: HashMap<String, Value>) -> Self {
        Self {
            constants,
            max_depth: MAX_SETUP_PY_AST_DEPTH,
            max_nodes: MAX_SETUP_PY_AST_NODES,
            nodes_visited: 0,
        }
    }

    fn insert_constant(&mut self, name: String, value: Value) {
        self.constants.insert(name, value);
    }

    fn evaluate_expr(&mut self, expr: &ast::Expr, depth: usize) -> Option<Value> {
        if depth >= self.max_depth || self.nodes_visited >= self.max_nodes {
            return None;
        }
        self.nodes_visited += 1;

        match expr {
            ast::Expr::Constant(ast::ExprConstant { value, .. }) => self.evaluate_constant(value),
            ast::Expr::Name(ast::ExprName { id, .. }) => self.constants.get(id.as_str()).cloned(),
            ast::Expr::List(ast::ExprList { elts, .. }) => {
                let mut values = Vec::new();
                for elt in elts {
                    values.push(self.evaluate_expr(elt, depth + 1)?);
                }
                Some(Value::List(values))
            }
            ast::Expr::Tuple(ast::ExprTuple { elts, .. }) => {
                let mut values = Vec::new();
                for elt in elts {
                    values.push(self.evaluate_expr(elt, depth + 1)?);
                }
                Some(Value::Tuple(values))
            }
            ast::Expr::Dict(ast::ExprDict { keys, values, .. }) => {
                let mut dict = HashMap::new();
                for (key_expr, value_expr) in keys.iter().zip(values.iter()) {
                    let key_expr = key_expr.as_ref()?;
                    let key_value = self.evaluate_expr(key_expr, depth + 1)?;
                    let key = value_to_string(&key_value)?;
                    let value = self.evaluate_expr(value_expr, depth + 1)?;
                    dict.insert(key, value);
                }
                Some(Value::Dict(dict))
            }
            ast::Expr::Call(ast::ExprCall {
                func,
                args,
                keywords,
                ..
            }) => {
                if !args.is_empty() {
                    return None;
                }

                if let ast::Expr::Name(ast::ExprName { id, .. }) = func.as_ref()
                    && id == "dict"
                {
                    let mut dict = HashMap::new();
                    for keyword in keywords {
                        let key = keyword.arg.as_ref().map(|name| name.as_str())?;
                        let value = self.evaluate_expr(&keyword.value, depth + 1)?;
                        dict.insert(key.to_string(), value);
                    }
                    return Some(Value::Dict(dict));
                }

                None
            }
            _ => None,
        }
    }

    fn evaluate_constant(&self, constant: &ast::Constant) -> Option<Value> {
        match constant {
            ast::Constant::Str(value) => Some(Value::String(value.clone())),
            ast::Constant::Bool(value) => Some(Value::Bool(*value)),
            ast::Constant::Int(value) => value.to_string().parse::<f64>().ok().map(Value::Number),
            ast::Constant::Float(value) => Some(Value::Number(*value)),
            ast::Constant::None => Some(Value::None),
            _ => None,
        }
    }
}

#[derive(Default)]
struct SetupAliases {
    setup_names: HashSet<String>,
    module_aliases: HashMap<String, String>,
}

fn extract_from_setup_py(path: &Path) -> PackageData {
    let content = match read_file_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            warn!("Failed to read setup.py at {:?}: {}", path, e);
            return default_package_data();
        }
    };

    if content.len() > MAX_SETUP_PY_BYTES {
        warn!("setup.py too large at {:?}: {} bytes", path, content.len());
        return extract_from_setup_py_regex(&content);
    }

    let mut package_data = match extract_from_setup_py_ast(&content) {
        Ok(Some(data)) => data,
        Ok(None) => extract_from_setup_py_regex(&content),
        Err(e) => {
            warn!("Failed to parse setup.py AST at {:?}: {}", path, e);
            extract_from_setup_py_regex(&content)
        }
    };

    if package_data.name.is_none() {
        package_data.name = extract_setup_value(&content, "name");
    }

    if package_data.version.is_none() {
        package_data.version = extract_setup_value(&content, "version");
    }

    if package_data.purl.is_none() {
        package_data.purl = build_setup_py_purl(
            package_data.name.as_deref(),
            package_data.version.as_deref(),
        );
    }

    package_data
}

/// Extracts package metadata from setup.py using AST parsing (NO CODE EXECUTION).
///
/// # Security Model
///
/// This function parses setup.py as a Python AST and evaluates only literal values
/// (strings, numbers, lists, dicts). It does NOT execute Python code, preventing
/// arbitrary code execution during scanning.
///
/// # DoS Prevention
///
/// - `MAX_SETUP_PY_BYTES`: Limits file size to 1MB
/// - `MAX_SETUP_PY_AST_DEPTH`: Limits recursion depth (50 levels)
/// - `MAX_SETUP_PY_AST_NODES`: Limits total nodes visited (10,000)
///
/// These limits prevent stack overflow and infinite loops on malformed/malicious inputs.
fn extract_from_setup_py_ast(content: &str) -> Result<Option<PackageData>, String> {
    let statements = ast::Suite::parse(content, "<setup.py>").map_err(|e| format!("{}", e))?;
    let aliases = collect_setup_aliases(&statements);
    let mut evaluator = LiteralEvaluator::new(HashMap::new());
    build_setup_py_constants(&statements, &mut evaluator);

    let setup_call = find_setup_call(&statements, &aliases);
    let Some(call_expr) = setup_call else {
        return Ok(None);
    };

    let setup_values = extract_setup_keywords(call_expr, &mut evaluator);
    Ok(Some(build_setup_py_package_data(&setup_values)))
}

fn build_setup_py_constants(statements: &[ast::Stmt], evaluator: &mut LiteralEvaluator) {
    for stmt in statements {
        if let ast::Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt {
            if targets.len() != 1 {
                continue;
            }

            let Some(name) = extract_assign_name(&targets[0]) else {
                continue;
            };

            if let Some(value) = evaluator.evaluate_expr(value.as_ref(), 0) {
                evaluator.insert_constant(name, value);
            }
        }
    }
}

fn extract_assign_name(target: &ast::Expr) -> Option<String> {
    match target {
        ast::Expr::Name(ast::ExprName { id, .. }) => Some(id.as_str().to_string()),
        _ => None,
    }
}

fn collect_setup_aliases(statements: &[ast::Stmt]) -> SetupAliases {
    let mut aliases = SetupAliases::default();
    aliases.setup_names.insert("setup".to_string());

    for stmt in statements {
        match stmt {
            ast::Stmt::Import(ast::StmtImport { names, .. }) => {
                for alias in names {
                    let module_name = alias.name.as_str();
                    if !is_setup_module(module_name) {
                        continue;
                    }
                    let alias_name = alias
                        .asname
                        .as_ref()
                        .map(|name| name.as_str())
                        .unwrap_or(module_name);
                    aliases
                        .module_aliases
                        .insert(alias_name.to_string(), module_name.to_string());
                }
            }
            ast::Stmt::ImportFrom(ast::StmtImportFrom { module, names, .. }) => {
                let Some(module_name) = module.as_ref().map(|name| name.as_str()) else {
                    continue;
                };
                if !is_setup_module(module_name) {
                    continue;
                }
                for alias in names {
                    if alias.name.as_str() != "setup" {
                        continue;
                    }
                    let alias_name = alias
                        .asname
                        .as_ref()
                        .map(|name| name.as_str())
                        .unwrap_or("setup");
                    aliases.setup_names.insert(alias_name.to_string());
                }
            }
            _ => {}
        }
    }

    aliases
}

fn is_setup_module(module_name: &str) -> bool {
    matches!(module_name, "setuptools" | "distutils" | "distutils.core")
}

fn find_setup_call<'a>(
    statements: &'a [ast::Stmt],
    aliases: &'a SetupAliases,
) -> Option<&'a ast::Expr> {
    let mut finder = SetupCallFinder {
        aliases,
        nodes_visited: 0,
    };
    finder.find_in_statements(statements)
}

struct SetupCallFinder<'a> {
    aliases: &'a SetupAliases,
    nodes_visited: usize,
}

impl<'a> SetupCallFinder<'a> {
    fn find_in_statements(&mut self, statements: &'a [ast::Stmt]) -> Option<&'a ast::Expr> {
        for stmt in statements {
            if self.nodes_visited >= MAX_SETUP_PY_AST_NODES {
                return None;
            }
            self.nodes_visited += 1;

            let found = match stmt {
                ast::Stmt::Expr(ast::StmtExpr { value, .. }) => self.visit_expr(value.as_ref()),
                ast::Stmt::Assign(ast::StmtAssign { value, .. }) => self.visit_expr(value.as_ref()),
                ast::Stmt::If(ast::StmtIf { body, orelse, .. }) => self
                    .find_in_statements(body)
                    .or_else(|| self.find_in_statements(orelse)),
                ast::Stmt::For(ast::StmtFor { body, orelse, .. })
                | ast::Stmt::While(ast::StmtWhile { body, orelse, .. }) => self
                    .find_in_statements(body)
                    .or_else(|| self.find_in_statements(orelse)),
                ast::Stmt::With(ast::StmtWith { body, .. }) => self.find_in_statements(body),
                ast::Stmt::Try(ast::StmtTry {
                    body,
                    orelse,
                    finalbody,
                    handlers,
                    ..
                })
                | ast::Stmt::TryStar(ast::StmtTryStar {
                    body,
                    orelse,
                    finalbody,
                    handlers,
                    ..
                }) => self
                    .find_in_statements(body)
                    .or_else(|| self.find_in_statements(orelse))
                    .or_else(|| self.find_in_statements(finalbody))
                    .or_else(|| {
                        for handler in handlers {
                            let ast::ExceptHandler::ExceptHandler(
                                ast::ExceptHandlerExceptHandler { body, .. },
                            ) = handler;
                            if let Some(found) = self.find_in_statements(body) {
                                return Some(found);
                            }
                        }
                        None
                    }),
                _ => None,
            };

            if found.is_some() {
                return found;
            }
        }

        None
    }

    fn visit_expr(&mut self, expr: &'a ast::Expr) -> Option<&'a ast::Expr> {
        if self.nodes_visited >= MAX_SETUP_PY_AST_NODES {
            return None;
        }
        self.nodes_visited += 1;

        match expr {
            ast::Expr::Call(ast::ExprCall { func, .. })
                if is_setup_call(func.as_ref(), self.aliases) =>
            {
                Some(expr)
            }
            _ => None,
        }
    }
}

fn is_setup_call(func: &ast::Expr, aliases: &SetupAliases) -> bool {
    let Some(dotted) = dotted_name(func, 0) else {
        return false;
    };

    if aliases.setup_names.contains(&dotted) {
        return true;
    }

    let Some(module) = dotted.strip_suffix(".setup") else {
        return false;
    };

    let resolved = resolve_module_alias(module, aliases);
    is_setup_module(&resolved)
}

fn dotted_name(expr: &ast::Expr, depth: usize) -> Option<String> {
    if depth >= MAX_SETUP_PY_AST_DEPTH {
        return None;
    }

    match expr {
        ast::Expr::Name(ast::ExprName { id, .. }) => Some(id.as_str().to_string()),
        ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
            let base = dotted_name(value.as_ref(), depth + 1)?;
            Some(format!("{}.{}", base, attr.as_str()))
        }
        _ => None,
    }
}

fn resolve_module_alias(module: &str, aliases: &SetupAliases) -> String {
    if let Some(mapped) = aliases.module_aliases.get(module) {
        return mapped.clone();
    }

    let Some((base, rest)) = module.split_once('.') else {
        return module.to_string();
    };

    if let Some(mapped) = aliases.module_aliases.get(base) {
        return format!("{}.{}", mapped, rest);
    }

    module.to_string()
}

fn extract_setup_keywords(
    call_expr: &ast::Expr,
    evaluator: &mut LiteralEvaluator,
) -> HashMap<String, Value> {
    let mut values = HashMap::new();
    let ast::Expr::Call(ast::ExprCall { keywords, .. }) = call_expr else {
        return values;
    };

    for keyword in keywords {
        if let Some(arg) = keyword.arg.as_ref().map(|name| name.as_str()) {
            if let Some(value) = evaluator.evaluate_expr(&keyword.value, 0) {
                values.insert(arg.to_string(), value);
            }
        } else if let Some(Value::Dict(dict)) = evaluator.evaluate_expr(&keyword.value, 0) {
            for (key, value) in dict {
                values.insert(key, value);
            }
        }
    }

    values
}

fn build_setup_py_package_data(values: &HashMap<String, Value>) -> PackageData {
    let name = get_value_string(values, "name");
    let version = get_value_string(values, "version");
    let description =
        get_value_string(values, "description").or_else(|| get_value_string(values, "summary"));
    let homepage_url =
        get_value_string(values, "url").or_else(|| get_value_string(values, "home_page"));
    let author = get_value_string(values, "author");
    let author_email = get_value_string(values, "author_email");
    let maintainer = get_value_string(values, "maintainer");
    let maintainer_email = get_value_string(values, "maintainer_email");
    let license = get_value_string(values, "license");

    let mut parties = Vec::new();
    if author.is_some() || author_email.is_some() {
        parties.push(Party {
            r#type: Some("person".to_string()),
            role: Some("author".to_string()),
            name: author,
            email: author_email,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    if maintainer.is_some() || maintainer_email.is_some() {
        parties.push(Party {
            r#type: Some("person".to_string()),
            role: Some("maintainer".to_string()),
            name: maintainer,
            email: maintainer_email,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    let declared_license_expression = license.as_ref().map(|value| value.to_lowercase());
    let declared_license_expression_spdx = license.clone();
    let license_detections = license
        .as_ref()
        .map_or(Vec::new(), |value| vec![create_license_detection(value)]);
    let extracted_license_statement = license.clone();

    let dependencies = build_setup_py_dependencies(values);
    let purl = build_setup_py_purl(name.as_deref(), version.as_deref());

    PackageData {
        package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version,
        qualifiers: None,
        subpath: None,
        primary_language: None,
        description,
        release_date: None,
        parties,
        keywords: Vec::new(),
        homepage_url,
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
        declared_license_expression,
        declared_license_expression_spdx,
        license_detections,
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement,
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
        datasource_id: None,
        purl,
    }
}

fn build_setup_py_dependencies(values: &HashMap<String, Value>) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    if let Some(reqs) = values
        .get("install_requires")
        .and_then(value_to_string_list)
    {
        dependencies.extend(build_setup_py_dependency_list(&reqs, "install", false));
    }

    if let Some(reqs) = values.get("tests_require").and_then(value_to_string_list) {
        dependencies.extend(build_setup_py_dependency_list(&reqs, "test", true));
    }

    if let Some(Value::Dict(extras)) = values.get("extras_require") {
        let mut extra_items: Vec<_> = extras.iter().collect();
        extra_items.sort_by_key(|(name, _)| *name);
        for (extra_name, extra_value) in extra_items {
            if let Some(reqs) = value_to_string_list(extra_value) {
                dependencies.extend(build_setup_py_dependency_list(
                    reqs.as_slice(),
                    extra_name,
                    true,
                ));
            }
        }
    }

    dependencies
}

fn build_setup_py_dependency_list(
    reqs: &[String],
    scope: &str,
    is_optional: bool,
) -> Vec<Dependency> {
    reqs.iter()
        .filter_map(|req| build_setup_cfg_dependency(req, scope, is_optional))
        .collect()
}

fn get_value_string(values: &HashMap<String, Value>, key: &str) -> Option<String> {
    values.get(key).and_then(value_to_string)
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_to_string_list(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::String(value) => Some(vec![value.clone()]),
        Value::List(values) | Value::Tuple(values) => {
            let mut items = Vec::new();
            for item in values {
                items.push(value_to_string(item)?);
            }
            Some(items)
        }
        _ => None,
    }
}

fn build_setup_py_purl(name: Option<&str>, version: Option<&str>) -> Option<String> {
    let name = name?;
    let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, name).ok()?;
    if let Some(version) = version {
        package_url.with_version(version).ok()?;
    }
    Some(package_url.to_string())
}

fn extract_from_setup_py_regex(content: &str) -> PackageData {
    let name = extract_setup_value(content, "name");
    let version = extract_setup_value(content, "version");
    let license_expression = extract_setup_value(content, "license");

    let declared_license_expression = license_expression
        .as_ref()
        .map(|value| value.to_lowercase());
    let declared_license_expression_spdx = license_expression.clone();
    let license_detections = license_expression.as_ref().map_or(Vec::new(), |license| {
        vec![create_license_detection(license)]
    });
    let extracted_license_statement = license_expression.clone();

    let dependencies = extract_setup_py_dependencies(content);
    let homepage_url = extract_setup_value(content, "url");
    let purl = build_setup_py_purl(name.as_deref(), version.as_deref());

    PackageData {
        package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version,
        qualifiers: None,
        subpath: None,
        primary_language: None,
        description: None,
        release_date: None,
        parties: Vec::new(),
        keywords: Vec::new(),
        homepage_url,
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
        declared_license_expression,
        declared_license_expression_spdx,
        license_detections,
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement,
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
        datasource_id: None,
        purl,
    }
}

fn package_data_to_resolved(pkg: &PackageData) -> crate::models::ResolvedPackage {
    crate::models::ResolvedPackage {
        package_type: pkg
            .package_type
            .clone()
            .unwrap_or_else(|| "pypi".to_string()),
        namespace: pkg.namespace.clone().unwrap_or_default(),
        name: pkg.name.clone().unwrap_or_default(),
        version: pkg.version.clone().unwrap_or_default(),
        primary_language: pkg.primary_language.clone(),
        download_url: pkg.download_url.clone(),
        sha1: pkg.sha1.clone(),
        sha256: pkg.sha256.clone(),
        sha512: pkg.sha512.clone(),
        md5: pkg.md5.clone(),
        is_virtual: pkg.is_virtual,
        dependencies: pkg.dependencies.clone(),
        repository_homepage_url: pkg.repository_homepage_url.clone(),
        repository_download_url: pkg.repository_download_url.clone(),
        api_data_url: pkg.api_data_url.clone(),
        datasource_id: pkg.datasource_id.clone(),
        purl: pkg.purl.clone(),
    }
}

fn extract_from_pip_inspect(path: &Path) -> PackageData {
    let content = match read_file_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            warn!("Failed to read pip-inspect.deplock at {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let root: serde_json::Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(e) => {
            warn!(
                "Failed to parse pip-inspect.deplock JSON at {:?}: {}",
                path, e
            );
            return default_package_data();
        }
    };

    let installed = match root.get("installed").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            warn!(
                "No 'installed' array found in pip-inspect.deplock at {:?}",
                path
            );
            return default_package_data();
        }
    };

    let pip_version = root
        .get("pip_version")
        .and_then(|v| v.as_str())
        .map(String::from);
    let inspect_version = root
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut main_package: Option<PackageData> = None;
    let mut dependencies: Vec<Dependency> = Vec::new();

    for package_entry in installed {
        let metadata = match package_entry.get("metadata") {
            Some(m) => m,
            None => continue,
        };

        let is_requested = package_entry
            .get("requested")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_direct_url = package_entry.get("direct_url").is_some();

        let name = metadata
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from);
        let version = metadata
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from);
        let summary = metadata
            .get("summary")
            .and_then(|v| v.as_str())
            .map(String::from);
        let home_page = metadata
            .get("home_page")
            .and_then(|v| v.as_str())
            .map(String::from);
        let author = metadata
            .get("author")
            .and_then(|v| v.as_str())
            .map(String::from);
        let author_email = metadata
            .get("author_email")
            .and_then(|v| v.as_str())
            .map(String::from);
        let license = metadata
            .get("license")
            .and_then(|v| v.as_str())
            .map(String::from);
        let description = metadata
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);
        let keywords = metadata
            .get("keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|k| k.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut parties = Vec::new();
        if author.is_some() || author_email.is_some() {
            parties.push(Party {
                r#type: Some("person".to_string()),
                role: Some("author".to_string()),
                name: author,
                email: author_email,
                url: None,
                organization: None,
                organization_url: None,
                timezone: None,
            });
        }

        let license_detections = license
            .as_ref()
            .map_or(Vec::new(), |lic| vec![create_license_detection(lic)]);

        let declared_license_expression = license.as_ref().map(|l| l.to_lowercase());
        let declared_license_expression_spdx = license.clone();
        let extracted_license_statement = license.clone();

        let purl = name.as_ref().and_then(|n| {
            let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, n).ok()?;
            if let Some(v) = &version {
                package_url.with_version(v).ok()?;
            }
            Some(package_url.to_string())
        });

        if is_requested && has_direct_url {
            let mut extra_data = HashMap::new();
            if let Some(pv) = &pip_version {
                extra_data.insert(
                    "pip_version".to_string(),
                    serde_json::Value::String(pv.clone()),
                );
            }
            if let Some(iv) = &inspect_version {
                extra_data.insert(
                    "inspect_version".to_string(),
                    serde_json::Value::String(iv.clone()),
                );
            }

            main_package = Some(PackageData {
                package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
                namespace: None,
                name,
                version,
                qualifiers: None,
                subpath: None,
                primary_language: Some("Python".to_string()),
                description: description.or(summary),
                release_date: None,
                parties,
                keywords,
                homepage_url: home_page,
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
                declared_license_expression,
                declared_license_expression_spdx,
                license_detections,
                other_license_expression: None,
                other_license_expression_spdx: None,
                other_license_detections: Vec::new(),
                extracted_license_statement,
                notice_text: None,
                source_packages: Vec::new(),
                file_references: Vec::new(),
                is_private: false,
                is_virtual: true,
                extra_data: if extra_data.is_empty() {
                    None
                } else {
                    Some(extra_data)
                },
                dependencies: Vec::new(),
                repository_homepage_url: None,
                repository_download_url: None,
                api_data_url: None,
                datasource_id: Some("pypi_inspect_deplock".to_string()),
                purl,
            });
        } else {
            let resolved_package = PackageData {
                package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
                namespace: None,
                name: name.clone(),
                version: version.clone(),
                qualifiers: None,
                subpath: None,
                primary_language: Some("Python".to_string()),
                description: description.or(summary),
                release_date: None,
                parties,
                keywords,
                homepage_url: home_page,
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
                declared_license_expression,
                declared_license_expression_spdx,
                license_detections,
                other_license_expression: None,
                other_license_expression_spdx: None,
                other_license_detections: Vec::new(),
                extracted_license_statement,
                notice_text: None,
                source_packages: Vec::new(),
                file_references: Vec::new(),
                is_private: false,
                is_virtual: true,
                extra_data: None,
                dependencies: Vec::new(),
                repository_homepage_url: None,
                repository_download_url: None,
                api_data_url: None,
                datasource_id: Some("pypi_inspect_deplock".to_string()),
                purl: purl.clone(),
            };

            let resolved = package_data_to_resolved(&resolved_package);
            dependencies.push(Dependency {
                purl,
                extracted_requirement: None,
                scope: None,
                is_runtime: Some(true),
                is_optional: Some(false),
                is_pinned: Some(true),
                is_direct: Some(is_requested),
                resolved_package: Some(Box::new(resolved)),
                extra_data: None,
            });
        }
    }

    if let Some(mut main_pkg) = main_package {
        main_pkg.dependencies = dependencies;
        main_pkg
    } else {
        default_package_data()
    }
}

type IniSections = HashMap<String, HashMap<String, Vec<String>>>;

fn extract_from_setup_cfg(path: &Path) -> PackageData {
    let content = match read_file_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            warn!("Failed to read setup.cfg at {:?}: {}", path, e);
            return default_package_data();
        }
    };

    let sections = parse_setup_cfg(&content);
    let name = get_ini_value(&sections, "metadata", "name");
    let version = get_ini_value(&sections, "metadata", "version");
    let author = get_ini_value(&sections, "metadata", "author");
    let author_email = get_ini_value(&sections, "metadata", "author_email");
    let license = get_ini_value(&sections, "metadata", "license");
    let homepage_url = get_ini_value(&sections, "metadata", "url");

    let mut parties = Vec::new();
    if author.is_some() || author_email.is_some() {
        parties.push(Party {
            r#type: Some("person".to_string()),
            role: Some("author".to_string()),
            name: author,
            email: author_email,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    let declared_license_expression = license.as_ref().map(|value| value.to_lowercase());
    let declared_license_expression_spdx = license.clone();
    let license_detections = license
        .as_ref()
        .map_or(Vec::new(), |value| vec![create_license_detection(value)]);

    let extracted_license_statement = license.clone();

    let dependencies = extract_setup_cfg_dependencies(&sections);

    let purl = name.as_ref().and_then(|n| {
        let mut package_url = PackageUrl::new(PythonParser::PACKAGE_TYPE, n).ok()?;
        if let Some(v) = &version {
            package_url.with_version(v).ok()?;
        }
        Some(package_url.to_string())
    });

    PackageData {
        package_type: Some(PythonParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version,
        qualifiers: None,
        subpath: None,
        primary_language: Some("Python".to_string()),
        description: None,
        release_date: None,
        parties,
        keywords: Vec::new(),
        homepage_url,
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
        declared_license_expression,
        declared_license_expression_spdx,
        license_detections,
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement,
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
        datasource_id: Some("pypi_setup_cfg".to_string()),
        purl,
    }
}

fn parse_setup_cfg(content: &str) -> IniSections {
    let mut sections: IniSections = HashMap::new();
    let mut current_section: Option<String> = None;
    let mut current_key: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let stripped = line.trim_start();
        if stripped.starts_with('#') || stripped.starts_with(';') {
            continue;
        }

        if stripped.starts_with('[') && stripped.ends_with(']') {
            let section_name = stripped
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_ascii_lowercase();
            current_section = if section_name.is_empty() {
                None
            } else {
                Some(section_name)
            };
            current_key = None;
            continue;
        }

        if (line.starts_with(' ') || line.starts_with('\t')) && current_key.is_some() {
            if let (Some(section), Some(key)) = (current_section.as_ref(), current_key.as_ref()) {
                let value = stripped.trim();
                if !value.is_empty() {
                    sections
                        .entry(section.clone())
                        .or_default()
                        .entry(key.clone())
                        .or_default()
                        .push(value.to_string());
                }
            }
            continue;
        }

        if let Some((key, value)) = stripped.split_once('=')
            && let Some(section) = current_section.as_ref()
        {
            let key_name = key.trim().to_ascii_lowercase();
            let value_trimmed = value.trim();
            let entry = sections
                .entry(section.clone())
                .or_default()
                .entry(key_name.clone())
                .or_default();
            if !value_trimmed.is_empty() {
                entry.push(value_trimmed.to_string());
            }
            current_key = Some(key_name);
        }
    }

    sections
}

fn get_ini_value(sections: &IniSections, section: &str, key: &str) -> Option<String> {
    sections
        .get(&section.to_ascii_lowercase())
        .and_then(|values| values.get(&key.to_ascii_lowercase()))
        .and_then(|entries| entries.first())
        .map(|value| value.trim().to_string())
}

fn get_ini_values(sections: &IniSections, section: &str, key: &str) -> Vec<String> {
    sections
        .get(&section.to_ascii_lowercase())
        .and_then(|values| values.get(&key.to_ascii_lowercase()))
        .cloned()
        .unwrap_or_default()
}

fn extract_setup_cfg_dependencies(sections: &IniSections) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    for (sub_section, scope) in [
        ("install_requires", "install"),
        ("tests_require", "test"),
        ("setup_requires", "setup"),
    ] {
        let reqs = get_ini_values(sections, "options", sub_section);
        dependencies.extend(parse_setup_cfg_requirements(&reqs, scope, false));
    }

    if let Some(extras) = sections.get("options.extras_require") {
        let mut extra_items: Vec<_> = extras.iter().collect();
        extra_items.sort_by_key(|(name, _)| *name);
        for (extra_name, reqs) in extra_items {
            dependencies.extend(parse_setup_cfg_requirements(reqs, extra_name, true));
        }
    }

    dependencies
}

fn parse_setup_cfg_requirements(
    reqs: &[String],
    scope: &str,
    is_optional: bool,
) -> Vec<Dependency> {
    reqs.iter()
        .filter_map(|req| build_setup_cfg_dependency(req, scope, is_optional))
        .collect()
}

fn build_setup_cfg_dependency(req: &str, scope: &str, is_optional: bool) -> Option<Dependency> {
    let trimmed = req.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let name = extract_setup_cfg_dependency_name(trimmed)?;
    let purl = PackageUrl::new(PythonParser::PACKAGE_TYPE, &name).ok()?;

    Some(Dependency {
        purl: Some(purl.to_string()),
        extracted_requirement: Some(normalize_setup_cfg_requirement(trimmed)),
        scope: Some(scope.to_string()),
        is_runtime: Some(true),
        is_optional: Some(is_optional),
        is_pinned: Some(false),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}

fn extract_setup_cfg_dependency_name(req: &str) -> Option<String> {
    let trimmed = req.trim();
    if trimmed.is_empty() {
        return None;
    }

    let end = trimmed
        .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '=' | '!' | '~' | ';' | '['))
        .unwrap_or(trimmed.len());
    let name = trimmed[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn normalize_setup_cfg_requirement(req: &str) -> String {
    req.chars().filter(|c| !c.is_whitespace()).collect()
}

fn extract_setup_value(content: &str, key: &str) -> Option<String> {
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

            if let Some(end_idx) = remaining.find(['"', '\'']) {
                return Some(remaining[..end_idx].to_string());
            }
        }
    }

    None
}

fn extract_setup_py_dependencies(content: &str) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    if let Some(tests_deps) = extract_tests_require(content) {
        dependencies.extend(tests_deps);
    }

    if let Some(extras_deps) = extract_extras_require(content) {
        dependencies.extend(extras_deps);
    }

    dependencies
}

fn extract_tests_require(content: &str) -> Option<Vec<Dependency>> {
    let pattern = r"tests_require\s*=\s*\[([^\]]+)\]";
    let re = Regex::new(pattern).ok()?;
    let captures = re.captures(content)?;
    let deps_str = captures.get(1)?.as_str();

    let deps = parse_setup_py_dep_list(deps_str, "test", true);
    if deps.is_empty() { None } else { Some(deps) }
}

fn extract_extras_require(content: &str) -> Option<Vec<Dependency>> {
    let pattern = r"extras_require\s*=\s*\{([^}]+)\}";
    let re = Regex::new(pattern).ok()?;
    let captures = re.captures(content)?;
    let dict_content = captures.get(1)?.as_str();

    let mut all_deps = Vec::new();

    let entry_pattern = r#"['"]([^'"]+)['"]\s*:\s*\[([^\]]+)\]"#;
    let entry_re = Regex::new(entry_pattern).ok()?;

    for entry_cap in entry_re.captures_iter(dict_content) {
        if let (Some(extra_name), Some(deps_str)) = (entry_cap.get(1), entry_cap.get(2)) {
            let deps = parse_setup_py_dep_list(deps_str.as_str(), extra_name.as_str(), true);
            all_deps.extend(deps);
        }
    }

    if all_deps.is_empty() {
        None
    } else {
        Some(all_deps)
    }
}

fn parse_setup_py_dep_list(deps_str: &str, scope: &str, is_optional: bool) -> Vec<Dependency> {
    let dep_pattern = r#"['"]([^'"]+)['"]"#;
    let re = match Regex::new(dep_pattern) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    re.captures_iter(deps_str)
        .filter_map(|cap| {
            let dep_str = cap.get(1)?.as_str().trim();
            if dep_str.is_empty() {
                return None;
            }

            let name = extract_setup_cfg_dependency_name(dep_str)?;
            let purl = PackageUrl::new(PythonParser::PACKAGE_TYPE, &name).ok()?;

            Some(Dependency {
                purl: Some(purl.to_string()),
                extracted_requirement: Some(dep_str.to_string()),
                scope: Some(scope.to_string()),
                is_runtime: Some(true),
                is_optional: Some(is_optional),
                is_pinned: Some(false),
                is_direct: Some(true),
                resolved_package: None,
                extra_data: None,
            })
        })
        .collect()
}

/// Reads and parses a TOML file
pub(crate) fn read_toml_file(path: &Path) -> Result<TomlValue, String> {
    let content = read_file_to_string(path).map_err(|e| e.to_string())?;
    toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))
}

/// Calculates file size and SHA256 checksum for integrity verification in SBOMs.
///
/// Used for .whl and .egg archives to populate `size` and `sha256` fields in PackageData.
/// Essential for SBOM compliance and package integrity verification.
///
/// # Returns
///
/// - `(Some(size), Some(hash))` on success
/// - `(None, None)` if file cannot be opened
/// - `(Some(size), None)` if hash calculation fails during read
fn calculate_file_checksums(path: &Path) -> (Option<u64>, Option<String>) {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return (None, None),
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return (None, None),
    };
    let size = metadata.len();

    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 8192];

    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buffer[..n]),
            Err(_) => return (Some(size), None),
        }
    }

    let hash = format!("{:x}", hasher.finalize());
    (Some(size), Some(hash))
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: None,
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
        dependencies: Vec::new(),
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_id: None,
        purl: None,
    }
}

crate::register_parser!(
    "Python package manifests (pyproject.toml, setup.py, setup.cfg, PKG-INFO, METADATA, .whl, .egg)",
    &[
        "**/pyproject.toml",
        "**/setup.py",
        "**/setup.cfg",
        "**/PKG-INFO",
        "**/METADATA",
        "**/*.whl",
        "**/*.egg"
    ],
    "pypi",
    "Python",
    Some("https://packaging.python.org/"),
);
