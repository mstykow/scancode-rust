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

#[derive(Debug, Clone)]
pub struct TextDetectionOptions {
    pub detect_copyrights: bool,
    pub detect_emails: bool,
    pub detect_urls: bool,
    pub max_emails: usize,
    pub max_urls: usize,
    pub timeout_seconds: f64,
    pub verbose_paths: bool,
}

impl Default for TextDetectionOptions {
    fn default() -> Self {
        Self {
            detect_copyrights: true,
            detect_emails: false,
            detect_urls: false,
            max_emails: 50,
            max_urls: 50,
            timeout_seconds: 120.0,
            verbose_paths: false,
        }
    }
}

pub use self::count::count;
pub use self::process::{process, process_with_options};

#[cfg(test)]
mod tests {
    use super::TextDetectionOptions;

    #[test]
    fn default_options_keep_copyright_detection_enabled() {
        let options = TextDetectionOptions::default();
        assert!(options.detect_copyrights);
    }
}
