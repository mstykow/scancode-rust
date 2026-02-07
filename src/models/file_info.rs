use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::utils::spdx::combine_license_expressions;

#[derive(Debug, Builder, Serialize)]
#[builder(build_fn(skip))]
pub struct FileInfo {
    pub name: String,
    pub base_name: String,
    pub extension: String,
    pub path: String,
    #[serde(rename = "type")] // name used by ScanCode
    pub file_type: FileType,
    #[builder(default)]
    pub mime_type: Option<String>,
    pub size: u64,
    #[builder(default)]
    pub date: Option<String>,
    #[builder(default)]
    pub sha1: Option<String>,
    #[builder(default)]
    pub md5: Option<String>,
    #[builder(default)]
    pub sha256: Option<String>,
    #[builder(default)]
    pub programming_language: Option<String>,
    #[builder(default)]
    pub package_data: Vec<PackageData>,
    #[serde(rename = "detected_license_expression_spdx")] // name used by ScanCode
    #[builder(default)]
    pub license_expression: Option<String>,
    #[builder(default)]
    pub license_detections: Vec<LicenseDetection>,
    #[builder(default)]
    pub copyrights: Vec<Copyright>,
    #[builder(default)]
    pub urls: Vec<OutputURL>,
    #[builder(default)]
    pub scan_errors: Vec<String>,
}

impl FileInfoBuilder {
    pub fn build(&self) -> Result<FileInfo, String> {
        Ok(FileInfo::new(
            self.name.clone().ok_or("Missing field: name")?,
            self.base_name.clone().ok_or("Missing field: base_name")?,
            self.extension.clone().ok_or("Missing field: extension")?,
            self.path.clone().ok_or("Missing field: path")?,
            self.file_type.clone().ok_or("Missing field: file_type")?,
            self.mime_type.clone().flatten(),
            self.size.ok_or("Missing field: size")?,
            self.date.clone().flatten(),
            self.sha1.clone().flatten(),
            self.md5.clone().flatten(),
            self.sha256.clone().flatten(),
            self.programming_language.clone().flatten(),
            self.package_data.clone().unwrap_or_default(),
            self.license_expression.clone().flatten(),
            self.license_detections.clone().unwrap_or_default(),
            self.copyrights.clone().unwrap_or_default(),
            self.urls.clone().unwrap_or_default(),
            self.scan_errors.clone().unwrap_or_default(),
        ))
    }
}

impl FileInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        base_name: String,
        extension: String,
        path: String,
        file_type: FileType,
        mime_type: Option<String>,
        size: u64,
        date: Option<String>,
        sha1: Option<String>,
        md5: Option<String>,
        sha256: Option<String>,
        programming_language: Option<String>,
        package_data: Vec<PackageData>,
        mut license_expression: Option<String>,
        mut license_detections: Vec<LicenseDetection>,
        copyrights: Vec<Copyright>,
        urls: Vec<OutputURL>,
        scan_errors: Vec<String>,
    ) -> Self {
        // Combine license expressions from package data if license_expression is None
        license_expression = license_expression.or_else(|| {
            let expressions = package_data
                .iter()
                .filter_map(|pkg| pkg.get_license_expression());
            combine_license_expressions(expressions)
        });

        // Combine license detections from package data if none are provided
        if license_detections.is_empty() {
            for pkg in &package_data {
                license_detections.extend(pkg.license_detections.clone());
            }
        }

        // Combine license expressions from license detections if license_expression is still None
        if license_expression.is_none() && !license_detections.is_empty() {
            let expressions = license_detections
                .iter()
                .map(|detection| detection.license_expression.clone());
            license_expression = combine_license_expressions(expressions);
        }

        FileInfo {
            name,
            base_name,
            extension,
            path,
            file_type,
            mime_type,
            size,
            date,
            sha1,
            md5,
            sha256,
            programming_language,
            package_data,
            license_expression,
            license_detections,
            copyrights,
            urls,
            scan_errors,
        }
    }
}

/// Package metadata extracted from manifest files.
///
/// Compatible with ScanCode Toolkit output format. Contains standardized package
/// information including name, version, dependencies, licenses, and other metadata.
/// This is the primary data structure returned by all parsers.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageData {
    #[serde(rename = "type")] // name used by ScanCode
    pub package_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifiers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    pub parties: Vec<Party>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bug_tracking_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_view_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcs_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_license_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_license_expression_spdx: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub license_detections: Vec<LicenseDetection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_license_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_license_expression_spdx: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub other_license_detections: Vec<LicenseDetection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_license_statement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice_text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub source_packages: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub file_references: Vec<FileReference>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub is_private: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub is_virtual: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dependencies: Vec<Dependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_homepage_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_data_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,
}

// Helper function for serde skip_serializing_if
fn is_false(b: &bool) -> bool {
    !b
}

impl PackageData {
    /// Extracts a single license expression from all license detections in this package.
    /// Returns None if there are no license detections.
    pub fn get_license_expression(&self) -> Option<String> {
        if self.license_detections.is_empty() {
            return None;
        }

        let expressions = self
            .license_detections
            .iter()
            .map(|detection| detection.license_expression.clone());
        combine_license_expressions(expressions)
    }
}

/// License detection result containing matched license expressions.
///
/// Aggregates multiple license matches into a single SPDX license expression.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LicenseDetection {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub matches: Vec<Match>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

/// Individual license text match with location and confidence score.
///
/// Represents a specific region of text that matched a known license pattern.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Match {
    pub license_expression: String,
    pub license_expression_spdx: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_coverage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_relevance: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_text: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Copyright {
    pub copyright: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Package dependency information with version constraints.
///
/// Represents a declared dependency with scope (e.g., runtime, dev, optional)
/// and optional resolved package details.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Dependency {
    pub purl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_requirement: Option<String>,
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_runtime: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_direct: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_package: Option<Box<ResolvedPackage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolvedPackage {
    #[serde(rename = "type")]
    pub package_type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub namespace: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    pub is_virtual: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub dependencies: Vec<Dependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_homepage_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_data_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,
}

/// Author, maintainer, or contributor information.
///
/// Represents a person or organization associated with a package.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Party {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Reference to a file within a package archive with checksums.
///
/// Used in SBOM generation to track files within distribution archives.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileReference {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct OutputURL {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    File,
    Directory,
}

impl Serialize for FileType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = match self {
            FileType::File => "file",
            FileType::Directory => "directory",
        };
        serializer.serialize_str(value)
    }
}
