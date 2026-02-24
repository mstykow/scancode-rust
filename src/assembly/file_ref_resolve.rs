use std::collections::HashMap;

use crate::models::{DatasourceId, FileInfo, Package, TopLevelDependency};

struct DbPathConfig {
    datasource_ids: &'static [DatasourceId],
    path_suffix: &'static str,
}

const DB_PATH_CONFIGS: &[DbPathConfig] = &[
    DbPathConfig {
        datasource_ids: &[DatasourceId::AlpineInstalledDb],
        path_suffix: "lib/apk/db/installed",
    },
    DbPathConfig {
        datasource_ids: &[DatasourceId::RpmInstalledDatabaseBdb],
        path_suffix: "var/lib/rpm/Packages",
    },
    DbPathConfig {
        datasource_ids: &[DatasourceId::RpmInstalledDatabaseNdb],
        path_suffix: "usr/lib/sysimage/rpm/Packages.db",
    },
    DbPathConfig {
        datasource_ids: &[DatasourceId::RpmInstalledDatabaseSqlite],
        path_suffix: "usr/lib/sysimage/rpm/rpmdb.sqlite",
    },
    DbPathConfig {
        datasource_ids: &[DatasourceId::DebianInstalledStatusDb],
        path_suffix: "var/lib/dpkg/status",
    },
    DbPathConfig {
        datasource_ids: &[DatasourceId::DebianDistrolessInstalledDb],
        path_suffix: "var/lib/dpkg/status.d/",
    },
];

const RPM_DATASOURCE_IDS: &[DatasourceId] = &[
    DatasourceId::RpmInstalledDatabaseBdb,
    DatasourceId::RpmInstalledDatabaseNdb,
    DatasourceId::RpmInstalledDatabaseSqlite,
];

const NPM_DATASOURCE_IDS: &[DatasourceId] = &[
    DatasourceId::NpmPackageJson,
    DatasourceId::NpmPackageLockJson,
];

pub fn resolve_file_references(
    files: &mut [FileInfo],
    packages: &mut [Package],
    dependencies: &mut [TopLevelDependency],
) {
    let path_index = build_path_index(&*files);

    for package in packages.iter_mut() {
        if let Some(config) = find_db_config(package) {
            let datafile_path = match package.datafile_paths.first() {
                Some(path) => path,
                None => continue,
            };

            let root = compute_root(datafile_path, config.path_suffix);

            let file_references = collect_file_references(
                files,
                &path_index,
                datafile_path,
                &package.datasource_ids,
                config.datasource_ids,
                package.purl.as_deref(),
            );

            let mut missing_refs = Vec::new();

            for file_ref in &file_references {
                let ref_path = file_ref.path.trim_start_matches('/');
                let resolved_path = if root.is_empty() {
                    ref_path.to_string()
                } else {
                    format!("{}{}", root, ref_path)
                };

                if let Some(&file_idx) = path_index.get(&resolved_path) {
                    let package_uid = package.package_uid.clone();
                    if !files[file_idx].for_packages.contains(&package_uid) {
                        files[file_idx].for_packages.push(package_uid);
                    }
                } else {
                    missing_refs.push(file_ref.path.clone());
                }
            }

            if !missing_refs.is_empty() {
                missing_refs.sort();
                let missing_refs_json: Vec<serde_json::Value> = missing_refs
                    .into_iter()
                    .map(|path| serde_json::json!({"path": path}))
                    .collect();

                let extra_data = package.extra_data.get_or_insert_with(HashMap::new);
                extra_data.insert(
                    "missing_file_references".to_string(),
                    serde_json::Value::Array(missing_refs_json),
                );
            }

            if is_rpm_package(package)
                && let Some(namespace) = resolve_rpm_namespace(files, &path_index, &root)
            {
                package.namespace = Some(namespace.clone());

                for dep in dependencies.iter_mut() {
                    if dep.for_package_uid.as_ref() == Some(&package.package_uid) {
                        dep.namespace = Some(namespace.clone());
                    }
                }
            }
        }
    }

    resolve_node_modules_refs(files, packages);
}

fn resolve_node_modules_refs(files: &mut [FileInfo], packages: &[Package]) {
    let mut node_modules_packages: HashMap<String, (usize, String)> = HashMap::new();

    for (idx, file) in files.iter().enumerate() {
        if is_node_modules_package_json(&file.path)
            && let Some(pkg_data) = file.package_data.first()
            && pkg_data.purl.is_some()
            && let Some(parent_pkg) = find_parent_package_for_node_modules(&file.path, packages)
        {
            let pkg_dir = extract_node_modules_package_dir(&file.path);
            node_modules_packages.insert(pkg_dir, (idx, parent_pkg.package_uid.clone()));
        }
    }

    for file in files.iter_mut() {
        if let Some(pkg_dir) = find_containing_node_modules_package(&file.path)
            && let Some((_, package_uid)) = node_modules_packages.get(&pkg_dir)
            && !file.for_packages.contains(package_uid)
        {
            file.for_packages.push(package_uid.clone());
        }
    }
}

fn is_node_modules_package_json(path: &str) -> bool {
    path.ends_with("/package.json") && path.contains("/node_modules/")
}

fn extract_node_modules_package_dir(path: &str) -> String {
    if let Some(pos) = path.rfind("/package.json") {
        return path[..pos].to_string();
    }
    path.to_string()
}

fn find_parent_package_for_node_modules<'a>(
    node_modules_path: &str,
    packages: &'a [Package],
) -> Option<&'a Package> {
    for package in packages {
        if !is_npm_package(package) {
            continue;
        }
        for datafile_path in &package.datafile_paths {
            if let Some(nm_pos) = node_modules_path.find("/node_modules/") {
                let project_root = &node_modules_path[..nm_pos];
                if datafile_path.starts_with(project_root)
                    && !datafile_path.contains("/node_modules/")
                {
                    return Some(package);
                }
            }
        }
    }
    packages.first()
}

fn find_containing_node_modules_package(path: &str) -> Option<String> {
    if let Some(nm_pos) = path.find("/node_modules/") {
        let after_nm = &path[nm_pos + "/node_modules/".len()..];
        let package_part = if after_nm.starts_with('@') {
            let parts: Vec<&str> = after_nm.splitn(3, '/').collect();
            if parts.len() >= 2 {
                format!("{}/{}", parts[0], parts[1])
            } else {
                after_nm.split('/').next()?.to_string()
            }
        } else {
            after_nm.split('/').next()?.to_string()
        };
        return Some(format!(
            "{}node_modules/{}",
            &path[..nm_pos + 1],
            package_part
        ));
    }
    None
}

fn is_npm_package(package: &Package) -> bool {
    for &dsid in &package.datasource_ids {
        for &npm_dsid in NPM_DATASOURCE_IDS {
            if npm_dsid == dsid {
                return true;
            }
        }
    }
    false
}

fn build_path_index(files: &[FileInfo]) -> HashMap<String, usize> {
    files
        .iter()
        .enumerate()
        .map(|(idx, file)| (file.path.clone(), idx))
        .collect()
}

fn find_db_config(package: &Package) -> Option<&'static DbPathConfig> {
    for config in DB_PATH_CONFIGS {
        for &config_dsid in config.datasource_ids {
            for &pkg_dsid in &package.datasource_ids {
                if config_dsid == pkg_dsid {
                    return Some(config);
                }
            }
        }
    }
    None
}

fn compute_root(datafile_path: &str, suffix: &str) -> String {
    if let Some(pos) = datafile_path.rfind(suffix) {
        let root = &datafile_path[..pos];
        if root.is_empty() {
            String::new()
        } else {
            root.to_string()
        }
    } else {
        String::new()
    }
}

fn collect_file_references(
    files: &[FileInfo],
    path_index: &HashMap<String, usize>,
    datafile_path: &str,
    package_datasource_ids: &[DatasourceId],
    config_datasource_ids: &[DatasourceId],
    package_purl: Option<&str>,
) -> Vec<crate::models::FileReference> {
    let file_idx = match path_index.get(datafile_path) {
        Some(&idx) => idx,
        None => return Vec::new(),
    };

    let file = &files[file_idx];
    let mut refs = Vec::new();

    for pkg_data in &file.package_data {
        let dsid_matches = pkg_data.datasource_id.is_some_and(|dsid| {
            package_datasource_ids.contains(&dsid) || config_datasource_ids.contains(&dsid)
        });

        if !dsid_matches {
            continue;
        }

        let purl_matches = match (package_purl, pkg_data.purl.as_deref()) {
            (Some(pkg_purl), Some(data_purl)) => pkg_purl == data_purl,
            _ => true,
        };

        if purl_matches {
            refs.extend(pkg_data.file_references.clone());
        }
    }

    refs
}

fn is_rpm_package(package: &Package) -> bool {
    for &dsid in &package.datasource_ids {
        for &rpm_dsid in RPM_DATASOURCE_IDS {
            if rpm_dsid == dsid {
                return true;
            }
        }
    }
    false
}

fn resolve_rpm_namespace(
    files: &[FileInfo],
    path_index: &HashMap<String, usize>,
    root: &str,
) -> Option<String> {
    let os_release_paths = [
        format!("{}etc/os-release", root),
        format!("{}usr/lib/os-release", root),
    ];

    for os_release_path in &os_release_paths {
        if let Some(&file_idx) = path_index.get(os_release_path) {
            let file = &files[file_idx];
            for pkg_data in &file.package_data {
                if pkg_data.datasource_id == Some(DatasourceId::EtcOsRelease)
                    && let Some(namespace) = &pkg_data.namespace
                {
                    return Some(namespace.clone());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FileReference, FileType, PackageData, PackageType};

    #[test]
    fn test_find_root_from_path() {
        assert_eq!(
            compute_root("rootfs/lib/apk/db/installed", "lib/apk/db/installed"),
            "rootfs/"
        );
        assert_eq!(
            compute_root("lib/apk/db/installed", "lib/apk/db/installed"),
            ""
        );
        assert_eq!(
            compute_root("container/var/lib/rpm/Packages", "var/lib/rpm/Packages"),
            "container/"
        );
        assert_eq!(
            compute_root("var/lib/rpm/Packages", "var/lib/rpm/Packages"),
            ""
        );
    }

    #[test]
    fn test_resolve_basic_alpine() {
        let mut files = vec![
            FileInfo {
                name: "installed".to_string(),
                base_name: "installed".to_string(),
                extension: String::new(),
                path: "lib/apk/db/installed".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 100,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::AlpineInstalledDb),
                    purl: Some("pkg:alpine/musl@1.2.3".to_string()),
                    name: Some("musl".to_string()),
                    file_references: vec![
                        FileReference {
                            path: "lib/libc.so".to_string(),
                            size: None,
                            sha1: None,
                            md5: None,
                            sha256: None,
                            sha512: None,
                            extra_data: None,
                        },
                        FileReference {
                            path: "usr/bin/ldconfig".to_string(),
                            size: None,
                            sha1: None,
                            md5: None,
                            sha256: None,
                            sha512: None,
                            extra_data: None,
                        },
                    ],
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
            FileInfo {
                name: "libc.so".to_string(),
                base_name: "libc".to_string(),
                extension: "so".to_string(),
                path: "lib/libc.so".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 200,
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
            },
            FileInfo {
                name: "ldconfig".to_string(),
                base_name: "ldconfig".to_string(),
                extension: String::new(),
                path: "usr/bin/ldconfig".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 300,
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
            },
        ];

        let mut packages = vec![Package {
            package_type: Some(PackageType::Alpine),
            namespace: None,
            name: Some("musl".to_string()),
            version: Some("1.2.3".to_string()),
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
            purl: Some("pkg:alpine/musl@1.2.3".to_string()),
            package_uid: "pkg:alpine/musl@1.2.3?uuid=test-uuid".to_string(),
            datafile_paths: vec!["lib/apk/db/installed".to_string()],
            datasource_ids: vec![DatasourceId::AlpineInstalledDb],
        }];

        let mut dependencies = vec![];

        resolve_file_references(&mut files, &mut packages, &mut dependencies);

        assert_eq!(files[1].for_packages.len(), 1);
        assert_eq!(
            files[1].for_packages[0],
            "pkg:alpine/musl@1.2.3?uuid=test-uuid"
        );
        assert_eq!(files[2].for_packages.len(), 1);
        assert_eq!(
            files[2].for_packages[0],
            "pkg:alpine/musl@1.2.3?uuid=test-uuid"
        );
    }

    #[test]
    fn test_resolve_missing_refs() {
        let mut files = vec![FileInfo {
            name: "installed".to_string(),
            base_name: "installed".to_string(),
            extension: String::new(),
            path: "lib/apk/db/installed".to_string(),
            file_type: FileType::File,
            mime_type: None,
            size: 100,
            date: None,
            sha1: None,
            md5: None,
            sha256: None,
            programming_language: None,
            package_data: vec![PackageData {
                datasource_id: Some(DatasourceId::AlpineInstalledDb),
                purl: Some("pkg:alpine/test@1.0".to_string()),
                name: Some("test".to_string()),
                file_references: vec![
                    FileReference {
                        path: "missing/file1.txt".to_string(),
                        size: None,
                        sha1: None,
                        md5: None,
                        sha256: None,
                        sha512: None,
                        extra_data: None,
                    },
                    FileReference {
                        path: "another/missing.so".to_string(),
                        size: None,
                        sha1: None,
                        md5: None,
                        sha256: None,
                        sha512: None,
                        extra_data: None,
                    },
                ],
                ..Default::default()
            }],
            license_expression: None,
            license_detections: vec![],
            copyrights: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
        }];

        let mut packages = vec![Package {
            package_type: Some(PackageType::Alpine),
            namespace: None,
            name: Some("test".to_string()),
            version: Some("1.0".to_string()),
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
            purl: Some("pkg:alpine/test@1.0".to_string()),
            package_uid: "pkg:alpine/test@1.0?uuid=test-uuid".to_string(),
            datafile_paths: vec!["lib/apk/db/installed".to_string()],
            datasource_ids: vec![DatasourceId::AlpineInstalledDb],
        }];

        let mut dependencies = vec![];

        resolve_file_references(&mut files, &mut packages, &mut dependencies);

        assert!(packages[0].extra_data.is_some());
        let extra_data = packages[0].extra_data.as_ref().unwrap();
        assert!(extra_data.contains_key("missing_file_references"));

        let missing = extra_data.get("missing_file_references").unwrap();
        assert!(missing.is_array());
        let missing_array = missing.as_array().unwrap();
        assert_eq!(missing_array.len(), 2);
        assert_eq!(missing_array[0]["path"], "another/missing.so");
        assert_eq!(missing_array[1]["path"], "missing/file1.txt");
    }

    #[test]
    fn test_resolve_rpm_namespace() {
        let mut files = vec![
            FileInfo {
                name: "Packages".to_string(),
                base_name: "Packages".to_string(),
                extension: String::new(),
                path: "rootfs/var/lib/rpm/Packages".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 100,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::RpmInstalledDatabaseBdb),
                    purl: Some("pkg:rpm/bash@5.0".to_string()),
                    name: Some("bash".to_string()),
                    file_references: vec![],
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
            FileInfo {
                name: "os-release".to_string(),
                base_name: "os-release".to_string(),
                extension: String::new(),
                path: "rootfs/etc/os-release".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 50,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::EtcOsRelease),
                    namespace: Some("fedora".to_string()),
                    name: Some("fedora".to_string()),
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
        ];

        let mut packages = vec![Package {
            package_type: Some(PackageType::Rpm),
            namespace: None,
            name: Some("bash".to_string()),
            version: Some("5.0".to_string()),
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
            purl: Some("pkg:rpm/bash@5.0".to_string()),
            package_uid: "pkg:rpm/bash@5.0?uuid=test-uuid".to_string(),
            datafile_paths: vec!["rootfs/var/lib/rpm/Packages".to_string()],
            datasource_ids: vec![DatasourceId::RpmInstalledDatabaseBdb],
        }];

        let mut dependencies = vec![TopLevelDependency {
            purl: Some("pkg:rpm/readline@8.0".to_string()),
            extracted_requirement: None,
            scope: None,
            is_runtime: Some(true),
            is_optional: None,
            is_pinned: None,
            is_direct: None,
            resolved_package: None,
            extra_data: None,
            dependency_uid: "pkg:rpm/readline@8.0?uuid=dep-uuid".to_string(),
            for_package_uid: Some("pkg:rpm/bash@5.0?uuid=test-uuid".to_string()),
            datafile_path: "rootfs/var/lib/rpm/Packages".to_string(),
            datasource_id: DatasourceId::RpmInstalledDatabaseBdb,
            namespace: None,
        }];

        resolve_file_references(&mut files, &mut packages, &mut dependencies);

        assert_eq!(packages[0].namespace, Some("fedora".to_string()));
        assert_eq!(dependencies[0].namespace, Some("fedora".to_string()));
    }

    #[test]
    fn test_strip_leading_slash() {
        let mut files = vec![
            FileInfo {
                name: "installed".to_string(),
                base_name: "installed".to_string(),
                extension: String::new(),
                path: "lib/apk/db/installed".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 100,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::AlpineInstalledDb),
                    purl: Some("pkg:alpine/test@1.0".to_string()),
                    name: Some("test".to_string()),
                    file_references: vec![FileReference {
                        path: "/lib/test.so".to_string(),
                        size: None,
                        sha1: None,
                        md5: None,
                        sha256: None,
                        sha512: None,
                        extra_data: None,
                    }],
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
            FileInfo {
                name: "test.so".to_string(),
                base_name: "test".to_string(),
                extension: "so".to_string(),
                path: "lib/test.so".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 200,
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
            },
        ];

        let mut packages = vec![Package {
            package_type: Some(PackageType::Alpine),
            namespace: None,
            name: Some("test".to_string()),
            version: Some("1.0".to_string()),
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
            purl: Some("pkg:alpine/test@1.0".to_string()),
            package_uid: "pkg:alpine/test@1.0?uuid=test-uuid".to_string(),
            datafile_paths: vec!["lib/apk/db/installed".to_string()],
            datasource_ids: vec![DatasourceId::AlpineInstalledDb],
        }];

        let mut dependencies = vec![];

        resolve_file_references(&mut files, &mut packages, &mut dependencies);

        assert_eq!(files[1].for_packages.len(), 1);
        assert_eq!(
            files[1].for_packages[0],
            "pkg:alpine/test@1.0?uuid=test-uuid"
        );
    }

    #[test]
    fn test_node_modules_files_assigned_to_correct_package() {
        let mut files = vec![
            FileInfo {
                name: "package.json".to_string(),
                base_name: "package".to_string(),
                extension: "json".to_string(),
                path: "project/package.json".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 100,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::NpmPackageJson),
                    purl: Some("pkg:npm/project@1.0.0".to_string()),
                    name: Some("project".to_string()),
                    version: Some("1.0.0".to_string()),
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
            FileInfo {
                name: "package.json".to_string(),
                base_name: "package".to_string(),
                extension: "json".to_string(),
                path: "project/node_modules/express/package.json".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 100,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::NpmPackageJson),
                    purl: Some("pkg:npm/express@4.18.0".to_string()),
                    name: Some("express".to_string()),
                    version: Some("4.18.0".to_string()),
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
            FileInfo {
                name: "index.js".to_string(),
                base_name: "index".to_string(),
                extension: "js".to_string(),
                path: "project/node_modules/express/index.js".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 200,
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
            },
        ];

        let mut packages = vec![Package {
            package_type: Some(PackageType::Npm),
            namespace: None,
            name: Some("project".to_string()),
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
            purl: Some("pkg:npm/project@1.0.0".to_string()),
            package_uid: "pkg:npm/project@1.0.0?uuid=root-uuid".to_string(),
            datafile_paths: vec!["project/package.json".to_string()],
            datasource_ids: vec![DatasourceId::NpmPackageJson],
        }];

        let mut dependencies = vec![];

        resolve_file_references(&mut files, &mut packages, &mut dependencies);

        assert!(
            files[2]
                .for_packages
                .contains(&"pkg:npm/project@1.0.0?uuid=root-uuid".to_string()),
            "Files in node_modules should be assigned to parent package UID"
        );
    }

    #[test]
    fn test_node_modules_scoped_packages() {
        let mut files = vec![
            FileInfo {
                name: "package.json".to_string(),
                base_name: "package".to_string(),
                extension: "json".to_string(),
                path: "project/package.json".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 100,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::NpmPackageJson),
                    purl: Some("pkg:npm/project@1.0.0".to_string()),
                    name: Some("project".to_string()),
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
            FileInfo {
                name: "package.json".to_string(),
                base_name: "package".to_string(),
                extension: "json".to_string(),
                path: "project/node_modules/@types/node/package.json".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 100,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![PackageData {
                    datasource_id: Some(DatasourceId::NpmPackageJson),
                    purl: Some("pkg:npm/%40types/node@18.0.0".to_string()),
                    name: Some("@types/node".to_string()),
                    ..Default::default()
                }],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
            },
            FileInfo {
                name: "index.d.ts".to_string(),
                base_name: "index".to_string(),
                extension: "d.ts".to_string(),
                path: "project/node_modules/@types/node/index.d.ts".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 200,
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
            },
        ];

        let mut packages = vec![Package {
            package_type: Some(PackageType::Npm),
            namespace: None,
            name: Some("project".to_string()),
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
            purl: Some("pkg:npm/project@1.0.0".to_string()),
            package_uid: "pkg:npm/project@1.0.0?uuid=test-uuid".to_string(),
            datafile_paths: vec!["project/package.json".to_string()],
            datasource_ids: vec![DatasourceId::NpmPackageJson],
        }];

        let mut dependencies = vec![];

        resolve_file_references(&mut files, &mut packages, &mut dependencies);

        assert!(
            files[2]
                .for_packages
                .contains(&"pkg:npm/project@1.0.0?uuid=test-uuid".to_string()),
            "Scoped package files should be assigned to parent package UID"
        );
    }
}
