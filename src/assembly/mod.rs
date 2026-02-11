mod assemblers;
#[cfg(test)]
mod assembly_golden_test;
mod nested_merge;
mod sibling_merge;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::models::{DatasourceId, FileInfo, Package, PackageData, TopLevelDependency};

pub use assemblers::ASSEMBLERS;

/// Result of the assembly phase: top-level packages and dependencies,
/// plus updated file-to-package associations.
#[derive(serde::Serialize)]
pub struct AssemblyResult {
    pub packages: Vec<Package>,
    pub dependencies: Vec<TopLevelDependency>,
}

/// How an assembler groups PackageData into Packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyMode {
    /// Merge related files in the same directory (or nested) into one Package.
    SiblingMerge,
    /// Each PackageData becomes its own independent Package (e.g., database files
    /// containing many installed packages like Alpine DB, RPM DB, Debian status).
    OnePerPackageData,
}

pub struct AssemblerConfig {
    pub datasource_ids: &'static [DatasourceId],
    pub sibling_file_patterns: &'static [&'static str],
    pub mode: AssemblyMode,
}

/// Run the assembly phase over all scanned files.
///
/// Groups files by parent directory, finds related manifests/lockfiles,
/// merges them into top-level `Package` objects, and hoists dependencies.
/// Updates each `FileInfo.for_packages` with the UIDs of packages it belongs to.
pub fn assemble(files: &mut [FileInfo]) -> AssemblyResult {
    let assembler_lookup = build_assembler_lookup();
    let mut packages = Vec::new();
    let mut dependencies = Vec::new();
    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();

    let dir_files = group_files_by_directory(files);

    for (dir, file_indices) in &dir_files {
        if seen_dirs.contains(dir) {
            continue;
        }

        let mut groups: HashMap<DatasourceId, Vec<(usize, &PackageData)>> = HashMap::new();

        for &idx in file_indices {
            for pkg_data in &files[idx].package_data {
                if let Some(dsid) = pkg_data.datasource_id
                    && let Some(&config_key) = assembler_lookup.get(&dsid)
                {
                    groups.entry(config_key).or_default().push((idx, pkg_data));
                }
            }
        }

        for &config_key in groups.keys() {
            let config = ASSEMBLERS
                .iter()
                .find(|a| a.datasource_ids.first() == Some(&config_key))
                .expect("assembler config must exist");

            match config.mode {
                AssemblyMode::SiblingMerge => {
                    if let Some((pkg, deps, affected_indices)) =
                        sibling_merge::assemble_siblings(config, files, file_indices)
                    {
                        let package_uid = pkg.package_uid.clone();
                        for idx in &affected_indices {
                            if !files[*idx].for_packages.contains(&package_uid) {
                                files[*idx].for_packages.push(package_uid.clone());
                            }
                        }
                        packages.push(pkg);
                        dependencies.extend(deps);
                    }
                }
                AssemblyMode::OnePerPackageData => {
                    let results = assemble_one_per_package_data(config, files, file_indices);
                    for (pkg, deps, affected_idx) in results {
                        let package_uid = pkg.package_uid.clone();
                        if !files[affected_idx].for_packages.contains(&package_uid) {
                            files[affected_idx].for_packages.push(package_uid);
                        }
                        packages.push(pkg);
                        dependencies.extend(deps);
                    }
                }
            }
        }

        seen_dirs.insert(dir.clone());
    }

    let mut assembled_indices: HashSet<usize> = HashSet::new();
    for (idx, file) in files.iter().enumerate() {
        if !file.for_packages.is_empty() {
            assembled_indices.insert(idx);
        }
    }

    for config in ASSEMBLERS {
        if config.mode != AssemblyMode::SiblingMerge {
            continue;
        }
        if let Some((pkg, deps, affected_indices)) =
            nested_merge::assemble_nested_patterns(files, config)
        {
            let package_uid = pkg.package_uid.clone();
            let purl = pkg.purl.clone();

            packages.retain(|p| p.purl != purl);
            dependencies.retain(|d| d.for_package_uid.as_ref() != Some(&package_uid));

            for idx in &affected_indices {
                files[*idx].for_packages.clear();
                files[*idx].for_packages.push(package_uid.clone());
                assembled_indices.insert(*idx);
            }

            packages.push(pkg);
            dependencies.extend(deps);
        }
    }

    AssemblyResult {
        packages,
        dependencies,
    }
}

fn assemble_one_per_package_data(
    config: &AssemblerConfig,
    files: &[FileInfo],
    file_indices: &[usize],
) -> Vec<(Package, Vec<TopLevelDependency>, usize)> {
    let mut results = Vec::new();

    for &idx in file_indices {
        let file = &files[idx];
        for pkg_data in &file.package_data {
            let dsid_matches = pkg_data
                .datasource_id
                .is_some_and(|dsid| config.datasource_ids.contains(&dsid));

            if !dsid_matches || pkg_data.purl.is_none() {
                continue;
            }

            let datafile_path = file.path.clone();
            let datasource_id = pkg_data.datasource_id.expect("datasource_id must be Some");
            let pkg = Package::from_package_data(pkg_data, datafile_path.clone());
            let for_package_uid = Some(pkg.package_uid.clone());

            let deps: Vec<TopLevelDependency> = pkg_data
                .dependencies
                .iter()
                .filter(|dep| dep.purl.is_some())
                .map(|dep| {
                    TopLevelDependency::from_dependency(
                        dep,
                        datafile_path.clone(),
                        datasource_id,
                        for_package_uid.clone(),
                    )
                })
                .collect();

            results.push((pkg, deps, idx));
        }
    }

    results
}

fn build_assembler_lookup() -> HashMap<DatasourceId, DatasourceId> {
    let mut lookup = HashMap::new();
    for config in ASSEMBLERS {
        let key = *config
            .datasource_ids
            .first()
            .expect("assembler must have at least one datasource_id");
        for &dsid in config.datasource_ids {
            lookup.insert(dsid, key);
        }
    }
    lookup
}

/// Group file indices by their parent directory path.
fn group_files_by_directory(files: &[FileInfo]) -> HashMap<PathBuf, Vec<usize>> {
    let mut groups: HashMap<PathBuf, Vec<usize>> = HashMap::new();
    for (idx, file) in files.iter().enumerate() {
        if let Some(parent) = Path::new(&file.path).parent() {
            groups.entry(parent.to_path_buf()).or_default().push(idx);
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Dependency, FileType};

    fn create_test_file_info(
        path: &str,
        datasource_id: DatasourceId,
        purl: Option<&str>,
        name: Option<&str>,
        version: Option<&str>,
        dependencies: Vec<Dependency>,
    ) -> FileInfo {
        let path_parts: Vec<&str> = path.split('/').collect();
        let file_name = path_parts.last().unwrap_or(&"");
        let extension = file_name.split('.').next_back().unwrap_or("");

        FileInfo {
            name: file_name.to_string(),
            base_name: file_name.to_string(),
            extension: extension.to_string(),
            path: path.to_string(),
            file_type: FileType::File,
            mime_type: Some("application/json".to_string()),
            size: 100,
            date: None,
            sha1: None,
            md5: None,
            sha256: None,
            programming_language: None,
            package_data: vec![PackageData {
                datasource_id: Some(datasource_id),
                purl: purl.map(|s| s.to_string()),
                name: name.map(|s| s.to_string()),
                version: version.map(|s| s.to_string()),
                dependencies,
                ..Default::default()
            }],
            license_expression: None,
            license_detections: vec![],
            copyrights: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
        }
    }

    #[test]
    fn test_assemble_npm_package_json_with_lockfile() {
        let dep = Dependency {
            purl: Some("pkg:npm/express@4.18.0".to_string()),
            extracted_requirement: Some("^4.18.0".to_string()),
            scope: Some("dependencies".to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: Some(false),
            is_direct: Some(true),
            resolved_package: None,
            extra_data: None,
        };

        let mut files = vec![
            create_test_file_info(
                "project/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/my-app@1.0.0"),
                Some("my-app"),
                Some("1.0.0"),
                vec![dep],
            ),
            create_test_file_info(
                "project/package-lock.json",
                DatasourceId::NpmPackageLockJson,
                Some("pkg:npm/my-app@1.0.0"),
                Some("my-app"),
                Some("1.0.0"),
                vec![],
            ),
        ];

        let result = assemble(&mut files);

        assert_eq!(result.packages.len(), 1, "Expected exactly one package");
        let package = &result.packages[0];
        assert_eq!(package.name, Some("my-app".to_string()));
        assert!(
            package.package_uid.contains("uuid="),
            "Expected package_uid to contain uuid qualifier"
        );
        assert_eq!(
            package.datafile_paths.len(),
            2,
            "Expected both files in datafile_paths"
        );
        assert!(
            package
                .datafile_paths
                .contains(&"project/package.json".to_string())
        );
        assert!(
            package
                .datafile_paths
                .contains(&"project/package-lock.json".to_string())
        );
        assert_eq!(
            package.datasource_ids.len(),
            2,
            "Expected both datasource IDs"
        );
        assert!(
            package
                .datasource_ids
                .contains(&DatasourceId::NpmPackageJson)
        );
        assert!(
            package
                .datasource_ids
                .contains(&DatasourceId::NpmPackageLockJson)
        );

        assert_eq!(result.dependencies.len(), 1, "Expected one dependency");
        let dep = &result.dependencies[0];
        assert_eq!(dep.purl, Some("pkg:npm/express@4.18.0".to_string()));
        assert_eq!(dep.datafile_path, "project/package.json");
        assert_eq!(dep.datasource_id, DatasourceId::NpmPackageJson);
        assert!(
            dep.for_package_uid.is_some(),
            "Expected for_package_uid to be set"
        );
        assert!(
            dep.for_package_uid
                .as_ref()
                .expect("for_package_uid should be Some")
                .contains("uuid="),
            "Expected for_package_uid to contain uuid"
        );

        assert_eq!(
            files[0].for_packages.len(),
            1,
            "Expected package.json to have for_packages populated"
        );
        assert_eq!(
            files[1].for_packages.len(),
            1,
            "Expected package-lock.json to have for_packages populated"
        );
    }

    #[test]
    fn test_assemble_cargo_toml_with_lock() {
        let mut files = vec![
            create_test_file_info(
                "project/Cargo.toml",
                DatasourceId::CargoToml,
                Some("pkg:cargo/my-crate@0.1.0"),
                Some("my-crate"),
                Some("0.1.0"),
                vec![],
            ),
            create_test_file_info(
                "project/Cargo.lock",
                DatasourceId::CargoLock,
                Some("pkg:cargo/my-crate@0.1.0"),
                Some("my-crate"),
                Some("0.1.0"),
                vec![],
            ),
        ];

        let result = assemble(&mut files);

        assert_eq!(result.packages.len(), 1, "Expected exactly one package");
        let package = &result.packages[0];
        assert_eq!(package.name, Some("my-crate".to_string()));
        assert_eq!(package.version, Some("0.1.0".to_string()));
        assert!(
            package.package_uid.contains("uuid="),
            "Expected package_uid to contain uuid qualifier"
        );
        assert_eq!(
            package.datafile_paths.len(),
            2,
            "Expected both files in datafile_paths"
        );
        assert!(
            package
                .datafile_paths
                .contains(&"project/Cargo.toml".to_string())
        );
        assert!(
            package
                .datafile_paths
                .contains(&"project/Cargo.lock".to_string())
        );
        assert_eq!(
            package.datasource_ids.len(),
            2,
            "Expected both datasource IDs"
        );
        assert!(package.datasource_ids.contains(&DatasourceId::CargoToml));
        assert!(package.datasource_ids.contains(&DatasourceId::CargoLock));
    }

    #[test]
    fn test_assemble_no_matching_datasource() {
        let mut files = vec![create_test_file_info(
            "project/unknown.json",
            DatasourceId::Readme,
            Some("pkg:unknown/pkg@1.0.0"),
            Some("pkg"),
            Some("1.0.0"),
            vec![],
        )];

        let result = assemble(&mut files);

        assert_eq!(
            result.packages.len(),
            0,
            "Expected no packages for unknown datasource"
        );
        assert_eq!(
            result.dependencies.len(),
            0,
            "Expected no dependencies for unknown datasource"
        );
    }

    #[test]
    fn test_assemble_single_file_no_sibling() {
        let dep = Dependency {
            purl: Some("pkg:npm/lodash@4.17.21".to_string()),
            extracted_requirement: Some("^4.17.0".to_string()),
            scope: Some("dependencies".to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: Some(false),
            is_direct: Some(true),
            resolved_package: None,
            extra_data: None,
        };

        let mut files = vec![create_test_file_info(
            "project/package.json",
            DatasourceId::NpmPackageJson,
            Some("pkg:npm/solo-app@2.0.0"),
            Some("solo-app"),
            Some("2.0.0"),
            vec![dep],
        )];

        let result = assemble(&mut files);

        assert_eq!(
            result.packages.len(),
            1,
            "Expected one package even without lockfile"
        );
        let package = &result.packages[0];
        assert_eq!(package.name, Some("solo-app".to_string()));
        assert_eq!(
            package.datafile_paths.len(),
            1,
            "Expected only one file in datafile_paths"
        );
        assert_eq!(package.datafile_paths[0], "project/package.json");
        assert_eq!(
            package.datasource_ids.len(),
            1,
            "Expected only one datasource ID"
        );

        assert_eq!(result.dependencies.len(), 1, "Expected one dependency");
    }

    #[test]
    fn test_assemble_no_purl_no_package() {
        let mut files = vec![create_test_file_info(
            "project/package.json",
            DatasourceId::NpmPackageJson,
            None,
            Some("no-purl-app"),
            None,
            vec![],
        )];

        let result = assemble(&mut files);

        assert_eq!(
            result.packages.len(),
            0,
            "Expected no packages when PackageData has no purl"
        );
    }

    #[test]
    fn test_build_package_uid_format() {
        use crate::models::build_package_uid;

        let purl = "pkg:npm/test@1.0.0";
        let uid = build_package_uid(purl);

        assert!(
            uid.starts_with("pkg:npm/test@1.0.0?uuid="),
            "Expected UUID to be added as qualifier"
        );
        assert!(uid.contains("uuid="), "Expected uuid qualifier");

        let purl_with_qualifier = "pkg:npm/test@1.0.0?arch=x64";
        let uid2 = build_package_uid(purl_with_qualifier);

        assert!(
            uid2.contains("&uuid="),
            "Expected UUID to be appended with & when qualifiers exist"
        );
        assert!(uid2.starts_with("pkg:npm/test@1.0.0?arch=x64&uuid="));
    }

    #[test]
    fn test_package_update_merges_fields() {
        let initial_pkg_data = PackageData {
            datasource_id: Some(DatasourceId::NpmPackageJson),
            purl: Some("pkg:npm/test@1.0.0".to_string()),
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            description: Some("Initial description".to_string()),
            ..Default::default()
        };

        let mut package = Package::from_package_data(&initial_pkg_data, "file1.json".to_string());

        let update_pkg_data = PackageData {
            datasource_id: Some(DatasourceId::NpmPackageLockJson),
            purl: Some("pkg:npm/test@1.0.0".to_string()),
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            homepage_url: Some("https://example.com".to_string()),
            sha256: Some("abc123".to_string()),
            ..Default::default()
        };

        package.update(&update_pkg_data, "file2.json".to_string());

        assert_eq!(package.datafile_paths.len(), 2);
        assert_eq!(package.datasource_ids.len(), 2);
        assert!(
            package
                .datasource_ids
                .contains(&DatasourceId::NpmPackageJson)
        );
        assert!(
            package
                .datasource_ids
                .contains(&DatasourceId::NpmPackageLockJson)
        );
        assert_eq!(
            package.description,
            Some("Initial description".to_string()),
            "Original description should be preserved"
        );
        assert_eq!(
            package.homepage_url,
            Some("https://example.com".to_string()),
            "New homepage should be filled"
        );
        assert_eq!(
            package.sha256,
            Some("abc123".to_string()),
            "New sha256 should be filled"
        );
    }

    #[test]
    fn test_matches_pattern_exact() {
        use crate::assembly::sibling_merge::matches_pattern;

        assert!(matches_pattern("package.json", "package.json"));
        assert!(!matches_pattern("package-lock.json", "package.json"));
    }

    #[test]
    fn test_matches_pattern_case_insensitive() {
        use crate::assembly::sibling_merge::matches_pattern;

        assert!(matches_pattern("Cargo.toml", "cargo.toml"));
        assert!(matches_pattern("cargo.toml", "Cargo.toml"));
        assert!(matches_pattern("CARGO.TOML", "cargo.toml"));
    }

    #[test]
    fn test_matches_pattern_glob() {
        use crate::assembly::sibling_merge::matches_pattern;

        assert!(matches_pattern("MyLib.podspec", "*.podspec"));
        assert!(matches_pattern("test.podspec", "*.podspec"));
        assert!(!matches_pattern("podspec", "*.podspec"));
        assert!(!matches_pattern("test.txt", "*.podspec"));

        assert!(matches_pattern("MyLib.podspec.json", "*.podspec.json"));
        assert!(!matches_pattern("MyLib.podspec", "*.podspec.json"));
    }

    #[test]
    fn test_assemble_one_per_package_data_mode() {
        let dep = Dependency {
            purl: Some("pkg:alpine/scanelf".to_string()),
            extracted_requirement: None,
            scope: Some("install".to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: Some(false),
            is_direct: Some(true),
            resolved_package: None,
            extra_data: None,
        };

        let path = "rootfs/lib/apk/db/installed";
        let file_name = "installed";
        let extension = "";

        let mut files = vec![FileInfo {
            name: file_name.to_string(),
            base_name: file_name.to_string(),
            extension: extension.to_string(),
            path: path.to_string(),
            file_type: FileType::File,
            mime_type: Some("text/plain".to_string()),
            size: 5000,
            date: None,
            sha1: None,
            md5: None,
            sha256: None,
            programming_language: None,
            package_data: vec![
                PackageData {
                    datasource_id: Some(DatasourceId::AlpineInstalledDb),
                    purl: Some("pkg:alpine/musl@1.2.3-r0".to_string()),
                    name: Some("musl".to_string()),
                    version: Some("1.2.3-r0".to_string()),
                    dependencies: vec![dep],
                    ..Default::default()
                },
                PackageData {
                    datasource_id: Some(DatasourceId::AlpineInstalledDb),
                    purl: Some("pkg:alpine/busybox@1.35.0-r13".to_string()),
                    name: Some("busybox".to_string()),
                    version: Some("1.35.0-r13".to_string()),
                    dependencies: vec![],
                    ..Default::default()
                },
            ],
            license_expression: None,
            license_detections: vec![],
            copyrights: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
        }];

        let result = assemble(&mut files);

        assert_eq!(
            result.packages.len(),
            2,
            "Expected two independent packages from one database file"
        );

        let musl = result
            .packages
            .iter()
            .find(|p| p.name == Some("musl".to_string()));
        let busybox = result
            .packages
            .iter()
            .find(|p| p.name == Some("busybox".to_string()));

        assert!(musl.is_some(), "Expected musl package");
        assert!(busybox.is_some(), "Expected busybox package");

        let musl = musl.unwrap();
        assert_eq!(musl.version, Some("1.2.3-r0".to_string()));
        assert_eq!(musl.datafile_paths, vec![path.to_string()]);
        assert!(musl.package_uid.contains("uuid="));

        assert_eq!(result.dependencies.len(), 1);
        assert_eq!(
            result.dependencies[0].purl,
            Some("pkg:alpine/scanelf".to_string())
        );

        assert_eq!(
            files[0].for_packages.len(),
            2,
            "Expected database file to reference both packages"
        );
    }
}
