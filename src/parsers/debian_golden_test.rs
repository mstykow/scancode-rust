#[cfg(test)]
mod golden_tests {
    use crate::models::PackageData;
    use crate::parsers::PackageParser;
    use crate::parsers::debian::*;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    fn compare_debian_package_data(actual: &PackageData, expected_file: &PathBuf) {
        let expected_content =
            fs::read_to_string(expected_file).expect("expected Debian golden should exist");
        let expected_value: Value =
            serde_json::from_str(&expected_content).expect("expected Debian golden should parse");
        let expected_package = expected_value
            .as_array()
            .and_then(|items| items.first())
            .expect("expected Debian golden should contain one package");
        let actual_value = serde_json::to_value(actual).expect("actual package should serialize");

        assert_eq!(
            actual_value,
            *expected_package,
            "Debian golden mismatch\nactual:\n{}\nexpected:\n{}",
            serde_json::to_string_pretty(&actual_value).unwrap_or_default(),
            serde_json::to_string_pretty(expected_package).unwrap_or_default()
        );
    }

    #[test]
    fn test_golden_deb_archive_extraction() {
        let test_file = PathBuf::from("testdata/debian/deb/adduser_3.112ubuntu1_all.deb");
        let expected_file =
            PathBuf::from("testdata/debian/deb/adduser_3.112ubuntu1_all.deb.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = DebianDebParser::extract_first_package(&test_file);

        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_dsc_file() {
        let test_file = PathBuf::from("testdata/debian/dsc_files/zsh_5.7.1-1+deb10u1.dsc");
        let expected_file =
            PathBuf::from("testdata/debian/dsc_files/zsh_5.7.1-1+deb10u1.dsc.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = DebianDscParser::extract_first_package(&test_file);

        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_copyright_file() {
        let test_file = PathBuf::from("testdata/debian/copyright/copyright");
        let expected_file = PathBuf::from("testdata/debian/copyright/copyright.expected.json");

        let package_data = DebianCopyrightParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }
}
