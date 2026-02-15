use std::path::{Path, PathBuf};

use crate::models::{FileInfo, Package, PackageData, TopLevelDependency};

use super::AssemblerConfig;

pub fn assemble_nested_patterns(
    files: &[FileInfo],
    config: &AssemblerConfig,
) -> Option<(Package, Vec<TopLevelDependency>, Vec<usize>)> {
    if !has_nested_patterns(config) {
        return None;
    }

    let matching_files = find_matching_files(files, config);
    if matching_files.is_empty() {
        return None;
    }

    let package_root = find_package_root(&matching_files, files)?;

    let sibling_indices = find_nested_siblings(&package_root, files, config);

    if sibling_indices.len() < 2 {
        return None;
    }

    assemble_from_indices(config, files, &sibling_indices)
}

fn has_nested_patterns(config: &AssemblerConfig) -> bool {
    config
        .sibling_file_patterns
        .iter()
        .any(|p| p.contains("**"))
}

fn find_matching_files(files: &[FileInfo], config: &AssemblerConfig) -> Vec<usize> {
    files
        .iter()
        .enumerate()
        .filter(|(_, file)| {
            file.package_data.iter().any(|pkg_data| {
                pkg_data
                    .datasource_id
                    .is_some_and(|dsid| config.datasource_ids.contains(&dsid))
            })
        })
        .map(|(idx, _)| idx)
        .collect()
}

const NESTED_ANCHOR_DIRS: &[&str] = &["META-INF", "debian"];

fn find_package_root(matching_indices: &[usize], files: &[FileInfo]) -> Option<PathBuf> {
    for &idx in matching_indices {
        let file_path = Path::new(&files[idx].path);

        for &anchor in NESTED_ANCHOR_DIRS {
            if file_path.components().any(|c| c.as_os_str() == anchor) {
                let mut current = file_path;
                while let Some(parent) = current.parent() {
                    if parent
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|name| name == anchor)
                    {
                        return parent.parent().map(|p| p.to_path_buf());
                    }
                    current = parent;
                }
            }
        }

        if file_path.file_name().and_then(|n| n.to_str()) == Some("pom.xml") {
            return file_path.parent().map(|p| p.to_path_buf());
        }
    }

    None
}

fn find_nested_siblings(root: &Path, files: &[FileInfo], config: &AssemblerConfig) -> Vec<usize> {
    files
        .iter()
        .enumerate()
        .filter(|(_, file)| {
            let file_path = Path::new(&file.path);

            if !file_path.starts_with(root) {
                return false;
            }

            config.sibling_file_patterns.iter().any(|pattern| {
                if pattern.contains("**") {
                    matches_nested_pattern(&file.path, pattern)
                } else {
                    file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|name| matches_simple_pattern(name, pattern))
                }
            })
        })
        .map(|(idx, _)| idx)
        .collect()
}

fn matches_nested_pattern(file_path: &str, pattern: &str) -> bool {
    let pattern_without_prefix = pattern.strip_prefix("**/").unwrap_or(pattern);

    file_path.contains(pattern_without_prefix)
}

fn matches_simple_pattern(file_name: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('*') {
        file_name.ends_with(suffix)
            || file_name
                .to_ascii_lowercase()
                .ends_with(&suffix.to_ascii_lowercase())
    } else {
        file_name == pattern || file_name.eq_ignore_ascii_case(pattern)
    }
}

fn assemble_from_indices(
    config: &AssemblerConfig,
    files: &[FileInfo],
    indices: &[usize],
) -> Option<(Package, Vec<TopLevelDependency>, Vec<usize>)> {
    let mut package: Option<Package> = None;
    let mut dependencies = Vec::new();
    let mut affected_indices = Vec::new();

    for &pattern in config.sibling_file_patterns {
        for &idx in indices {
            let file = &files[idx];
            let file_path = Path::new(&file.path);
            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            let matches = if pattern.contains("**") {
                matches_nested_pattern(&file.path, pattern)
            } else {
                matches_simple_pattern(file_name, pattern)
            };

            if !matches {
                continue;
            }

            if file.package_data.is_empty() {
                continue;
            }

            affected_indices.push(idx);

            for pkg_data in &file.package_data {
                if !is_handled_by(pkg_data, config) {
                    continue;
                }

                let datafile_path = file.path.clone();
                let Some(datasource_id) = pkg_data.datasource_id else {
                    continue;
                };

                match &mut package {
                    None => {
                        if pkg_data.purl.is_some() {
                            package =
                                Some(Package::from_package_data(pkg_data, datafile_path.clone()));
                        }
                    }
                    Some(pkg) => {
                        pkg.update(pkg_data, datafile_path.clone());
                    }
                }

                let for_package_uid = package.as_ref().map(|p| p.package_uid.clone());

                for dep in &pkg_data.dependencies {
                    if dep.purl.is_some() {
                        dependencies.push(TopLevelDependency::from_dependency(
                            dep,
                            datafile_path.clone(),
                            datasource_id,
                            for_package_uid.clone(),
                        ));
                    }
                }
            }
        }
    }

    package.map(|pkg| (pkg, dependencies, affected_indices))
}

fn is_handled_by(pkg_data: &PackageData, config: &AssemblerConfig) -> bool {
    pkg_data
        .datasource_id
        .is_some_and(|dsid| config.datasource_ids.contains(&dsid))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::DatasourceId;

    #[test]
    fn test_has_nested_patterns() {
        let config_nested = AssemblerConfig {
            datasource_ids: &[DatasourceId::MavenPom],
            sibling_file_patterns: &["pom.xml", "**/META-INF/MANIFEST.MF"],
            mode: crate::assembly::AssemblyMode::SiblingMerge,
        };
        assert!(has_nested_patterns(&config_nested));

        let config_simple = AssemblerConfig {
            datasource_ids: &[DatasourceId::NpmPackageJson],
            sibling_file_patterns: &["package.json", "package-lock.json"],
            mode: crate::assembly::AssemblyMode::SiblingMerge,
        };
        assert!(!has_nested_patterns(&config_simple));
    }

    #[test]
    fn test_matches_nested_pattern() {
        assert!(matches_nested_pattern(
            "my-lib/META-INF/MANIFEST.MF",
            "**/META-INF/MANIFEST.MF"
        ));
        assert!(matches_nested_pattern(
            "path/to/jar/META-INF/MANIFEST.MF",
            "**/META-INF/MANIFEST.MF"
        ));
        assert!(!matches_nested_pattern(
            "path/to/jar/pom.xml",
            "**/META-INF/MANIFEST.MF"
        ));
    }

    #[test]
    fn test_matches_simple_pattern() {
        assert!(matches_simple_pattern("pom.xml", "pom.xml"));
        assert!(matches_simple_pattern("Cargo.toml", "cargo.toml"));
        assert!(matches_simple_pattern("MyLib.podspec", "*.podspec"));
        assert!(!matches_simple_pattern("package.json", "pom.xml"));
    }

    #[test]
    fn test_find_package_root() {
        use crate::models::FileType;

        let files = vec![
            FileInfo::new(
                "pom.xml".to_string(),
                "pom".to_string(),
                "xml".to_string(),
                "my-lib/pom.xml".to_string(),
                FileType::File,
                Some("application/xml".to_string()),
                100,
                None,
                None,
                None,
                None,
                None,
                vec![],
                None,
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            FileInfo::new(
                "MANIFEST.MF".to_string(),
                "MANIFEST".to_string(),
                "MF".to_string(),
                "my-lib/META-INF/MANIFEST.MF".to_string(),
                FileType::File,
                Some("text/plain".to_string()),
                50,
                None,
                None,
                None,
                None,
                None,
                vec![],
                None,
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            ),
        ];

        let root = find_package_root(&[0, 1], &files);
        assert_eq!(root, Some(PathBuf::from("my-lib")));
    }

    #[test]
    fn test_find_package_root_debian() {
        use crate::models::FileType;

        let files = vec![
            FileInfo::new(
                "control".to_string(),
                "control".to_string(),
                "".to_string(),
                "my-pkg/debian/control".to_string(),
                FileType::File,
                Some("text/plain".to_string()),
                200,
                None,
                None,
                None,
                None,
                None,
                vec![],
                None,
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            FileInfo::new(
                "copyright".to_string(),
                "copyright".to_string(),
                "".to_string(),
                "my-pkg/debian/copyright".to_string(),
                FileType::File,
                Some("text/plain".to_string()),
                150,
                None,
                None,
                None,
                None,
                None,
                vec![],
                None,
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            ),
        ];

        let root = find_package_root(&[0, 1], &files);
        assert_eq!(root, Some(PathBuf::from("my-pkg")));
    }
}
