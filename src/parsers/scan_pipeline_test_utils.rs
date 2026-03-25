use std::path::Path;
use std::sync::Arc;

use crate::assembly;
use crate::models::FileInfo;
use crate::progress::{ProgressMode, ScanProgress};
use crate::scanner::{TextDetectionOptions, collect_paths, process_collected};

pub(crate) fn scan_and_assemble(path: &Path) -> (Vec<FileInfo>, assembly::AssemblyResult) {
    let progress = Arc::new(ScanProgress::new(ProgressMode::Quiet));
    let collected = collect_paths(path, 0, &[]);
    let result = process_collected(
        &collected,
        progress,
        None,
        false,
        &TextDetectionOptions {
            detect_packages: true,
            ..TextDetectionOptions::default()
        },
    );

    let mut files = result.files;
    let assembly_result = assembly::assemble(&mut files);
    (files, assembly_result)
}

pub(crate) fn strip_root_paths(files: &mut [FileInfo], scan_root: &Path) {
    for entry in files {
        let current_path = Path::new(&entry.path);

        if let Some(stripped) = strip_root_prefix(current_path, scan_root) {
            entry.path = stripped.to_string_lossy().to_string();
        }
    }
}

fn strip_root_prefix(path: &Path, root: &Path) -> Option<std::path::PathBuf> {
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
