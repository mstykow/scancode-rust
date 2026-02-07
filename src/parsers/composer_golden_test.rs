#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::composer::ComposerLockParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_composer_lock() {
        let test_file = PathBuf::from("testdata/composer-golden/composer-lock/composer.lock");
        let expected_file =
            PathBuf::from("testdata/composer-golden/composer-lock/composer.lock.expected");

        let package_data = ComposerLockParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
