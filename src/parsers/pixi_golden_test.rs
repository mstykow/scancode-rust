#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::pixi::{PixiLockParser, PixiTomlParser};
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_pixi_toml() {
        let test_file = PathBuf::from("testdata/pixi-golden/basic-manifest/pixi.toml");
        let expected_file =
            PathBuf::from("testdata/pixi-golden/basic-manifest/pixi.toml.expected.json");

        let package_data = PixiTomlParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(error) => panic!("Golden test failed: {}", error),
        }
    }

    #[test]
    fn test_golden_pixi_lock() {
        let test_file = PathBuf::from("testdata/pixi-golden/basic-lock/pixi.lock");
        let expected_file =
            PathBuf::from("testdata/pixi-golden/basic-lock/pixi.lock.expected.json");

        let package_data = PixiLockParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(error) => panic!("Golden test failed: {}", error),
        }
    }
}
