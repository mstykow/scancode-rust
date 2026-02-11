//! Buck BUILD and METADATA.bzl parsers
//!
//! Extracts package metadata from Buck build system files using Starlark (Python-like) syntax.
//!
//! ## Features
//! - **BuckBuildParser**: Parses BUCK files with multiple package support
//! - **BuckMetadataBzlParser**: Parses METADATA.bzl dictionary assignments with package_url support
//!
//! ## Usage
//! - `BuckBuildParser::extract_packages()` - Returns ALL packages from BUCK file
//! - `BuckMetadataBzlParser::extract_first_package()` - Returns single package from METADATA.bzl
//!
//! ## Reference
//! Python implementation: `reference/scancode-toolkit/src/packagedcode/build.py`
//! - BuckPackageHandler (lines 310-325)
//! - BuckMetadataBzlHandler (lines 328-432)

use std::collections::HashMap;
use std::path::Path;

use log::warn;
use packageurl::PackageUrl;
use rustpython_parser::{Parse, ast};

use crate::models::{DatasourceId, PackageData, PackageType, Party};

use super::PackageParser;

/// Parser for Buck BUCK files (build rules)
pub struct BuckBuildParser;

impl PackageParser for BuckBuildParser {
    const PACKAGE_TYPE: PackageType = PackageType::Buck;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "BUCK")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        match parse_buck_build(path) {
            Ok(packages) if !packages.is_empty() => packages,
            Ok(_) => vec![fallback_package_data(path)],
            Err(e) => {
                warn!("Failed to parse Buck BUCK file {:?}: {}", path, e);
                vec![fallback_package_data(path)]
            }
        }
    }
}

/// Parser for Buck METADATA.bzl files (metadata dictionaries)
pub struct BuckMetadataBzlParser;

impl PackageParser for BuckMetadataBzlParser {
    const PACKAGE_TYPE: PackageType = PackageType::Buck;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "METADATA.bzl")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        vec![match parse_metadata_bzl(path) {
            Ok(pkg) => pkg,
            Err(e) => {
                warn!("Failed to parse Buck METADATA.bzl {:?}: {}", path, e);
                PackageData {
                    package_type: Some(Self::PACKAGE_TYPE),
                    datasource_id: Some(DatasourceId::BuckMetadata),
                    ..Default::default()
                }
            }
        }]
    }
}

/// Parse a Buck BUCK file (same logic as Bazel BUILD)
fn parse_buck_build(path: &Path) -> Result<Vec<PackageData>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let module = ast::Suite::parse(&content, "<BUCK>")
        .map_err(|e| format!("Failed to parse Starlark: {}", e))?;

    let mut packages = Vec::new();

    for statement in &module {
        if let Some(package_data) = extract_from_statement(statement) {
            packages.push(package_data);
        }
    }

    Ok(packages)
}

/// Parse a Buck METADATA.bzl file
fn parse_metadata_bzl(path: &Path) -> Result<PackageData, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let module = ast::Suite::parse(&content, "<METADATA.bzl>")
        .map_err(|e| format!("Failed to parse Starlark: {}", e))?;

    // Look for METADATA = {...} assignment
    for statement in &module {
        if let ast::Stmt::Assign(ast::StmtAssign { targets, value, .. }) = statement {
            // Check if assigning to variable named "METADATA"
            for target in targets {
                if let ast::Expr::Name(ast::ExprName { id, .. }) = target
                    && id.as_str() == "METADATA"
                {
                    // Extract dictionary contents
                    if let ast::Expr::Dict(dict) = value.as_ref() {
                        return Ok(extract_metadata_dict(dict));
                    }
                }
            }
        }
    }

    // No METADATA found
    Ok(PackageData {
        package_type: Some(BuckMetadataBzlParser::PACKAGE_TYPE),
        datasource_id: Some(DatasourceId::BuckMetadata),
        ..Default::default()
    })
}

/// Extract metadata from a dictionary AST node
fn extract_metadata_dict(dict: &ast::ExprDict) -> PackageData {
    let mut fields: HashMap<String, MetadataValue> = HashMap::new();

    for (key, value) in dict.keys.iter().zip(dict.values.iter()) {
        // Extract key name
        let key_name = match key {
            Some(ast::Expr::Constant(ast::ExprConstant { value, .. })) => {
                if let ast::Constant::Str(s) = value {
                    s.clone()
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        // Extract value
        let metadata_value = match value {
            ast::Expr::Constant(ast::ExprConstant {
                value: ast::Constant::Str(s),
                ..
            }) => MetadataValue::String(s.clone()),
            ast::Expr::Constant(_) => continue,
            ast::Expr::List(ast::ExprList { elts, .. }) => {
                let mut list_values = Vec::new();
                for elt in elts {
                    if let ast::Expr::Constant(ast::ExprConstant { value, .. }) = elt
                        && let ast::Constant::Str(s) = value
                    {
                        list_values.push(s.clone());
                    }
                }
                MetadataValue::List(list_values)
            }
            _ => continue,
        };

        fields.insert(key_name, metadata_value);
    }

    build_package_from_metadata(fields)
}

/// Metadata value types
enum MetadataValue {
    String(String),
    List(Vec<String>),
}

/// Build PackageData from extracted metadata fields
fn build_package_from_metadata(fields: HashMap<String, MetadataValue>) -> PackageData {
    let mut pkg = PackageData {
        package_type: Some(BuckMetadataBzlParser::PACKAGE_TYPE),
        datasource_id: Some(DatasourceId::BuckMetadata),
        ..Default::default()
    };

    // Extract name
    if let Some(MetadataValue::String(s)) = fields.get("name") {
        pkg.name = Some(s.clone());
    }

    // Extract version
    if let Some(MetadataValue::String(s)) = fields.get("version") {
        pkg.version = Some(s.clone());
    }

    // Extract package type (upstream_type or package_type)
    if let Some(MetadataValue::String(s)) = fields.get("upstream_type") {
        pkg.package_type = s.parse::<PackageType>().ok();
    } else if let Some(MetadataValue::String(s)) = fields.get("package_type") {
        pkg.package_type = s.parse::<PackageType>().ok();
    }

    // Extract licenses (licenses or license_expression)
    if let Some(MetadataValue::List(licenses)) = fields.get("licenses") {
        pkg.extracted_license_statement = Some(licenses.join(", "));
    } else if let Some(MetadataValue::String(s)) = fields.get("license_expression") {
        pkg.extracted_license_statement = Some(s.clone());
    }

    // Extract homepage (upstream_address or homepage_url)
    if let Some(MetadataValue::String(s)) = fields.get("upstream_address") {
        pkg.homepage_url = Some(s.clone());
    } else if let Some(MetadataValue::String(s)) = fields.get("homepage_url") {
        pkg.homepage_url = Some(s.clone());
    }

    // Extract download_url
    if let Some(MetadataValue::String(s)) = fields.get("download_url") {
        pkg.download_url = Some(s.clone());
    }

    // Extract vcs_url
    if let Some(MetadataValue::String(s)) = fields.get("vcs_url") {
        pkg.vcs_url = Some(s.clone());
    }

    // Extract sha1 (download_archive_sha1)
    if let Some(MetadataValue::String(s)) = fields.get("download_archive_sha1") {
        pkg.sha1 = Some(s.clone());
    }

    // Extract maintainers
    if let Some(MetadataValue::List(maintainers)) = fields.get("maintainers") {
        pkg.parties = maintainers
            .iter()
            .map(|name| Party {
                r#type: Some("organization".to_string()),
                name: Some(name.clone()),
                role: Some("maintainer".to_string()),
                email: None,
                url: None,
                organization: None,
                organization_url: None,
                timezone: None,
            })
            .collect();
    }

    // Extract extra_data fields
    let mut extra_data = HashMap::new();
    if let Some(MetadataValue::String(s)) = fields.get("vcs_commit_hash") {
        extra_data.insert(
            "vcs_commit_hash".to_string(),
            serde_json::Value::String(s.clone()),
        );
    }
    if let Some(MetadataValue::String(s)) = fields.get("upstream_hash") {
        extra_data.insert(
            "upstream_hash".to_string(),
            serde_json::Value::String(s.clone()),
        );
    }
    if !extra_data.is_empty() {
        pkg.extra_data = Some(extra_data);
    }

    // Parse package_url if present and update package fields
    if let Some(MetadataValue::String(purl_str)) = fields.get("package_url")
        && let Ok(purl) = purl_str.parse::<PackageUrl>()
    {
        // Override package fields with purl data
        pkg.package_type = purl.ty().parse::<PackageType>().ok();
        if let Some(ns) = purl.namespace() {
            pkg.namespace = Some(ns.to_string());
        }
        pkg.name = Some(purl.name().to_string());
        if let Some(ver) = purl.version() {
            pkg.version = Some(ver.to_string());
        }
        // Qualifiers
        if !purl.qualifiers().is_empty() {
            let quals: HashMap<String, String> = purl
                .qualifiers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            pkg.qualifiers = Some(quals);
        }
        // Subpath
        if let Some(sp) = purl.subpath() {
            pkg.subpath = Some(sp.to_string());
        }
    }

    pkg
}

/// Extract package data from a single AST statement (for BUCK files)
fn extract_from_statement(statement: &ast::Stmt) -> Option<PackageData> {
    match statement {
        ast::Stmt::Expr(ast::StmtExpr { value, .. }) => {
            if let ast::Expr::Call(call) = value.as_ref() {
                return extract_from_call(call);
            }
        }
        ast::Stmt::Assign(ast::StmtAssign { value, .. }) => {
            if let ast::Expr::Call(call) = value.as_ref() {
                return extract_from_call(call);
            }
        }
        _ => {}
    }
    None
}

/// Extract package data from a function call (for BUCK files)
fn extract_from_call(call: &ast::ExprCall) -> Option<PackageData> {
    let rule_name = match call.func.as_ref() {
        ast::Expr::Name(ast::ExprName { id, .. }) => id.as_str(),
        _ => return None,
    };

    if !check_rule_name_ending(rule_name) {
        return None;
    }

    let mut name: Option<String> = None;
    let mut licenses: Option<Vec<String>> = None;

    for keyword in &call.keywords {
        let arg_name = keyword.arg.as_ref()?.as_str();

        match arg_name {
            "name" => {
                if let ast::Expr::Constant(ast::ExprConstant { value, .. }) = &keyword.value
                    && let ast::Constant::Str(s) = value
                {
                    name = Some(s.clone());
                }
            }
            "licenses" => {
                if let ast::Expr::List(ast::ExprList { elts, .. }) = &keyword.value {
                    let mut license_list = Vec::new();
                    for elt in elts {
                        if let ast::Expr::Constant(ast::ExprConstant { value, .. }) = elt
                            && let ast::Constant::Str(s) = value
                        {
                            license_list.push(s.clone());
                        }
                    }
                    if !license_list.is_empty() {
                        licenses = Some(license_list);
                    }
                }
            }
            _ => {}
        }
    }

    let package_name = name?;

    Some(PackageData {
        package_type: Some(BuckBuildParser::PACKAGE_TYPE),
        name: Some(package_name),
        extracted_license_statement: licenses.map(|l| l.join(", ")),
        datasource_id: Some(DatasourceId::BuckFile),
        ..Default::default()
    })
}

/// Check if rule name ends with "binary" or "library"
fn check_rule_name_ending(rule_name: &str) -> bool {
    rule_name.ends_with("binary") || rule_name.ends_with("library")
}

/// Create fallback package data using parent directory name
fn fallback_package_data(path: &Path) -> PackageData {
    let name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    PackageData {
        package_type: Some(BuckBuildParser::PACKAGE_TYPE),
        name,
        datasource_id: Some(DatasourceId::BuckFile),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_buck_build_is_match() {
        assert!(BuckBuildParser::is_match(&PathBuf::from("BUCK")));
        assert!(BuckBuildParser::is_match(&PathBuf::from("path/to/BUCK")));
        assert!(!BuckBuildParser::is_match(&PathBuf::from("BUILD")));
        assert!(!BuckBuildParser::is_match(&PathBuf::from("buck")));
    }

    #[test]
    fn test_metadata_bzl_is_match() {
        assert!(BuckMetadataBzlParser::is_match(&PathBuf::from(
            "METADATA.bzl"
        )));
        assert!(BuckMetadataBzlParser::is_match(&PathBuf::from(
            "path/to/METADATA.bzl"
        )));
        assert!(!BuckMetadataBzlParser::is_match(&PathBuf::from(
            "metadata.bzl"
        )));
        assert!(!BuckMetadataBzlParser::is_match(&PathBuf::from("METADATA")));
    }

    #[test]
    fn test_check_rule_name_ending() {
        assert!(check_rule_name_ending("android_binary"));
        assert!(check_rule_name_ending("android_library"));
        assert!(check_rule_name_ending("java_binary"));
        assert!(!check_rule_name_ending("filegroup"));
    }
}

crate::register_parser!(
    "Buck build file and METADATA.bzl",
    &["**/BUCK", "**/METADATA.bzl"],
    "buck",
    "",
    Some("https://buck.build/"),
);
