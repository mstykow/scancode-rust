#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::readme::ReadmeParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_android_basic() {
        let test_file = PathBuf::from("testdata/readme-golden/android/basic/README.android");
        let expected_file =
            PathBuf::from("testdata/readme-golden/android/basic/README.android.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_chromium_basic() {
        let test_file = PathBuf::from("testdata/readme-golden/chromium/basic/README.chromium");
        let expected_file =
            PathBuf::from("testdata/readme-golden/chromium/basic/README.chromium.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_basic() {
        let test_file = PathBuf::from("testdata/readme-golden/facebook/basic/README.facebook");
        let expected_file =
            PathBuf::from("testdata/readme-golden/facebook/basic/README.facebook.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_capital_filename() {
        let test_file =
            PathBuf::from("testdata/readme-golden/facebook/capital-filename/README.FACEBOOK");
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/capital-filename/README.FACEBOOK.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_download_link_as_download_url() {
        let test_file = PathBuf::from(
            "testdata/readme-golden/facebook/download-link-as-download_url/README.facebook",
        );
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/download-link-as-download_url/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_downloaded_from_as_download_url() {
        let test_file = PathBuf::from(
            "testdata/readme-golden/facebook/downloaded-from-as-download_url/README.facebook",
        );
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/downloaded-from-as-download_url/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_missing_type() {
        let test_file =
            PathBuf::from("testdata/readme-golden/facebook/missing-type/README.facebook");
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/missing-type/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_project_as_name() {
        let test_file =
            PathBuf::from("testdata/readme-golden/facebook/project-as-name/README.facebook");
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/project-as-name/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_repo_as_homepage_url() {
        let test_file =
            PathBuf::from("testdata/readme-golden/facebook/repo-as-homepage_url/README.facebook");
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/repo-as-homepage_url/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_source_as_homepage_url() {
        let test_file =
            PathBuf::from("testdata/readme-golden/facebook/source-as-homepage_url/README.facebook");
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/source-as-homepage_url/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_use_parent_dir_name_as_package_name() {
        let test_file = PathBuf::from(
            "testdata/readme-golden/facebook/use-parent-dir-name-as-package-name/setuptools/README.facebook",
        );
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/use-parent-dir-name-as-package-name/setuptools/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_facebook_website_as_homepage_url() {
        let test_file = PathBuf::from(
            "testdata/readme-golden/facebook/website-as-homepage_url/README.facebook",
        );
        let expected_file = PathBuf::from(
            "testdata/readme-golden/facebook/website-as-homepage_url/README.facebook.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_google_basic() {
        let test_file = PathBuf::from("testdata/readme-golden/google/basic/README.google");
        let expected_file =
            PathBuf::from("testdata/readme-golden/google/basic/README.google.expected.json");

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_thirdparty_basic() {
        let test_file = PathBuf::from("testdata/readme-golden/thirdparty/basic/README.thirdparty");
        let expected_file = PathBuf::from(
            "testdata/readme-golden/thirdparty/basic/README.thirdparty.expected.json",
        );

        if !test_file.exists() {
            return;
        }

        let package_data = ReadmeParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
