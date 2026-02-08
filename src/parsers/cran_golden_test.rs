#[cfg(test)]
mod golden_tests {
    use super::super::PackageParser;
    use super::super::cran::CranParser;

    use std::fs;
    use std::path::PathBuf;

    /// Helper function to run golden tests.
    ///
    /// Compares parsed output against expected JSON files.
    fn run_golden(test_file: &str, expected_file: &str) {
        let test_path = PathBuf::from(test_file);
        let package_data = CranParser::extract_package_data(&test_path);

        let expected_json = fs::read_to_string(expected_file)
            .unwrap_or_else(|_| panic!("Failed to read expected file: {}", expected_file));

        let actual_json =
            serde_json::to_string_pretty(&package_data).expect("Failed to serialize PackageData");

        let expected_value: serde_json::Value =
            serde_json::from_str(&expected_json).expect("Failed to parse expected JSON");
        let actual_value: serde_json::Value =
            serde_json::from_str(&actual_json).expect("Failed to parse actual JSON");

        if expected_value != actual_value {
            println!("\n=== EXPECTED ===");
            println!("{}", serde_json::to_string_pretty(&expected_value).unwrap());
            println!("\n=== ACTUAL ===");
            println!("{}", serde_json::to_string_pretty(&actual_value).unwrap());
            panic!("Golden test failed: output does not match expected");
        }
    }

    #[test]
    #[ignore] // Enable when golden files are created
    fn test_golden_geometry() {
        run_golden(
            "testdata/cran/geometry/DESCRIPTION",
            "testdata/cran/geometry/expected.json",
        );
    }

    #[test]
    #[ignore] // Enable when golden files are created
    fn test_golden_codetools() {
        run_golden(
            "testdata/cran/codetools/DESCRIPTION",
            "testdata/cran/codetools/expected.json",
        );
    }

    #[test]
    #[ignore] // Enable when golden files are created
    fn test_golden_package() {
        run_golden(
            "testdata/cran/package/DESCRIPTION",
            "testdata/cran/package/expected.json",
        );
    }
}
