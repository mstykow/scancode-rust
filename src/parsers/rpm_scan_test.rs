#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::scan_test_utils::scan_and_assemble;
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_rpm_specfile_scan_assembles_package_and_dependencies() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        fs::copy(
            "testdata/rpm/specfile/cpio.spec",
            temp_dir.path().join("cpio.spec"),
        )
        .expect("copy cpio.spec fixture");

        let (files, result) = scan_and_assemble(temp_dir.path());

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("cpio"))
            .expect("cpio package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Rpm));
        assert_eq!(package.version.as_deref(), Some("2.9"));
        assert_eq!(package.purl.as_deref(), Some("pkg:rpm/cpio@2.9"));
        assert!(result.dependencies.iter().any(|dep| {
            dep.purl.as_deref() == Some("pkg:rpm/texinfo")
                && dep.scope.as_deref() == Some("build")
                && dep.for_package_uid.as_deref() == Some(package.package_uid.as_str())
        }));
        assert!(result.dependencies.iter().any(|dep| {
            dep.purl.as_deref() == Some("pkg:rpm/%2Fsbin%2Finstall-info")
                && dep.scope.as_deref() == Some("post")
                && dep.for_package_uid.as_deref() == Some(package.package_uid.as_str())
        }));

        let specfile = files
            .iter()
            .find(|file| file.path.ends_with("/cpio.spec"))
            .expect("cpio.spec should be scanned");
        assert!(specfile.for_packages.contains(&package.package_uid));
        assert!(
            specfile
                .package_data
                .iter()
                .any(|pkg_data| pkg_data.datasource_id == Some(DatasourceId::RpmSpecfile))
        );
    }
}
