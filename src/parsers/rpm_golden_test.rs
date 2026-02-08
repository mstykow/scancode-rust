#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::rpm_parser::*;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_rpm_archive() {
        let test_file = PathBuf::from("testdata/rpm/fping-2.4b2-10.fc12.x86_64.rpm");
        let expected_file =
            PathBuf::from("testdata/rpm/fping-2.4b2-10.fc12.x86_64.rpm.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = RpmParser::extract_package_data(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for RPM archive: {}", e),
        }
    }
}
