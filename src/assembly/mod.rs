mod assemblers;
#[cfg(test)]
mod assembly_golden_test;
#[cfg(test)]
mod assembly_test;
mod cargo_workspace_merge;
pub mod file_ref_resolve;
mod hackage_merge;
mod nested_merge;
mod sibling_merge;
mod swift_merge;
mod workspace_merge;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::models::{
    DatasourceId, FileInfo, Package, PackageData, PackageType, TopLevelDependency,
};

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

            if config_key == DatasourceId::SwiftPackageManifestJson {
                continue;
            }

            if config_key == DatasourceId::HackageCabal {
                let results = hackage_merge::assemble_hackage_packages(files, file_indices);
                for (package, deps, assigned_indices) in results {
                    if let Some(package) = package {
                        let package_uid = package.package_uid.clone();
                        for idx in &assigned_indices {
                            if !files[*idx].for_packages.contains(&package_uid) {
                                files[*idx].for_packages.push(package_uid.clone());
                            }
                        }
                        packages.push(package);
                    }
                    dependencies.extend(deps);
                }
                continue;
            }

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
            let removed_package_uids: Vec<String> = packages
                .iter()
                .filter(|p| p.purl == purl)
                .map(|p| p.package_uid.clone())
                .collect();

            packages.retain(|p| p.purl != purl);
            dependencies.retain(|d| {
                d.for_package_uid.as_ref() != Some(&package_uid)
                    && !removed_package_uids
                        .iter()
                        .any(|old_uid| d.for_package_uid.as_ref() == Some(old_uid))
            });

            for idx in &affected_indices {
                files[*idx].for_packages.clear();
                files[*idx].for_packages.push(package_uid.clone());
                assembled_indices.insert(*idx);
            }

            packages.push(pkg);
            dependencies.extend(deps);
        }
    }

    swift_merge::assemble_swift_packages(files, &mut packages, &mut dependencies);

    merge_conda_rootfs_metadata(files, &mut packages, &mut dependencies);

    assign_npm_package_resources(files, &packages);

    file_ref_resolve::resolve_file_references(files, &mut packages, &mut dependencies);
    file_ref_resolve::merge_rpm_yumdb_metadata(files, &mut packages);

    // Post-processing: workspace assembly for npm/pnpm monorepos
    workspace_merge::assemble_workspaces(files, &mut packages, &mut dependencies);

    // Post-processing: workspace assembly for Cargo workspaces
    cargo_workspace_merge::assemble_cargo_workspaces(files, &mut packages, &mut dependencies);

    assign_cargo_package_resources(files, &packages);
    assign_composer_package_resources(files, &packages);
    assign_ruby_package_resources(files, &packages);

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

fn merge_conda_rootfs_metadata(
    files: &mut [FileInfo],
    packages: &mut Vec<Package>,
    dependencies: &mut [TopLevelDependency],
) {
    let conda_json_data: Vec<(String, PackageData, String)> = files
        .iter()
        .flat_map(|file| {
            file.package_data.iter().filter_map(|pkg_data| {
                if pkg_data.datasource_id != Some(DatasourceId::CondaMetaJson) {
                    return None;
                }

                Some((
                    file.path.clone(),
                    pkg_data.clone(),
                    pkg_data
                        .extra_data
                        .as_ref()?
                        .get("extracted_package_dir")?
                        .as_str()?
                        .replace('\\', "/")
                        .split("/pkgs/")
                        .nth(1)?
                        .to_string(),
                ))
            })
        })
        .collect();

    let json_package_uids: HashMap<Option<String>, String> = packages
        .iter()
        .filter(|package| {
            package
                .datasource_ids
                .contains(&DatasourceId::CondaMetaJson)
        })
        .map(|package| (package.purl.clone(), package.package_uid.clone()))
        .collect();

    let mut removal_indices = Vec::new();

    for (json_path, pkg_data, package_dir_name) in conda_json_data {
        let Some(target_idx) = packages.iter().enumerate().find_map(|(idx, package)| {
            if !package
                .datasource_ids
                .contains(&DatasourceId::CondaMetaYaml)
            {
                return None;
            }

            let matches_recipe = package.datafile_paths.iter().any(|path| {
                path.contains(&format!("pkgs/{package_dir_name}/info/recipe/meta.yaml"))
                    || path.contains(&format!("pkgs/{package_dir_name}/info/recipe/meta.yml"))
                    || path.contains(&format!(
                        "pkgs/{package_dir_name}/info/recipe.tar-extract/recipe/meta.yaml"
                    ))
                    || path.contains(&format!(
                        "pkgs/{package_dir_name}/info/recipe.tar-extract/recipe/meta.yml"
                    ))
            });

            (matches_recipe && package.purl == pkg_data.purl).then_some(idx)
        }) else {
            continue;
        };

        let old_uid = json_package_uids.get(&pkg_data.purl).cloned();
        packages[target_idx].update(&pkg_data, json_path);
        let new_uid = packages[target_idx].package_uid.clone();

        if let Some(old_uid) = old_uid {
            for file in files.iter_mut() {
                for package_uid in &mut file.for_packages {
                    if *package_uid == old_uid {
                        *package_uid = new_uid.clone();
                    }
                }
            }

            for dep in dependencies.iter_mut() {
                if dep.for_package_uid.as_deref() == Some(old_uid.as_str()) {
                    dep.for_package_uid = Some(new_uid.clone());
                }
            }

            if let Some(idx) = packages
                .iter()
                .position(|package| package.package_uid == old_uid)
            {
                removal_indices.push(idx);
            }
        }
    }

    removal_indices.sort_unstable();
    removal_indices.dedup();
    for idx in removal_indices.into_iter().rev() {
        packages.remove(idx);
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
    let mut package_roots: Vec<(PathBuf, String)> = packages
        .iter()
        .filter(|package| package.package_type == Some(PackageType::Npm))
        .filter_map(|package| {
            package
                .datafile_paths
                .first()
                .and_then(|path| Path::new(path).parent())
                .map(|root| (root.to_path_buf(), package.package_uid.clone()))
        })
        .collect();

    package_roots.sort_by(|(left_root, _), (right_root, _)| {
        right_root
            .components()
            .count()
            .cmp(&left_root.components().count())
    });

    for file in files.iter_mut() {
        let path = Path::new(&file.path);
        if let Some((_, package_uid)) = package_roots
            .iter()
            .find(|(root, _)| path.starts_with(root) && !is_first_level_node_modules(path, root))
        {
            file.for_packages.clear();
            file.for_packages.push(package_uid.clone());
        }
    }
}

fn assign_cargo_package_resources(files: &mut [FileInfo], packages: &[Package]) {
    let cargo_roots: Vec<(PathBuf, String)> = packages
        .iter()
        .filter(|package| package.package_type == Some(PackageType::Cargo))
        .filter_map(|package| {
            let root = package
                .datafile_paths
                .iter()
                .find(|path| {
                    Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.eq_ignore_ascii_case("cargo.toml"))
                })
                .and_then(|path| Path::new(path).parent())?
                .to_path_buf();

            Some((root, package.package_uid.clone()))
        })
        .collect();

    if cargo_roots.is_empty() {
        return;
    }

    for file in files.iter_mut() {
        let path = Path::new(&file.path);

        for (root, package_uid) in &cargo_roots {
            if !path.starts_with(root) || is_target_path(path, root) {
                continue;
            }

            if !file.for_packages.contains(package_uid) {
                file.for_packages.push(package_uid.clone());
            }
        }
    }
}

fn assign_composer_package_resources(files: &mut [FileInfo], packages: &[Package]) {
    let composer_roots: Vec<(PathBuf, String)> = packages
        .iter()
        .filter(|package| package.package_type == Some(PackageType::Composer))
        .filter_map(|package| {
            let root = package
                .datafile_paths
                .iter()
                .find(|path| {
                    Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(is_composer_manifest_filename)
                })
                .and_then(|path| Path::new(path).parent())?
                .to_path_buf();

            Some((root, package.package_uid.clone()))
        })
        .collect();

    if composer_roots.is_empty() {
        return;
    }

    for file in files.iter_mut() {
        let path = Path::new(&file.path);

        for (root, package_uid) in &composer_roots {
            if !path.starts_with(root)
                || is_vendor_path(path, root)
                || is_scancode_cache_path(path, root)
            {
                continue;
            }

            if composer_roots.iter().any(|(other_root, _)| {
                other_root != root && other_root.starts_with(root) && path.starts_with(other_root)
            }) {
                continue;
            }

            if !file.for_packages.contains(package_uid) {
                file.for_packages.push(package_uid.clone());
            }
        }
    }
}

fn is_vendor_path(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .is_some_and(|component| component.as_os_str() == "vendor")
}

fn is_target_path(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .is_some_and(|component| component.as_os_str() == "target")
}

fn is_composer_manifest_filename(name: &str) -> bool {
    name == "composer.json"
        || name.ends_with(".composer.json")
        || (name.starts_with("composer.") && name.ends_with(".json"))
}

fn assign_ruby_package_resources(files: &mut [FileInfo], packages: &[Package]) {
    let ruby_roots: Vec<(PathBuf, String)> = packages
        .iter()
        .filter(|package| package.package_type == Some(PackageType::Gem))
        .filter_map(|package| {
            ruby_package_root(package).map(|root| (root, package.package_uid.clone()))
        })
        .collect();

    if ruby_roots.is_empty() {
        return;
    }

    for file in files.iter_mut() {
        let path = Path::new(&file.path);

        for (root, package_uid) in &ruby_roots {
            if !path.starts_with(root) || is_scancode_cache_path(path, root) {
                continue;
            }

            if ruby_roots.iter().any(|(other_root, _)| {
                other_root != root && other_root.starts_with(root) && path.starts_with(other_root)
            }) {
                continue;
            }

            if !file.for_packages.contains(package_uid) {
                file.for_packages.push(package_uid.clone());
            }
        }
    }
}

fn ruby_package_root(package: &Package) -> Option<PathBuf> {
    for datafile_path in &package.datafile_paths {
        let path = Path::new(datafile_path);

        if path.file_name().and_then(|n| n.to_str()) == Some("metadata.gz-extract") {
            return path.parent().map(|p| p.to_path_buf());
        }

        if path
            .components()
            .any(|c| c.as_os_str() == "data.gz-extract")
        {
            let mut current = path;
            while let Some(parent) = current.parent() {
                if parent.file_name().and_then(|n| n.to_str()) == Some("data.gz-extract") {
                    return parent.parent().map(|p| p.to_path_buf());
                }
                current = parent;
            }
        }

        if let Some(parent) = path.parent() {
            return Some(parent.to_path_buf());
        }
    }

    None
}

fn is_scancode_cache_path(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .is_some_and(|component| component.as_os_str() == ".scancode-cache")
}

fn is_first_level_node_modules(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .is_some_and(|component| component.as_os_str() == "node_modules")
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
