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

use crate::models::{DatasourceId, Dependency, PackageData, PackageType};
use packageurl::PackageUrl;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::path::Path;

use log::warn;
use rustpython_parser::{Parse, ast};

use super::PackageParser;

pub struct BazelBuildParser;

impl PackageParser for BazelBuildParser {
    const PACKAGE_TYPE: PackageType = PackageType::Bazel;

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
        package_type: Some(BazelBuildParser::PACKAGE_TYPE),
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
        package_type: Some(BazelBuildParser::PACKAGE_TYPE),
        name,
        datasource_id: Some(DatasourceId::BazelBuild),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PackageType;
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
        assert_eq!(pkg.package_type, Some(PackageType::Bazel));
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

pub struct BazelModuleParser;

impl PackageParser for BazelModuleParser {
    const PACKAGE_TYPE: PackageType = PackageType::Bazel;

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "MODULE.bazel")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        match parse_bazel_module(path) {
            Ok(package) => vec![package],
            Err(e) => {
                warn!("Failed to parse Bazel MODULE.bazel {:?}: {}", path, e);
                vec![default_bazel_module_package_data()]
            }
        }
    }
}

fn parse_bazel_module(path: &Path) -> Result<PackageData, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let module = ast::Suite::parse(&content, "<MODULE.bazel>")
        .map_err(|e| format!("Failed to parse Starlark: {}", e))?;

    let mut package = default_bazel_module_package_data();
    let mut extra_data = JsonMap::new();
    let mut dependencies = Vec::new();
    let mut overrides = Vec::new();

    for statement in &module {
        let Some(call) = extract_call(statement) else {
            continue;
        };

        let Some(function_name) = extract_call_name(call) else {
            continue;
        };

        match function_name {
            "module" => {
                package.name = extract_string_kwarg(call, "name");
                package.version = extract_string_kwarg(call, "version");
                package.purl = package
                    .name
                    .as_deref()
                    .and_then(|name| build_bazel_purl(name, package.version.as_deref()));

                if let Some(repo_name) = extract_string_kwarg(call, "repo_name") {
                    extra_data.insert("repo_name".to_string(), JsonValue::String(repo_name));
                }
                if let Some(compatibility_level) = extract_int_kwarg(call, "compatibility_level") {
                    extra_data.insert(
                        "compatibility_level".to_string(),
                        JsonValue::Number(compatibility_level.into()),
                    );
                }
                if let Some(bazel_compatibility) = extract_kwarg_json(call, "bazel_compatibility") {
                    extra_data.insert("bazel_compatibility".to_string(), bazel_compatibility);
                }
            }
            "bazel_dep" => {
                if let Some(dep) = extract_bazel_dependency(call) {
                    dependencies.push(dep);
                }
            }
            "archive_override"
            | "git_override"
            | "local_path_override"
            | "single_version_override"
            | "multiple_version_override" => {
                overrides.push(extract_override(function_name, call));
            }
            _ => {}
        }
    }

    if package.name.is_none() {
        return Ok(default_bazel_module_package_data());
    }

    if !overrides.is_empty() {
        extra_data.insert("overrides".to_string(), JsonValue::Array(overrides));
    }

    package.dependencies = dependencies;
    package.extra_data = (!extra_data.is_empty()).then(|| extra_data.into_iter().collect());
    Ok(package)
}

fn extract_call(statement: &ast::Stmt) -> Option<&ast::ExprCall> {
    match statement {
        ast::Stmt::Expr(ast::StmtExpr { value, .. }) => {
            if let ast::Expr::Call(call) = value.as_ref() {
                Some(call)
            } else {
                None
            }
        }
        ast::Stmt::Assign(ast::StmtAssign { value, .. }) => {
            if let ast::Expr::Call(call) = value.as_ref() {
                Some(call)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_call_name(call: &ast::ExprCall) -> Option<&str> {
    match call.func.as_ref() {
        ast::Expr::Name(ast::ExprName { id, .. }) => Some(id.as_str()),
        _ => None,
    }
}

fn extract_string_kwarg(call: &ast::ExprCall, key: &str) -> Option<String> {
    call.keywords.iter().find_map(|keyword| {
        let arg_name = keyword.arg.as_ref()?.as_str();
        if arg_name != key {
            return None;
        }
        match &keyword.value {
            ast::Expr::Constant(ast::ExprConstant {
                value: ast::Constant::Str(value),
                ..
            }) => Some(value.clone()),
            _ => None,
        }
    })
}

fn extract_bool_kwarg(call: &ast::ExprCall, key: &str) -> Option<bool> {
    call.keywords.iter().find_map(|keyword| {
        let arg_name = keyword.arg.as_ref()?.as_str();
        if arg_name != key {
            return None;
        }
        match &keyword.value {
            ast::Expr::Constant(ast::ExprConstant {
                value: ast::Constant::Bool(value),
                ..
            }) => Some(*value),
            _ => None,
        }
    })
}

fn extract_int_kwarg(call: &ast::ExprCall, key: &str) -> Option<i64> {
    call.keywords.iter().find_map(|keyword| {
        let arg_name = keyword.arg.as_ref()?.as_str();
        if arg_name != key {
            return None;
        }
        match &keyword.value {
            ast::Expr::Constant(ast::ExprConstant {
                value: ast::Constant::Int(value),
                ..
            }) => value.to_string().parse::<i64>().ok(),
            _ => None,
        }
    })
}

fn extract_kwarg_json(call: &ast::ExprCall, key: &str) -> Option<JsonValue> {
    call.keywords.iter().find_map(|keyword| {
        let arg_name = keyword.arg.as_ref()?.as_str();
        if arg_name != key {
            return None;
        }
        expr_to_json(&keyword.value)
    })
}

fn extract_bazel_dependency(call: &ast::ExprCall) -> Option<Dependency> {
    let name = extract_string_kwarg(call, "name")?;
    let version = extract_string_kwarg(call, "version");
    let is_dev = extract_bool_kwarg(call, "dev_dependency").unwrap_or(false);
    let mut extra_data = JsonMap::new();

    for field in ["repo_name", "max_compatibility_level", "registry"] {
        if let Some(value) = extract_kwarg_json(call, field) {
            extra_data.insert(field.to_string(), value);
        }
    }

    Some(Dependency {
        purl: build_bazel_purl(&name, version.as_deref()),
        extracted_requirement: version.clone(),
        scope: Some(if is_dev { "dev" } else { "dependencies" }.to_string()),
        is_runtime: Some(!is_dev),
        is_optional: Some(is_dev),
        is_pinned: Some(version.is_some()),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: (!extra_data.is_empty()).then(|| extra_data.into_iter().collect()),
    })
}

fn extract_override(kind: &str, call: &ast::ExprCall) -> JsonValue {
    let mut override_map = JsonMap::new();
    override_map.insert("kind".to_string(), JsonValue::String(kind.to_string()));
    for keyword in &call.keywords {
        if let Some(arg_name) = keyword.arg.as_ref().map(|arg| arg.to_string())
            && let Some(value) = expr_to_json(&keyword.value)
        {
            override_map.insert(arg_name, value);
        }
    }
    JsonValue::Object(override_map)
}

fn expr_to_json(expr: &ast::Expr) -> Option<JsonValue> {
    match expr {
        ast::Expr::Constant(ast::ExprConstant { value, .. }) => match value {
            ast::Constant::Str(value) => Some(JsonValue::String(value.clone())),
            ast::Constant::Bool(value) => Some(JsonValue::Bool(*value)),
            ast::Constant::Int(value) => value
                .to_string()
                .parse::<i64>()
                .ok()
                .map(|value| JsonValue::Number(value.into()))
                .or_else(|| Some(JsonValue::String(value.to_string()))),
            ast::Constant::None => Some(JsonValue::Null),
            _ => None,
        },
        ast::Expr::List(ast::ExprList { elts, .. })
        | ast::Expr::Tuple(ast::ExprTuple { elts, .. }) => Some(JsonValue::Array(
            elts.iter().filter_map(expr_to_json).collect(),
        )),
        ast::Expr::Dict(ast::ExprDict { keys, values, .. }) => {
            let mut map = JsonMap::new();
            for (key, value) in keys.iter().zip(values.iter()) {
                let Some(ast::Expr::Constant(ast::ExprConstant {
                    value: ast::Constant::Str(key),
                    ..
                })) = key
                else {
                    continue;
                };
                if let Some(value) = expr_to_json(value) {
                    map.insert(key.clone(), value);
                }
            }
            Some(JsonValue::Object(map))
        }
        _ => None,
    }
}

fn build_bazel_purl(name: &str, version: Option<&str>) -> Option<String> {
    let mut purl = PackageUrl::new("bazel", name).ok()?;
    if let Some(version) = version.filter(|value| !value.trim().is_empty()) {
        purl.with_version(version).ok()?;
    }
    Some(purl.to_string())
}

fn default_bazel_module_package_data() -> PackageData {
    PackageData {
        package_type: Some(BazelModuleParser::PACKAGE_TYPE),
        datasource_id: Some(DatasourceId::BazelModule),
        ..Default::default()
    }
}

crate::register_parser!(
    "Bazel MODULE.bazel file",
    &["**/MODULE.bazel"],
    "bazel",
    "",
    Some("https://bazel.build/external/module"),
);
