use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::models::{DatasourceId, Dependency, FileInfo, PackageData, TopLevelDependency};

#[derive(Default)]
struct ResolvedCpmData {
    dependencies: Vec<Dependency>,
    manage_package_versions_centrally: Option<bool>,
    central_package_version_override_enabled: Option<bool>,
}

pub(super) fn resolve_nuget_cpm_versions(
    files: &[FileInfo],
    dependencies: &mut [TopLevelDependency],
) {
    let props_by_path = collect_directory_packages_props(files);
    let projects_by_path = collect_nuget_project_packages(files);

    for dependency in dependencies {
        if !is_nuget_project_dependency(dependency.datasource_id)
            || dependency.extracted_requirement.is_some()
        {
            continue;
        }

        let Some(project_dir) = Path::new(&dependency.datafile_path).parent() else {
            continue;
        };

        let Some(props_path) =
            find_nearest_directory_packages_props_path(project_dir, &props_by_path)
        else {
            continue;
        };

        let props_data = resolve_effective_directory_packages_props(
            &props_path,
            &props_by_path,
            &mut HashSet::new(),
        );

        if !is_central_package_management_enabled(&props_data) {
            continue;
        }

        let project_override_enabled = projects_by_path
            .get(&PathBuf::from(&dependency.datafile_path))
            .and_then(project_central_package_version_override_enabled)
            .unwrap_or(false);

        if let Some(version_override) =
            resolve_package_version_override(dependency, &props_data, project_override_enabled)
        {
            dependency.extracted_requirement = Some(version_override);
            continue;
        }

        if let Some(version) = resolve_central_package_version(dependency, &props_data) {
            dependency.extracted_requirement = Some(version);
        }
    }
}

fn collect_directory_packages_props(files: &[FileInfo]) -> HashMap<PathBuf, &PackageData> {
    let mut props_by_path = HashMap::new();

    for file in files {
        let Some(package_data) = file.package_data.iter().find(|package_data| {
            package_data.datasource_id == Some(DatasourceId::NugetDirectoryPackagesProps)
        }) else {
            continue;
        };

        props_by_path.insert(PathBuf::from(&file.path), package_data);
    }

    props_by_path
}

fn collect_nuget_project_packages(files: &[FileInfo]) -> HashMap<PathBuf, &PackageData> {
    let mut projects = HashMap::new();

    for file in files {
        let Some(package_data) = file.package_data.iter().find(|package_data| {
            package_data
                .datasource_id
                .is_some_and(is_nuget_project_dependency)
        }) else {
            continue;
        };

        projects.insert(PathBuf::from(&file.path), package_data);
    }

    projects
}

fn find_nearest_directory_packages_props_path(
    project_dir: &Path,
    props_by_path: &HashMap<PathBuf, &PackageData>,
) -> Option<PathBuf> {
    for ancestor in project_dir.ancestors() {
        let candidate = ancestor.join("Directory.Packages.props");
        if props_by_path.contains_key(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn is_nuget_project_dependency(datasource_id: DatasourceId) -> bool {
    matches!(
        datasource_id,
        DatasourceId::NugetCsproj | DatasourceId::NugetFsproj | DatasourceId::NugetVbproj
    )
}

fn resolve_effective_directory_packages_props(
    props_path: &Path,
    props_by_path: &HashMap<PathBuf, &PackageData>,
    visited: &mut HashSet<PathBuf>,
) -> ResolvedCpmData {
    let canonical = props_path
        .canonicalize()
        .unwrap_or_else(|_| props_path.to_path_buf());
    if !visited.insert(canonical) {
        return ResolvedCpmData::default();
    }

    let Some(package_data) = props_by_path.get(props_path) else {
        return ResolvedCpmData::default();
    };

    let mut resolved = ResolvedCpmData::default();
    for import_path in package_data
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("import_projects"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
    {
        let Some(imported_path) =
            resolve_recorded_import_project(props_path, import_path, props_by_path)
        else {
            continue;
        };
        let imported =
            resolve_effective_directory_packages_props(&imported_path, props_by_path, visited);
        merge_resolved_cpm_data(&mut resolved, imported);
    }

    if let Some(value) = package_data
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("manage_package_versions_centrally"))
        .and_then(|value| value.as_bool())
    {
        resolved.manage_package_versions_centrally = Some(value);
    }
    if let Some(value) = package_data
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("central_package_version_override_enabled"))
        .and_then(|value| value.as_bool())
    {
        resolved.central_package_version_override_enabled = Some(value);
    }

    replace_matching_dependencies(&mut resolved.dependencies, &package_data.dependencies);
    resolved
        .dependencies
        .extend(package_data.dependencies.iter().cloned());

    resolved
}

fn merge_resolved_cpm_data(target: &mut ResolvedCpmData, source: ResolvedCpmData) {
    if target.manage_package_versions_centrally.is_none() {
        target.manage_package_versions_centrally = source.manage_package_versions_centrally;
    }
    if target.central_package_version_override_enabled.is_none() {
        target.central_package_version_override_enabled =
            source.central_package_version_override_enabled;
    }
    replace_matching_dependencies(&mut target.dependencies, &source.dependencies);
    target.dependencies.extend(source.dependencies);
}

fn replace_matching_dependencies(target: &mut Vec<Dependency>, source: &[Dependency]) {
    if source.is_empty() {
        return;
    }

    let source_keys = source.iter().map(dependency_key).collect::<Vec<_>>();
    target.retain(|candidate| {
        !source_keys
            .iter()
            .any(|key| *key == dependency_key(candidate))
    });
}

fn dependency_key(dependency: &Dependency) -> (Option<String>, Option<String>, Option<String>) {
    (
        dependency.purl.clone(),
        dependency.scope.clone(),
        candidate_condition(dependency).map(ToOwned::to_owned),
    )
}

fn resolve_recorded_import_project(
    current_path: &Path,
    import_project: &str,
    props_by_path: &HashMap<PathBuf, &PackageData>,
) -> Option<PathBuf> {
    let trimmed = import_project.trim();
    if trimmed.is_empty() {
        return None;
    }

    if is_get_path_of_file_above_import(trimmed) {
        let start_dir = current_path.parent()?.parent()?;
        for ancestor in start_dir.ancestors() {
            let candidate = ancestor.join("Directory.Packages.props");
            if props_by_path.contains_key(&candidate) {
                return Some(candidate);
            }
        }
        return None;
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.file_name().and_then(|name| name.to_str()) != Some("Directory.Packages.props") {
        return None;
    }

    if candidate.is_absolute() {
        props_by_path.contains_key(&candidate).then_some(candidate)
    } else {
        let resolved = current_path.parent()?.join(candidate);
        props_by_path.contains_key(&resolved).then_some(resolved)
    }
}

fn is_get_path_of_file_above_import(project: &str) -> bool {
    project.replace(' ', "")
        == "$([MSBuild]::GetPathOfFileAbove(Directory.Packages.props,$(MSBuildThisFileDirectory)..))"
}

fn is_central_package_management_enabled(package_data: &ResolvedCpmData) -> bool {
    package_data.manage_package_versions_centrally == Some(true)
}

fn is_central_package_version_override_enabled(package_data: &ResolvedCpmData) -> bool {
    package_data.central_package_version_override_enabled == Some(true)
}

fn resolve_package_version_override(
    dependency: &TopLevelDependency,
    package_data: &ResolvedCpmData,
    project_override_enabled: bool,
) -> Option<String> {
    if !project_override_enabled && !is_central_package_version_override_enabled(package_data) {
        return None;
    }

    let version_override = dependency_version_override(dependency)?;
    if !is_literal_version_override(version_override) {
        return None;
    }

    let dependency_purl = dependency.purl.as_deref()?;
    let dependency_condition = dependency_condition(dependency);

    let matching_central_versions = package_data
        .dependencies
        .iter()
        .filter(|candidate| candidate.scope.as_deref() == Some("package_version"))
        .filter(|candidate| candidate.purl.as_deref() == Some(dependency_purl))
        .filter(|candidate| condition_matches(dependency_condition, candidate_condition(candidate)))
        .count();

    (matching_central_versions == 1).then(|| version_override.to_string())
}

fn resolve_central_package_version(
    dependency: &TopLevelDependency,
    package_data: &ResolvedCpmData,
) -> Option<String> {
    let dependency_purl = dependency.purl.as_deref()?;
    let dependency_condition = dependency_condition(dependency);

    let matches: Vec<&str> = package_data
        .dependencies
        .iter()
        .filter(|candidate| candidate.scope.as_deref() == Some("package_version"))
        .filter(|candidate| candidate.purl.as_deref() == Some(dependency_purl))
        .filter_map(|candidate| {
            let version = candidate.extracted_requirement.as_deref()?;
            condition_matches(dependency_condition, candidate_condition(candidate))
                .then_some(version)
        })
        .collect();

    match matches.as_slice() {
        [version] => Some((*version).to_string()),
        _ => None,
    }
}

fn dependency_condition(dependency: &TopLevelDependency) -> Option<&str> {
    dependency
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("condition"))
        .and_then(|value| value.as_str())
}

fn dependency_version_override(dependency: &TopLevelDependency) -> Option<&str> {
    if let Some(resolved) = dependency
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("version_override_resolved"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(resolved);
    }

    dependency
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("version_override"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn project_central_package_version_override_enabled(package_data: &&PackageData) -> Option<bool> {
    package_data
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("central_package_version_override_enabled"))
        .and_then(|value| value.as_bool())
}

fn is_literal_version_override(value: &str) -> bool {
    !value.contains("$(")
}

fn candidate_condition(candidate: &crate::models::Dependency) -> Option<&str> {
    candidate
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("condition"))
        .and_then(|value| value.as_str())
}

fn condition_matches(
    dependency_condition: Option<&str>,
    candidate_condition: Option<&str>,
) -> bool {
    match candidate_condition {
        None => true,
        Some(candidate_condition) => dependency_condition == Some(candidate_condition),
    }
}
