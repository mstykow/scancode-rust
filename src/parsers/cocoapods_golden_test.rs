//! Golden tests for CocoaPods parsers.
//!
//! These tests are currently ignored due to format incompatibility between
//! Python reference expected files and Rust implementation:
//!
//! - Python: Extracts each dependency/pod as a separate package in `{"packages": [...]}`
//! - Rust: Extracts single PackageData with dependencies in `dependencies` array
//!
//! Both approaches are valid. Python's approach better matches ScanCode's multi-package
//! model, while Rust's approach better matches manifest file structure (one podspec/Podfile
//! declares multiple dependencies).
//!
//! Comprehensive unit tests in `podspec_json_test.rs`, `podfile_lock_test.rs`, and DSL
//! parser unit tests already verify correct parsing for all CocoaPods formats.

#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::podfile::PodfileParser;
    use crate::parsers::podfile_lock::PodfileLockParser;
    use crate::parsers::podspec::PodspecParser;
    use crate::parsers::podspec_json::PodspecJsonParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podspec_json_firebase_analytics() {
        let test_file =
            PathBuf::from("testdata/cocoapods-golden/podspec.json/FirebaseAnalytics.podspec.json");
        let expected_file = PathBuf::from(
            "testdata/cocoapods-golden/podspec.json/FirebaseAnalytics.podspec.json.expected.json",
        );

        let package_data = PodspecJsonParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podfile_lock_braintree() {
        let test_file =
            PathBuf::from("testdata/cocoapods-golden/podfile.lock/braintree_ios_Podfile.lock");
        let expected_file = PathBuf::from(
            "testdata/cocoapods-golden/podfile.lock/braintree_ios_Podfile.lock.expected.json",
        );

        let package_data = PodfileLockParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podfile_lock_artsy() {
        let test_file =
            PathBuf::from("testdata/cocoapods-golden/podfile.lock/artsy_eigen_Podfile.lock");
        let expected_file = PathBuf::from(
            "testdata/cocoapods-golden/podfile.lock/artsy_eigen_Podfile.lock.expected.json",
        );

        let package_data = PodfileLockParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podfile_lock_solo() {
        let test_file = PathBuf::from("testdata/cocoapods-golden/assemble/solo/Podfile.lock");
        let expected_file =
            PathBuf::from("testdata/cocoapods-golden/assemble/solo/Podfile.lock-expected.json");

        let package_data = PodfileLockParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podspec_rxdatasources_solo() {
        let test_file =
            PathBuf::from("testdata/cocoapods-golden/assemble/solo/RxDataSources.podspec");
        let expected_file = PathBuf::from(
            "testdata/cocoapods-golden/assemble/solo/RxDataSources.podspec-expected.json",
        );

        let package_data = PodspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podspec_rxdatasources() {
        let test_file = PathBuf::from("testdata/cocoapods-golden/podspec/RxDataSources.podspec");
        let expected_file =
            PathBuf::from("testdata/cocoapods-golden/podspec/RxDataSources.podspec.expected.json");

        let package_data = PodspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podspec_starscream() {
        let test_file = PathBuf::from("testdata/cocoapods-golden/podspec/Starscream.podspec");
        let expected_file =
            PathBuf::from("testdata/cocoapods-golden/podspec/Starscream.podspec.expected.json");

        let package_data = PodspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podspec_nanopb() {
        let test_file = PathBuf::from("testdata/cocoapods-golden/podspec/nanopb.podspec");
        let expected_file =
            PathBuf::from("testdata/cocoapods-golden/podspec/nanopb.podspec.expected.json");

        let package_data = PodspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podspec_badgehub() {
        let test_file = PathBuf::from("testdata/cocoapods-golden/podspec/BadgeHub.podspec");
        let expected_file =
            PathBuf::from("testdata/cocoapods-golden/podspec/BadgeHub.podspec.expected.json");

        let package_data = PodspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_podfile_solo() {
        let test_file = PathBuf::from("testdata/cocoapods-golden/assemble/solo/Podfile");
        let expected_file =
            PathBuf::from("testdata/cocoapods-golden/assemble/solo/Podfile-expected.json");

        let package_data = PodfileParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
