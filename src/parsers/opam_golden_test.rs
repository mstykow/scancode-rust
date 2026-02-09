#[cfg(test)]
mod golden_tests {
    use super::super::PackageParser;
    use super::super::opam::OpamParser;

    use std::fs;
    use std::path::PathBuf;

    /// Helper function to run golden tests.
    ///
    /// Compares parsed output against expected JSON files.
    fn run_golden(test_file: &str, expected_file: &str) {
        let test_path = PathBuf::from(test_file);
        let package_data = OpamParser::extract_first_package(&test_path);

        let expected_json = fs::read_to_string(expected_file)
            .unwrap_or_else(|_| panic!("Failed to read expected file: {}", expected_file));

        let actual_json =
            serde_json::to_string_pretty(&package_data).expect("Failed to serialize PackageData");

        let expected_value: serde_json::Value =
            serde_json::from_str(&expected_json).expect("Failed to parse expected JSON");
        let actual_value: serde_json::Value =
            serde_json::from_str(&actual_json).expect("Failed to parse actual JSON");

        // For OPAM expected files, we need to extract the first element of the array
        let expected_pkg = if let Some(arr) = expected_value.as_array() {
            arr.first()
                .expect("Expected array should have at least one element")
                .clone()
        } else {
            expected_value
        };

        if expected_pkg != actual_value {
            println!("\n=== EXPECTED ===");
            println!("{}", serde_json::to_string_pretty(&expected_pkg).unwrap());
            println!("\n=== ACTUAL ===");
            println!("{}", serde_json::to_string_pretty(&actual_value).unwrap());
            panic!("Golden test failed: output does not match expected");
        }
    }

    #[test]
    #[ignore] // TODO: Verify expected output matches Rust implementation
    fn test_golden_sample1() {
        run_golden(
            "testdata/opam/sample1/sample1.opam",
            "testdata/opam/sample1/output.opam.expected",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_sample2() {
        run_golden(
            "testdata/opam/sample2/sample2.opam",
            "testdata/opam/sample2/output.opam.expected",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_sample3() {
        run_golden(
            "testdata/opam/sample3/sample3.opam",
            "testdata/opam/sample3/output.opam.expected",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_sample4() {
        run_golden(
            "testdata/opam/sample4/sample4.opam",
            "testdata/opam/sample4/output.opam.expected",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_sample5() {
        run_golden(
            "testdata/opam/sample5/sample5.opam",
            "testdata/opam/sample5/output.opam.expected",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_sample6() {
        run_golden(
            "testdata/opam/sample6/sample6.opam",
            "testdata/opam/sample6/output.opam.expected",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_sample7() {
        run_golden(
            "testdata/opam/sample7/sample7.opam",
            "testdata/opam/sample7/output.opam.expected",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_sample8() {
        run_golden(
            "testdata/opam/sample8/sample8.opam",
            "testdata/opam/sample8/output.opam.expected",
        );
    }
}
