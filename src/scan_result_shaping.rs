use glob::Pattern;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};

use crate::models::FileInfo;

#[cfg(test)]
#[path = "scan_result_shaping_test.rs"]
mod scan_result_shaping_test;

fn matches_patterns(path: &str, patterns: &[Pattern]) -> bool {
    if patterns.is_empty() {
        return true;
    }

    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    patterns
        .iter()
        .any(|pattern| pattern.matches(path) || pattern.matches(file_name))
}

pub(crate) fn apply_include_filter(files: &mut Vec<FileInfo>, include_patterns: &[Pattern]) {
    let mut explicitly_included_files = HashSet::new();
    let mut explicitly_included_dirs = Vec::<String>::new();

    for entry in files.iter() {
        if matches_patterns(&entry.path, include_patterns) {
            match entry.file_type {
                crate::models::FileType::File => {
                    explicitly_included_files.insert(entry.path.clone());
                }
                crate::models::FileType::Directory => {
                    explicitly_included_dirs.push(entry.path.clone());
                }
            }
        }
    }

    let mut kept_file_paths = HashSet::new();
    for entry in files.iter() {
        if entry.file_type != crate::models::FileType::File {
            continue;
        }

        let explicitly = explicitly_included_files.contains(&entry.path);
        let under_included_dir = explicitly_included_dirs
            .iter()
            .any(|dir| Path::new(&entry.path).starts_with(Path::new(dir)));

        if explicitly || under_included_dir {
            kept_file_paths.insert(entry.path.clone());
        }
    }

    files.retain(|entry| match entry.file_type {
        crate::models::FileType::File => kept_file_paths.contains(&entry.path),
        crate::models::FileType::Directory => {
            explicitly_included_dirs.contains(&entry.path)
                || kept_file_paths
                    .iter()
                    .any(|path| Path::new(path).starts_with(Path::new(&entry.path)))
        }
    });
}

fn has_findings(file: &FileInfo) -> bool {
    file.license_expression.is_some()
        || !file.license_detections.is_empty()
        || !file.copyrights.is_empty()
        || !file.holders.is_empty()
        || !file.authors.is_empty()
        || !file.emails.is_empty()
        || !file.urls.is_empty()
        || !file.package_data.is_empty()
        || !file.scan_errors.is_empty()
}

pub(crate) fn apply_only_findings_filter(files: &mut Vec<FileInfo>) {
    let kept_file_paths: HashSet<String> = files
        .iter()
        .filter(|entry| entry.file_type == crate::models::FileType::File && has_findings(entry))
        .map(|entry| entry.path.clone())
        .collect();

    files.retain(|entry| match entry.file_type {
        crate::models::FileType::File => kept_file_paths.contains(&entry.path),
        crate::models::FileType::Directory => kept_file_paths
            .iter()
            .any(|path| Path::new(path).starts_with(Path::new(&entry.path))),
    });
}

fn dedupe_vec_by_key<T, K, F>(items: &mut Vec<T>, mut key_fn: F)
where
    K: std::hash::Hash + Eq,
    F: FnMut(&T) -> K,
{
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(key_fn(item)));
}

pub(crate) fn filter_redundant_clues(files: &mut [FileInfo]) {
    for file in files.iter_mut() {
        dedupe_vec_by_key(&mut file.copyrights, |c| {
            (c.copyright.clone(), c.start_line, c.end_line)
        });
        dedupe_vec_by_key(&mut file.holders, |h| {
            (h.holder.clone(), h.start_line, h.end_line)
        });
        dedupe_vec_by_key(&mut file.authors, |a| {
            (a.author.clone(), a.start_line, a.end_line)
        });
        dedupe_vec_by_key(&mut file.emails, |e| {
            (e.email.clone(), e.start_line, e.end_line)
        });
        dedupe_vec_by_key(&mut file.urls, |u| {
            (u.url.clone(), u.start_line, u.end_line)
        });
    }
}

pub(crate) fn normalize_paths(
    files: &mut [FileInfo],
    scan_root: &str,
    strip_root: bool,
    full_root: bool,
) {
    for entry in files.iter_mut() {
        let current_path = PathBuf::from(&entry.path);

        if full_root {
            let absolute_candidate = if current_path.is_absolute() {
                current_path.clone()
            } else {
                env::current_dir()
                    .map(|cwd| cwd.join(&current_path))
                    .unwrap_or(current_path.clone())
            };
            let absolute = absolute_candidate
                .canonicalize()
                .unwrap_or(absolute_candidate);
            entry.path = absolute.to_string_lossy().to_string();
            continue;
        }

        if strip_root && let Some(stripped) = strip_root_prefix(&current_path, Path::new(scan_root))
        {
            entry.path = stripped.to_string_lossy().to_string();
        }
    }
}

fn strip_root_prefix(path: &Path, root: &Path) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(root)
        && !stripped.as_os_str().is_empty()
    {
        return Some(stripped.to_path_buf());
    }

    let canonical_path = path.canonicalize().ok()?;
    let canonical_root = root.canonicalize().ok()?;
    let stripped = canonical_path.strip_prefix(canonical_root).ok()?;
    if stripped.as_os_str().is_empty() {
        None
    } else {
        Some(stripped.to_path_buf())
    }
}

pub(crate) fn apply_mark_source(files: &mut [FileInfo]) {
    let mut index_by_path = HashMap::<String, usize>::new();
    for (idx, entry) in files.iter().enumerate() {
        index_by_path.insert(entry.path.clone(), idx);
    }

    for entry in files.iter_mut() {
        if entry.file_type == crate::models::FileType::File {
            if entry.is_source != Some(false) {
                entry.is_source = entry.programming_language.as_ref().map(|_| true);
            }
            entry.source_count = None;
        }
    }

    let mut dir_paths = files
        .iter()
        .filter(|entry| entry.file_type == crate::models::FileType::Directory)
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    dir_paths.sort_by_key(|path| usize::MAX - Path::new(path).components().count());

    let mut direct_file_count = HashMap::<String, usize>::new();
    let mut direct_source_file_count = HashMap::<String, usize>::new();
    let mut child_dirs = HashMap::<String, Vec<String>>::new();

    for entry in files.iter() {
        if let Some(parent) = Path::new(&entry.path).parent().and_then(|p| p.to_str()) {
            let parent_key = parent.to_string();
            if entry.file_type == crate::models::FileType::File {
                if entry.is_source != Some(false) {
                    *direct_file_count.entry(parent_key.clone()).or_insert(0) += 1;
                    if entry.is_source.unwrap_or(false) {
                        *direct_source_file_count.entry(parent_key).or_insert(0) += 1;
                    }
                }
            } else {
                child_dirs
                    .entry(parent_key)
                    .or_default()
                    .push(entry.path.clone());
            }
        }
    }

    let mut descendant_file_count = HashMap::<String, usize>::new();
    let mut descendant_source_count = HashMap::<String, usize>::new();

    for dir_path in dir_paths {
        let mut total_files = *direct_file_count.get(&dir_path).unwrap_or(&0);
        let mut source_files = *direct_source_file_count.get(&dir_path).unwrap_or(&0);

        if let Some(children) = child_dirs.get(&dir_path) {
            for child in children {
                total_files += descendant_file_count.get(child).copied().unwrap_or(0);
                source_files += descendant_source_count.get(child).copied().unwrap_or(0);
            }
        }

        let qualifies = total_files > 0 && (source_files as f64 / total_files as f64) >= 0.9;

        if let Some(idx) = index_by_path.get(&dir_path)
            && let Some(entry) = files.get_mut(*idx)
        {
            if qualifies && source_files > 0 {
                entry.is_source = Some(true);
                entry.source_count = Some(source_files);
            } else {
                entry.is_source = None;
                entry.source_count = None;
            }
        }

        descendant_file_count.insert(dir_path.clone(), total_files);
        descendant_source_count.insert(dir_path, if qualifies { source_files } else { 0 });
    }
}
