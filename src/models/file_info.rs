use serde::{Serialize, Deserialize};

#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub base_name: String,
    pub extension: String,
    pub path: String,
    #[serde(rename = "type")] // name used by ScanCode
    pub file_type: FileType,
    pub mime_type: Option<String>,
    pub size: u64,
    pub date: Option<String>,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub sha256: Option<String>,
    pub programming_language: Option<String>,
    #[serde(rename = "detected_license_expression_spdx")] // name used by ScanCode
    pub license_expression: Option<String>,
    pub license_detections: Vec<LicenseDetection>,
    pub copyrights: Vec<Copyright>,
    pub urls: Vec<OutputURL>,
    pub scan_errors: Vec<String>,
}

#[derive(Serialize, Debug)]
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

#[derive(Serialize, Debug)]
pub struct LicenseDetection {
    #[serde(rename = "license_expression_spdx")] // name used by ScanCode
    pub license_expression: String,
    pub matches: Vec<Match>,
}

#[derive(Serialize, Debug)]
pub struct Match {
    pub score: f64,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(rename = "license_expression_spdx")] // name used by ScanCode
    pub license_expression: String,
    pub rule_identifier: String,
    pub matched_text: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Copyright {
    pub copyright: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Serialize, Debug)]
pub struct Dependency {
    pub purl: Option<String>,
    pub scope: Option<String>,
    pub is_optional: bool,
}

#[derive(Serialize, Debug)]
pub struct Party {
    pub email: StringOrArray,
}

#[derive(Serialize, Debug)]
pub struct OutputURL {
    pub url: String,
}

#[derive(Debug)]
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