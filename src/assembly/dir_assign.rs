use std::collections::HashMap;
use std::path::Path;

use crate::models::{FileInfo, Package};

const EXCLUSION_PATTERNS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "vendor",
    "venv",
    ".venv",
    "__pycache__",
];

fn should_exclude_from_parent(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|c| EXCLUSION_PATTERNS.contains(&c.as_os_str().to_string_lossy().as_ref()))
}

pub fn assign_files_to_packages(files: &mut [FileInfo], packages: &[Package]) {
    let mut dir_to_packages: HashMap<String, Vec<String>> = HashMap::new();

    for pkg in packages {
        for datafile_path in &pkg.datafile_paths {
            if let Some(dir) = Path::new(datafile_path).parent() {
                let dir_str = dir.to_string_lossy().to_string();
                dir_to_packages
                    .entry(dir_str)
                    .or_default()
                    .push(pkg.package_uid.clone());
            }
        }
    }

    for file in files.iter_mut() {
        if !file.for_packages.is_empty() {
            continue;
        }

        if should_exclude_from_parent(&file.path) {
            continue;
        }

        let file_path = Path::new(&file.path);

        let mut best_match: Option<(&String, &String)> = None;
        for (dir, pkg_uids) in &dir_to_packages {
            if file_path.starts_with(dir) {
                match &best_match {
                    None => best_match = Some((dir, &pkg_uids[0])),
                    Some((existing_dir, _)) if dir.len() > existing_dir.len() => {
                        best_match = Some((dir, &pkg_uids[0]));
                    }
                    _ => {}
                }
            }
        }

        if let Some((_, pkg_uid)) = best_match {
            file.for_packages.push(pkg_uid.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FileType, PackageType};

    fn create_file_info(path: &str) -> FileInfo {
        FileInfo {
            name: path.split('/').next_back().unwrap_or("").to_string(),
            base_name: path.split('/').next_back().unwrap_or("").to_string(),
            extension: String::new(),
            path: path.to_string(),
            file_type: FileType::File,
            mime_type: None,
            size: 100,
            date: None,
            sha1: None,
            md5: None,
            sha256: None,
            programming_language: None,
            package_data: vec![],
            license_expression: None,
            license_detections: vec![],
            copyrights: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
        }
    }

    fn create_package(name: &str, datafile_path: &str) -> Package {
        Package {
            package_type: Some(PackageType::Npm),
            namespace: None,
            name: Some(name.to_string()),
            version: Some("1.0.0".to_string()),
            qualifiers: None,
            subpath: None,
            primary_language: None,
            description: None,
            release_date: None,
            parties: vec![],
            keywords: vec![],
            homepage_url: None,
            download_url: None,
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            bug_tracking_url: None,
            code_view_url: None,
            vcs_url: None,
            copyright: None,
            holder: None,
            declared_license_expression: None,
            declared_license_expression_spdx: None,
            license_detections: vec![],
            other_license_expression: None,
            other_license_expression_spdx: None,
            other_license_detections: vec![],
            extracted_license_statement: None,
            notice_text: None,
            source_packages: vec![],
            is_private: false,
            is_virtual: false,
            extra_data: None,
            repository_homepage_url: None,
            repository_download_url: None,
            api_data_url: None,
            purl: Some(format!("pkg:npm/{}@1.0.0", name)),
            package_uid: format!("pkg:npm/{}@1.0.0?uuid=test", name),
            datafile_paths: vec![datafile_path.to_string()],
            datasource_ids: vec![crate::models::DatasourceId::NpmPackageJson],
        }
    }

    #[test]
    fn test_files_assigned_to_package_in_same_dir() {
        let mut files = vec![
            create_file_info("project/package.json"),
            create_file_info("project/index.js"),
        ];
        let packages = vec![create_package("my-app", "project/package.json")];

        assign_files_to_packages(&mut files, &packages);

        assert!(
            files[1]
                .for_packages
                .contains(&"pkg:npm/my-app@1.0.0?uuid=test".to_string())
        );
    }

    #[test]
    fn test_nested_directories_assigned_correctly() {
        let mut files = vec![
            create_file_info("project/Cargo.toml"),
            create_file_info("project/src/lib/mod.rs"),
        ];
        let packages = vec![Package {
            package_type: Some(PackageType::Cargo),
            datafile_paths: vec!["project/Cargo.toml".to_string()],
            package_uid: "pkg:cargo/my-crate@0.1.0?uuid=test".to_string(),
            purl: Some("pkg:cargo/my-crate@0.1.0".to_string()),
            datasource_ids: vec![crate::models::DatasourceId::CargoToml],
            ..create_package("my-crate", "project/Cargo.toml")
        }];

        assign_files_to_packages(&mut files, &packages);

        assert!(
            files[1]
                .for_packages
                .contains(&"pkg:cargo/my-crate@0.1.0?uuid=test".to_string())
        );
    }

    #[test]
    fn test_node_modules_not_assigned_to_parent() {
        let mut files = vec![
            create_file_info("project/package.json"),
            create_file_info("project/node_modules/lodash/package.json"),
        ];
        let packages = vec![create_package("my-app", "project/package.json")];

        assign_files_to_packages(&mut files, &packages);

        assert!(files[1].for_packages.is_empty());
    }

    #[test]
    fn test_longest_prefix_wins() {
        let mut files = vec![
            create_file_info("monorepo/package.json"),
            create_file_info("monorepo/packages/lib-a/package.json"),
            create_file_info("monorepo/packages/lib-a/index.js"),
        ];
        let packages = vec![
            create_package("root", "monorepo/package.json"),
            Package {
                package_uid: "pkg:npm/lib-a@1.0.0?uuid=lib-a".to_string(),
                datafile_paths: vec!["monorepo/packages/lib-a/package.json".to_string()],
                ..create_package("lib-a", "monorepo/packages/lib-a/package.json")
            },
        ];

        assign_files_to_packages(&mut files, &packages);

        assert!(
            files[2]
                .for_packages
                .contains(&"pkg:npm/lib-a@1.0.0?uuid=lib-a".to_string())
        );
    }

    #[test]
    fn test_already_assigned_files_unchanged() {
        let mut files = vec![
            create_file_info("project/package.json"),
            FileInfo {
                for_packages: vec!["existing-uid".to_string()],
                ..create_file_info("project/index.js")
            },
        ];
        let packages = vec![create_package("my-app", "project/package.json")];

        assign_files_to_packages(&mut files, &packages);

        assert_eq!(files[1].for_packages, vec!["existing-uid".to_string()]);
    }

    #[test]
    fn test_src_layout_python_project() {
        let mut files = vec![
            create_file_info("project/pyproject.toml"),
            create_file_info("project/src/my_pkg/__init__.py"),
            create_file_info("project/src/my_pkg/module.py"),
        ];
        let packages = vec![Package {
            package_type: Some(PackageType::Pypi),
            datafile_paths: vec!["project/pyproject.toml".to_string()],
            package_uid: "pkg:pypi/my-pkg@1.0.0?uuid=test".to_string(),
            purl: Some("pkg:pypi/my-pkg@1.0.0".to_string()),
            datasource_ids: vec![crate::models::DatasourceId::PypiPyprojectToml],
            ..create_package("my-pkg", "project/pyproject.toml")
        }];

        assign_files_to_packages(&mut files, &packages);

        assert!(
            files[1]
                .for_packages
                .contains(&"pkg:pypi/my-pkg@1.0.0?uuid=test".to_string())
        );
        assert!(
            files[2]
                .for_packages
                .contains(&"pkg:pypi/my-pkg@1.0.0?uuid=test".to_string())
        );
    }

    #[test]
    fn test_exclusion_patterns() {
        let mut files = vec![
            create_file_info("project/package.json"),
            create_file_info("project/.git/config"),
            create_file_info("project/target/debug/main"),
            create_file_info("project/vendor/lib.rs"),
        ];
        let packages = vec![create_package("my-app", "project/package.json")];

        assign_files_to_packages(&mut files, &packages);

        assert!(files[1].for_packages.is_empty());
        assert!(files[2].for_packages.is_empty());
        assert!(files[3].for_packages.is_empty());
    }
}
