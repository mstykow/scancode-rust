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
        &TextDetectionOptions::default(),
    );

    let mut files = result.files;
    let assembly_result = assembly::assemble(&mut files);
    (files, assembly_result)
}
