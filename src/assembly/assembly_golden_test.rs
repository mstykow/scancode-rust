#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use regex::Regex;
    use serde_json::{Value, json};

    use crate::assembly::{AssemblyResult, assemble};
    use crate::models::{FileInfo, FileType};
    use crate::parsers::try_parse_file;

    /// Normalize all UUID v4 values to a fixed placeholder for deterministic testing.
    ///
    /// Replaces `uuid=<any-uuid-v4>` with `uuid=fixed-uid-done-for-testing-5642512d1758`
    /// to match the format used in cocoapods golden tests.
    fn normalize_uuids(json_str: &str) -> String {
        // UUID v4 pattern: 8-4-4-4-12 hex chars
        let re = Regex::new(r"uuid=[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
            .unwrap();
        re.replace_all(json_str, "uuid=fixed-uid-done-for-testing-5642512d1758")
            .to_string()
    }

    /// Build FileInfo objects from real files in a test directory.
    ///
    /// This discovers all parseable files in the directory (recursively), runs the appropriate parser,
    /// and constructs FileInfo objects with relative paths (required for proper assembly grouping).
    fn build_file_infos_from_directory(test_dir: &Path) -> Result<Vec<FileInfo>, String> {
        let mut file_infos = Vec::new();

        visit_dir_recursive(test_dir, test_dir, &mut file_infos)?;

        if file_infos.is_empty() {
            return Err(format!(
                "No parseable files found in directory: {:?}",
                test_dir
            ));
        }

        // Sort by path for deterministic order across platforms
        // (fs::read_dir order is OS-dependent)
        file_infos.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(file_infos)
    }

    fn visit_dir_recursive(
        dir: &Path,
        base_dir: &Path,
        file_infos: &mut Vec<FileInfo>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                visit_dir_recursive(&path, base_dir, file_infos)?;
                continue;
            }

            if !path.is_file() {
                continue;
            }

            if path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .contains("expected.json")
            {
                continue;
            }

            if let Some(package_data_vec) = try_parse_file(&path) {
                let relative_path = path
                    .strip_prefix(base_dir)
                    .map_err(|e| format!("Failed to strip prefix: {}", e))?
                    .to_str()
                    .ok_or_else(|| format!("Invalid path: {:?}", path))?
                    .to_string();

                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let extension = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();

                let metadata = fs::metadata(&path)
                    .map_err(|e| format!("Failed to read file metadata: {}", e))?;
                let size = metadata.len();

                let file_info = FileInfo {
                    name: file_name.clone(),
                    base_name: file_name.clone(),
                    extension,
                    path: relative_path,
                    file_type: FileType::File,
                    mime_type: Some("text/plain".to_string()),
                    size,
                    date: None,
                    sha1: None,
                    md5: None,
                    sha256: None,
                    programming_language: None,
                    package_data: package_data_vec,
                    license_expression: None,
                    license_detections: vec![],
                    copyrights: vec![],
                    urls: vec![],
                    for_packages: vec![],
                    scan_errors: vec![],
                };

                file_infos.push(file_info);
            }
        }

        Ok(())
    }

    /// Compare assembly output against expected JSON file.
    ///
    /// Normalizes UUIDs before comparison and validates the structure matches.
    fn compare_assembly_output(
        actual: &AssemblyResult,
        expected_path: &Path,
    ) -> Result<(), String> {
        // Read expected file
        let expected_str = fs::read_to_string(expected_path)
            .map_err(|e| format!("Failed to read expected file: {}", e))?;

        // Serialize actual result to JSON
        let actual_json = json!({
            "packages": actual.packages,
            "dependencies": actual.dependencies,
        });
        let actual_str = serde_json::to_string_pretty(&actual_json)
            .map_err(|e| format!("Failed to serialize actual result: {}", e))?;

        // Normalize UUIDs in both
        let actual_normalized = normalize_uuids(&actual_str);
        let expected_normalized = normalize_uuids(&expected_str);

        // Parse normalized strings back to JSON for comparison
        let actual_value: Value = serde_json::from_str(&actual_normalized)
            .map_err(|e| format!("Failed to parse normalized actual JSON: {}", e))?;
        let expected_value: Value = serde_json::from_str(&expected_normalized)
            .map_err(|e| format!("Failed to parse normalized expected JSON: {}", e))?;

        // Deep compare
        compare_json_values(&actual_value, &expected_value, "")
    }

    /// Recursively compare two JSON values with helpful error messages.
    fn compare_json_values(actual: &Value, expected: &Value, path: &str) -> Result<(), String> {
        match (actual, expected) {
            (Value::Null, Value::Null) => Ok(()),
            (Value::Bool(a), Value::Bool(e)) if a == e => Ok(()),
            (Value::Number(a), Value::Number(e)) if a == e => Ok(()),
            (Value::String(a), Value::String(e)) if a == e => Ok(()),

            (Value::Array(a), Value::Array(e)) => {
                if a.len() != e.len() {
                    return Err(format!(
                        "Array length mismatch at {}: actual={}, expected={}",
                        path,
                        a.len(),
                        e.len()
                    ));
                }
                for (i, (actual_item, expected_item)) in a.iter().zip(e.iter()).enumerate() {
                    let item_path = if path.is_empty() {
                        format!("[{}]", i)
                    } else {
                        format!("{}[{}]", path, i)
                    };
                    compare_json_values(actual_item, expected_item, &item_path)?;
                }
                Ok(())
            }

            (Value::Object(a), Value::Object(e)) => {
                // Check all keys exist in both objects
                for key in e.keys() {
                    if !a.contains_key(key) {
                        // Allow missing empty values
                        match e.get(key) {
                            Some(Value::Null) => continue,
                            Some(Value::Array(arr)) if arr.is_empty() => continue,
                            Some(Value::Object(obj)) if obj.is_empty() => continue,
                            _ => {
                                let field_path = if path.is_empty() {
                                    key.to_string()
                                } else {
                                    format!("{}.{}", path, key)
                                };
                                return Err(format!("Missing key in actual: {}", field_path));
                            }
                        }
                    }
                }

                // Check no extra keys in actual
                for key in a.keys() {
                    if !e.contains_key(key) {
                        // Allow extra empty values
                        match a.get(key) {
                            Some(Value::Null) => continue,
                            Some(Value::Array(arr)) if arr.is_empty() => continue,
                            Some(Value::Object(obj)) if obj.is_empty() => continue,
                            _ => {
                                let field_path = if path.is_empty() {
                                    key.to_string()
                                } else {
                                    format!("{}.{}", path, key)
                                };
                                return Err(format!("Extra key in actual: {}", field_path));
                            }
                        }
                    }
                }

                // Compare common keys
                for key in a.keys() {
                    if let (Some(actual_val), Some(expected_val)) = (a.get(key), e.get(key)) {
                        let field_path = if path.is_empty() {
                            key.to_string()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        compare_json_values(actual_val, expected_val, &field_path)?;
                    }
                }
                Ok(())
            }

            _ => Err(format!(
                "Type or value mismatch at {}: actual={}, expected={}",
                path,
                serde_json::to_string(actual).unwrap_or_default(),
                serde_json::to_string(expected).unwrap_or_default()
            )),
        }
    }

    /// Run assembly on a test directory and compare against expected output.
    fn run_assembly_golden_test(test_dir_name: &str) -> Result<(), String> {
        let test_dir = PathBuf::from("testdata/assembly-golden").join(test_dir_name);
        let expected_file = test_dir.join("expected.json");

        if !test_dir.exists() {
            return Err(format!("Test directory does not exist: {:?}", test_dir));
        }

        if !expected_file.exists() {
            // Generate expected file on first run
            eprintln!("Expected file not found, generating: {:?}", expected_file);

            let mut file_infos = build_file_infos_from_directory(&test_dir)?;
            let result = assemble(&mut file_infos);

            let output_json = json!({
                "packages": result.packages,
                "dependencies": result.dependencies,
            });
            let output_str = serde_json::to_string_pretty(&output_json)
                .map_err(|e| format!("Failed to serialize output: {}", e))?;

            let normalized = normalize_uuids(&output_str);

            fs::write(&expected_file, normalized)
                .map_err(|e| format!("Failed to write expected file: {}", e))?;

            return Err(format!(
                "Expected file generated at {:?}. Please review and re-run test.",
                expected_file
            ));
        }

        // Build FileInfo from real files
        let mut file_infos = build_file_infos_from_directory(&test_dir)?;

        // Run assembly
        let result = assemble(&mut file_infos);

        // Compare against expected
        compare_assembly_output(&result, &expected_file)
    }

    #[test]
    fn test_assembly_npm_basic() {
        match run_assembly_golden_test("npm-basic") {
            Ok(_) => (),
            Err(e) => panic!("Assembly golden test failed for npm-basic: {}", e),
        }
    }

    #[test]
    fn test_assembly_cargo_basic() {
        match run_assembly_golden_test("cargo-basic") {
            Ok(_) => (),
            Err(e) => panic!("Assembly golden test failed for cargo-basic: {}", e),
        }
    }

    #[test]
    fn test_assembly_go_basic() {
        match run_assembly_golden_test("go-basic") {
            Ok(_) => (),
            Err(e) => panic!("Assembly golden test failed for go-basic: {}", e),
        }
    }

    #[test]
    fn test_assembly_composer_basic() {
        match run_assembly_golden_test("composer-basic") {
            Ok(_) => (),
            Err(e) => panic!("Assembly golden test failed for composer-basic: {}", e),
        }
    }

    #[test]
    fn test_assembly_maven_basic() {
        match run_assembly_golden_test("maven-basic") {
            Ok(_) => (),
            Err(e) => panic!("Assembly golden test failed for maven-basic: {}", e),
        }
    }

    #[test]
    fn test_assembly_npm_workspace() {
        match run_assembly_golden_test("npm-workspace") {
            Ok(_) => (),
            Err(e) => panic!("Assembly golden test failed for npm-workspace: {}", e),
        }
    }

    #[test]
    fn test_assembly_pnpm_workspace() {
        match run_assembly_golden_test("pnpm-workspace") {
            Ok(_) => (),
            Err(e) => panic!("Assembly golden test failed for pnpm-workspace: {}", e),
        }
    }

    #[test]
    fn test_uuid_normalization() {
        let input =
            r#"{"package_uid": "pkg:npm/test@1.0.0?uuid=12345678-1234-1234-1234-123456789abc"}"#;
        let expected =
            r#"{"package_uid": "pkg:npm/test@1.0.0?uuid=fixed-uid-done-for-testing-5642512d1758"}"#;
        let actual = normalize_uuids(input);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_uuid_normalization_multiple() {
        let input = r#"{"pkg1": "pkg:npm/a@1.0.0?uuid=aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "pkg2": "pkg:npm/b@2.0.0?uuid=bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"}"#;
        let expected = r#"{"pkg1": "pkg:npm/a@1.0.0?uuid=fixed-uid-done-for-testing-5642512d1758", "pkg2": "pkg:npm/b@2.0.0?uuid=fixed-uid-done-for-testing-5642512d1758"}"#;
        let actual = normalize_uuids(input);
        assert_eq!(actual, expected);
    }
}
