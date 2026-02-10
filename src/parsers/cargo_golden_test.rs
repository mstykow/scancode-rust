#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::cargo::CargoParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_clap() {
        let test_file = PathBuf::from("testdata/cargo-golden/clap/Cargo.toml");
        let expected_file = PathBuf::from("testdata/cargo-golden/clap/Cargo.toml.expected");

        if !test_file.exists() || !expected_file.exists() {
            return;
        }

        let package_data = CargoParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for clap: {}", e),
        }
    }

    #[test]
    fn test_golden_package() {
        let test_file = PathBuf::from("testdata/cargo-golden/package/Cargo.toml");
        let expected_file = PathBuf::from("testdata/cargo-golden/package/Cargo.toml.expected");

        if !test_file.exists() || !expected_file.exists() {
            return;
        }

        let package_data = CargoParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for package: {}", e),
        }
    }

    #[test]
    fn test_golden_rustup() {
        let test_file = PathBuf::from("testdata/cargo-golden/rustup/Cargo.toml");
        let expected_file = PathBuf::from("testdata/cargo-golden/rustup/Cargo.toml.expected");

        if !test_file.exists() || !expected_file.exists() {
            return;
        }

        let package_data = CargoParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for rustup: {}", e),
        }
    }

    #[test]
    fn test_golden_scan() {
        let test_file = PathBuf::from("testdata/cargo-golden/scan/Cargo.toml");
        let expected_file = PathBuf::from("testdata/cargo-golden/scan/Cargo.toml.expected");

        if !test_file.exists() || !expected_file.exists() {
            return;
        }

        let package_data = CargoParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for scan: {}", e),
        }
    }

    #[test]
    fn test_golden_single_file_scan() {
        let test_file = PathBuf::from("testdata/cargo-golden/single-file-scan/Cargo.toml");
        let expected_file =
            PathBuf::from("testdata/cargo-golden/single-file-scan/Cargo.toml.expected");

        if !test_file.exists() || !expected_file.exists() {
            return;
        }

        let package_data = CargoParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for single-file-scan: {}", e),
        }
    }

    #[test]
    fn test_golden_tauri() {
        let test_file = PathBuf::from("testdata/cargo-golden/tauri/Cargo.toml");
        let expected_file = PathBuf::from("testdata/cargo-golden/tauri/Cargo.toml.expected");

        if !test_file.exists() || !expected_file.exists() {
            return;
        }

        let package_data = CargoParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for tauri: {}", e),
        }
    }
}
