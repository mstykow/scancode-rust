#[cfg(test)]
mod golden_tests {
    use super::super::PackageParser;
    use super::super::conda::{CondaEnvironmentYmlParser, CondaMetaYamlParser};

    use std::fs;
    use std::path::PathBuf;

    /// Helper function to run golden tests.
    ///
    /// Compares parsed output against expected JSON files.
    fn run_golden(test_file: &str, expected_file: &str, parser_type: &str) {
        let test_path = PathBuf::from(test_file);

        let package_data = match parser_type {
            "meta" => CondaMetaYamlParser::extract_first_package(&test_path),
            "env" => CondaEnvironmentYmlParser::extract_first_package(&test_path),
            _ => panic!("Unknown parser type: {}", parser_type),
        };

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
    #[ignore] // TODO: Verify expected output matches Rust implementation
    fn test_golden_meta_yaml_abeona() {
        run_golden(
            "testdata/conda/meta-yaml/abeona/meta.yaml",
            "testdata/conda/meta-yaml/abeona/meta.yaml-expected.json",
            "meta",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_meta_yaml_gcnvkernel() {
        run_golden(
            "testdata/conda/meta-yaml/gcnvkernel/meta.yaml",
            "testdata/conda/meta-yaml/gcnvkernel/meta.yaml-expected.json",
            "meta",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_meta_yaml_pippy() {
        run_golden(
            "testdata/conda/meta-yaml/pippy/meta.yaml",
            "testdata/conda/meta-yaml/pippy/meta.yaml-expected.json",
            "meta",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_environment_ringer() {
        run_golden(
            "testdata/conda/conda-yaml/ringer/environment.yaml",
            "testdata/conda/conda-yaml/ringer/environment.yaml-expected.json",
            "env",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_environment_test() {
        run_golden(
            "testdata/conda/conda-yaml/test/environment_host_port.yml",
            "testdata/conda/conda-yaml/test/environment_host_port.yml-expected.json",
            "env",
        );
    }

    #[test]
    #[ignore]
    fn test_golden_environment_phc_gnn() {
        run_golden(
            "testdata/conda/conda-yaml/phc-gnn/environment_gpu.yml",
            "testdata/conda/conda-yaml/phc-gnn/environment_gpu.yml-expected.json",
            "env",
        );
    }
}
