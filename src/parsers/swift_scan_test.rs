#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    use regex::Regex;
    use serde_json::{Value, json};

    use super::super::scan_pipeline_test_utils::strip_root_paths;
    use crate::assembly;
    use crate::cache::{DEFAULT_CACHE_DIR_NAME, build_collection_exclude_patterns};
    use crate::progress::{ProgressMode, ScanProgress};
    use crate::scanner::{TextDetectionOptions, collect_paths, process_collected};

    fn normalize_test_uuids(json_str: &str) -> String {
        let re = Regex::new(r"uuid=[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
            .expect("uuid regex should compile");
        re.replace_all(json_str, "uuid=fixed-uid-done-for-testing-5642512d1758")
            .to_string()
    }

    fn compare_scan_json_values(
        actual: &Value,
        expected: &Value,
        path: &str,
    ) -> Result<(), String> {
        if path.ends_with("package_data") {
            return Ok(());
        }

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

                for (index, (actual_item, expected_item)) in a.iter().zip(e.iter()).enumerate() {
                    let item_path = if path.is_empty() {
                        format!("[{}]", index)
                    } else {
                        format!("{}[{}]", path, index)
                    };
                    compare_scan_json_values(actual_item, expected_item, &item_path)?;
                }

                Ok(())
            }
            (Value::Object(a), Value::Object(e)) => {
                if path.ends_with("resolved_package") && e.is_empty() {
                    return Ok(());
                }

                for key in e.keys() {
                    if !a.contains_key(key) {
                        match e.get(key) {
                            Some(Value::Null) => continue,
                            Some(Value::Bool(false)) => continue,
                            Some(Value::Array(values)) if values.is_empty() => continue,
                            Some(Value::Object(values)) if values.is_empty() => continue,
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

                for key in a.keys() {
                    if !e.contains_key(key) {
                        if path.ends_with("extra_data") {
                            continue;
                        }

                        match a.get(key) {
                            Some(Value::Null) => continue,
                            Some(Value::Bool(false)) => continue,
                            Some(Value::Array(values)) if values.is_empty() => continue,
                            Some(Value::Object(values)) if values.is_empty() => continue,
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

                for key in a.keys() {
                    if let (Some(actual_val), Some(expected_val)) = (a.get(key), e.get(key)) {
                        let field_path = if path.is_empty() {
                            key.to_string()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        compare_scan_json_values(actual_val, expected_val, &field_path)?;
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

    fn normalize_scan_json(value: &mut Value, parent_key: Option<&str>) {
        match value {
            Value::Array(values) => {
                for item in values.iter_mut() {
                    normalize_scan_json(item, parent_key);
                }

                if parent_key.is_some_and(|key| {
                    matches!(
                        key,
                        "packages"
                            | "dependencies"
                            | "files"
                            | "package_data"
                            | "datafile_paths"
                            | "datasource_ids"
                            | "for_packages"
                    )
                }) {
                    values
                        .sort_by_cached_key(|item| serde_json::to_string(item).unwrap_or_default());
                }
            }
            Value::Object(map) => {
                for (key, item) in map.iter_mut() {
                    normalize_scan_json(item, Some(key));
                }
            }
            _ => {}
        }
    }

    fn swift_scan_and_assemble(path: &Path) -> Value {
        let progress = Arc::new(ScanProgress::new(ProgressMode::Quiet));
        let collected = collect_paths(
            path,
            0,
            &build_collection_exclude_patterns(path, &path.join(DEFAULT_CACHE_DIR_NAME)),
        );
        let result = process_collected(
            &collected,
            progress,
            None,
            false,
            &TextDetectionOptions {
                collect_info: false,
                detect_packages: true,
                ..TextDetectionOptions::default()
            },
        );

        let mut files = result.files;
        strip_root_paths(&mut files, path);
        let assembly_result = assembly::assemble(&mut files);

        files.sort_by(|left, right| left.path.cmp(&right.path));
        let files_json: Vec<Value> = files
            .into_iter()
            .filter(|file| !file.path.is_empty())
            .map(|file| {
                json!({
                    "path": file.path,
                    "type": file.file_type,
                    "package_data": file.package_data,
                    "for_packages": file.for_packages,
                    "scan_errors": file.scan_errors,
                })
            })
            .collect();

        json!({
            "packages": assembly_result.packages,
            "dependencies": assembly_result.dependencies,
            "files": files_json,
        })
    }

    fn assert_swift_scan_matches_expected(fixture_dir: &str, expected_file: &str) {
        let actual = swift_scan_and_assemble(Path::new(fixture_dir));
        let actual_str =
            serde_json::to_string_pretty(&actual).expect("actual scan JSON should serialize");
        let expected_str =
            fs::read_to_string(expected_file).expect("expected scan JSON should be readable");

        let actual_normalized = normalize_test_uuids(&actual_str);
        let expected_normalized = normalize_test_uuids(&expected_str);

        let mut actual_value: Value =
            serde_json::from_str(&actual_normalized).expect("normalized actual JSON should parse");
        let mut expected_value: Value = serde_json::from_str(&expected_normalized)
            .expect("normalized expected JSON should parse");

        normalize_scan_json(&mut actual_value, None);
        normalize_scan_json(&mut expected_value, None);

        if let Err(error) = compare_scan_json_values(&actual_value, &expected_value, "") {
            panic!(
                "Swift scan golden mismatch for fixture {} vs {}: {}",
                fixture_dir, expected_file, error
            );
        }
    }

    #[test]
    fn test_swift_scan_uses_show_dependencies_only_fixture() {
        assert_swift_scan_matches_expected(
            "testdata/swift-golden/packages/vercelui_show_dependencies",
            "testdata/swift-golden/swift-vercelui-show-dependencies-expected.json",
        );
    }

    #[test]
    fn test_swift_scan_uses_resolved_only_fixture() {
        assert_swift_scan_matches_expected(
            "testdata/swift-golden/packages/fastlane_resolved_v1",
            "testdata/swift-golden/swift-fastlane-resolved-v1-package-expected.json",
        );
    }

    #[test]
    fn test_swift_scan_prefers_show_dependencies_over_manifest_dependencies() {
        assert_swift_scan_matches_expected(
            "testdata/swift-golden/packages/vercelui",
            "testdata/swift-golden/swift-vercelui-expected.json",
        );
    }

    #[test]
    fn test_swift_scan_falls_back_to_resolved_when_show_dependencies_missing() {
        assert_swift_scan_matches_expected(
            "testdata/swift-golden/packages/mapboxmaps_manifest_and_resolved",
            "testdata/swift-golden/swift-mapboxmaps-manifest-and-resolved-package-expected.json",
        );
    }
}
