use crate::utils::file::is_path_excluded;
use glob::Pattern;
use std::fs;
use std::path::Path;

pub fn count_with_size<P: AsRef<Path>>(
    path: P,
    max_depth: usize,
    exclude_patterns: &[Pattern],
) -> std::io::Result<(usize, usize, usize, u64)> {
    let depth_limit = depth_limit_from_cli(max_depth);
    count_internal(path.as_ref(), depth_limit, exclude_patterns)
}

fn depth_limit_from_cli(max_depth: usize) -> Option<usize> {
    if max_depth == 0 {
        None
    } else {
        Some(max_depth)
    }
}

fn count_internal(
    path: &Path,
    depth_limit: Option<usize>,
    exclude_patterns: &[Pattern],
) -> std::io::Result<(usize, usize, usize, u64)> {
    if is_path_excluded(path, exclude_patterns) {
        return Ok((0, 0, 1, 0));
    }

    let mut files_count = 0;
    let mut dirs_count = 1; // Count the current directory
    let mut excluded_count = 0;
    let mut total_file_bytes = 0_u64;

    // Process entries in current directory
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if is_path_excluded(&entry_path, exclude_patterns) {
            excluded_count += 1;
            continue;
        }

        let metadata = entry.metadata()?;
        if metadata.is_file() {
            files_count += 1;
            total_file_bytes += metadata.len();
        } else if metadata.is_dir() {
            dirs_count += 1;

            let should_recurse = match depth_limit {
                None => true,
                Some(remaining_depth) => remaining_depth > 0,
            };

            if should_recurse {
                let next_depth_limit = depth_limit.map(|remaining_depth| remaining_depth - 1);
                let (sub_files, sub_dirs, sub_excluded, sub_bytes) =
                    count_internal(&entry_path, next_depth_limit, exclude_patterns)?;

                files_count += sub_files;
                dirs_count += sub_dirs - 1; // Avoid double-counting this directory
                excluded_count += sub_excluded;
                total_file_bytes += sub_bytes;
            }
        }
    }

    Ok((files_count, dirs_count, excluded_count, total_file_bytes))
}
