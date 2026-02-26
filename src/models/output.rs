use super::{FileInfo, Package, TopLevelDependency};
use serde::Serialize;

pub const SCANCODE_OUTPUT_FORMAT_VERSION: &str = "4.0.0";

#[derive(Serialize, Debug)]
/// Top-level ScanCode-compatible JSON payload.
pub struct Output {
    pub headers: Vec<Header>,
    pub packages: Vec<Package>,
    pub dependencies: Vec<TopLevelDependency>,
    pub files: Vec<FileInfo>,
    pub license_references: Vec<LicenseReference>,
    pub license_rule_references: Vec<LicenseRuleReference>,
}

#[derive(Serialize, Debug)]
/// Scan execution metadata stored in `output.headers`.
pub struct Header {
    pub start_timestamp: String,
    pub end_timestamp: String,
    pub duration: f64,
    pub extra_data: ExtraData,
    pub errors: Vec<String>,
    pub output_format_version: String,
}

#[derive(Serialize, Debug)]
/// Additional counters and environment details for a scan run.
pub struct ExtraData {
    pub files_count: usize,
    pub directories_count: usize,
    pub excluded_count: usize,
    pub system_environment: SystemEnvironment,
}

#[derive(Serialize, Debug)]
/// Host environment information captured during scan execution.
pub struct SystemEnvironment {
    pub operating_system: Option<String>,
    pub cpu_architecture: String,
    pub platform: String,
    pub rust_version: String,
}

#[derive(Serialize, Debug)]
/// Reference entry for a detected license.
pub struct LicenseReference {
    pub name: String,
    pub short_name: String,
    pub spdx_license_key: String,
    pub text: String,
}

#[derive(Serialize, Debug)]
/// Reference metadata for a license detection rule.
pub struct LicenseRuleReference {
    pub identifier: String,
    pub license_expression: String,
    pub is_license_text: bool,
    pub is_license_notice: bool,
    pub is_license_reference: bool,
    pub is_license_tag: bool,
    pub is_license_clue: bool,
    pub is_license_intro: bool,
}
