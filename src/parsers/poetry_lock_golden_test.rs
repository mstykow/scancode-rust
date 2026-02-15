#[cfg(all(test, feature = "golden-tests"))]
mod golden_tests {
    use crate::parsers::{PackageParser, PoetryLockParser};
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_parse_poetry_lock_golden() {
        let test_file = PathBuf::from("testdata/python/golden/poetry_lock/poetry.lock");
        let expected_file =
            PathBuf::from("testdata/python/golden/poetry_lock/poetry.lock-expected.json");

        let package_data = PoetryLockParser::extract_first_package(&test_file);

        assert_eq!(package_data.dependencies.len(), 5);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
