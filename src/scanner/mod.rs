mod collect;
mod process;

use std::path::PathBuf;

use crate::models::FileInfo;

pub struct ProcessResult {
    pub files: Vec<FileInfo>,
    pub excluded_count: usize,
}

#[derive(Debug, Clone)]
pub struct TextDetectionOptions {
    pub detect_copyrights: bool,
    pub detect_generated: bool,
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
            detect_generated: false,
            detect_emails: false,
            detect_urls: false,
            max_emails: 50,
            max_urls: 50,
            timeout_seconds: 120.0,
            scan_cache_dir: None,
        }
    }
}

#[allow(unused_imports)]
pub use self::collect::{CollectedPaths, collect_paths};
pub use self::process::process_collected;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use tempfile::TempDir;

    use crate::models::FileType;
    use crate::progress::{ProgressMode, ScanProgress};

    use super::{TextDetectionOptions, collect_paths, process_collected};

    #[test]
    fn default_options_keep_copyright_detection_enabled() {
        let options = TextDetectionOptions::default();
        assert!(options.detect_copyrights);
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
        let collected = collect_paths(temp_dir.path(), 0, &[]);
        let result = process_collected(&collected, progress, None, false, options);

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
            detect_generated: false,
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

    #[test]
    fn scanner_skips_pem_certificate_text_detection() {
        let options = TextDetectionOptions {
            detect_copyrights: true,
            detect_generated: false,
            detect_emails: true,
            detect_urls: true,
            max_emails: 50,
            max_urls: 50,
            timeout_seconds: 120.0,
            scan_cache_dir: None,
        };
        let pem_fixture = concat!(
            "-----BEGIN CERTIFICATE-----\n",
            "MIID8TCCAtmgAwIBAgIQQT1yx/RrH4FDffHSKFTfmjANBgkqhkiG9w0BAQUFADCB\n",
            "ijELMAkGA1UEBhMCQ0gxEDAOBgNVBAoTB1dJU2VLZXkxGzAZBgNVBAsTEkNvcHly\n",
            "-----END CERTIFICATE-----\n",
            "Certificate:\n",
            "    Data:\n",
            "        Signature Algorithm: sha1WithRSAEncryption\n",
            "        Issuer: C=CH, O=WISeKey, OU=Copyright (c) 2005, OU=OISTE Foundation Endorsed\n",
            "        Subject: C=CH, O=WISeKey, OU=Copyright (c) 2005, OU=OISTE Foundation Endorsed\n",
            "        Contact: cert-owner@example.com\n",
        );
        let scanned = scan_single_file("cert.pem", pem_fixture, &options);

        assert!(
            scanned.copyrights.is_empty(),
            "copyrights: {:#?}",
            scanned.copyrights
        );
        assert!(
            scanned.holders.is_empty(),
            "holders: {:#?}",
            scanned.holders
        );
        assert!(
            scanned.authors.is_empty(),
            "authors: {:#?}",
            scanned.authors
        );
        assert!(scanned.emails.is_empty(), "emails: {:#?}", scanned.emails);
        assert!(scanned.urls.is_empty(), "urls: {:#?}", scanned.urls);
        assert!(
            scanned.license_detections.is_empty(),
            "licenses: {:#?}",
            scanned.license_detections
        );
    }

    #[test]
    fn scanner_detects_structured_credits_authors() {
        let options = TextDetectionOptions {
            detect_copyrights: true,
            detect_generated: false,
            detect_emails: false,
            detect_urls: false,
            max_emails: 50,
            max_urls: 50,
            timeout_seconds: 120.0,
            scan_cache_dir: None,
        };
        let credits_fixture = concat!(
            "N: Jack Lloyd\n",
            "E: lloyd@randombit.net\n",
            "W: http://www.randombit.net/\n",
        );
        let scanned = scan_single_file("CREDITS", credits_fixture, &options);

        let authors: Vec<(&str, usize, usize)> = scanned
            .authors
            .iter()
            .map(|author| (author.author.as_str(), author.start_line, author.end_line))
            .collect();

        assert_eq!(
            authors,
            vec![(
                "Jack Lloyd lloyd@randombit.net http://www.randombit.net/",
                1,
                3,
            )]
        );
        assert!(scanned.copyrights.is_empty());
        assert!(scanned.holders.is_empty());
    }

    #[test]
    fn scanner_sets_generated_flag_when_enabled() {
        let options = TextDetectionOptions {
            detect_copyrights: false,
            detect_generated: true,
            detect_emails: false,
            detect_urls: false,
            max_emails: 50,
            max_urls: 50,
            timeout_seconds: 120.0,
            scan_cache_dir: None,
        };
        let scanned = scan_single_file(
            "generated.c",
            "/* DO NOT EDIT THIS FILE - it is machine generated */\n",
            &options,
        );

        assert_eq!(scanned.is_generated, Some(true));
    }
}
