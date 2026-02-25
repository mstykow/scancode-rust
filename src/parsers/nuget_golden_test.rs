#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::nuget::NuspecParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_bootstrap() {
        let test_file = PathBuf::from("testdata/nuget-golden/bootstrap/bootstrap.nuspec");
        let expected_file =
            PathBuf::from("testdata/nuget-golden/bootstrap/bootstrap.nuspec.expected");

        let package_data = NuspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_castle_core() {
        let test_file = PathBuf::from("testdata/nuget-golden/castle-core/Castle.Core.nuspec");
        let expected_file =
            PathBuf::from("testdata/nuget-golden/castle-core/Castle.Core.nuspec.expected");

        let package_data = NuspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_entity_framework() {
        let test_file =
            PathBuf::from("testdata/nuget-golden/entity-framework/EntityFramework.nuspec");
        let expected_file =
            PathBuf::from("testdata/nuget-golden/entity-framework/EntityFramework.nuspec.expected");

        let package_data = NuspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_jquery_ui() {
        let test_file = PathBuf::from("testdata/nuget-golden/jquery-ui/jQuery.UI.Combined.nuspec");
        let expected_file =
            PathBuf::from("testdata/nuget-golden/jquery-ui/jQuery.UI.Combined.nuspec.expected");

        let package_data = NuspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_aspnet_mvc() {
        let test_file =
            PathBuf::from("testdata/nuget-golden/aspnet-mvc/Microsoft.AspNet.Mvc.nuspec");
        let expected_file =
            PathBuf::from("testdata/nuget-golden/aspnet-mvc/Microsoft.AspNet.Mvc.nuspec.expected");

        let package_data = NuspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_net_http() {
        let test_file = PathBuf::from("testdata/nuget-golden/net-http/Microsoft.Net.Http.nuspec");
        let expected_file =
            PathBuf::from("testdata/nuget-golden/net-http/Microsoft.Net.Http.nuspec.expected");

        let package_data = NuspecParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
