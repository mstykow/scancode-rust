#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::ruby::GemspecParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Requires: (1) Multi-line %q{} string literal parsing, (2) Conditional dependency extraction from if/else blocks"]
    fn test_golden_arel_gemspec() {
        let test_file = PathBuf::from("testdata/ruby-golden/arel-gemspec/arel.gemspec");
        let expected_file =
            PathBuf::from("testdata/ruby-golden/arel-gemspec/arel.gemspec.expected");

        let package_data = GemspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_cat_gemspec() {
        let test_file = PathBuf::from("testdata/ruby-golden/cat-gemspec/cat.gemspec");
        let expected_file = PathBuf::from("testdata/ruby-golden/cat-gemspec/cat.gemspec.expected");

        let package_data = GemspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - Python expects null but we extract 'mit' from s.licenses"]
    fn test_golden_oj_gemspec() {
        let test_file = PathBuf::from("testdata/ruby-golden/oj-gemspec/oj.gemspec");
        let expected_file = PathBuf::from("testdata/ruby-golden/oj-gemspec/oj.gemspec.expected");

        let package_data = GemspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - Python expects null but we extract 'MIT' from s.licenses"]
    fn test_golden_rubocop_gemspec() {
        let test_file = PathBuf::from("testdata/ruby-golden/rubocop-gemspec/rubocop.gemspec");
        let expected_file =
            PathBuf::from("testdata/ruby-golden/rubocop-gemspec/rubocop.gemspec.expected");

        let package_data = GemspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
