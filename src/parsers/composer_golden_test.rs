#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::composer::{ComposerJsonParser, ComposerLockParser};
    use crate::test_utils::compare_package_data_parser_only;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_golden_composer_lock() {
        let test_file = PathBuf::from("testdata/composer-golden/composer-lock/composer.lock");
        let expected_file =
            PathBuf::from("testdata/composer-golden/composer-lock/composer.lock.expected");

        let package_data = ComposerLockParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_composer_json_a_timer() {
        let test_file = PathBuf::from("testdata/composer-golden/a-timer/composer.json");
        let expected_file =
            PathBuf::from("testdata/composer-golden/a-timer/composer.json.expected");

        let package_data = ComposerJsonParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_composer_json_license_normalization() {
        let test_file =
            PathBuf::from("testdata/composer-golden/license-normalization/composer.json");
        let expected_file =
            PathBuf::from("testdata/composer-golden/license-normalization/composer.json.expected");
        let license_expected_file = PathBuf::from(
            "testdata/composer-golden/license-normalization/composer.json.license.expected.json",
        );

        let package_data = ComposerJsonParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }

        let expected_license: Value = serde_json::from_str(
            &fs::read_to_string(license_expected_file).expect("expected license fixture"),
        )
        .expect("valid expected license json");

        assert_eq!(
            package_data.extracted_license_statement.as_deref(),
            expected_license
                .get("extracted_license_statement")
                .and_then(|value| value.as_str())
        );
        assert_eq!(
            package_data.declared_license_expression.as_deref(),
            expected_license
                .get("declared_license_expression")
                .and_then(|value| value.as_str())
        );
        assert_eq!(
            package_data.declared_license_expression_spdx.as_deref(),
            expected_license
                .get("declared_license_expression_spdx")
                .and_then(|value| value.as_str())
        );
        assert_eq!(package_data.license_detections.len(), 1);
    }
}
