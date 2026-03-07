mod assemblers;
#[cfg(test)]
mod assembly_golden_test;
mod cargo_workspace_merge;
pub mod file_ref_resolve;
mod nested_merge;
mod sibling_merge;
mod workspace_merge;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::models::{DatasourceId, FileInfo, Package, PackageData, TopLevelDependency};

pub use assemblers::ASSEMBLERS;

/// Pre-computed lookup: DatasourceId → config key (first DatasourceId in config).
/// Built once on first use, avoiding HashMap allocation on every `assemble()` call.
static ASSEMBLER_LOOKUP: LazyLock<HashMap<DatasourceId, DatasourceId>> = LazyLock::new(|| {
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
});

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
    let assembler_lookup = &*ASSEMBLER_LOOKUP;
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
                        if let Some(pkg) = pkg {
                            let package_uid = pkg.package_uid.clone();
                            for idx in &affected_indices {
                                if !files[*idx].for_packages.contains(&package_uid) {
                                    files[*idx].for_packages.push(package_uid.clone());
                                }
                            }
                            packages.push(pkg);
                        }
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

    ensure_npm_manifest_packages(files, &mut packages, &mut dependencies);

    assign_npm_package_resources(files, &packages);

    file_ref_resolve::resolve_file_references(files, &mut packages, &mut dependencies);

    // Post-processing: workspace assembly for npm/pnpm monorepos
    workspace_merge::assemble_workspaces(files, &mut packages, &mut dependencies);

    // Post-processing: workspace assembly for Cargo workspaces
    cargo_workspace_merge::assemble_cargo_workspaces(files, &mut packages, &mut dependencies);

    for package in &mut packages {
        package.datafile_paths.sort();
        package.datafile_paths.dedup();
        package.datasource_ids.sort_by_key(|left| left.to_string());
        package.datasource_ids.dedup();
    }

    for file in files.iter_mut() {
        file.for_packages
            .sort_by(|left, right| stable_uid_key(left).cmp(stable_uid_key(right)));
        file.for_packages.dedup();
    }

    packages
        .sort_by(|left, right| stable_package_sort_key(left).cmp(&stable_package_sort_key(right)));
    dependencies.sort_by(|left, right| {
        left.purl
            .as_deref()
            .cmp(&right.purl.as_deref())
            .then_with(|| {
                left.extracted_requirement
                    .as_deref()
                    .cmp(&right.extracted_requirement.as_deref())
            })
            .then_with(|| left.scope.as_deref().cmp(&right.scope.as_deref()))
            .then_with(|| left.datafile_path.cmp(&right.datafile_path))
            .then_with(|| {
                left.datasource_id
                    .to_string()
                    .cmp(&right.datasource_id.to_string())
            })
            .then_with(|| {
                left.for_package_uid
                    .as_deref()
                    .map(stable_uid_key)
                    .cmp(&right.for_package_uid.as_deref().map(stable_uid_key))
            })
    });

    AssemblyResult {
        packages,
        dependencies,
    }
}

fn stable_package_sort_key(package: &Package) -> (Option<&str>, Option<&str>, Option<&str>, &str) {
    (
        package.purl.as_deref(),
        package.name.as_deref(),
        package.version.as_deref(),
        package
            .datafile_paths
            .first()
            .map(String::as_str)
            .unwrap_or(""),
    )
}

fn stable_uid_key(uid: &str) -> &str {
    uid.split_once("?uuid=")
        .map(|(prefix, _)| prefix)
        .or_else(|| uid.split_once("&uuid=").map(|(prefix, _)| prefix))
        .unwrap_or(uid)
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

fn assign_npm_package_resources(files: &mut [FileInfo], packages: &[Package]) {
    let mut package_roots = collect_npm_package_roots(files, packages);

    package_roots.sort_by(|(left_root, _), (right_root, _)| {
        root_depth(right_root).cmp(&root_depth(left_root))
    });

    for file in files.iter_mut() {
        if !file.package_data.is_empty() {
            continue;
        }

        if let Some((_, package_uid)) = package_roots.iter().find(|(root, _)| {
            path_is_within_root(&file.path, root) && !is_first_level_node_modules(&file.path, root)
        }) {
            file.for_packages.clear();
            file.for_packages.push(package_uid.clone());
        }
    }
}

fn ensure_npm_manifest_packages(
    files: &mut [FileInfo],
    packages: &mut Vec<Package>,
    dependencies: &mut Vec<TopLevelDependency>,
) {
    for file in files.iter_mut() {
        let Some(pkg_data) = file.package_data.iter().find(|package_data| {
            package_data.datasource_id == Some(DatasourceId::NpmPackageJson)
                && package_data.purl.is_some()
        }) else {
            continue;
        };

        if !file.for_packages.is_empty() {
            continue;
        }

        if let Some(existing_package_uid) = packages
            .iter()
            .find(|package| package.datafile_paths.contains(&file.path))
            .map(|package| package.package_uid.clone())
        {
            file.for_packages.push(existing_package_uid);
            continue;
        }

        let package = Package::from_package_data(pkg_data, file.path.clone());
        let package_uid = package.package_uid.clone();
        let datasource_id = pkg_data
            .datasource_id
            .expect("npm manifest datasource_id must be present");

        let manifest_dependencies: Vec<TopLevelDependency> = pkg_data
            .dependencies
            .iter()
            .filter(|dependency| dependency.purl.is_some())
            .map(|dependency| {
                TopLevelDependency::from_dependency(
                    dependency,
                    file.path.clone(),
                    datasource_id,
                    Some(package_uid.clone()),
                )
            })
            .collect();

        file.for_packages.push(package_uid.clone());
        packages.push(package);
        dependencies.extend(manifest_dependencies);
    }
}

fn collect_npm_package_roots(files: &[FileInfo], packages: &[Package]) -> Vec<(String, String)> {
    let mut package_roots: Vec<(String, String)> = packages
        .iter()
        .filter(|package| {
            package
                .purl
                .as_deref()
                .is_some_and(|purl| purl.starts_with("pkg:npm/"))
        })
        .flat_map(|package| {
            files.iter().filter_map(move |file| {
                let is_matching_manifest = package.datafile_paths.contains(&file.path)
                    && file.package_data.iter().any(|package_data| {
                        package_data.datasource_id == Some(DatasourceId::NpmPackageJson)
                            && package_data.purl.as_deref() == package.purl.as_deref()
                    });

                if is_matching_manifest {
                    Some((package_dir(&file.path), package.package_uid.clone()))
                } else {
                    None
                }
            })
        })
        .collect();

    if package_roots.is_empty() {
        package_roots = files
            .iter()
            .filter(|file| {
                file.package_data.iter().any(|package_data| {
                    package_data.datasource_id == Some(DatasourceId::NpmPackageJson)
                })
            })
            .filter_map(|file| {
                file.for_packages
                    .first()
                    .map(|package_uid| (package_dir(&file.path), package_uid.clone()))
            })
            .collect();
    }

    package_roots.sort();
    package_roots.dedup();
    package_roots
}

fn package_dir(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(directory, _)| directory.to_string())
        .unwrap_or_default()
}

fn root_depth(root: &str) -> usize {
    if root.is_empty() {
        0
    } else {
        root.matches('/').count() + 1
    }
}

fn path_is_within_root(path: &str, root: &str) -> bool {
    root.is_empty()
        || path == root
        || path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_first_level_node_modules(path: &str, root: &str) -> bool {
    let relative = if root.is_empty() {
        Some(path)
    } else {
        path.strip_prefix(root)
            .and_then(|suffix| suffix.strip_prefix('/').or(Some(suffix)))
    };

    relative
        .and_then(|relative| relative.split('/').next())
        .is_some_and(|component| component == "node_modules")
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
            holders: vec![],
            authors: vec![],
            emails: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
            is_source: None,
            source_count: None,
        }
    }

    fn create_plain_file_info(path: &str) -> FileInfo {
        let path_parts: Vec<&str> = path.split('/').collect();
        let file_name = path_parts.last().unwrap_or(&"");
        let extension = file_name.split('.').next_back().unwrap_or("");

        FileInfo {
            name: file_name.to_string(),
            base_name: file_name.to_string(),
            extension: extension.to_string(),
            path: path.to_string(),
            file_type: FileType::File,
            mime_type: Some("text/plain".to_string()),
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
            holders: vec![],
            authors: vec![],
            emails: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
            is_source: None,
            source_count: None,
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
    fn test_assemble_npm_package_json_skips_mismatched_lockfile() {
        let mut files = vec![
            create_test_file_info(
                "project/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/my-app@1.0.0"),
                Some("my-app"),
                Some("1.0.0"),
                vec![],
            ),
            create_test_file_info(
                "project/package-lock.json",
                DatasourceId::NpmPackageLockJson,
                Some("pkg:npm/other-app@2.0.0"),
                Some("other-app"),
                Some("2.0.0"),
                vec![Dependency {
                    purl: Some("pkg:npm/left-pad@1.3.0".to_string()),
                    extracted_requirement: Some("1.3.0".to_string()),
                    scope: Some("dependencies".to_string()),
                    is_runtime: Some(true),
                    is_optional: Some(false),
                    is_pinned: Some(true),
                    is_direct: Some(false),
                    resolved_package: None,
                    extra_data: None,
                }],
            ),
        ];

        let result = assemble(&mut files);

        assert_eq!(
            result.packages.len(),
            1,
            "Expected only the manifest package"
        );
        let package = &result.packages[0];
        assert_eq!(package.name, Some("my-app".to_string()));
        assert_eq!(
            package.datafile_paths,
            vec!["project/package.json".to_string()]
        );
        assert!(
            result.dependencies.is_empty(),
            "Mismatched lockfile deps should not merge"
        );
        assert_eq!(files[0].for_packages.len(), 1);
        assert!(
            files[1].for_packages.is_empty(),
            "Mismatched lockfile should remain unassigned"
        );
    }

    #[test]
    fn test_assemble_npm_package_json_skips_lockfile_with_same_name_different_version() {
        let mut files = vec![
            create_test_file_info(
                "project/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/my-app@1.0.0"),
                Some("my-app"),
                Some("1.0.0"),
                vec![],
            ),
            create_test_file_info(
                "project/package-lock.json",
                DatasourceId::NpmPackageLockJson,
                Some("pkg:npm/my-app@2.0.0"),
                Some("my-app"),
                Some("2.0.0"),
                vec![Dependency {
                    purl: Some("pkg:npm/left-pad@1.3.0".to_string()),
                    extracted_requirement: Some("1.3.0".to_string()),
                    scope: Some("dependencies".to_string()),
                    is_runtime: Some(true),
                    is_optional: Some(false),
                    is_pinned: Some(true),
                    is_direct: Some(false),
                    resolved_package: None,
                    extra_data: None,
                }],
            ),
        ];

        let result = assemble(&mut files);

        assert_eq!(result.packages.len(), 1);
        assert_eq!(result.packages[0].name, Some("my-app".to_string()));
        assert_eq!(result.packages[0].version, Some("1.0.0".to_string()));
        assert_eq!(
            result.packages[0].datafile_paths,
            vec!["project/package.json".to_string()]
        );
        assert!(result.dependencies.is_empty());
        assert!(files[1].for_packages.is_empty());
    }

    #[test]
    fn test_assemble_npm_package_json_skips_lockfile_with_same_version_different_name() {
        let mut files = vec![
            create_test_file_info(
                "project/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/my-app@1.0.0"),
                Some("my-app"),
                Some("1.0.0"),
                vec![],
            ),
            create_test_file_info(
                "project/package-lock.json",
                DatasourceId::NpmPackageLockJson,
                Some("pkg:npm/other-app@1.0.0"),
                Some("other-app"),
                Some("1.0.0"),
                vec![Dependency {
                    purl: Some("pkg:npm/left-pad@1.3.0".to_string()),
                    extracted_requirement: Some("1.3.0".to_string()),
                    scope: Some("dependencies".to_string()),
                    is_runtime: Some(true),
                    is_optional: Some(false),
                    is_pinned: Some(true),
                    is_direct: Some(false),
                    resolved_package: None,
                    extra_data: None,
                }],
            ),
        ];

        let result = assemble(&mut files);

        assert_eq!(result.packages.len(), 1);
        assert_eq!(result.packages[0].name, Some("my-app".to_string()));
        assert_eq!(result.packages[0].version, Some("1.0.0".to_string()));
        assert_eq!(
            result.packages[0].datafile_paths,
            vec!["project/package.json".to_string()]
        );
        assert!(result.dependencies.is_empty());
        assert!(files[1].for_packages.is_empty());
    }

    #[test]
    fn test_assemble_npm_package_json_skips_lockfile_with_missing_identity() {
        let mut files = vec![
            create_test_file_info(
                "project/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/my-app@1.0.0"),
                Some("my-app"),
                Some("1.0.0"),
                vec![],
            ),
            create_test_file_info(
                "project/package-lock.json",
                DatasourceId::NpmPackageLockJson,
                None,
                Some("my-app"),
                None,
                vec![Dependency {
                    purl: Some("pkg:npm/left-pad@1.3.0".to_string()),
                    extracted_requirement: Some("1.3.0".to_string()),
                    scope: Some("dependencies".to_string()),
                    is_runtime: Some(true),
                    is_optional: Some(false),
                    is_pinned: Some(true),
                    is_direct: Some(false),
                    resolved_package: None,
                    extra_data: None,
                }],
            ),
        ];

        let result = assemble(&mut files);

        assert_eq!(result.packages.len(), 1);
        assert_eq!(result.packages[0].name, Some("my-app".to_string()));
        assert_eq!(result.packages[0].version, Some("1.0.0".to_string()));
        assert_eq!(
            result.packages[0].datafile_paths,
            vec!["project/package.json".to_string()]
        );
        assert!(result.dependencies.is_empty());
        assert!(files[1].for_packages.is_empty());
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
    fn test_assemble_npm_lockfile_does_not_create_package_when_manifest_has_no_purl() {
        let dep = Dependency {
            purl: Some("pkg:npm/express@4.18.0".to_string()),
            extracted_requirement: Some("4.18.0".to_string()),
            scope: Some("dependencies".to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: Some(true),
            is_direct: Some(true),
            resolved_package: None,
            extra_data: None,
        };

        let mut files = vec![
            create_test_file_info(
                "project/package.json",
                DatasourceId::NpmPackageJson,
                None,
                None,
                None,
                vec![],
            ),
            create_test_file_info(
                "project/package-lock.json",
                DatasourceId::NpmPackageLockJson,
                Some("pkg:npm/lock-only@1.0.0"),
                Some("lock-only"),
                Some("1.0.0"),
                vec![dep],
            ),
        ];

        let result = assemble(&mut files);

        assert!(result.packages.is_empty());
        assert_eq!(result.dependencies.len(), 1);
        assert_eq!(result.dependencies[0].for_package_uid, None);
        assert!(files[0].for_packages.is_empty());
        assert!(files[1].for_packages.is_empty());
    }

    #[test]
    fn test_assign_npm_package_resources_uses_manifest_roots_without_package_type() {
        let mut files = vec![
            create_plain_file_info(".pnp.cjs"),
            create_plain_file_info("index.js"),
            create_test_file_info(
                "package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/root-app@1.0.0"),
                Some("root-app"),
                Some("1.0.0"),
                vec![],
            ),
            create_plain_file_info("node_modules/child/index.js"),
            create_test_file_info(
                "node_modules/child/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/child@2.0.0"),
                Some("child"),
                Some("2.0.0"),
                vec![],
            ),
            create_plain_file_info("node_modules/child/node_modules/grand/index.js"),
            create_test_file_info(
                "node_modules/child/node_modules/grand/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/grand@3.0.0"),
                Some("grand"),
                Some("3.0.0"),
                vec![],
            ),
        ];

        let result = assemble(&mut files);

        assert_eq!(result.packages.len(), 3);

        let ownerships: HashMap<String, String> = files
            .iter()
            .filter(|file| !file.for_packages.is_empty())
            .map(|file| {
                (
                    file.path.clone(),
                    stable_uid_key(&file.for_packages[0]).to_string(),
                )
            })
            .collect();

        assert_eq!(ownerships.len(), 7);
        assert_eq!(
            ownerships.get(".pnp.cjs"),
            Some(&"pkg:npm/root-app@1.0.0".to_string())
        );
        assert_eq!(
            ownerships.get("index.js"),
            Some(&"pkg:npm/root-app@1.0.0".to_string())
        );
        assert_eq!(
            ownerships.get("package.json"),
            Some(&"pkg:npm/root-app@1.0.0".to_string())
        );
        assert_eq!(
            ownerships.get("node_modules/child/index.js"),
            Some(&"pkg:npm/child@2.0.0".to_string())
        );
        assert_eq!(
            ownerships.get("node_modules/child/package.json"),
            Some(&"pkg:npm/child@2.0.0".to_string())
        );
        assert_eq!(
            ownerships.get("node_modules/child/node_modules/grand/index.js"),
            Some(&"pkg:npm/grand@3.0.0".to_string())
        );
        assert_eq!(
            ownerships.get("node_modules/child/node_modules/grand/package.json"),
            Some(&"pkg:npm/grand@3.0.0".to_string())
        );
    }

    #[test]
    fn test_npm_resource_path_helpers_use_segment_aware_matching() {
        assert_eq!(package_dir("package.json"), "");
        assert_eq!(
            package_dir("node_modules/child/package.json"),
            "node_modules/child"
        );

        assert!(path_is_within_root("index.js", ""));
        assert!(path_is_within_root(
            "node_modules/child/index.js",
            "node_modules/child"
        ));
        assert!(!path_is_within_root(
            "node_modules/childish/index.js",
            "node_modules/child"
        ));

        assert!(is_first_level_node_modules(
            "node_modules/child/index.js",
            ""
        ));
        assert!(is_first_level_node_modules(
            "packages/app/node_modules/foo/index.js",
            "packages/app"
        ));
        assert!(!is_first_level_node_modules(
            "node_modules/child/index.js",
            "node_modules/child"
        ));
    }

    #[test]
    fn test_collect_npm_package_roots_uses_assembled_packages_when_file_ownership_is_partial() {
        let root_uid = "pkg:npm/root-app@1.0.0?uuid=root".to_string();
        let child_uid = "pkg:npm/child@2.0.0?uuid=child".to_string();
        let grand_uid = "pkg:npm/grand@3.0.0?uuid=grand".to_string();

        let mut root_manifest = create_test_file_info(
            "package.json",
            DatasourceId::NpmPackageJson,
            Some("pkg:npm/root-app@1.0.0"),
            Some("root-app"),
            Some("1.0.0"),
            vec![],
        );
        root_manifest.for_packages.push(root_uid.clone());

        let child_manifest = create_test_file_info(
            "node_modules/child/package.json",
            DatasourceId::NpmPackageJson,
            Some("pkg:npm/child@2.0.0"),
            Some("child"),
            Some("2.0.0"),
            vec![],
        );

        let grand_manifest = create_test_file_info(
            "node_modules/child/node_modules/grand/package.json",
            DatasourceId::NpmPackageJson,
            Some("pkg:npm/grand@3.0.0"),
            Some("grand"),
            Some("3.0.0"),
            vec![],
        );

        let files = vec![root_manifest, child_manifest, grand_manifest];
        let packages = vec![
            Package {
                package_uid: root_uid.clone(),
                purl: Some("pkg:npm/root-app@1.0.0".to_string()),
                datafile_paths: vec!["package.json".to_string()],
                datasource_ids: vec![DatasourceId::NpmPackageJson],
                ..Package::from_package_data(
                    &PackageData {
                        datasource_id: Some(DatasourceId::NpmPackageJson),
                        purl: Some("pkg:npm/root-app@1.0.0".to_string()),
                        name: Some("root-app".to_string()),
                        version: Some("1.0.0".to_string()),
                        ..Default::default()
                    },
                    "package.json".to_string(),
                )
            },
            Package {
                package_uid: child_uid.clone(),
                purl: Some("pkg:npm/child@2.0.0".to_string()),
                datafile_paths: vec!["node_modules/child/package.json".to_string()],
                datasource_ids: vec![DatasourceId::NpmPackageJson],
                ..Package::from_package_data(
                    &PackageData {
                        datasource_id: Some(DatasourceId::NpmPackageJson),
                        purl: Some("pkg:npm/child@2.0.0".to_string()),
                        name: Some("child".to_string()),
                        version: Some("2.0.0".to_string()),
                        ..Default::default()
                    },
                    "node_modules/child/package.json".to_string(),
                )
            },
            Package {
                package_uid: grand_uid.clone(),
                purl: Some("pkg:npm/grand@3.0.0".to_string()),
                datafile_paths: vec![
                    "node_modules/child/node_modules/grand/package.json".to_string(),
                ],
                datasource_ids: vec![DatasourceId::NpmPackageJson],
                ..Package::from_package_data(
                    &PackageData {
                        datasource_id: Some(DatasourceId::NpmPackageJson),
                        purl: Some("pkg:npm/grand@3.0.0".to_string()),
                        name: Some("grand".to_string()),
                        version: Some("3.0.0".to_string()),
                        ..Default::default()
                    },
                    "node_modules/child/node_modules/grand/package.json".to_string(),
                )
            },
        ];

        let roots = collect_npm_package_roots(&files, &packages);

        assert_eq!(
            roots,
            vec![
                ("".to_string(), root_uid),
                ("node_modules/child".to_string(), child_uid),
                (
                    "node_modules/child/node_modules/grand".to_string(),
                    grand_uid,
                ),
            ]
        );
    }

    #[test]
    fn test_ensure_npm_manifest_packages_restores_unowned_nested_manifests() {
        let mut files = vec![
            create_test_file_info(
                "package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/root-app@1.0.0"),
                Some("root-app"),
                Some("1.0.0"),
                vec![],
            ),
            create_test_file_info(
                "node_modules/child/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/child@2.0.0"),
                Some("child"),
                Some("2.0.0"),
                vec![],
            ),
            create_test_file_info(
                "node_modules/child/node_modules/grand/package.json",
                DatasourceId::NpmPackageJson,
                Some("pkg:npm/grand@3.0.0"),
                Some("grand"),
                Some("3.0.0"),
                vec![],
            ),
        ];
        let mut packages = Vec::new();
        let mut dependencies = Vec::new();

        ensure_npm_manifest_packages(&mut files, &mut packages, &mut dependencies);

        assert_eq!(packages.len(), 3);
        assert!(dependencies.is_empty());

        let ownerships: std::collections::HashMap<String, Vec<String>> = files
            .iter()
            .map(|file| (file.path.clone(), file.for_packages.clone()))
            .collect();

        assert_eq!(ownerships.get("package.json").map(Vec::len), Some(1));
        assert_eq!(
            ownerships
                .get("node_modules/child/package.json")
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            ownerships
                .get("node_modules/child/node_modules/grand/package.json")
                .map(Vec::len),
            Some(1)
        );

        let package_paths: Vec<String> = packages
            .iter()
            .flat_map(|package| package.datafile_paths.clone())
            .collect();
        assert!(package_paths.contains(&"package.json".to_string()));
        assert!(package_paths.contains(&"node_modules/child/package.json".to_string()));
        assert!(
            package_paths
                .contains(&"node_modules/child/node_modules/grand/package.json".to_string())
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
            holders: vec![],
            authors: vec![],
            emails: vec![],
            urls: vec![],
            for_packages: vec![],
            scan_errors: vec![],
            is_source: None,
            source_count: None,
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
