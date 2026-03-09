#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::go::{GoModParser, GoSumParser};
    use crate::parsers::go_mod_graph::GoModGraphParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_kingpin_mod() {
        let test_file = PathBuf::from("testdata/go-golden/kingpin-mod/go.mod");
        let expected_file = PathBuf::from("testdata/go-golden/kingpin-mod/go.mod.expected");

        let package_data = GoModParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_sample_mod() {
        let test_file = PathBuf::from("testdata/go-golden/sample-mod/go.mod");
        let expected_file = PathBuf::from("testdata/go-golden/sample-mod/go.mod.expected");

        let package_data = GoModParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_opencensus_service_mod() {
        let test_file = PathBuf::from("testdata/go-golden/opencensus-service/go.mod");
        let expected_file = PathBuf::from("testdata/go-golden/opencensus-service/go.mod.expected");

        let package_data = GoModParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_sample2_sum() {
        let test_file = PathBuf::from("testdata/go-golden/sample2-sum/go.sum");
        let expected_file = PathBuf::from("testdata/go-golden/sample2-sum/go.sum.expected");

        let package_data = GoSumParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_sample3_sum() {
        let test_file = PathBuf::from("testdata/go-golden/sample3-sum/go.sum");
        let expected_file = PathBuf::from("testdata/go-golden/sample3-sum/go.sum.expected");

        let package_data = GoSumParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_sample_graph() {
        let test_file = PathBuf::from("testdata/go-golden/sample-graph/go.mod.graph");
        let expected_file = PathBuf::from("testdata/go-golden/sample-graph/go.mod.graph.expected");

        let package_data = GoModGraphParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
