mod count;
mod process;

use std::path::PathBuf;

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
    pub scan_cache_dir: Option<PathBuf>,
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
            scan_cache_dir: None,
        }
    }
}

pub use self::count::count_with_size;
pub use self::process::{process, process_with_options};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use tempfile::TempDir;

    use crate::askalono::{ScanStrategy, Store};
    use crate::models::FileType;
    use crate::progress::{ProgressMode, ScanProgress};

    use super::TextDetectionOptions;
    use super::process_with_options;

    #[test]
    fn default_options_keep_copyright_detection_enabled() {
        let options = TextDetectionOptions::default();
        assert!(options.detect_copyrights);
    }

    fn scan_strategy_without_licenses() -> ScanStrategy<'static> {
        let store = Box::leak(Box::new(Store::new()));
        ScanStrategy::new(store)
    }

    fn scan_single_file(
        file_name: &str,
        content: &str,
        options: &TextDetectionOptions,
    ) -> crate::models::FileInfo {
        let temp_dir = TempDir::new().expect("create temp dir");
        let file_path = temp_dir.path().join(file_name);
        fs::write(&file_path, content).expect("write test file");

        let progress = Arc::new(ScanProgress::new(ProgressMode::Quiet));
        let strategy = scan_strategy_without_licenses();
        let result = process_with_options(temp_dir.path(), 0, progress, &[], &strategy, options)
            .expect("scan should succeed");

        result
            .files
            .into_iter()
            .find(|entry| {
                entry.file_type == FileType::File && entry.path == file_path.to_string_lossy()
            })
            .expect("scanned file entry")
    }

    #[test]
    fn scanner_reports_repeated_email_occurrences() {
        let options = TextDetectionOptions {
            detect_copyrights: false,
            detect_emails: true,
            detect_urls: false,
            max_emails: 50,
            max_urls: 50,
            timeout_seconds: 120.0,
            scan_cache_dir: None,
        };
        let scanned = scan_single_file(
            "contacts.txt",
            "linux@3ware.com\nlinux@3ware.com\nandre@suse.com\nlinux@3ware.com\n",
            &options,
        );

        let emails: Vec<(&str, usize)> = scanned
            .emails
            .iter()
            .map(|email| (email.email.as_str(), email.start_line))
            .collect();

        assert_eq!(emails.len(), 4, "emails: {emails:#?}");
        assert_eq!(
            emails,
            vec![
                ("linux@3ware.com", 1),
                ("linux@3ware.com", 2),
                ("andre@suse.com", 3),
                ("linux@3ware.com", 4),
            ]
        );
    }
}
