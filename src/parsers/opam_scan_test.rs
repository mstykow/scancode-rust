#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::scan_pipeline_test_utils::scan_and_assemble;
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_opam_scan_assembles_named_package_and_hoists_dependencies() {
        let (files, result) = scan_and_assemble(Path::new("testdata/opam/sample5"));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("bap-elf"))
            .expect("bap-elf package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Opam));
        assert_eq!(package.version.as_deref(), Some("1.0.0"));
        assert_eq!(package.purl.as_deref(), Some("pkg:opam/bap-elf@1.0.0"));
        assert_eq!(package.declared_license_expression.as_deref(), Some("mit"));
        assert!(result.dependencies.iter().any(|dep| {
            dep.purl.as_deref() == Some("pkg:opam/bap-std")
                && dep.extracted_requirement.as_deref() == Some("= 1.0.0")
                && dep.for_package_uid.as_deref() == Some(package.package_uid.as_str())
        }));

        let opam_file = files
            .iter()
            .find(|file| file.path.ends_with("/opam"))
            .expect("opam manifest should be scanned");
        assert!(opam_file.for_packages.contains(&package.package_uid));
        assert!(
            opam_file
                .package_data
                .iter()
                .any(|pkg_data| pkg_data.datasource_id == Some(DatasourceId::OpamFile))
        );
    }
}
