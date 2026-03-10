use super::*;
use crate::models::{
    Author, Copyright, DatasourceId, FileInfo, FileReference, FileType, Holder, Match, OutputEmail,
    OutputURL, Package, PackageType,
};
use clap::Parser;
use serde_json::json;
use std::fs;

fn file(path: &str) -> FileInfo {
    FileInfo::new(
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .extension()
            .and_then(|n| n.to_str())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default(),
        path.to_string(),
        FileType::File,
        None,
        1,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

fn dir(path: &str) -> FileInfo {
    FileInfo::new(
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        String::new(),
        path.to_string(),
        FileType::Directory,
        None,
        0,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

fn package(uid: &str, path: &str) -> Package {
    Package {
        package_type: Some(PackageType::Gem),
        namespace: None,
        name: Some("inspec-bin".to_string()),
        version: Some("6.8.2".to_string()),
        qualifiers: None,
        subpath: None,
        primary_language: Some("Ruby".to_string()),
        description: None,
        release_date: None,
        parties: vec![],
        keywords: vec![],
        homepage_url: None,
        download_url: None,
        size: None,
        sha1: None,
        md5: None,
        sha256: None,
        sha512: None,
        bug_tracking_url: None,
        code_view_url: None,
        vcs_url: None,
        copyright: None,
        holder: None,
        declared_license_expression: None,
        declared_license_expression_spdx: None,
        license_detections: vec![],
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: vec![],
        extracted_license_statement: None,
        notice_text: None,
        source_packages: vec![],
        is_private: false,
        is_virtual: false,
        extra_data: None,
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_ids: vec![DatasourceId::GemArchiveExtracted],
        purl: Some("pkg:gem/inspec-bin@6.8.2".to_string()),
        package_uid: uid.to_string(),
        datafile_paths: vec![path.to_string()],
    }
}

#[test]
fn include_filter_keeps_matching_files_and_parent_dirs() {
    let mut files = vec![
        dir("project"),
        dir("project/src"),
        file("project/src/main.rs"),
        file("project/README.md"),
    ];
    let include_patterns = vec![Pattern::new("*.rs").expect("valid pattern")];

    apply_include_filter(&mut files, &include_patterns);

    let paths: HashSet<_> = files.into_iter().map(|f| f.path).collect();
    assert!(paths.contains("project/src/main.rs"));
    assert!(paths.contains("project/src"));
    assert!(paths.contains("project"));
    assert!(!paths.contains("project/README.md"));
}

#[test]
fn only_findings_keeps_file_with_findings_and_parent_dirs() {
    let mut files = vec![dir("project"), file("project/a.txt"), file("project/b.txt")];
    files[2].copyrights = vec![Copyright {
        copyright: "Copyright Example".to_string(),
        start_line: 1,
        end_line: 1,
    }];

    apply_only_findings_filter(&mut files);

    let paths: HashSet<_> = files.into_iter().map(|f| f.path).collect();
    assert!(paths.contains("project"));
    assert!(paths.contains("project/b.txt"));
    assert!(!paths.contains("project/a.txt"));
}

#[test]
fn filter_redundant_clues_dedupes_exact_duplicates() {
    let mut files = vec![file("project/a.txt")];
    files[0].holders = vec![
        Holder {
            holder: "ACME".to_string(),
            start_line: 1,
            end_line: 1,
        },
        Holder {
            holder: "ACME".to_string(),
            start_line: 1,
            end_line: 1,
        },
    ];
    files[0].authors = vec![
        Author {
            author: "Jane".to_string(),
            start_line: 2,
            end_line: 2,
        },
        Author {
            author: "Jane".to_string(),
            start_line: 2,
            end_line: 2,
        },
    ];
    files[0].emails = vec![
        OutputEmail {
            email: "a@example.com".to_string(),
            start_line: 3,
            end_line: 3,
        },
        OutputEmail {
            email: "a@example.com".to_string(),
            start_line: 3,
            end_line: 3,
        },
    ];
    files[0].urls = vec![
        OutputURL {
            url: "https://example.com".to_string(),
            start_line: 4,
            end_line: 4,
        },
        OutputURL {
            url: "https://example.com".to_string(),
            start_line: 4,
            end_line: 4,
        },
    ];

    filter_redundant_clues(&mut files);

    assert_eq!(files[0].holders.len(), 1);
    assert_eq!(files[0].authors.len(), 1);
    assert_eq!(files[0].emails.len(), 1);
    assert_eq!(files[0].urls.len(), 1);
}

#[test]
fn normalize_paths_strip_root_removes_scan_root_prefix() {
    let mut files = vec![file("project/src/main.rs")];
    normalize_paths(&mut files, "project", true, false);
    assert_eq!(files[0].path, "src/main.rs");
}

#[test]
fn load_scan_from_json_reads_files_and_metadata_sections() {
    let temp_path = std::env::temp_dir().join("scancode-rust-from-json-test.json");
    let content = json!({
        "files": [
            {
                "name": "main.rs",
                "base_name": "main",
                "extension": ".rs",
                "path": "src/main.rs",
                "type": "file",
                "size": 10,
                "programming_language": "Rust"
            }
        ],
        "packages": [],
        "dependencies": [],
        "license_references": [
            {"name":"MIT","short_name":"MIT","spdx_license_key":"MIT","text":"..."}
        ],
        "license_rule_references": []
    });
    fs::write(&temp_path, content.to_string()).expect("write json fixture");

    let parsed = load_scan_from_json(temp_path.to_str().expect("utf-8 path"))
        .expect("from-json loading should succeed");

    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].path, "src/main.rs");
    assert_eq!(parsed.license_references.len(), 1);

    let _ = fs::remove_file(temp_path);
}

#[test]
fn mark_source_sets_directory_flags_at_threshold() {
    let mut files = vec![
        dir("project"),
        dir("project/src"),
        file("project/src/a.rs"),
        file("project/src/b.rs"),
        file("project/src/c.txt"),
    ];
    files[2].programming_language = Some("Rust".to_string());
    files[3].programming_language = Some("Rust".to_string());

    apply_mark_source(&mut files);

    let src = files
        .iter()
        .find(|f| f.path == "project/src")
        .expect("src directory exists");
    assert_eq!(src.is_source, None);
    assert_eq!(src.source_count, None);

    files[4].programming_language = Some("Rust".to_string());
    apply_mark_source(&mut files);

    let src_after = files
        .iter()
        .find(|f| f.path == "project/src")
        .expect("src directory exists");
    assert_eq!(src_after.is_source, Some(true));
    assert_eq!(src_after.source_count, Some(3));
}

#[test]
fn mark_source_ignores_go_test_only_files_for_directory_threshold() {
    let mut files = vec![
        dir("module"),
        file("module/main.go"),
        file("module/helper.go"),
        file("module/helper_test.go"),
    ];
    files[1].programming_language = Some("Go".to_string());
    files[2].programming_language = Some("Go".to_string());
    files[3].programming_language = Some("Go".to_string());
    files[3].is_source = Some(false);

    apply_mark_source(&mut files);

    let module_dir = files
        .iter()
        .find(|f| f.path == "module")
        .expect("module dir exists");
    assert_eq!(module_dir.is_source, Some(true));
    assert_eq!(module_dir.source_count, Some(2));

    let test_file = files
        .iter()
        .find(|f| f.path == "module/helper_test.go")
        .expect("test file exists");
    assert_eq!(test_file.is_source, Some(false));
}

#[test]
fn classify_key_files_marks_nested_ruby_license_from_file_references() {
    let uid = "pkg:gem/inspec-bin@6.8.2?uuid=test";
    let mut metadata_file = file("inspec-6.8.2/metadata.gz-extract");
    metadata_file.for_packages.push(uid.to_string());
    metadata_file.package_data = vec![crate::models::PackageData {
        package_type: Some(PackageType::Gem),
        datasource_id: Some(DatasourceId::GemArchiveExtracted),
        file_references: vec![FileReference {
            path: "inspec-6.8.2/inspec-bin/LICENSE".to_string(),
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            extra_data: None,
        }],
        ..Default::default()
    }];

    let mut license_file = file("inspec-6.8.2/inspec-bin/LICENSE");
    license_file.for_packages.push(uid.to_string());
    license_file.license_expression = Some("Apache-2.0".to_string());
    license_file.copyrights = vec![Copyright {
        copyright: "Copyright (c) 2019 Chef Software Inc.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    license_file.holders = vec![Holder {
        holder: "Chef Software Inc.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    license_file.license_detections = vec![crate::models::LicenseDetection {
        license_expression: "apache-2.0".to_string(),
        license_expression_spdx: "Apache-2.0".to_string(),
        matches: vec![Match {
            license_expression: "apache-2.0".to_string(),
            license_expression_spdx: "Apache-2.0".to_string(),
            from_file: Some("inspec-6.8.2/inspec-bin/LICENSE".to_string()),
            start_line: 1,
            end_line: 20,
            matcher: None,
            score: 100.0,
            matched_length: None,
            match_coverage: None,
            rule_relevance: None,
            rule_identifier: None,
            rule_url: None,
            matched_text: None,
        }],
        identifier: None,
    }];

    let mut files = vec![metadata_file, license_file];
    let packages = vec![package(uid, "inspec-6.8.2/metadata.gz-extract")];

    classify_key_files(&mut files, &packages);
    let license = files
        .iter()
        .find(|f| f.path.ends_with("/LICENSE"))
        .expect("license file exists");

    assert!(license.is_legal);
    assert!(license.is_top_level);
    assert!(license.is_key_file);
}

#[test]
fn promote_package_metadata_and_summary_from_key_files() {
    let uid = "pkg:gem/inspec-bin@6.8.2?uuid=test";
    let mut metadata_file = file("inspec-6.8.2/metadata.gz-extract");
    metadata_file.for_packages.push(uid.to_string());
    metadata_file.package_data = vec![crate::models::PackageData {
        package_type: Some(PackageType::Gem),
        datasource_id: Some(DatasourceId::GemArchiveExtracted),
        file_references: vec![FileReference {
            path: "inspec-6.8.2/inspec-bin/LICENSE".to_string(),
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            extra_data: None,
        }],
        ..Default::default()
    }];

    let mut license_file = file("inspec-6.8.2/inspec-bin/LICENSE");
    license_file.for_packages.push(uid.to_string());
    license_file.license_expression = Some("Apache-2.0".to_string());
    license_file.license_detections = vec![crate::models::LicenseDetection {
        license_expression: "apache-2.0".to_string(),
        license_expression_spdx: "Apache-2.0".to_string(),
        matches: vec![Match {
            license_expression: "apache-2.0".to_string(),
            license_expression_spdx: "Apache-2.0".to_string(),
            from_file: Some("inspec-6.8.2/inspec-bin/LICENSE".to_string()),
            start_line: 1,
            end_line: 20,
            matcher: None,
            score: 100.0,
            matched_length: None,
            match_coverage: None,
            rule_relevance: None,
            rule_identifier: None,
            rule_url: None,
            matched_text: None,
        }],
        identifier: None,
    }];
    license_file.copyrights = vec![Copyright {
        copyright: "Copyright (c) 2019 Chef Software Inc.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    license_file.holders = vec![Holder {
        holder: "Chef Software Inc.".to_string(),
        start_line: 1,
        end_line: 1,
    }];

    let mut files = vec![metadata_file, license_file];
    let mut packages = vec![package(uid, "inspec-6.8.2/metadata.gz-extract")];

    classify_key_files(&mut files, &packages);
    promote_package_metadata_from_key_files(&files, &mut packages);
    let summary = compute_summary(&files, &packages).expect("summary exists");

    assert_eq!(packages[0].holder.as_deref(), Some("Chef Software Inc."));
    assert_eq!(
        packages[0].declared_license_expression_spdx.as_deref(),
        Some("Apache-2.0")
    );
    assert_eq!(
        summary.declared_license_expression.as_deref(),
        Some("apache-2.0")
    );
    let score = summary.license_clarity_score.expect("score exists");
    assert_eq!(score.score, 100);
    assert!(score.declared_license);
    assert!(score.identification_precision);
    assert!(score.has_license_text);
    assert!(score.declared_copyrights);
    assert!(!score.ambiguous_compound_licensing);
}

#[test]
fn manifest_declared_license_contributes_to_summary_and_package_promotion() {
    let uid = "pkg:gem/demo@1.0.0?uuid=test";
    let mut gemspec = file("demo/demo.gemspec");
    gemspec.for_packages.push(uid.to_string());
    gemspec.package_data = vec![crate::models::PackageData {
        package_type: Some(PackageType::Gem),
        datasource_id: Some(DatasourceId::Gemspec),
        declared_license_expression: Some("mit".to_string()),
        declared_license_expression_spdx: Some("MIT".to_string()),
        ..Default::default()
    }];

    let mut files = vec![gemspec];
    let mut packages = vec![package(uid, "demo/demo.gemspec")];

    classify_key_files(&mut files, &packages);
    promote_package_metadata_from_key_files(&files, &mut packages);
    let summary = compute_summary(&files, &packages).expect("summary exists");

    assert!(files[0].is_manifest);
    assert!(files[0].is_key_file);
    assert_eq!(
        packages[0].declared_license_expression_spdx.as_deref(),
        Some("MIT")
    );
    assert_eq!(summary.declared_license_expression.as_deref(), Some("mit"));
    assert_eq!(summary.license_clarity_score.unwrap().score, 80);
}

#[test]
fn classify_key_files_does_not_tag_unreferenced_nested_legal_file() {
    let uid = "pkg:gem/demo@1.0.0?uuid=test";
    let mut gemspec = file("demo/demo.gemspec");
    gemspec.for_packages.push(uid.to_string());
    gemspec.package_data = vec![crate::models::PackageData {
        package_type: Some(PackageType::Gem),
        datasource_id: Some(DatasourceId::Gemspec),
        ..Default::default()
    }];

    let mut nested_license = file("demo/subdir/LICENSE");
    nested_license.for_packages.push(uid.to_string());

    let mut files = vec![gemspec, nested_license];
    let packages = vec![package(uid, "demo/demo.gemspec")];

    classify_key_files(&mut files, &packages);
    let nested = files
        .iter()
        .find(|f| f.path.ends_with("subdir/LICENSE"))
        .unwrap();

    assert!(nested.is_legal);
    assert!(!nested.is_top_level);
    assert!(!nested.is_key_file);
}

#[test]
fn compute_summary_uses_package_holder_and_primary_language() {
    let uid = "pkg:gem/demo@1.0.0?uuid=test";
    let mut root_package = package(uid, "demo/demo.gemspec");
    root_package.holder = Some("Example Corp.".to_string());
    root_package.primary_language = Some("Ruby".to_string());

    let mut other = package("pkg:pypi/demo?uuid=test2", "demo/setup.py");
    other.package_type = Some(PackageType::Pypi);
    other.purl = Some("pkg:pypi/demo".to_string());
    other.primary_language = Some("Python".to_string());
    other.holder = None;

    let mut extra_ruby = package("pkg:gem/demo-extra@1.0.0?uuid=test3", "demo/extra.gemspec");
    extra_ruby.name = Some("demo-extra".to_string());
    extra_ruby.purl = Some("pkg:gem/demo-extra@1.0.0".to_string());

    let files = vec![];
    let summary =
        compute_summary(&files, &[root_package, other, extra_ruby]).expect("summary exists");

    assert_eq!(summary.declared_holder.as_deref(), Some("Example Corp."));
    assert_eq!(summary.primary_language.as_deref(), Some("Ruby"));
    assert_eq!(summary.other_languages.len(), 1);
    assert_eq!(summary.other_languages[0].value.as_deref(), Some("Python"));
    assert_eq!(summary.other_languages[0].count, 1);
}

#[test]
fn compute_summary_uses_source_file_languages_when_packages_lack_them() {
    let mut ruby = file("project/lib/demo.rb");
    ruby.programming_language = Some("Ruby".to_string());
    ruby.is_source = Some(true);

    let mut ruby_two = file("project/lib/more.rb");
    ruby_two.programming_language = Some("Ruby".to_string());
    ruby_two.is_source = Some(true);

    let mut python = file("project/tools/helper.py");
    python.programming_language = Some("Python".to_string());
    python.is_source = Some(true);

    let summary = compute_summary(&[ruby, ruby_two, python], &[]).expect("summary exists");

    assert_eq!(summary.primary_language.as_deref(), Some("Ruby"));
    assert_eq!(summary.other_languages.len(), 1);
    assert_eq!(summary.other_languages[0].value.as_deref(), Some("Python"));
}

#[test]
fn compute_summary_without_license_evidence_has_no_clarity_score() {
    let uid = "pkg:gem/demo@1.0.0?uuid=test";
    let mut root_package = package(uid, "demo/demo.gemspec");
    root_package.holder = Some("Example Corp.".to_string());
    root_package.primary_language = Some("Ruby".to_string());

    let summary = compute_summary(&[], &[root_package]).expect("summary exists");

    assert_eq!(summary.declared_holder.as_deref(), Some("Example Corp."));
    assert_eq!(summary.primary_language.as_deref(), Some("Ruby"));
    assert!(summary.license_clarity_score.is_none());
}

#[test]
fn resolve_thread_count_supports_reference_compat_values() {
    assert_eq!(resolve_thread_count(-1), 1);
    assert_eq!(resolve_thread_count(0), default_parallel_threads());
    assert_eq!(resolve_thread_count(4), 4);
}

#[test]
fn validate_scan_option_compatibility_rejects_scan_flags_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "scancode-rust",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--copyright",
        "sample-scan.json",
    ])
    .expect("cli parse should succeed");

    let result = validate_scan_option_compatibility(&cli);
    assert!(result.is_err());
}

#[test]
fn validate_scan_option_compatibility_allows_scan_flags_without_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "scancode-rust",
        "--json-pp",
        "scan.json",
        "--copyright",
        "sample-dir",
    ])
    .expect("cli parse should succeed");

    let result = validate_scan_option_compatibility(&cli);
    assert!(result.is_ok());
}

#[test]
fn validate_scan_option_compatibility_allows_multiple_inputs_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "scancode-rust",
        "--json-pp",
        "scan.json",
        "--from-json",
        "scan-a.json",
        "scan-b.json",
    ])
    .expect("cli parse should succeed");

    let result = validate_scan_option_compatibility(&cli);
    assert!(result.is_ok());
}

#[test]
fn validate_scan_option_compatibility_rejects_multiple_paths_without_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "scancode-rust",
        "--json-pp",
        "scan.json",
        "dir-a",
        "dir-b",
    ])
    .expect("cli parse should succeed");

    let result = validate_scan_option_compatibility(&cli);
    assert!(result.is_err());
}

#[test]
fn progress_mode_from_cli_maps_quiet_verbose_default() {
    let default_cli =
        crate::cli::Cli::try_parse_from(["scancode-rust", "--json-pp", "scan.json", "sample-dir"])
            .expect("cli parse should succeed");
    assert_eq!(
        progress_mode_from_cli(&default_cli),
        crate::progress::ProgressMode::Default
    );

    let quiet_cli = crate::cli::Cli::try_parse_from([
        "scancode-rust",
        "--json-pp",
        "scan.json",
        "--quiet",
        "sample-dir",
    ])
    .expect("cli parse should succeed");
    assert_eq!(
        progress_mode_from_cli(&quiet_cli),
        crate::progress::ProgressMode::Quiet
    );

    let cli = crate::cli::Cli::try_parse_from([
        "scancode-rust",
        "--json-pp",
        "scan.json",
        "--verbose",
        "sample-dir",
    ])
    .expect("cli parse should succeed");

    assert_eq!(
        progress_mode_from_cli(&cli),
        crate::progress::ProgressMode::Verbose
    );
}

#[test]
fn prepare_cache_for_scan_defaults_to_scan_root_cache_directory() {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let scan_root = temp_dir.path().join("scan");
    fs::create_dir_all(&scan_root).expect("create scan root");

    let cli =
        crate::cli::Cli::try_parse_from(["scancode-rust", "--json-pp", "scan.json", "sample-dir"])
            .expect("cli parse should succeed");

    let config = prepare_cache_for_scan(scan_root.to_str().expect("utf-8 path"), &cli)
        .expect("cache preparation should succeed");

    assert_eq!(config.root_dir(), scan_root.join(".scancode-cache"));
    assert!(config.index_dir().exists());
    assert!(config.scan_results_dir().exists());
}

#[test]
fn prepare_cache_for_scan_respects_cache_dir_and_cache_clear() {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let scan_root = temp_dir.path().join("scan");
    fs::create_dir_all(&scan_root).expect("create scan root");

    let explicit_cache_dir = temp_dir.path().join("explicit-cache");
    fs::create_dir_all(explicit_cache_dir.join("index")).expect("create stale cache dir");
    let stale_file = explicit_cache_dir.join("index").join("stale.txt");
    fs::write(&stale_file, "old").expect("write stale file");

    let cli = crate::cli::Cli::try_parse_from([
        "scancode-rust",
        "--json-pp",
        "scan.json",
        "--cache-dir",
        explicit_cache_dir.to_str().expect("utf-8 path"),
        "--cache-clear",
        "sample-dir",
    ])
    .expect("cli parse should succeed");

    let config = prepare_cache_for_scan(scan_root.to_str().expect("utf-8 path"), &cli)
        .expect("cache preparation should succeed");

    assert_eq!(config.root_dir(), explicit_cache_dir);
    assert!(!stale_file.exists());
    assert!(config.index_dir().exists());
    assert!(config.scan_results_dir().exists());
}
