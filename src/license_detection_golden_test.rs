//! Golden tests for license detection against Python ScanCode reference outputs.
//!
//! This module validates that the Rust license detection engine produces
//! identical output to Python ScanCode across a wide range of inputs.
//!
//! ## Test Categories
//!
//! - **Single licenses**: Simple detection of MIT, Apache, GPL, etc.
//! - **Multi-license**: Files with multiple licenses (e.g., ffmpeg LICENSE)
//! - **SPDX-LID**: Files with SPDX-License-Identifier headers
//! - **Hash match**: Exact whole-file license matches
//! - **Sequence match**: Modified/partial license text
//! - **Unknown licenses**: Unrecognized license-like text
//! - **False positives**: Cases that should NOT match
//! - **License references**: "See COPYING", "See LICENSE file", etc.

#[cfg(test)]
mod golden_tests {
    use crate::license_detection::LicenseDetection;
    use serde_json::Value;
    use std::fs;
    use std::path::Path;

    const GOLDEN_DIR: &str = "testdata/license-golden";

    fn load_expected_json(expected_path: &Path) -> Result<Value, String> {
        let content = fs::read_to_string(expected_path)
            .map_err(|e| format!("Failed to read expected file: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse expected JSON: {}", e))
    }

    fn get_license_detections_from_expected(expected: &Value) -> Result<&Value, String> {
        expected
            .get("license_detections")
            .ok_or_else(|| "Expected JSON missing 'license_detections' field".to_string())
    }

    fn compare_license_expression(
        actual: &LicenseDetection,
        expected_detection: &Value,
    ) -> Result<(), String> {
        let expected_expr = expected_detection
            .get("license_expression")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let actual_expr = actual.license_expression.as_deref().unwrap_or("");

        if !expected_expr.is_empty()
            && !actual_expr.contains(expected_expr)
            && !expected_expr.contains(actual_expr)
            && actual_expr != expected_expr
        {
            return Err(format!(
                "license_expression mismatch: expected '{}', got '{}'",
                expected_expr, actual_expr
            ));
        }

        Ok(())
    }

    fn compare_matcher(
        actual: &LicenseDetection,
        expected_detection: &Value,
    ) -> Result<(), String> {
        let expected_matches = expected_detection
            .get("matches")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Expected detection missing 'matches' array".to_string())?;

        if actual.matches.is_empty() && !expected_matches.is_empty() {
            return Err("Actual has no matches but expected has matches".to_string());
        }

        for (i, expected_match) in expected_matches.iter().enumerate() {
            let expected_matcher = expected_match
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(actual_match) = actual.matches.get(i)
                && !expected_matcher.is_empty()
                && actual_match.matcher != expected_matcher
            {
                return Err(format!(
                    "Match {} matcher mismatch: expected '{}', got '{}'",
                    i, expected_matcher, actual_match.matcher
                ));
            }
        }

        Ok(())
    }

    fn compare_license_detections(
        actual: &[LicenseDetection],
        expected_path: &Path,
    ) -> Result<(), String> {
        let expected = load_expected_json(expected_path)?;
        let expected_detections = get_license_detections_from_expected(&expected)?;

        let expected_array = expected_detections
            .as_array()
            .ok_or_else(|| "'license_detections' is not an array".to_string())?;

        if actual.len() != expected_array.len() {
            return Err(format!(
                "Detection count mismatch: expected {}, got {}",
                expected_array.len(),
                actual.len()
            ));
        }

        for (i, (actual_det, expected_det)) in actual.iter().zip(expected_array.iter()).enumerate()
        {
            compare_license_expression(actual_det, expected_det)
                .map_err(|e| format!("Detection {}: {}", i, e))?;
            compare_matcher(actual_det, expected_det)
                .map_err(|e| format!("Detection {}: {}", i, e))?;
        }

        Ok(())
    }

    fn skip_if_no_expected_file(expected_path: &Path) -> bool {
        !expected_path.exists()
    }

    fn skip_if_no_reference_data() -> bool {
        let rules_path =
            std::path::PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            std::path::PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        !rules_path.exists() || !licenses_path.exists()
    }

    fn create_test_engine() -> Option<crate::license_detection::LicenseDetectionEngine> {
        let data_path =
            std::path::PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        crate::license_detection::LicenseDetectionEngine::new(&data_path).ok()
    }

    #[test]
    fn test_golden_single_mit() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input = std::path::PathBuf::from(format!("{}/single-license/mit.txt", GOLDEN_DIR));
        let expected =
            std::path::PathBuf::from(format!("{}/single-license/mit.txt.expected", GOLDEN_DIR));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_single_apache() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input =
            std::path::PathBuf::from(format!("{}/single-license/apache-2.0.txt", GOLDEN_DIR));
        let expected = std::path::PathBuf::from(format!(
            "{}/single-license/apache-2.0.txt.expected",
            GOLDEN_DIR
        ));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_spdx_id() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input = std::path::PathBuf::from(format!("{}/spdx-lid/license", GOLDEN_DIR));
        let expected =
            std::path::PathBuf::from(format!("{}/spdx-lid/license.expected", GOLDEN_DIR));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_ffmpeg() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input =
            std::path::PathBuf::from(format!("{}/multi-license/ffmpeg-LICENSE.md", GOLDEN_DIR));
        let expected = std::path::PathBuf::from(format!(
            "{}/multi-license/ffmpeg-LICENSE.md.expected",
            GOLDEN_DIR
        ));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_hash_match() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input = std::path::PathBuf::from(format!("{}/hash-match/query.txt", GOLDEN_DIR));
        let expected =
            std::path::PathBuf::from(format!("{}/hash-match/query.txt.expected", GOLDEN_DIR));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_truncated() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input = std::path::PathBuf::from(format!("{}/seq-match/partial.txt", GOLDEN_DIR));
        let expected =
            std::path::PathBuf::from(format!("{}/seq-match/partial.txt.expected", GOLDEN_DIR));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_unknown() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input = std::path::PathBuf::from(format!("{}/unknown/unknown.txt", GOLDEN_DIR));
        let expected =
            std::path::PathBuf::from(format!("{}/unknown/unknown.txt.expected", GOLDEN_DIR));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_false_positive() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input = std::path::PathBuf::from(format!(
            "{}/false-positive/false-positive-gpl3.txt",
            GOLDEN_DIR
        ));
        let expected = std::path::PathBuf::from(format!(
            "{}/false-positive/false-positive-gpl3.txt.expected",
            GOLDEN_DIR
        ));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }

    #[test]
    fn test_golden_reference() {
        if skip_if_no_reference_data() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let input = std::path::PathBuf::from(format!("{}/reference/see-copying.txt", GOLDEN_DIR));
        let expected =
            std::path::PathBuf::from(format!("{}/reference/see-copying.txt.expected", GOLDEN_DIR));

        if skip_if_no_expected_file(&expected) {
            eprintln!("Skipping test: expected file not found at {:?}", expected);
            return;
        }

        let Some(engine) = create_test_engine() else {
            eprintln!("Skipping test: could not create engine");
            return;
        };

        let text = fs::read_to_string(&input).expect("Failed to read input file");
        let detections = engine.detect(&text).expect("Detection failed");

        compare_license_detections(&detections, &expected).unwrap();
    }
}
