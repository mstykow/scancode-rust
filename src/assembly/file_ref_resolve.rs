use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use crate::models::{DatasourceId, FileInfo, Package, TopLevelDependency};
use packageurl::PackageUrl;

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
const RPM_YUMDB_PATH_SUFFIX: &str = "var/lib/yum/yumdb/";
const CONDA_META_PATH_SEGMENT: &str = "conda-meta/";

pub fn resolve_file_references(
    files: &mut [FileInfo],
    packages: &mut [Package],
    dependencies: &mut [TopLevelDependency],
) {
    let path_index = build_path_index(&*files);

    for package in packages.iter_mut() {
        if package.datasource_ids.contains(&DatasourceId::AboutFile)
            && let Some(datafile_path) = package.datafile_paths.first()
        {
            let root = Path::new(datafile_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let file_references = collect_file_references(
                files,
                &path_index,
                datafile_path,
                &package.datasource_ids,
                &[DatasourceId::AboutFile],
                package.purl.as_deref(),
            );

            let mut missing_refs = Vec::new();
            for file_ref in &file_references {
                let resolved_path = if root.is_empty() {
                    file_ref.path.clone()
                } else {
                    format!("{}/{}", root, file_ref.path.trim_start_matches('/'))
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
            continue;
        }

        if is_conda_meta_package(package)
            && let Some(conda_meta_path) = package
                .datafile_paths
                .iter()
                .find(|path| path.contains(CONDA_META_PATH_SEGMENT))
            && let Some(root) = compute_conda_root(Some(conda_meta_path.as_str()))
        {
            let file_references = collect_file_references(
                files,
                &path_index,
                conda_meta_path,
                &package.datasource_ids,
                &[DatasourceId::CondaMetaJson],
                package.purl.as_deref(),
            );

            let mut missing_refs = Vec::new();
            for file_ref in &file_references {
                let resolved_path = format!("{}{}", root, file_ref.path.trim_start_matches('/'));
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
            continue;
        }

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
                apply_rpm_namespace(files, package, dependencies, &namespace);
            }
        }
    }
}

fn is_conda_meta_package(package: &Package) -> bool {
    package
        .datasource_ids
        .contains(&DatasourceId::CondaMetaJson)
}

fn compute_conda_root(datafile_path: Option<&str>) -> Option<String> {
    let path = datafile_path?;
    let idx = path.rfind(CONDA_META_PATH_SEGMENT)?;
    Some(path[..idx].to_string())
}

pub fn merge_rpm_yumdb_metadata(files: &mut [FileInfo], packages: &mut Vec<Package>) {
    let yumdb_indices: Vec<usize> = packages
        .iter()
        .enumerate()
        .filter_map(|(idx, package)| {
            package
                .datasource_ids
                .contains(&DatasourceId::RpmYumdb)
                .then_some(idx)
        })
        .collect();
    let mut removal_indices = Vec::new();

    for yumdb_idx in yumdb_indices {
        let yumdb_package = packages[yumdb_idx].clone();
        let Some(yumdb_path) = yumdb_package.datafile_paths.first() else {
            continue;
        };
        let yumdb_root = compute_root(yumdb_path, RPM_YUMDB_PATH_SUFFIX);
        let yumdb_arch = yumdb_package
            .qualifiers
            .as_ref()
            .and_then(|qualifiers| qualifiers.get("arch"));

        let Some(target_idx) = packages.iter().enumerate().find_map(|(idx, package)| {
            if idx == yumdb_idx || !is_rpm_package(package) {
                return None;
            }

            let config = find_db_config(package)?;
            let datafile_path = package.datafile_paths.first()?;
            let target_root = compute_root(datafile_path, config.path_suffix);
            let target_arch = package
                .qualifiers
                .as_ref()
                .and_then(|qualifiers| qualifiers.get("arch"));

            (target_root == yumdb_root
                && package.name == yumdb_package.name
                && package.version == yumdb_package.version
                && target_arch == yumdb_arch)
                .then_some(idx)
        }) else {
            continue;
        };

        let target_package_uid = packages[target_idx].package_uid.clone();
        {
            let target = &mut packages[target_idx];
            target
                .datafile_paths
                .extend(yumdb_package.datafile_paths.clone());
            target
                .datasource_ids
                .extend(yumdb_package.datasource_ids.clone());

            if let Some(yumdb_extra) = yumdb_package.extra_data.clone()
                && !yumdb_extra.is_empty()
            {
                let extra_data = target.extra_data.get_or_insert_with(HashMap::new);
                let mut merged_yumdb = extra_data
                    .get("yumdb")
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
                for (key, value) in yumdb_extra {
                    merged_yumdb.insert(key, value);
                }
                extra_data.insert("yumdb".to_string(), serde_json::Value::Object(merged_yumdb));
            }
        }

        for file in files.iter_mut() {
            for package_uid in &mut file.for_packages {
                if *package_uid == yumdb_package.package_uid {
                    *package_uid = target_package_uid.clone();
                }
            }
        }

        removal_indices.push(yumdb_idx);
    }

    removal_indices.sort_unstable();
    removal_indices.dedup();
    for idx in removal_indices.into_iter().rev() {
        packages.remove(idx);
    }
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

fn replace_uid_base(old_uid: &str, new_purl: &str) -> String {
    if let Some((_, suffix)) = old_uid.split_once("?uuid=") {
        return format!("{}?uuid={}", new_purl, suffix);
    }

    if let Some((_, suffix)) = old_uid.split_once("&uuid=") {
        let separator = if new_purl.contains('?') { '&' } else { '?' };
        return format!("{}{separator}uuid={suffix}", new_purl);
    }

    old_uid.to_string()
}

fn rewrite_purl_namespace(existing_purl: &str, namespace: &str) -> Option<String> {
    let parsed = PackageUrl::from_str(existing_purl).ok()?;
    let mut updated = PackageUrl::new(parsed.ty(), parsed.name()).ok()?;

    updated.with_namespace(namespace).ok()?;

    if let Some(version) = parsed.version() {
        updated.with_version(version).ok()?;
    }

    if let Some(subpath) = parsed.subpath() {
        updated.with_subpath(subpath).ok()?;
    }

    for (key, value) in parsed.qualifiers() {
        updated
            .add_qualifier(key.to_string(), value.to_string())
            .ok()?;
    }

    Some(updated.to_string())
}

fn apply_rpm_namespace(
    files: &mut [FileInfo],
    package: &mut Package,
    dependencies: &mut [TopLevelDependency],
    namespace: &str,
) {
    let old_package_uid = package.package_uid.clone();

    package.namespace = Some(namespace.to_string());

    if let Some(current_purl) = package.purl.as_deref()
        && let Some(updated_purl) = rewrite_purl_namespace(current_purl, namespace)
    {
        package.purl = Some(updated_purl.clone());
        package.package_uid = replace_uid_base(&old_package_uid, &updated_purl);
    }

    for file in files.iter_mut() {
        for package_uid in &mut file.for_packages {
            if *package_uid == old_package_uid {
                *package_uid = package.package_uid.clone();
            }
        }
    }

    for dep in dependencies.iter_mut() {
        if dep.for_package_uid.as_deref() == Some(old_package_uid.as_str()) {
            dep.for_package_uid = Some(package.package_uid.clone());
        }

        if dep.for_package_uid.as_deref() == Some(package.package_uid.as_str()) {
            dep.namespace = Some(namespace.to_string());

            if let Some(current_purl) = dep.purl.as_deref()
                && let Some(updated_purl) = rewrite_purl_namespace(current_purl, namespace)
            {
                dep.purl = Some(updated_purl.clone());
                dep.dependency_uid = replace_uid_base(&dep.dependency_uid, &updated_purl);
            }
        }
    }
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
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
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
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
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
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
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
            holders: vec![],
            authors: vec![],
            emails: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
            is_source: None,
            source_count: None,
            is_legal: false,
            is_manifest: false,
            is_readme: false,
            is_top_level: false,
            is_key_file: false,
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
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
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
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
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
        assert_eq!(packages[0].purl.as_deref(), Some("pkg:rpm/fedora/bash@5.0"));
        assert!(
            packages[0]
                .package_uid
                .starts_with("pkg:rpm/fedora/bash@5.0?uuid=")
        );
        assert_eq!(dependencies[0].namespace, Some("fedora".to_string()));
        assert_eq!(
            dependencies[0].purl.as_deref(),
            Some("pkg:rpm/fedora/readline@8.0")
        );
        assert_eq!(
            dependencies[0].for_package_uid.as_deref(),
            Some(packages[0].package_uid.as_str())
        );
    }

    #[test]
    fn test_merge_rpm_yumdb_metadata() {
        let mut files = vec![
            FileInfo {
                name: "Packages".to_string(),
                base_name: "Packages".to_string(),
                extension: String::new(),
                path: "rootfs/var/lib/rpm/Packages".to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 1,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec!["pkg:rpm/bash@5.0-1.el8?uuid=rpm-uuid".to_string()],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
            },
            FileInfo {
                name: "from_repo".to_string(),
                base_name: "from_repo".to_string(),
                extension: String::new(),
                path: "rootfs/var/lib/yum/yumdb/p/abc123-bash-5.0-1.el8.x86_64/from_repo"
                    .to_string(),
                file_type: FileType::File,
                mime_type: None,
                size: 1,
                date: None,
                sha1: None,
                md5: None,
                sha256: None,
                programming_language: None,
                package_data: vec![],
                license_expression: None,
                license_detections: vec![],
                copyrights: vec![],
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec!["pkg:rpm/bash@5.0-1.el8?uuid=yumdb-uuid".to_string()],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
            },
        ];

        let mut packages = vec![
            Package {
                package_type: Some(PackageType::Rpm),
                namespace: None,
                name: Some("bash".to_string()),
                version: Some("5.0-1.el8".to_string()),
                qualifiers: Some(
                    std::iter::once(("arch".to_string(), "x86_64".to_string())).collect(),
                ),
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
                purl: Some("pkg:rpm/bash@5.0-1.el8?arch=x86_64".to_string()),
                package_uid: "pkg:rpm/bash@5.0-1.el8?uuid=rpm-uuid".to_string(),
                datafile_paths: vec!["rootfs/var/lib/rpm/Packages".to_string()],
                datasource_ids: vec![DatasourceId::RpmInstalledDatabaseBdb],
            },
            Package {
                package_type: Some(PackageType::Rpm),
                namespace: None,
                name: Some("bash".to_string()),
                version: Some("5.0-1.el8".to_string()),
                qualifiers: Some(
                    std::iter::once(("arch".to_string(), "x86_64".to_string())).collect(),
                ),
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
                is_virtual: true,
                extra_data: Some(
                    [
                        (
                            "from_repo".to_string(),
                            serde_json::Value::String("baseos".to_string()),
                        ),
                        (
                            "releasever".to_string(),
                            serde_json::Value::String("8".to_string()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
                repository_homepage_url: None,
                repository_download_url: None,
                api_data_url: None,
                purl: Some("pkg:rpm/bash@5.0-1.el8?arch=x86_64".to_string()),
                package_uid: "pkg:rpm/bash@5.0-1.el8?uuid=yumdb-uuid".to_string(),
                datafile_paths: vec![
                    "rootfs/var/lib/yum/yumdb/p/abc123-bash-5.0-1.el8.x86_64/from_repo".to_string(),
                ],
                datasource_ids: vec![DatasourceId::RpmYumdb],
            },
        ];

        merge_rpm_yumdb_metadata(&mut files, &mut packages);

        assert_eq!(packages.len(), 1);
        assert!(packages[0].datasource_ids.contains(&DatasourceId::RpmYumdb));
        assert!(
            packages[0]
                .datafile_paths
                .iter()
                .any(|path| path.contains("var/lib/yum/yumdb"))
        );
        let yumdb = packages[0]
            .extra_data
            .as_ref()
            .and_then(|extra| extra.get("yumdb"))
            .and_then(|value| value.as_object())
            .unwrap();
        assert_eq!(yumdb["from_repo"], "baseos");
        assert_eq!(yumdb["releasever"], "8");
        assert_eq!(
            files[1].for_packages,
            vec!["pkg:rpm/bash@5.0-1.el8?uuid=rpm-uuid".to_string()]
        );
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
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
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
                holders: vec![],
                authors: vec![],
                emails: vec![],
                urls: vec![],
                for_packages: vec![],
                scan_errors: vec![],
                is_source: None,
                source_count: None,
                is_legal: false,
                is_manifest: false,
                is_readme: false,
                is_top_level: false,
                is_key_file: false,
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
}
