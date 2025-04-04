use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Builder, Serialize)]
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
    pub rule_identifier: String,
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
    pub scope: Option<String>,
    pub is_optional: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct Party {
    pub email: StringOrArray,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum StringOrArray {
    String(String),
    Array(Vec<String>),
    Null,
}
