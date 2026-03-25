#[cfg(test)]
mod tests {
    use super::super::scan_pipeline_test_utils::scan_and_assemble;

    #[test]
    fn test_debian_status_d_scan_assigns_installed_files_and_keeps_dependencies() {
        let temp_dir = tempfile::TempDir::new().expect("create temp dir");
        let status_dir = temp_dir.path().join("var/lib/dpkg/status.d");
        let info_dir = temp_dir.path().join("var/lib/dpkg/info");
        let bin_dir = temp_dir.path().join("bin");
        let doc_dir = temp_dir.path().join("usr/share/doc/bash");

        std::fs::create_dir_all(&status_dir).unwrap();
        std::fs::create_dir_all(&info_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&doc_dir).unwrap();

        std::fs::write(status_dir.join("bash"), "Package: bash\nStatus: install ok installed\nPriority: required\nSection: shells\nMaintainer: GNU Bash Maintainers <bash@example.com>\nArchitecture: amd64\nVersion: 5.2-1\nDepends: libc6 (>= 2.36)\nDescription: GNU Bourne Again SHell\n shell\n").unwrap();
        std::fs::write(
            info_dir.join("bash.list"),
            "/bin/bash\n/usr/share/doc/bash/copyright\n",
        )
        .unwrap();
        std::fs::write(info_dir.join("bash.md5sums"), "77506afebd3b7e19e937a678a185b62e  bin/bash\n9632d707e9eca8b3ba2b1a98c1c3fdce  usr/share/doc/bash/copyright\n").unwrap();
        std::fs::write(bin_dir.join("bash"), "#!/bin/sh\n").unwrap();
        std::fs::write(doc_dir.join("copyright"), "copyright text\n").unwrap();

        let (files, result) = scan_and_assemble(temp_dir.path());
        let package = result
            .packages
            .iter()
            .find(|package| package.name.as_deref() == Some("bash"))
            .unwrap();
        assert!(result.dependencies.iter().any(|dep| {
            dep.purl.as_deref() == Some("pkg:deb/debian/libc6")
                && dep.scope.as_deref() == Some("depends")
                && dep.for_package_uid.as_deref() == Some(&package.package_uid)
        }));
        let bash_file = files
            .iter()
            .find(|file| file.path.ends_with("/bin/bash"))
            .unwrap();
        let copyright_file = files
            .iter()
            .find(|file| file.path.ends_with("/usr/share/doc/bash/copyright"))
            .unwrap();
        assert!(bash_file.for_packages.contains(&package.package_uid));
        assert!(copyright_file.for_packages.contains(&package.package_uid));
    }
}
