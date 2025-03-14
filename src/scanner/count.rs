use crate::utils::file::is_path_excluded;
use glob::Pattern;
use std::fs;
use std::path::Path;

pub fn count<P: AsRef<Path>>(
    path: P,
    max_depth: usize,
    exclude_patterns: &[Pattern],
) -> std::io::Result<(usize, usize, usize)> {
    let path = path.as_ref();

    if is_path_excluded(path, exclude_patterns) {
        return Ok((0, 0, 1));
    }

    let mut files_count = 0;
    let mut dirs_count = 1; // Count the current directory
    let mut excluded_count = 0;

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
        } else if metadata.is_dir() {
            dirs_count += 1;

            // Recursively process subdirectories if not at max depth
            if max_depth > 0 {
                let (sub_files, sub_dirs, sub_excluded) =
                    count(&entry_path, max_depth - 1, exclude_patterns)?;

                files_count += sub_files;
                dirs_count += sub_dirs - 1; // Avoid double-counting this directory
                excluded_count += sub_excluded;
            }
        }
    }

    Ok((files_count, dirs_count, excluded_count))
}
