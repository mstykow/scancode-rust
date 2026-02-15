#[cfg(all(test, feature = "golden-tests"))]
mod golden_tests {
    use crate::parsers::{PackageParser, RequirementsTxtParser};
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_parse_requirements_txt_basic_golden() {
        let test_file =
            PathBuf::from("testdata/python/golden/requirements_txt/basic-requirements.txt");
        let expected_file =
            PathBuf::from("testdata/python/golden/requirements_txt/basic-expected.json");

        let package_data = RequirementsTxtParser::extract_first_package(&test_file);

        assert_eq!(package_data.dependencies.len(), 3);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_parse_requirements_txt_complex_golden() {
        let test_file =
            PathBuf::from("testdata/python/golden/requirements_txt/complex-requirements.txt");
        let expected_file =
            PathBuf::from("testdata/python/golden/requirements_txt/complex-expected.json");

        let package_data = RequirementsTxtParser::extract_first_package(&test_file);

        assert_eq!(package_data.dependencies.len(), 4);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
