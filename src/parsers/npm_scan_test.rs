#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::scan_test_utils::{assert_dependency_present, scan_and_assemble};
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_npm_scoped_package_scan_preserves_namespace_and_leaf_name() {
        let (files, result) = scan_and_assemble(Path::new(
            "testdata/summarycode-golden/tallies/packages/scan/scoped1",
        ));

        let package = result
            .packages
            .iter()
            .find(|package| package.namespace.as_deref() == Some("@ionic"))
            .expect("scoped npm package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Npm));
        assert_eq!(package.name.as_deref(), Some("app-scripts"));
        assert_eq!(package.version.as_deref(), Some("3.0.1-201710301651"));

        let manifest = files
            .iter()
            .find(|file| file.path.ends_with("/package.json"))
            .expect("package.json should be scanned");
        assert!(manifest.for_packages.contains(&package.package_uid));
        assert!(
            manifest
                .package_data
                .iter()
                .any(|pkg_data| pkg_data.datasource_id == Some(DatasourceId::NpmPackageJson))
        );
    }

    #[test]
    fn test_bun_basic_scan_assembles_package_and_bun_lock() {
        let (files, result) = scan_and_assemble(Path::new("testdata/assembly-golden/bun-basic"));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("test-package"))
            .expect("bun package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Npm));
        assert_eq!(package.version.as_deref(), Some("1.0.0"));
        assert_eq!(package.purl.as_deref(), Some("pkg:npm/test-package@1.0.0"));
        assert_dependency_present(&result.dependencies, "pkg:npm/express", "package.json");
        assert_dependency_present(&result.dependencies, "pkg:npm/express@4.18.0", "bun.lock");

        let package_json = files
            .iter()
            .find(|file| file.path.ends_with("/package.json"))
            .expect("package.json should be scanned");
        let bun_lock = files
            .iter()
            .find(|file| file.path.ends_with("/bun.lock"))
            .expect("bun.lock should be scanned");
        assert!(package_json.for_packages.contains(&package.package_uid));
        assert!(bun_lock.for_packages.contains(&package.package_uid));
        assert!(
            bun_lock
                .package_data
                .iter()
                .any(|pkg_data| pkg_data.datasource_id == Some(DatasourceId::BunLock))
        );
    }
}
