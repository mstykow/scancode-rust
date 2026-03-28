#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::scan_test_utils::{
        assert_dependency_present, assert_file_links_to_package, scan_and_assemble,
    };
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_conda_assembly_scan_keeps_conda_and_pypi_package_contracts() {
        let (files, result) = scan_and_assemble(Path::new("testdata/conda/assembly"));

        let conda_package = result
            .packages
            .iter()
            .find(|package| {
                package.package_type == Some(PackageType::Conda)
                    && package.name.as_deref() == Some("requests")
            })
            .expect("conda requests package should be assembled");
        let pypi_package = result
            .packages
            .iter()
            .find(|package| {
                package.package_type == Some(PackageType::Pypi)
                    && package.name.as_deref() == Some("requests")
            })
            .expect("embedded pypi requests package should be assembled");

        assert_eq!(conda_package.version.as_deref(), Some("2.32.3"));
        assert_eq!(
            conda_package.purl.as_deref(),
            Some("pkg:conda/requests@2.32.3")
        );
        assert_eq!(pypi_package.version.as_deref(), Some("2.32.3"));
        assert_eq!(
            pypi_package.purl.as_deref(),
            Some("pkg:pypi/requests@2.32.3")
        );
        assert_dependency_present(&result.dependencies, "pkg:conda/zlib", "meta.yaml");
        assert_file_links_to_package(
            &files,
            "/requests-2.32.3-py312h06a4308_1.json",
            &conda_package.package_uid,
            DatasourceId::CondaMetaJson,
        );
        assert_file_links_to_package(
            &files,
            "/site-packages/requests-2.32.3.dist-info/METADATA",
            &pypi_package.package_uid,
            DatasourceId::PypiWheelMetadata,
        );
    }
}
