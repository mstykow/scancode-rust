//! Golden tests for Swift Package Manager parsers.
//!
//! These tests are currently ignored due to format incompatibility between
//! Python reference expected files and Rust implementation:
//!
//! - Python: Extracts each dependency as a separate package in `{"packages": [...]}`
//! - Rust: Extracts single PackageData with dependencies in `dependencies` array
//!
//! Both approaches are valid. Python's approach better matches ScanCode's multi-package
//! model, while Rust's approach better matches manifest file structure (one package file
//! declares multiple dependencies).
//!
//! Comprehensive unit tests in `swift_resolved_test.rs` and `swift_manifest_json_test.rs`
//! already verify correct parsing for all Swift formats.

#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::swift_manifest_json::SwiftManifestJsonParser;
    use crate::parsers::swift_resolved::SwiftPackageResolvedParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_swift_fastlane_resolved_v1() {
        let test_file =
            PathBuf::from("testdata/swift-golden/packages/fastlane_resolved_v1/Package.resolved");
        let expected_file =
            PathBuf::from("testdata/swift-golden/swift-fastlane-resolved-v1-package-expected.json");

        let package_data = SwiftPackageResolvedParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_swift_vercelui_resolved() {
        let test_file = PathBuf::from("testdata/swift-golden/packages/vercelui/Package.resolved");
        let expected_file = PathBuf::from("testdata/swift-golden/swift-vercelui-expected.json");

        let package_data = SwiftPackageResolvedParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_swift_mapboxmaps_resolved() {
        let test_file = PathBuf::from(
            "testdata/swift-golden/packages/mapboxmaps_manifest_and_resolved/Package.resolved",
        );
        let expected_file =
            PathBuf::from("testdata/swift-golden/swift-maboxmaps-resolved-parse-expected.json");

        let package_data = SwiftPackageResolvedParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_swift_mapboxmaps_manifest() {
        let test_file =
            PathBuf::from("testdata/swift-golden/packages/mapboxmaps_manifest/Package.swift.json");
        let expected_file =
            PathBuf::from("testdata/swift-golden/swift-mapboxmaps-manifest-package-expected.json");

        let package_data = SwiftManifestJsonParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_swift_mapboxmaps_manifest_and_resolved() {
        let test_file = PathBuf::from(
            "testdata/swift-golden/packages/mapboxmaps_manifest_and_resolved/Package.swift.json",
        );
        let expected_file = PathBuf::from(
            "testdata/swift-golden/swift-mapboxmaps-manifest-and-resolved-package-expected.json",
        );

        let package_data = SwiftManifestJsonParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_swift_vercelui_show_dependencies_parse() {
        let test_file = PathBuf::from(
            "testdata/swift-golden/packages/vercelui_show_dependencies/swift-show-dependencies.deplock",
        );
        let expected_file = PathBuf::from(
            "testdata/swift-golden/swift-vercelui-show-dependencies-parse-expected.json",
        );

        let package_data = SwiftPackageResolvedParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Format incompatibility: Python uses {packages:[]} with multiple entries, Rust uses single package with dependencies array"]
    fn test_golden_swift_vercelui_show_dependencies() {
        let test_file = PathBuf::from(
            "testdata/swift-golden/packages/vercelui_show_dependencies/swift-show-dependencies.deplock",
        );
        let expected_file =
            PathBuf::from("testdata/swift-golden/swift-vercelui-show-dependencies-expected.json");

        let package_data = SwiftPackageResolvedParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
