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
        let mut actual_value =
            serde_json::to_value(actual).expect("actual package should serialize");
        strip_expected_empty_array_drift(&mut actual_value, expected_package);

        assert_eq!(
            actual_value,
            *expected_package,
            "Debian golden mismatch\nactual:\n{}\nexpected:\n{}",
            serde_json::to_string_pretty(&actual_value).unwrap_or_default(),
            serde_json::to_string_pretty(expected_package).unwrap_or_default()
        );
    }

    fn strip_expected_empty_array_drift(actual: &mut Value, expected: &Value) {
        let Some(actual_obj) = actual.as_object_mut() else {
            return;
        };
        let expected_obj = expected.as_object();

        for key in ["license_detections", "dependencies"] {
            if !expected_obj.is_some_and(|obj| obj.contains_key(key))
                && actual_obj
                    .get(key)
                    .and_then(Value::as_array)
                    .is_some_and(|arr| arr.is_empty())
            {
                actual_obj.remove(key);
            }
        }
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

    #[test]
    fn test_golden_debian_control() {
        let test_file = PathBuf::from("testdata/debian/project/debian/control");
        let expected_file = PathBuf::from("testdata/debian/project/debian/control.expected.json");

        let package_data = DebianControlParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_installed_status() {
        let test_file = PathBuf::from("testdata/debian/var/lib/dpkg/status");
        let expected_file = PathBuf::from("testdata/debian/var/lib/dpkg/status.expected.json");

        let package_data = DebianInstalledParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_distroless_installed() {
        let test_file = PathBuf::from("testdata/debian/var/lib/dpkg/status.d/base-files");
        let expected_file =
            PathBuf::from("testdata/debian/var/lib/dpkg/status.d/base-files.expected.json");

        let package_data = DebianDistrolessInstalledParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_orig_tar() {
        let test_file = PathBuf::from("testdata/debian/example_1.0.orig.tar.gz");
        let expected_file = PathBuf::from("testdata/debian/example_1.0.orig.tar.gz.expected.json");

        let package_data = DebianOrigTarParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_debian_tar() {
        let test_file = PathBuf::from("testdata/debian/example_1.0.debian.tar.xz");
        let expected_file =
            PathBuf::from("testdata/debian/example_1.0.debian.tar.xz.expected.json");

        let package_data = DebianDebianTarParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_installed_list() {
        let test_file = PathBuf::from("testdata/debian/var/lib/dpkg/info/bash.list");
        let expected_file =
            PathBuf::from("testdata/debian/var/lib/dpkg/info/bash.list.expected.json");

        let package_data = DebianInstalledListParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_installed_md5sums() {
        let test_file = PathBuf::from("testdata/debian/var/lib/dpkg/info/bash.md5sums");
        let expected_file =
            PathBuf::from("testdata/debian/var/lib/dpkg/info/bash.md5sums.expected.json");

        let package_data = DebianInstalledMd5sumsParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_control_in_extracted_deb() {
        let test_file = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/control",
        );
        let expected_file = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/control.expected.json",
        );

        let package_data = DebianControlInExtractedDebParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }

    #[test]
    fn test_golden_debian_md5sum_in_package() {
        let test_file = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/md5sums",
        );
        let expected_file = PathBuf::from(
            "testdata/debian/extracted-md5sums/example_1.0-1_amd64.deb-extract/control.tar.gz-extract/md5sums.expected.json",
        );

        let package_data = DebianMd5sumInPackageParser::extract_first_package(&test_file);
        compare_debian_package_data(&package_data, &expected_file);
    }
}
