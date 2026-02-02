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

#[derive(Serialize, Debug, Clone)]
pub struct PackageData {
    #[serde(rename = "type")] // name used by ScanCode
    pub package_type: Option<String>,
    pub namespace: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub homepage_url: Option<String>,
    pub download_url: Option<String>,
    pub copyright: Option<String>,
    pub license_detections: Vec<LicenseDetection>,
    pub dependencies: Vec<Dependency>,
    pub parties: Vec<Party>,
    pub purl: Option<String>,
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

#[derive(Serialize, Debug, Clone)]
pub struct LicenseDetection {
    #[serde(rename = "license_expression_spdx")] // name used by ScanCode
    pub license_expression: String,
    pub matches: Vec<Match>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Match {
    pub score: f64,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(rename = "license_expression_spdx")] // name used by ScanCode
    pub license_expression: String,
    pub rule_identifier: Option<String>,
    pub matched_text: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Copyright {
    pub copyright: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Serialize, Debug, Clone)]
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
}

#[derive(Serialize, Debug, Clone)]
pub struct ResolvedPackage {
    #[serde(rename = "type")]
    pub package_type: String,
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
    pub sha512: Option<String>,
    pub is_virtual: bool,
    pub dependencies: Vec<Dependency>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Party {
    pub email: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct OutputURL {
    pub url: String,
}

#[derive(Debug, Clone)]
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
