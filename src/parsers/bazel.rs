//! Bazel BUILD file parser
//!
//! Extracts package metadata from Bazel BUILD files using Starlark (Python-like) syntax.
//!
//! ## Features
//! - Parses Starlark syntax using rustpython_parser
//! - Extracts build rules ending with "binary" or "library" (e.g., cc_binary, cc_library)
//! - Extracts name and licenses fields from rule arguments
//! - Falls back to parent directory name if no rules found
//! - **Supports multiple packages**: `extract_packages()` returns all rules (100% parity)
//!
//! ## Usage
//! - `extract_first_package()` - Returns first package (convenience method)
//! - `extract_packages()` - Returns ALL packages (recommended for BUILD files)
//!
//! ## Reference
//! Python implementation: `reference/scancode-toolkit/src/packagedcode/build.py` (BazelBuildHandler)

use crate::models::DatasourceId;
use std::path::Path;

use log::warn;
use rustpython_parser::{Parse, ast};

use crate::models::PackageData;

use super::PackageParser;

pub struct BazelBuildParser;

impl PackageParser for BazelBuildParser {
    const PACKAGE_TYPE: &'static str = "bazel";

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "BUILD")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        match parse_bazel_build(path) {
            Ok(packages) if !packages.is_empty() => packages,
            Ok(_) => vec![fallback_package_data(path)],
            Err(e) => {
                warn!("Failed to parse Bazel BUILD file {:?}: {}", path, e);
                vec![fallback_package_data(path)]
            }
        }
    }
}

/// Parse a Bazel BUILD file and extract all package data
fn parse_bazel_build(path: &Path) -> Result<Vec<PackageData>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let module = ast::Suite::parse(&content, "<BUILD>")
        .map_err(|e| format!("Failed to parse Starlark: {}", e))?;

    let mut packages = Vec::new();

    for statement in &module {
        if let Some(package_data) = extract_from_statement(statement) {
            packages.push(package_data);
        }
    }

    Ok(packages)
}

/// Extract package data from a single AST statement
fn extract_from_statement(statement: &ast::Stmt) -> Option<PackageData> {
    match statement {
        // Direct function call: cc_binary(name="foo", ...)
        ast::Stmt::Expr(ast::StmtExpr { value, .. }) => {
            if let ast::Expr::Call(call) = value.as_ref() {
                return extract_from_call(call);
            }
        }
        // Assignment to function call: x = cc_binary(name="foo", ...)
        ast::Stmt::Assign(ast::StmtAssign { value, .. }) => {
            if let ast::Expr::Call(call) = value.as_ref() {
                return extract_from_call(call);
            }
        }
        _ => {}
    }
    None
}

/// Extract package data from a function call
fn extract_from_call(call: &ast::ExprCall) -> Option<PackageData> {
    // Get the function name
    let rule_name = match call.func.as_ref() {
        ast::Expr::Name(ast::ExprName { id, .. }) => id.as_str(),
        _ => return None,
    };

    // Check if rule name ends with "binary" or "library"
    if !check_rule_name_ending(rule_name) {
        return None;
    }

    // Extract arguments
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

    // Must have a name to create a package
    let package_name = name?;

    Some(PackageData {
        package_type: Some(BazelBuildParser::PACKAGE_TYPE.to_string()),
        name: Some(package_name),
        extracted_license_statement: licenses.map(|l| l.join(", ")),
        datasource_id: Some(DatasourceId::BazelBuild),
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
        package_type: Some(BazelBuildParser::PACKAGE_TYPE.to_string()),
        name,
        datasource_id: Some(DatasourceId::BazelBuild),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_match() {
        assert!(BazelBuildParser::is_match(&PathBuf::from("BUILD")));
        assert!(BazelBuildParser::is_match(&PathBuf::from("path/to/BUILD")));
        assert!(!BazelBuildParser::is_match(&PathBuf::from("BUILD.bazel")));
        assert!(!BazelBuildParser::is_match(&PathBuf::from("build")));
        assert!(!BazelBuildParser::is_match(&PathBuf::from("BUCK")));
    }

    #[test]
    fn test_check_rule_name_ending() {
        assert!(check_rule_name_ending("cc_binary"));
        assert!(check_rule_name_ending("cc_library"));
        assert!(check_rule_name_ending("java_binary"));
        assert!(check_rule_name_ending("py_library"));
        assert!(!check_rule_name_ending("filegroup"));
        assert!(!check_rule_name_ending("load"));
        assert!(!check_rule_name_ending("cc_test"));
    }

    #[test]
    fn test_fallback_package_data() {
        let path = PathBuf::from("/path/to/myproject/BUILD");
        let pkg = fallback_package_data(&path);
        assert_eq!(pkg.package_type, Some("bazel".to_string()));
        assert_eq!(pkg.name, Some("myproject".to_string()));
    }
}

crate::register_parser!(
    "Bazel BUILD file",
    &["**/BUILD"],
    "bazel",
    "",
    Some("https://bazel.build/"),
);
