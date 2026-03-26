#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::scan_pipeline_test_utils::scan_and_assemble;
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_cocoapods_scan_assembles_single_podspec_and_hoists_lockfile_dependencies() {
        let (files, result) = scan_and_assemble(Path::new(
            "testdata/cocoapods-golden/assemble/single-podspec",
        ));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("RxDataSources"))
            .expect("RxDataSources package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Cocoapods));
        assert_eq!(
            package.purl.as_deref(),
            Some("pkg:cocoapods/RxDataSources@4.0.1")
        );
        assert!(result.dependencies.iter().any(|dep| {
            dep.purl.as_deref() == Some("pkg:cocoapods/boost@1.76.0")
                && dep.for_package_uid.as_deref() == Some(package.package_uid.as_str())
                && dep.datafile_path.ends_with("Podfile.lock")
        }));

        let podfile = files
            .iter()
            .find(|file| file.path.ends_with("/Podfile"))
            .expect("Podfile should be scanned");
        let podfile_lock = files
            .iter()
            .find(|file| file.path.ends_with("/Podfile.lock"))
            .expect("Podfile.lock should be scanned");
        let podspec = files
            .iter()
            .find(|file| file.path.ends_with("/RxDataSources.podspec"))
            .expect("podspec should be scanned");

        assert!(
            podfile
                .package_data
                .iter()
                .any(|pkg_data| pkg_data.datasource_id == Some(DatasourceId::CocoapodsPodfile))
        );
        assert!(podfile_lock.for_packages.contains(&package.package_uid));
        assert!(podspec.for_packages.contains(&package.package_uid));
    }
}
