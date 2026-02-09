#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::debian::*;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_deb_archive_extraction() {
        let test_file = PathBuf::from("testdata/debian/deb/adduser_3.112ubuntu1_all.deb");
        let expected_file =
            PathBuf::from("testdata/debian/deb/adduser_3.112ubuntu1_all.deb.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = DebianDebParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for .deb archive: {}", e),
        }
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

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for .dsc file: {}", e),
        }
    }

    #[test]
    fn test_golden_copyright_file() {
        let test_file = PathBuf::from("testdata/debian/copyright/libseccomp_copyright");
        let expected_file =
            PathBuf::from("testdata/debian/copyright/libseccomp_copyright.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = DebianCopyrightParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed for copyright file: {}", e),
        }
    }
}
