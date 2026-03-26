#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::scan_pipeline_test_utils::{
        assert_dependency_present, assert_file_links_to_package, scan_and_assemble,
    };
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_cargo_basic_scan_assembles_manifest_and_lockfile() {
        let (files, result) = scan_and_assemble(Path::new("testdata/assembly-golden/cargo-basic"));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("test-crate"))
            .expect("cargo package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Cargo));
        assert_eq!(package.version.as_deref(), Some("0.1.0"));
        assert_eq!(package.purl.as_deref(), Some("pkg:cargo/test-crate@0.1.0"));
        assert_dependency_present(&result.dependencies, "pkg:cargo/serde", "Cargo.toml");
        assert_dependency_present(
            &result.dependencies,
            "pkg:cargo/serde@1.0.195",
            "Cargo.lock",
        );
        assert_file_links_to_package(
            &files,
            "/Cargo.toml",
            &package.package_uid,
            DatasourceId::CargoToml,
        );
        assert_file_links_to_package(
            &files,
            "/Cargo.lock",
            &package.package_uid,
            DatasourceId::CargoLock,
        );
    }
}
