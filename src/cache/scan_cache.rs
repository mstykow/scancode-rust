use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::io::{CacheIoError, load_snapshot_payload, write_snapshot_payload};
use super::metadata::{CacheInvalidationKey, CacheSnapshotMetadata};
use super::paths::scan_result_cache_path;
use crate::models::{
    Author, Copyright, FileInfo, Holder, LicenseDetection, Match, OutputEmail, OutputURL,
    PackageData,
};

const SCAN_CACHE_SCHEMA_VERSION: u32 = 2;
const SCAN_CACHE_ENGINE_VERSION: &str = "scan-result-cache-v2";
const SCAN_CACHE_RULES_FINGERPRINT: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedScanFindings {
    pub package_data: Vec<PackageData>,
    pub license_expression: Option<String>,
    pub license_detections: Vec<LicenseDetection>,
    pub license_clues: Vec<Match>,
    pub percentage_of_license_text: Option<f64>,
    pub copyrights: Vec<Copyright>,
    pub holders: Vec<Holder>,
    pub authors: Vec<Author>,
    pub emails: Vec<OutputEmail>,
    pub urls: Vec<OutputURL>,
    pub programming_language: Option<String>,
}

impl CachedScanFindings {
    pub fn from_file_info(file_info: &FileInfo) -> Self {
        Self {
            package_data: file_info.package_data.clone(),
            license_expression: file_info.license_expression.clone(),
            license_detections: file_info.license_detections.clone(),
            license_clues: file_info.license_clues.clone(),
            percentage_of_license_text: file_info.percentage_of_license_text,
            copyrights: file_info.copyrights.clone(),
            holders: file_info.holders.clone(),
            authors: file_info.authors.clone(),
            emails: file_info.emails.clone(),
            urls: file_info.urls.clone(),
            programming_language: file_info.programming_language.clone(),
        }
    }
}

pub fn read_cached_findings(
    scan_results_dir: &Path,
    sha256: &str,
    options_fingerprint: &str,
) -> Result<Option<CachedScanFindings>, CacheIoError> {
    let Some(path) = scan_result_cache_path(scan_results_dir, sha256) else {
        return Ok(None);
    };

    let key = CacheInvalidationKey {
        cache_schema_version: SCAN_CACHE_SCHEMA_VERSION,
        engine_version: SCAN_CACHE_ENGINE_VERSION,
        rules_fingerprint: SCAN_CACHE_RULES_FINGERPRINT,
        build_options_fingerprint: options_fingerprint,
    };

    let Some(payload) = load_snapshot_payload(&path, &key)? else {
        return Ok(None);
    };

    match rmp_serde::decode::from_slice::<CachedScanFindings>(&payload) {
        Ok(findings) => Ok(Some(findings)),
        Err(_) => Ok(None),
    }
}

pub fn write_cached_findings(
    scan_results_dir: &Path,
    sha256: &str,
    options_fingerprint: &str,
    findings: &CachedScanFindings,
) -> Result<(), CacheIoError> {
    let Some(path) = scan_result_cache_path(scan_results_dir, sha256) else {
        return Ok(());
    };

    let metadata = CacheSnapshotMetadata {
        cache_schema_version: SCAN_CACHE_SCHEMA_VERSION,
        engine_version: SCAN_CACHE_ENGINE_VERSION.to_string(),
        rules_fingerprint: SCAN_CACHE_RULES_FINGERPRINT.to_string(),
        build_options_fingerprint: options_fingerprint.to_string(),
        created_at: Utc::now().to_rfc3339(),
    };

    let payload = rmp_serde::to_vec(findings).map_err(CacheIoError::Encode)?;
    write_snapshot_payload(&path, &metadata, &payload)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn sample_sha256() -> &'static str {
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }

    #[test]
    fn test_write_and_read_cached_findings_roundtrip() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let scan_results_dir = temp_dir.path().join("scan-results");
        let findings = CachedScanFindings {
            package_data: Vec::new(),
            license_expression: Some("mit".to_string()),
            license_detections: Vec::new(),
            license_clues: Vec::new(),
            percentage_of_license_text: Some(100.0),
            copyrights: Vec::new(),
            holders: Vec::new(),
            authors: Vec::new(),
            emails: Vec::new(),
            urls: Vec::new(),
            programming_language: Some("Rust".to_string()),
        };

        write_cached_findings(
            &scan_results_dir,
            sample_sha256(),
            "cache-options-v1",
            &findings,
        )
        .expect("write cache entry");

        let loaded = read_cached_findings(&scan_results_dir, sample_sha256(), "cache-options-v1")
            .expect("read cache entry")
            .expect("cache hit");

        assert_eq!(loaded.license_expression, findings.license_expression);
        assert_eq!(loaded.license_clues, findings.license_clues);
        assert_eq!(
            loaded.percentage_of_license_text,
            findings.percentage_of_license_text
        );
        assert_eq!(loaded.programming_language, findings.programming_language);
    }

    #[test]
    fn test_write_and_read_cached_findings_roundtrip_with_license_clues() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let scan_results_dir = temp_dir.path().join("scan-results");
        let findings = CachedScanFindings {
            package_data: Vec::new(),
            license_expression: None,
            license_detections: Vec::new(),
            license_clues: vec![Match {
                license_expression: "unknown-license-reference".to_string(),
                license_expression_spdx: "LicenseRef-scancode-unknown-license-reference"
                    .to_string(),
                from_file: Some("NOTICE".to_string()),
                start_line: 1,
                end_line: 2,
                matcher: Some("2-aho".to_string()),
                score: 100.0,
                matched_length: Some(19),
                match_coverage: Some(100.0),
                rule_relevance: Some(100),
                rule_identifier: Some("license-clue_1.RULE".to_string()),
                rule_url: Some("https://example.com/license-clue_1.RULE".to_string()),
                matched_text: Some(
                    "This product currently only contains code developed by authors".to_string(),
                ),
                referenced_filenames: None,
                matched_text_diagnostics: Some(
                    "This product currently only contains code developed by [authors]".to_string(),
                ),
            }],
            percentage_of_license_text: Some(42.0),
            copyrights: Vec::new(),
            holders: Vec::new(),
            authors: Vec::new(),
            emails: Vec::new(),
            urls: Vec::new(),
            programming_language: None,
        };

        write_cached_findings(
            &scan_results_dir,
            sample_sha256(),
            "cache-options-v1",
            &findings,
        )
        .expect("write cache entry");

        let loaded = read_cached_findings(&scan_results_dir, sample_sha256(), "cache-options-v1")
            .expect("read cache entry")
            .expect("cache hit");

        assert_eq!(loaded.license_clues, findings.license_clues);
        assert_eq!(
            loaded.percentage_of_license_text,
            findings.percentage_of_license_text
        );
    }

    #[test]
    fn test_read_cached_findings_misses_on_fingerprint_change() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let scan_results_dir = temp_dir.path().join("scan-results");
        let findings = CachedScanFindings {
            package_data: Vec::new(),
            license_expression: Some("apache-2.0".to_string()),
            license_detections: Vec::new(),
            license_clues: Vec::new(),
            percentage_of_license_text: None,
            copyrights: Vec::new(),
            holders: Vec::new(),
            authors: Vec::new(),
            emails: Vec::new(),
            urls: Vec::new(),
            programming_language: Some("Rust".to_string()),
        };

        write_cached_findings(
            &scan_results_dir,
            sample_sha256(),
            "cache-options-v1",
            &findings,
        )
        .expect("write cache entry");

        let loaded = read_cached_findings(&scan_results_dir, sample_sha256(), "cache-options-v2")
            .expect("read cache entry");

        assert!(loaded.is_none());
    }
}
