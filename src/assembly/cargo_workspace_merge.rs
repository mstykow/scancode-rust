use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::warn;

use crate::models::{DatasourceId, FileInfo, Package, PackageData, TopLevelDependency};

pub fn assemble_cargo_workspaces(
    files: &mut [FileInfo],
    packages: &mut Vec<Package>,
    dependencies: &mut Vec<TopLevelDependency>,
) {
    let workspace_roots = find_workspace_roots(files);

    if workspace_roots.is_empty() {
        return;
    }

    for workspace_root in workspace_roots {
        process_workspace(files, packages, dependencies, &workspace_root);
    }
}

struct WorkspaceRoot {
    root_dir: PathBuf,
    root_cargo_toml_idx: usize,
    members: Vec<String>,
    workspace_data: WorkspaceData,
}

struct WorkspaceData {
    package: HashMap<String, serde_json::Value>,
    dependencies: HashMap<String, serde_json::Value>,
}

fn find_workspace_roots(files: &[FileInfo]) -> Vec<WorkspaceRoot> {
    let mut roots = Vec::new();

    for (idx, file) in files.iter().enumerate() {
        let path = Path::new(&file.path);
        let file_name = if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            name
        } else {
            continue;
        };

        if file_name != "Cargo.toml" {
            continue;
        }

        for pkg_data in &file.package_data {
            if pkg_data.datasource_id != Some(DatasourceId::CargoToml) {
                continue;
            }

            if let Some(workspace_info) = extract_workspace_info(pkg_data)
                && let Some(parent) = path.parent()
            {
                roots.push(WorkspaceRoot {
                    root_dir: parent.to_path_buf(),
                    root_cargo_toml_idx: idx,
                    members: workspace_info.members,
                    workspace_data: workspace_info.data,
                });
            }
        }
    }

    roots
}

struct WorkspaceInfo {
    members: Vec<String>,
    data: WorkspaceData,
}

fn extract_workspace_info(pkg_data: &PackageData) -> Option<WorkspaceInfo> {
    let extra_data = pkg_data.extra_data.as_ref()?;
    let workspace_value = extra_data.get("workspace")?;

    let members: Vec<String> = workspace_value
        .get("members")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    if members.is_empty() {
        return None;
    }

    let mut package_map = HashMap::new();
    if let Some(pkg_obj) = workspace_value.get("package").and_then(|v| v.as_object()) {
        for (key, value) in pkg_obj {
            package_map.insert(key.clone(), value.clone());
        }
    }

    let mut dependencies_map = HashMap::new();
    if let Some(deps_obj) = workspace_value
        .get("dependencies")
        .and_then(|v| v.as_object())
    {
        for (key, value) in deps_obj {
            dependencies_map.insert(key.clone(), value.clone());
        }
    }

    Some(WorkspaceInfo {
        members,
        data: WorkspaceData {
            package: package_map,
            dependencies: dependencies_map,
        },
    })
}

fn process_workspace(
    files: &mut [FileInfo],
    packages: &mut Vec<Package>,
    dependencies: &mut Vec<TopLevelDependency>,
    workspace_root: &WorkspaceRoot,
) {
    let member_indices = discover_members(files, workspace_root);

    if member_indices.is_empty() {
        warn!(
            "No workspace members found for patterns {:?} in {:?}",
            workspace_root.members, workspace_root.root_dir
        );
        return;
    }

    remove_root_package(
        files,
        workspace_root.root_cargo_toml_idx,
        packages,
        dependencies,
    );
    remove_member_packages(files, &member_indices, packages, dependencies);

    let member_packages =
        create_member_packages(files, &member_indices, &workspace_root.workspace_data);

    let member_uids: Vec<String> = member_packages
        .iter()
        .map(|(pkg, _deps)| pkg.package_uid.clone())
        .collect();

    for (pkg, deps) in member_packages {
        packages.push(pkg);
        dependencies.extend(deps);
    }

    assign_for_packages(files, workspace_root, &member_indices, &member_uids);
}

fn discover_members(files: &[FileInfo], workspace_root: &WorkspaceRoot) -> Vec<usize> {
    let mut member_indices = Vec::new();

    for (idx, file) in files.iter().enumerate() {
        let path = Path::new(&file.path);

        if path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml") {
            continue;
        }

        if !path.starts_with(&workspace_root.root_dir) {
            continue;
        }

        if idx == workspace_root.root_cargo_toml_idx {
            continue;
        }

        let has_valid_package = file
            .package_data
            .iter()
            .any(|pkg| pkg.datasource_id == Some(DatasourceId::CargoToml) && pkg.purl.is_some());
        if !has_valid_package {
            continue;
        }

        let relative_path = if let Ok(rel) = path.strip_prefix(&workspace_root.root_dir) {
            rel
        } else {
            continue;
        };

        let mut matched = false;
        for pattern in &workspace_root.members {
            if matches_member_pattern(relative_path, pattern) {
                matched = true;
                break;
            }
        }

        if matched {
            member_indices.push(idx);
        }
    }

    member_indices
}

fn matches_member_pattern(path: &Path, pattern: &str) -> bool {
    let path_str = path.to_str().unwrap_or("");

    if !pattern.contains('*') {
        let expected = format!("{}/Cargo.toml", pattern);
        return path_str == expected;
    }

    if let Ok(glob_pattern) = glob::Pattern::new(&format!("{}/Cargo.toml", pattern)) {
        return glob_pattern.matches(path_str);
    }

    false
}

fn remove_root_package(
    files: &[FileInfo],
    root_idx: usize,
    packages: &mut Vec<Package>,
    dependencies: &mut Vec<TopLevelDependency>,
) {
    let root_file = &files[root_idx];
    let root_purl = root_file
        .package_data
        .iter()
        .find(|pkg| pkg.datasource_id == Some(DatasourceId::CargoToml))
        .and_then(|pkg| pkg.purl.as_ref())
        .cloned();

    let Some(purl) = root_purl else {
        return;
    };

    let mut removed_uid = None;
    packages.retain(|pkg| {
        if pkg.purl.as_ref() == Some(&purl) {
            removed_uid = Some(pkg.package_uid.clone());
            false
        } else {
            true
        }
    });

    if let Some(uid) = &removed_uid {
        dependencies.retain(|dep| dep.for_package_uid.as_ref() != Some(uid));
    }
}

fn remove_member_packages(
    files: &[FileInfo],
    member_indices: &[usize],
    packages: &mut Vec<Package>,
    dependencies: &mut Vec<TopLevelDependency>,
) {
    let member_paths: Vec<&str> = member_indices
        .iter()
        .map(|&idx| files[idx].path.as_str())
        .collect();

    let removed_uids: Vec<String> = packages
        .iter()
        .filter(|pkg| {
            pkg.datafile_paths
                .iter()
                .any(|dp| member_paths.contains(&dp.as_str()))
        })
        .map(|pkg| pkg.package_uid.clone())
        .collect();

    packages.retain(|pkg| !removed_uids.contains(&pkg.package_uid));
    dependencies.retain(|dep| {
        dep.for_package_uid
            .as_ref()
            .is_none_or(|uid| !removed_uids.contains(uid))
    });
}

fn create_member_packages(
    files: &[FileInfo],
    member_indices: &[usize],
    workspace_data: &WorkspaceData,
) -> Vec<(Package, Vec<TopLevelDependency>)> {
    let mut results = Vec::new();

    for &idx in member_indices {
        let file = &files[idx];

        let pkg_data =
            if let Some(pkg) = file.package_data.iter().find(|pkg| {
                pkg.datasource_id == Some(DatasourceId::CargoToml) && pkg.purl.is_some()
            }) {
                pkg
            } else {
                continue;
            };

        let mut resolved_pkg_data = pkg_data.clone();
        apply_workspace_inheritance(&mut resolved_pkg_data, workspace_data);

        let datafile_path = file.path.clone();
        let datasource_id = DatasourceId::CargoToml;
        let package = Package::from_package_data(&resolved_pkg_data, datafile_path.clone());
        let for_package_uid = Some(package.package_uid.clone());

        let deps: Vec<TopLevelDependency> = resolved_pkg_data
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

        results.push((package, deps));
    }

    results
}

fn apply_workspace_inheritance(pkg_data: &mut PackageData, workspace_data: &WorkspaceData) {
    use packageurl::PackageUrl;

    let extra_data = if let Some(ed) = &mut pkg_data.extra_data {
        ed
    } else {
        return;
    };

    if extra_data.get("version").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(version_value) = workspace_data.package.get("version")
        && let Some(version_str) = version_value.as_str()
    {
        pkg_data.version = Some(version_str.to_string());
        extra_data.remove("version");
    }

    if extra_data.get("license").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(license_value) = workspace_data.package.get("license")
        && let Some(license_str) = license_value.as_str()
    {
        pkg_data.extracted_license_statement = Some(license_str.to_string());
        extra_data.remove("license");
    }

    if extra_data.get("homepage").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(homepage_value) = workspace_data.package.get("homepage")
        && let Some(homepage_str) = homepage_value.as_str()
    {
        pkg_data.homepage_url = Some(homepage_str.to_string());
        extra_data.remove("homepage");
    }

    if extra_data.get("repository").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(repo_value) = workspace_data.package.get("repository")
        && let Some(repo_str) = repo_value.as_str()
    {
        pkg_data.vcs_url = Some(repo_str.to_string());
        extra_data.remove("repository");
    }

    if extra_data.get("categories").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(categories_value) = workspace_data.package.get("categories")
        && let Some(categories_arr) = categories_value.as_array()
    {
        let categories: Vec<String> = categories_arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        pkg_data.keywords.extend(categories);
        extra_data.remove("categories");
    }

    if extra_data.get("edition").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(edition_value) = workspace_data.package.get("edition")
        && let Some(edition_str) = edition_value.as_str()
    {
        extra_data.insert("rust_edition".to_string(), serde_json::json!(edition_str));
        extra_data.remove("edition");
    }

    if extra_data.get("rust-version").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(rust_version_value) = workspace_data.package.get("rust-version")
        && let Some(rust_version_str) = rust_version_value.as_str()
    {
        extra_data.insert(
            "rust_version".to_string(),
            serde_json::json!(rust_version_str),
        );
        extra_data.remove("rust-version");
    }

    if extra_data.get("authors").and_then(|v| v.as_str()) == Some("workspace")
        && let Some(authors_value) = workspace_data.package.get("authors")
        && let Some(authors_arr) = authors_value.as_array()
    {
        use crate::parsers::utils::split_name_email;
        let parties: Vec<crate::models::Party> = authors_arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|author_str| {
                let (name, email) = split_name_email(author_str);
                crate::models::Party {
                    r#type: None,
                    role: Some("author".to_string()),
                    name,
                    email,
                    url: None,
                    organization: None,
                    organization_url: None,
                    timezone: None,
                }
            })
            .collect();
        pkg_data.parties = parties;
        extra_data.remove("authors");
    }

    if let (Some(name), Some(version)) = (&pkg_data.name, &pkg_data.version)
        && let Ok(purl) = PackageUrl::new("cargo", name)
    {
        let mut purl = purl;
        let _ = purl.with_version(version);
        pkg_data.purl = Some(purl.to_string());

        pkg_data.repository_download_url = Some(format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            name, version
        ));
    }

    for dep in &mut pkg_data.dependencies {
        if let Some(dep_extra) = &dep.extra_data
            && dep_extra.get("workspace").and_then(|v| v.as_bool()) == Some(true)
        {
            let dep_name = if let Some(purl_str) = &dep.purl {
                extract_cargo_dep_name(purl_str)
            } else {
                None
            };

            if let Some(dep_name) = dep_name
                && let Some(dep_value) = workspace_data.dependencies.get(&dep_name)
            {
                if let Some(version_str) = dep_value.as_str() {
                    dep.extracted_requirement = Some(version_str.to_string());
                } else if let Some(dep_obj) = dep_value.as_object()
                    && let Some(version_str) = dep_obj.get("version").and_then(|v| v.as_str())
                {
                    dep.extracted_requirement = Some(version_str.to_string());
                }
            }
        }
    }
}

fn extract_cargo_dep_name(purl: &str) -> Option<String> {
    let after_type = purl.strip_prefix("pkg:cargo/")?;
    let without_query = after_type.split('?').next().unwrap_or(after_type);
    let name_part = without_query.split('@').next().unwrap_or(without_query);
    Some(name_part.to_string())
}

fn assign_for_packages(
    files: &mut [FileInfo],
    workspace_root: &WorkspaceRoot,
    member_indices: &[usize],
    member_uids: &[String],
) {
    let mut member_dirs: Vec<PathBuf> = Vec::new();
    for &idx in member_indices {
        if let Some(parent) = Path::new(&files[idx].path).parent() {
            member_dirs.push(parent.to_path_buf());
        }
    }

    for file in files.iter_mut() {
        let path = Path::new(&file.path);
        if !path.starts_with(&workspace_root.root_dir) {
            continue;
        }

        file.for_packages.clear();

        let mut assigned = false;
        for (i, member_dir) in member_dirs.iter().enumerate() {
            if path.starts_with(member_dir) {
                file.for_packages.push(member_uids[i].clone());
                assigned = true;
                break;
            }
        }

        if assigned {
            continue;
        }

        if let Ok(rel) = path.strip_prefix(&workspace_root.root_dir)
            && let Some(first_component) = rel.components().next()
            && first_component.as_os_str() == "target"
        {
            continue;
        }

        for uid in member_uids {
            file.for_packages.push(uid.clone());
        }
    }
}
