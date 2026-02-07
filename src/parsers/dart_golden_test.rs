#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::dart::{PubspecLockParser, PubspecYamlParser};
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_mini_lock() {
        let test_file = PathBuf::from("testdata/dart-golden/mini-lock/pubspec.lock");
        let expected_file = PathBuf::from("testdata/dart-golden/mini-lock/pubspec.lock.expected");

        let package_data = PubspecLockParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_stock_lock() {
        let test_file = PathBuf::from("testdata/dart-golden/stock-lock/pubspec.lock");
        let expected_file = PathBuf::from("testdata/dart-golden/stock-lock/pubspec.lock.expected");

        let package_data = PubspecLockParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_simple_yaml() {
        let test_file = PathBuf::from("testdata/dart-golden/simple-yaml/pubspec.yaml");
        let expected_file = PathBuf::from("testdata/dart-golden/simple-yaml/pubspec.yaml.expected");

        let package_data = PubspecYamlParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_many_deps_yaml() {
        let test_file = PathBuf::from("testdata/dart-golden/many-deps-yaml/pubspec.yaml");
        let expected_file =
            PathBuf::from("testdata/dart-golden/many-deps-yaml/pubspec.yaml.expected");

        let package_data = PubspecYamlParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
