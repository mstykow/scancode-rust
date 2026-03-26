#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::scan_pipeline_test_utils::{
        assert_dependency_present, assert_file_links_to_package, scan_and_assemble,
    };
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_nix_flake_scan_assembles_manifest_and_lockfile() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let root = temp_dir.path().join("flake-demo");
        fs::create_dir_all(&root).expect("create nix fixture dir");
        fs::copy(
            "testdata/nix-golden/flake-demo/flake.nix",
            root.join("flake.nix"),
        )
        .expect("copy flake.nix fixture");
        fs::copy(
            "testdata/nix-golden/lock-demo/flake.lock",
            root.join("flake.lock"),
        )
        .expect("copy flake.lock fixture");

        let (files, result) = scan_and_assemble(temp_dir.path());

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("flake-demo"))
            .expect("nix flake package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Nix));
        assert_eq!(package.purl.as_deref(), Some("pkg:nix/flake-demo"));
        assert_dependency_present(
            &result.dependencies,
            "pkg:nix/crate2nix@ghi789",
            "flake.lock",
        );
        assert_file_links_to_package(
            &files,
            "/flake.nix",
            &package.package_uid,
            DatasourceId::NixFlakeNix,
        );
        assert_file_links_to_package(
            &files,
            "/flake.lock",
            &package.package_uid,
            DatasourceId::NixFlakeLock,
        );
    }
}
