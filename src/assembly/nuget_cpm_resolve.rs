use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::models::{DatasourceId, FileInfo, PackageData, TopLevelDependency};

pub(super) fn resolve_nuget_cpm_versions(
    files: &[FileInfo],
    dependencies: &mut [TopLevelDependency],
) {
    let props_by_path = collect_directory_packages_props(files);

    for dependency in dependencies {
        if !is_nuget_project_dependency(dependency.datasource_id)
            || dependency.extracted_requirement.is_some()
        {
            continue;
        }

        let Some(project_dir) = Path::new(&dependency.datafile_path).parent() else {
            continue;
        };

        let Some(props_data) = find_nearest_directory_packages_props(project_dir, &props_by_path)
        else {
            continue;
        };

        if !is_central_package_management_enabled(props_data) {
            continue;
        }

        if let Some(version) = resolve_central_package_version(dependency, props_data) {
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

fn find_nearest_directory_packages_props<'a>(
    project_dir: &Path,
    props_by_path: &'a HashMap<PathBuf, &'a PackageData>,
) -> Option<&'a PackageData> {
    for ancestor in project_dir.ancestors() {
        let candidate = ancestor.join("Directory.Packages.props");
        if let Some(package_data) = props_by_path.get(&candidate) {
            return Some(package_data);
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

fn is_central_package_management_enabled(package_data: &PackageData) -> bool {
    package_data
        .extra_data
        .as_ref()
        .and_then(|extra_data| extra_data.get("manage_package_versions_centrally"))
        .and_then(|value| value.as_bool())
        == Some(true)
}

fn resolve_central_package_version(
    dependency: &TopLevelDependency,
    package_data: &PackageData,
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
