#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::scan_pipeline_test_utils::{
        assert_dependency_present, assert_file_links_to_package, scan_and_assemble,
    };
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_nuget_basic_scan_assembles_csproj_and_packages_config() {
        let (files, result) = scan_and_assemble(Path::new("testdata/assembly-golden/nuget-basic"));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("Contoso.Utility"))
            .expect("nuget package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Nuget));
        assert_eq!(package.version.as_deref(), Some("1.0.0"));
        assert_eq!(
            package.purl.as_deref(),
            Some("pkg:nuget/Contoso.Utility@1.0.0")
        );
        assert_dependency_present(&result.dependencies, "pkg:nuget/NUnit", "packages.config");
        assert_file_links_to_package(
            &files,
            "/Contoso.Utility.csproj",
            &package.package_uid,
            DatasourceId::NugetCsproj,
        );
        assert_file_links_to_package(
            &files,
            "/packages.config",
            &package.package_uid,
            DatasourceId::NugetPackagesConfig,
        );
    }
}
