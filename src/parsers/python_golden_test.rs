#[cfg(all(test, feature = "golden-tests"))]
mod golden_tests {
    use crate::parsers::{PackageParser, PythonParser};
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_metadata() {
        let test_file = PathBuf::from("testdata/python/golden/metadata/METADATA");
        let expected_file = PathBuf::from("testdata/python/golden/metadata/METADATA-expected.json");

        let package_data = PythonParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_setup_cfg() {
        let test_file = PathBuf::from("testdata/python/golden/setup_cfg_wheel/setup.cfg");
        let expected_file =
            PathBuf::from("testdata/python/setup_cfg_wheel/setup.cfg-expected-corrected.json");

        let package_data = PythonParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
