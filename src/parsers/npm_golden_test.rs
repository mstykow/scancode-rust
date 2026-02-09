#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::npm::NpmParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_basic() {
        let test_file = PathBuf::from("testdata/npm-golden/basic/package.json");
        let expected_file = PathBuf::from("testdata/npm-golden/basic/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - license_detections array mismatch"]
    fn test_golden_authors_list_dicts() {
        let test_file = PathBuf::from("testdata/npm-golden/authors_list_dicts/package.json");
        let expected_file =
            PathBuf::from("testdata/npm-golden/authors_list_dicts/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_authors_list_strings() {
        let test_file = PathBuf::from("testdata/npm-golden/authors_list_strings/package.json");
        let expected_file =
            PathBuf::from("testdata/npm-golden/authors_list_strings/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - SPDX normalization ('Apache 2.0' vs 'Apache-2.0')"]
    fn test_golden_double_license() {
        let test_file = PathBuf::from("testdata/npm-golden/double_license/package.json");
        let expected_file =
            PathBuf::from("testdata/npm-golden/double_license/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - license_detections array length mismatch"]
    fn test_golden_express_jwt() {
        let test_file = PathBuf::from("testdata/npm-golden/express-jwt-3.4.0/package.json");
        let expected_file =
            PathBuf::from("testdata/npm-golden/express-jwt-3.4.0/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - license_detections and SPDX normalization"]
    fn test_golden_from_npmjs() {
        let test_file = PathBuf::from("testdata/npm-golden/from_npmjs/package.json");
        let expected_file = PathBuf::from("testdata/npm-golden/from_npmjs/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_bundled_deps() {
        let test_file = PathBuf::from("testdata/npm-golden/bundledDeps/package.json");
        let expected_file = PathBuf::from("testdata/npm-golden/bundledDeps/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_casepath() {
        let test_file = PathBuf::from("testdata/npm-golden/casepath/package.json");
        let expected_file = PathBuf::from("testdata/npm-golden/casepath/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - license normalization and detection"]
    fn test_golden_chartist() {
        let test_file = PathBuf::from("testdata/npm-golden/chartist/package.json");
        let expected_file = PathBuf::from("testdata/npm-golden/chartist/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - license_detections and normalization"]
    fn test_golden_dist() {
        let test_file = PathBuf::from("testdata/npm-golden/dist/package.json");
        let expected_file = PathBuf::from("testdata/npm-golden/dist/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires license detection engine - license normalization"]
    fn test_golden_electron() {
        let test_file = PathBuf::from("testdata/npm-golden/electron/package.json");
        let expected_file = PathBuf::from("testdata/npm-golden/electron/package.json.expected");

        let package_data = NpmParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
