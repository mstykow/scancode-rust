#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::super::scan_pipeline_test_utils::scan_and_assemble;
    use crate::models::{DatasourceId, PackageType};

    #[test]
    fn test_rpm_sqlite_scan_assigns_referenced_files_from_rootfs_layout() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let rpmdb_dir = temp_dir.path().join("usr/lib/sysimage/rpm");
        let licenses_dir = temp_dir.path().join("usr/share/licenses/libgcc");

        fs::create_dir_all(&rpmdb_dir).expect("create rpm db dir");
        fs::create_dir_all(&licenses_dir).expect("create licenses dir");
        fs::copy(
            Path::new("testdata/rpm/rpmdb.sqlite"),
            rpmdb_dir.join("rpmdb.sqlite"),
        )
        .expect("copy rpmdb sqlite fixture");
        for suffix in ["-wal", "-shm"] {
            let source = Path::new("testdata/rpm").join(format!("rpmdb.sqlite{suffix}"));
            if source.exists() {
                fs::copy(&source, rpmdb_dir.join(format!("rpmdb.sqlite{suffix}")))
                    .expect("copy rpmdb sidecar fixture");
            }
        }
        fs::write(licenses_dir.join("COPYING"), "license text\n").expect("write COPYING");

        let (files, result) = scan_and_assemble(temp_dir.path());

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("libgcc"))
            .expect("libgcc package should be assembled");
        let copying = files
            .iter()
            .find(|file| file.path.ends_with("/usr/share/licenses/libgcc/COPYING"))
            .expect("COPYING should be scanned");
        let rpmdb = files
            .iter()
            .find(|file| file.path.ends_with("/usr/lib/sysimage/rpm/rpmdb.sqlite"))
            .expect("rpmdb sqlite should be scanned");

        assert_eq!(package.package_type, Some(PackageType::Rpm));
        assert!(copying.for_packages.contains(&package.package_uid));
        assert!(rpmdb.package_data.iter().any(|pkg_data| {
            pkg_data.datasource_id == Some(DatasourceId::RpmInstalledDatabaseSqlite)
        }));
    }
}
