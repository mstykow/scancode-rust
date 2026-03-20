use std::path::{Path, PathBuf};

use crate::models::{FileInfo, Package, PackageType};

pub fn assign_npm_package_resources(files: &mut [FileInfo], packages: &[Package]) {
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

fn is_first_level_node_modules(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .is_some_and(|component| component.as_os_str() == "node_modules")
}
