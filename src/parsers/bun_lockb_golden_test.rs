#[cfg(all(test, feature = "golden-tests"))]
mod golden_tests {
    use std::path::PathBuf;

    use crate::parsers::PackageParser;
    use crate::parsers::bun_lockb::BunLockbParser;
    use crate::parsers::golden_test_utils::compare_package_data_parser_only;

    #[test]
    fn test_golden_bun_lockb_v2() {
        let test_file = PathBuf::from("testdata/bun/legacy/bun.lockb.v2");
        let expected_file = PathBuf::from("testdata/bun/golden/bun-lockb-v2-expected.json");

        let package_data = BunLockbParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
