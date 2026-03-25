#[cfg(test)]
mod tests {
    use super::super::scan_pipeline_test_utils::scan_and_assemble;

    #[test]
    fn test_python_metadata_scan_assigns_referenced_site_packages_files() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let site_packages = temp_dir.path().join("venv/lib/python3.11/site-packages");
        let dist_info = site_packages.join("click-8.0.4.dist-info");
        let package_dir = site_packages.join("click");

        std::fs::create_dir_all(&dist_info).expect("create dist-info dir");
        std::fs::create_dir_all(&package_dir).expect("create package dir");
        std::fs::write(
            dist_info.join("METADATA"),
            "Metadata-Version: 2.1\nName: click\nVersion: 8.0.4\n",
        )
        .unwrap();
        std::fs::write(
            dist_info.join("RECORD"),
            "click/__init__.py,,0\nclick/core.py,,10\nclick-8.0.4.dist-info/LICENSE.rst,,20\n",
        )
        .unwrap();
        std::fs::write(dist_info.join("LICENSE.rst"), "license text").unwrap();
        std::fs::write(package_dir.join("__init__.py"), "").unwrap();
        std::fs::write(package_dir.join("core.py"), "def click():\n    pass\n").unwrap();

        let (files, result) = scan_and_assemble(temp_dir.path());
        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("click"))
            .unwrap();
        let core_file = files
            .iter()
            .find(|file| file.path.ends_with("site-packages/click/core.py"))
            .unwrap();
        let license_file = files
            .iter()
            .find(|file| {
                file.path
                    .ends_with("site-packages/click-8.0.4.dist-info/LICENSE.rst")
            })
            .unwrap();
        assert!(core_file.for_packages.contains(&package.package_uid));
        assert!(license_file.for_packages.contains(&package.package_uid));
    }

    #[test]
    fn test_python_pkg_info_scan_assigns_installed_files_entries() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let site_packages = temp_dir.path().join("venv/lib/python3.11/site-packages");
        let egg_info = site_packages.join("examplepkg.egg-info");
        let package_dir = site_packages.join("examplepkg");

        std::fs::create_dir_all(&egg_info).unwrap();
        std::fs::create_dir_all(&package_dir).unwrap();
        std::fs::write(
            egg_info.join("PKG-INFO"),
            "Metadata-Version: 1.2\nName: examplepkg\nVersion: 1.0.0\n",
        )
        .unwrap();
        std::fs::write(
            egg_info.join("installed-files.txt"),
            "../examplepkg/__init__.py\n../examplepkg/core.py\n",
        )
        .unwrap();
        std::fs::write(package_dir.join("__init__.py"), "").unwrap();
        std::fs::write(package_dir.join("core.py"), "VALUE = 1\n").unwrap();

        let (files, result) = scan_and_assemble(temp_dir.path());
        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("examplepkg"))
            .unwrap();
        let core_file = files
            .iter()
            .find(|file| file.path.ends_with("site-packages/examplepkg/core.py"))
            .unwrap();
        assert!(core_file.for_packages.contains(&package.package_uid));
    }

    #[test]
    fn test_python_pkg_info_scan_assigns_sources_entries() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let egg_info = temp_dir.path().join("PyJPString.egg-info");
        let package_dir = temp_dir.path().join("jpstring");

        std::fs::create_dir_all(&egg_info).unwrap();
        std::fs::create_dir_all(&package_dir).unwrap();
        std::fs::write(
            egg_info.join("PKG-INFO"),
            "Metadata-Version: 1.0\nName: PyJPString\nVersion: 0.0.3\n",
        )
        .unwrap();
        std::fs::write(
            egg_info.join("SOURCES.txt"),
            "setup.py\nPyJPString.egg-info/PKG-INFO\nPyJPString.egg-info/top_level.txt\njpstring/__init__.py\n",
        )
        .unwrap();
        std::fs::write(
            temp_dir.path().join("setup.py"),
            "from setuptools import setup\n",
        )
        .unwrap();
        std::fs::write(egg_info.join("top_level.txt"), "jpstring\n").unwrap();
        std::fs::write(package_dir.join("__init__.py"), "").unwrap();

        let (files, result) = scan_and_assemble(temp_dir.path());
        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("PyJPString"))
            .unwrap();
        let setup_file = files
            .iter()
            .find(|file| file.path.ends_with("setup.py"))
            .unwrap();
        let module_init = files
            .iter()
            .find(|file| file.path.ends_with("jpstring/__init__.py"))
            .unwrap();
        let top_level = files
            .iter()
            .find(|file| file.path.ends_with("PyJPString.egg-info/top_level.txt"))
            .unwrap();
        assert!(setup_file.for_packages.contains(&package.package_uid));
        assert!(module_init.for_packages.contains(&package.package_uid));
        assert!(top_level.for_packages.contains(&package.package_uid));
    }
}
