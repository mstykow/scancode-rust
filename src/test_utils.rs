#[cfg(test)]
use crate::models::PackageData;
#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;

#[cfg(test)]
pub fn compare_package_data_parser_only(
    actual: &PackageData,
    expected_path: &Path,
) -> Result<(), String> {
    let expected_content = fs::read_to_string(expected_path)
        .map_err(|e| format!("Failed to read expected file: {}", e))?;

    let expected_array: Vec<Value> = serde_json::from_str(&expected_content)
        .map_err(|e| format!("Failed to parse expected JSON: {}", e))?;

    if expected_array.is_empty() {
        return Err("Expected file contains empty array".to_string());
    }

    let expected_json = &expected_array[0];
    let actual_json = serde_json::to_value(actual)
        .map_err(|e| format!("Failed to serialize actual PackageData: {}", e))?;

    compare_json_values_parser_only(&actual_json, expected_json, "")
}

#[cfg(test)]
fn compare_json_values_parser_only(
    actual: &Value,
    expected: &Value,
    path: &str,
) -> Result<(), String> {
    const SKIP_FIELDS: &[&str] = &[
        "identifier",
        "matched_text",
        "matcher",
        "matched_length",
        "match_coverage",
        "rule_relevance",
        "rule_identifier",
        "rule_url",
        "start_line",
        "end_line",
    ];

    if SKIP_FIELDS.iter().any(|&field| path.ends_with(field)) {
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
            for (i, (actual_item, expected_item)) in a.iter().zip(e.iter()).enumerate() {
                let item_path = format!("{}[{}]", path, i);
                compare_json_values_parser_only(actual_item, expected_item, &item_path)?;
            }
            Ok(())
        }

        (Value::Object(a), Value::Object(e)) => {
            if e.is_empty() && path.ends_with("resolved_package") {
                return Ok(());
            }

            let all_keys: std::collections::HashSet<_> = a.keys().chain(e.keys()).collect();

            for key in all_keys {
                let field_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", path, key)
                };

                if SKIP_FIELDS.contains(&key.as_str()) {
                    continue;
                }

                match (a.get(key), e.get(key)) {
                    (Some(actual_val), Some(expected_val)) => {
                        compare_json_values_parser_only(actual_val, expected_val, &field_path)?;
                    }
                    (None, Some(expected_val)) => match expected_val {
                        Value::Null => continue,
                        Value::Bool(false) => continue,
                        Value::Array(arr) if arr.is_empty() => continue,
                        Value::Object(obj) if obj.is_empty() => continue,
                        _ => {
                            if !SKIP_FIELDS.contains(&key.as_str()) {
                                return Err(format!("Missing field in actual: {}", field_path));
                            }
                        }
                    },
                    (Some(_), None) => {
                        return Err(format!("Extra field in actual: {}", field_path));
                    }
                    (None, None) => unreachable!(),
                }
            }
            Ok(())
        }

        _ => Err(format!(
            "Type mismatch at {}: actual={:?}, expected={:?}",
            path, actual, expected
        )),
    }
}
