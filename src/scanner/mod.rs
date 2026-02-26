mod count;
mod process;

use crate::models::FileInfo;

/// Aggregated result of scanning a directory tree.
///
/// Includes discovered file entries and the count of paths skipped by
/// exclusion patterns.
pub struct ProcessResult {
    /// File and directory entries produced by the scan.
    pub files: Vec<FileInfo>,
    /// Number of excluded paths encountered during traversal.
    pub excluded_count: usize,
}

pub use self::count::count;
pub use self::process::process;
