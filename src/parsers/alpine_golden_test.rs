#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::alpine::*;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_alpine_installed_db() {
        let test_file = PathBuf::from("testdata/alpine/lib/apk/db/installed");
        let expected_file = PathBuf::from("testdata/alpine/lib/apk/db/installed.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = AlpineInstalledParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for Alpine installed DB: {}", e),
        }
    }
}
