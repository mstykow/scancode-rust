#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::super::scan_pipeline_test_utils::scan_and_assemble;
    use crate::models::{DatasourceId, PackageType};

    fn assert_dependency_present(
        dependencies: &[crate::models::TopLevelDependency],
        purl: &str,
        datafile_suffix: &str,
    ) {
        assert!(
            dependencies.iter().any(|dep| {
                dep.purl.as_deref() == Some(purl) && dep.datafile_path.ends_with(datafile_suffix)
            }),
            "expected dependency {purl} from {datafile_suffix}, found: {:?}",
            dependencies
                .iter()
                .map(|dep| (dep.purl.clone(), dep.datafile_path.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_gitmodules_scan_keeps_manifest_unassembled_and_hoists_known_dependencies() {
        let (files, result) = scan_and_assemble(Path::new("testdata/gitmodules"));

        assert!(result.packages.is_empty());
        assert_eq!(result.dependencies.len(), 3);
        assert_dependency_present(
            &result.dependencies,
            "pkg:github/example/dep1",
            ".gitmodules",
        );
        assert_dependency_present(&result.dependencies, "pkg:github/org/lib2", ".gitmodules");
        assert_dependency_present(
            &result.dependencies,
            "pkg:gitlab/company/project",
            ".gitmodules",
        );
        assert!(
            result
                .dependencies
                .iter()
                .all(|dep| dep.for_package_uid.is_none())
        );

        let gitmodules = files
            .iter()
            .find(|file| file.path.ends_with("/.gitmodules"))
            .expect(".gitmodules should be scanned");
        assert!(gitmodules.for_packages.is_empty());
        assert!(
            gitmodules
                .package_data
                .iter()
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::Gitmodules) })
        );
    }

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
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::OpamFile) })
        );
    }

    #[test]
    fn test_gradle_scan_merges_build_and_lockfile_dependency_surfaces() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let gradle_dir = temp_dir.path().join("gradle");
        fs::create_dir_all(&gradle_dir).expect("create gradle dir");
        fs::copy(
            Path::new("testdata/gradle-golden/groovy/version-catalog/build.gradle"),
            temp_dir.path().join("build.gradle"),
        )
        .expect("copy build.gradle fixture");
        fs::copy(
            Path::new("testdata/gradle-golden/groovy/version-catalog/gradle/libs.versions.toml"),
            gradle_dir.join("libs.versions.toml"),
        )
        .expect("copy libs.versions.toml fixture");
        fs::copy(
            Path::new("testdata/gradle-lock/basic/gradle.lockfile"),
            temp_dir.path().join("gradle.lockfile"),
        )
        .expect("copy gradle.lockfile fixture");

        let (files, result) = scan_and_assemble(temp_dir.path());

        assert!(result.packages.is_empty());
        assert_dependency_present(
            &result.dependencies,
            "pkg:maven/androidx.appcompat/appcompat@1.7.0",
            "build.gradle",
        );
        assert_dependency_present(
            &result.dependencies,
            "pkg:maven/org.springframework.boot/spring-boot-starter-web@2.7.0",
            "gradle.lockfile",
        );

        let build_gradle = files
            .iter()
            .find(|file| file.path.ends_with("/build.gradle"))
            .expect("build.gradle should be scanned");
        let gradle_lockfile = files
            .iter()
            .find(|file| file.path.ends_with("/gradle.lockfile"))
            .expect("gradle.lockfile should be scanned");

        assert!(build_gradle.for_packages.is_empty());
        assert!(gradle_lockfile.for_packages.is_empty());
        assert!(
            build_gradle
                .package_data
                .iter()
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::BuildGradle) })
        );
        assert!(
            gradle_lockfile
                .package_data
                .iter()
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::GradleLockfile) })
        );
    }

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
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::CocoapodsPodfile) })
        );
        assert!(podfile_lock.for_packages.contains(&package.package_uid));
        assert!(podspec.for_packages.contains(&package.package_uid));
    }

    #[test]
    fn test_freebsd_scan_assembles_package_identity_and_declared_license() {
        let (files, result) = scan_and_assemble(Path::new("testdata/freebsd/basic"));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("dmidecode"))
            .expect("dmidecode package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Freebsd));
        assert_eq!(package.version.as_deref(), Some("2.12"));
        assert_eq!(
            package.declared_license_expression.as_deref(),
            Some("gpl-2.0")
        );
        assert_eq!(
            package.purl.as_deref(),
            Some("pkg:freebsd/dmidecode@2.12?arch=freebsd:10:x86:64&origin=sysutils/dmidecode")
        );

        let manifest = files
            .iter()
            .find(|file| file.path.ends_with("/+COMPACT_MANIFEST"))
            .expect("+COMPACT_MANIFEST should be scanned");
        assert!(manifest.for_packages.contains(&package.package_uid));
        assert!(manifest.package_data.iter().any(|pkg_data| {
            pkg_data.datasource_id == Some(DatasourceId::FreebsdCompactManifest)
        }));
    }

    #[test]
    fn test_maven_repository_pom_scan_assembles_package_from_repo_style_filename() {
        let (files, result) = scan_and_assemble(Path::new(
            "testdata/summarycode-golden/tallies/packages/scan/aopalliance",
        ));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("aopalliance"))
            .expect("aopalliance package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Maven));
        assert_eq!(package.namespace.as_deref(), Some("aopalliance"));
        assert_eq!(package.version.as_deref(), Some("1.0"));
        assert_eq!(
            package.declared_license_expression.as_deref(),
            Some("public-domain")
        );

        let pom = files
            .iter()
            .find(|file| file.path.ends_with("/aopalliance-1.0.pom"))
            .expect("repository pom should be scanned");
        assert!(pom.for_packages.contains(&package.package_uid));
        assert!(
            pom.package_data
                .iter()
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::MavenPom) })
        );
    }

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
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::NpmPackageJson) })
        );
    }

    #[test]
    fn test_rpm_specfile_scan_assembles_package_and_dependencies() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        fs::copy(
            Path::new("testdata/rpm/specfile/cpio.spec"),
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
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::RpmSpecfile) })
        );
    }

    #[test]
    fn test_rpm_yumdb_scan_assembles_virtual_package_and_preserves_metadata() {
        let (files, result) = scan_and_assemble(Path::new("testdata/rpm/var/lib/yum/yumdb"));

        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("bash"))
            .expect("bash yumdb package should be assembled");

        assert_eq!(package.package_type, Some(PackageType::Rpm));
        assert_eq!(package.version.as_deref(), Some("5.0-1.el8"));
        assert!(package.is_virtual);
        assert_eq!(
            package.purl.as_deref(),
            Some("pkg:rpm/bash@5.0-1.el8?arch=x86_64")
        );
        let extra = package
            .extra_data
            .as_ref()
            .expect("yumdb extra_data should exist");
        assert_eq!(
            extra.get("from_repo").and_then(|v| v.as_str()),
            Some("baseos")
        );
        assert_eq!(extra.get("reason").and_then(|v| v.as_str()), Some("dep"));
        assert_eq!(extra.get("releasever").and_then(|v| v.as_str()), Some("8"));

        let from_repo = files
            .iter()
            .find(|file| file.path.ends_with("/from_repo"))
            .expect("from_repo file should be scanned");
        assert!(from_repo.for_packages.contains(&package.package_uid));
        assert!(
            from_repo
                .package_data
                .iter()
                .any(|pkg_data| { pkg_data.datasource_id == Some(DatasourceId::RpmYumdb) })
        );
    }
}
