use super::*;
use clap::Parser;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::DEFAULT_CACHE_DIR_NAME;
use crate::scan_result_shaping::{
    JsonScanInput, apply_user_path_filters_to_collected, is_included_path, load_scan_from_json,
    normalize_loaded_json_scan, normalize_scan_relative_path, resolve_native_scan_inputs,
};
use crate::scanner::collect_paths;

#[test]
fn load_scan_from_json_reads_files_and_metadata_sections() {
    let temp_path = std::env::temp_dir().join("provenant-from-json-test.json");
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
fn resolve_thread_count_supports_reference_compat_values() {
    assert_eq!(resolve_thread_count(-1), 1);
    assert_eq!(resolve_thread_count(0), default_parallel_threads());
    assert_eq!(resolve_thread_count(4), 4);
}

#[test]
fn validate_scan_option_compatibility_rejects_scan_flags_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--copyright",
        "sample-scan.json",
    ])
    .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_err());
}

#[test]
fn validate_scan_option_compatibility_rejects_package_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--package",
        "sample-scan.json",
    ])
    .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_err());
}

#[test]
fn validate_scan_option_compatibility_rejects_generated_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--generated",
        "sample-scan.json",
    ])
    .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_err());
}

#[test]
fn validate_scan_option_compatibility_allows_strip_root_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--strip-root",
        "sample-scan.json",
    ])
    .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_ok());
}

#[test]
fn validate_scan_option_compatibility_allows_full_root_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--full-root",
        "sample-scan.json",
    ])
    .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_ok());
}

#[test]
fn validate_scan_option_compatibility_allows_scan_flags_without_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--copyright",
        "sample-dir",
    ])
    .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_ok());
}

#[test]
fn validate_scan_option_compatibility_allows_multiple_inputs_with_from_json() {
    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "scan-a.json",
        "scan-b.json",
    ])
    .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_ok());
}

#[test]
fn compile_regex_patterns_rejects_invalid_regex() {
    let result = compile_regex_patterns("--ignore-author", &["[".to_string()]);

    assert!(result.is_err());
    let error = result.err().unwrap().to_string();
    assert!(error.contains("--ignore-author"));
    assert!(error.contains("Invalid regex"));
}

#[test]
fn is_included_path_requires_include_match_before_excludes() {
    assert!(is_included_path(
        "user/src/test/sample.doc",
        &["*.doc".to_string()],
        &[]
    ));
    assert!(!is_included_path(
        "user/src/test/sample.txt",
        &["*.doc".to_string()],
        &[]
    ));
}

#[test]
fn is_included_path_applies_exclude_after_include() {
    assert!(!is_included_path(
        "src/dist/build/mylib.so",
        &["/src/*".to_string()],
        &["/src/*.so".to_string()]
    ));
    assert!(is_included_path(
        "some/src/this/that",
        &["src".to_string()],
        &["src/*.so".to_string()]
    ));
}

#[test]
fn apply_user_path_filters_to_collected_filters_files_without_pruning_directories() {
    let scan_root = PathBuf::from("/scan");
    let placeholder_metadata = fs::metadata(std::env::temp_dir()).expect("temp dir metadata");
    let mut collected = crate::scanner::CollectedPaths {
        files: vec![
            (
                scan_root.join("src/test/sample.doc"),
                placeholder_metadata.clone(),
            ),
            (
                scan_root.join("src/test/sample.txt"),
                placeholder_metadata.clone(),
            ),
        ],
        directories: vec![
            (scan_root.clone(), placeholder_metadata.clone()),
            (scan_root.join("src"), placeholder_metadata.clone()),
            (scan_root.join("src/test"), placeholder_metadata.clone()),
            (scan_root.join("other"), placeholder_metadata.clone()),
        ],
        excluded_count: 0,
        total_file_bytes: 0,
        collection_errors: Vec::new(),
    };

    let removed = apply_user_path_filters_to_collected(
        &mut collected,
        &scan_root,
        &["*.doc".to_string()],
        &[],
    );

    assert_eq!(removed, 2);
    assert_eq!(collected.files.len(), 1);
    let kept_dirs: Vec<_> = collected
        .directories
        .iter()
        .map(|(path, _)| normalize_scan_relative_path(path, &scan_root))
        .collect();
    assert_eq!(
        kept_dirs,
        vec!["".to_string(), "src".to_string(), "src/test".to_string()]
    );
    assert_eq!(
        normalize_scan_relative_path(&collected.files[0].0, &scan_root),
        "src/test/sample.doc"
    );
}

#[test]
fn is_included_path_treats_directory_include_patterns_recursively() {
    assert!(is_included_path(
        "src/foo/bar/baz.txt",
        &["src/foo".to_string()],
        &[]
    ));
    assert!(!is_included_path(
        "src/other/bar.txt",
        &["src/foo".to_string()],
        &[]
    ));
}

#[test]
fn from_json_with_no_assemble_preserves_preloaded_package_sections() {
    let temp_path = std::env::temp_dir().join("provenant-from-json-with-packages-test.json");
    let content = json!({
        "files": [],
        "packages": [
            {
                "package_uid": "pkg:npm/demo@1.0.0",
                "type": "npm",
                "name": "demo",
                "version": "1.0.0",
                "parties": [],
                "datafile_paths": ["package.json"],
                "datasource_ids": ["npm_package_json"]
            }
        ],
        "dependencies": [
            {
                "purl": "pkg:npm/dep@2.0.0",
                "scope": "dependencies",
                "is_runtime": true,
                "is_optional": false,
                "is_pinned": true,
                "dependency_uid": "pkg:npm/dep@2.0.0?uuid=test",
                "for_package_uid": "pkg:npm/demo@1.0.0",
                "datafile_path": "package.json",
                "datasource_id": "npm_package_json"
            }
        ],
        "license_references": [],
        "license_rule_references": []
    });
    fs::write(&temp_path, content.to_string()).expect("write json fixture");

    let parsed = load_scan_from_json(temp_path.to_str().expect("utf-8 path"))
        .expect("from-json loading should succeed");

    let preloaded = assembly::AssemblyResult {
        packages: parsed.packages,
        dependencies: parsed.dependencies,
    };

    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--no-assemble",
        temp_path.to_str().expect("utf-8 path"),
    ])
    .expect("cli parse should succeed");

    let assembly_result = if cli.from_json
        && (!preloaded.packages.is_empty() || !preloaded.dependencies.is_empty())
    {
        preloaded
    } else if cli.no_assemble {
        assembly::AssemblyResult {
            packages: Vec::new(),
            dependencies: Vec::new(),
        }
    } else {
        unreachable!("test only covers from-json preload precedence")
    };

    assert_eq!(assembly_result.packages.len(), 1);
    assert_eq!(assembly_result.dependencies.len(), 1);

    let _ = fs::remove_file(temp_path);
}

#[test]
fn validate_scan_option_compatibility_allows_multiple_paths_without_from_json() {
    let cli =
        crate::cli::Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "dir-a", "dir-b"])
            .unwrap();
    assert!(validate_scan_option_compatibility(&cli).is_ok());
}

#[test]
fn resolve_native_scan_inputs_builds_common_prefix_and_synthetic_includes() {
    let (scan_root, includes) =
        resolve_native_scan_inputs(&["src/foo".to_string(), "src/bar/baz".to_string()])
            .expect("multiple relative inputs should resolve");

    assert_eq!(scan_root, "src");
    assert_eq!(includes, vec!["src/foo", "src/bar/baz"]);
}

#[test]
fn resolve_native_scan_inputs_uses_component_aware_prefix_for_siblings() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let parent = temp_dir.path().join("src");
    fs::create_dir_all(parent.join("bar")).expect("create bar dir");
    fs::create_dir_all(parent.join("baz")).expect("create baz dir");

    let old_cwd = std::env::current_dir().expect("current dir");
    std::env::set_current_dir(temp_dir.path()).expect("set cwd");

    let result = resolve_native_scan_inputs(&["src/bar".to_string(), "src/baz".to_string()]);

    std::env::set_current_dir(old_cwd).expect("restore cwd");

    let (scan_root, includes) = result.expect("sibling inputs should resolve");
    assert_eq!(scan_root, "src");
    assert_eq!(includes, vec!["src/bar", "src/baz"]);
}

#[test]
fn normalize_loaded_json_scan_applies_strip_root_per_loaded_input() {
    let mut loaded = JsonScanInput {
        files: vec![
            json_file("archive/root", crate::models::FileType::Directory),
            json_file("archive/root/src/main.rs", crate::models::FileType::File),
        ],
        packages: vec![],
        dependencies: vec![],
        license_references: vec![],
        license_rule_references: vec![],
        excluded_count: 0,
    };

    normalize_loaded_json_scan(&mut loaded, true, false);

    let paths: Vec<_> = loaded.files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, vec!["root", "src/main.rs"]);
}

#[test]
fn normalize_loaded_json_scan_trims_full_root_display_without_absolutizing() {
    let mut loaded = JsonScanInput {
        files: vec![json_file(
            "/tmp/archive/root/src/main.rs",
            crate::models::FileType::File,
        )],
        packages: vec![],
        dependencies: vec![],
        license_references: vec![],
        license_rule_references: vec![],
        excluded_count: 0,
    };

    normalize_loaded_json_scan(&mut loaded, false, true);

    assert_eq!(loaded.files[0].path, "tmp/archive/root/src/main.rs");
}

#[test]
fn from_json_skips_final_native_projection_block() {
    let mut loaded = JsonScanInput {
        files: vec![json_file(
            "/tmp/archive/root/src/main.rs",
            crate::models::FileType::File,
        )],
        packages: vec![],
        dependencies: vec![],
        license_references: vec![],
        license_rule_references: vec![],
        excluded_count: 0,
    };

    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--from-json",
        "--full-root",
        "sample-scan.json",
    ])
    .expect("cli parse should succeed");

    normalize_loaded_json_scan(&mut loaded, false, true);

    if !cli.from_json && (cli.strip_root || cli.full_root) {
        normalize_paths(
            &mut loaded.files,
            cli.dir_path.first().expect("input path exists"),
            cli.strip_root,
            cli.full_root,
        );
    }

    assert_eq!(loaded.files[0].path, "tmp/archive/root/src/main.rs");
}

fn json_file(path: &str, file_type: crate::models::FileType) -> crate::models::FileInfo {
    crate::models::FileInfo::new(
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .extension()
            .and_then(|name| name.to_str())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default(),
        path.to_string(),
        file_type,
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
        Vec::new(),
    )
}

#[test]
fn progress_mode_from_cli_maps_quiet_verbose_default() {
    let default_cli =
        crate::cli::Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "sample-dir"])
            .unwrap();
    assert_eq!(
        progress_mode_from_cli(&default_cli),
        crate::progress::ProgressMode::Default
    );

    let quiet_cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--quiet",
        "sample-dir",
    ])
    .unwrap();
    assert_eq!(
        progress_mode_from_cli(&quiet_cli),
        crate::progress::ProgressMode::Quiet
    );

    let verbose_cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--verbose",
        "sample-dir",
    ])
    .unwrap();
    assert_eq!(
        progress_mode_from_cli(&verbose_cli),
        crate::progress::ProgressMode::Verbose
    );
}

#[test]
fn prepare_cache_for_scan_defaults_to_scan_root_cache_directory() {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let scan_root = temp_dir.path().join("scan");
    fs::create_dir_all(&scan_root).expect("create scan root");

    let cli =
        crate::cli::Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "sample-dir"])
            .unwrap();
    let config = prepare_cache_for_scan(scan_root.to_str().unwrap(), &cli).unwrap();

    assert_eq!(config.root_dir(), scan_root.join(DEFAULT_CACHE_DIR_NAME));
    assert!(config.index_dir().exists());
    assert!(config.scan_results_dir().exists());
}

#[test]
fn prepare_cache_for_scan_respects_cache_dir_and_cache_clear() {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let scan_root = temp_dir.path().join("scan");
    fs::create_dir_all(&scan_root).expect("create scan root");

    let explicit_cache_dir = temp_dir.path().join("explicit-cache");
    fs::create_dir_all(explicit_cache_dir.join("index")).unwrap();
    let stale_file = explicit_cache_dir.join("index").join("stale.txt");
    fs::write(&stale_file, "old").unwrap();

    let cli = crate::cli::Cli::try_parse_from([
        "provenant",
        "--json-pp",
        "scan.json",
        "--cache-dir",
        explicit_cache_dir.to_str().unwrap(),
        "--cache-clear",
        "sample-dir",
    ])
    .unwrap();
    let config = prepare_cache_for_scan(scan_root.to_str().unwrap(), &cli).unwrap();

    assert_eq!(config.root_dir(), explicit_cache_dir);
    assert!(!stale_file.exists());
    assert!(config.index_dir().exists());
    assert!(config.scan_results_dir().exists());
}

#[test]
fn build_collection_exclude_patterns_skips_default_cache_dir() {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let scan_root = temp_dir.path().join("scan");
    fs::create_dir_all(scan_root.join("src")).unwrap();
    fs::create_dir_all(scan_root.join(DEFAULT_CACHE_DIR_NAME).join("index")).unwrap();
    fs::write(scan_root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(
        scan_root
            .join(DEFAULT_CACHE_DIR_NAME)
            .join("index")
            .join("stale.txt"),
        "cached",
    )
    .unwrap();

    let config = crate::cache::CacheConfig::from_scan_root(&scan_root);
    let exclude_patterns = build_collection_exclude_patterns(&scan_root, &config);
    let collected = collect_paths(&scan_root, 0, &exclude_patterns);

    assert!(
        collected
            .files
            .iter()
            .all(|(path, _)| !path.starts_with(config.root_dir()))
    );
    assert!(collected.excluded_count >= 1);
}

#[test]
fn build_collection_exclude_patterns_skips_explicit_in_tree_cache_dir() {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let scan_root = temp_dir.path().join("scan");
    let explicit_cache_dir = scan_root.join("custom-cache");
    fs::create_dir_all(scan_root.join("docs")).unwrap();
    fs::create_dir_all(explicit_cache_dir.join("scan-results")).unwrap();
    fs::write(scan_root.join("docs").join("README.md"), "hello").unwrap();
    fs::write(
        explicit_cache_dir
            .join("scan-results")
            .join("entry.msgpack.zst"),
        "cached",
    )
    .unwrap();

    let config = crate::cache::CacheConfig::new(explicit_cache_dir.clone());
    let exclude_patterns = build_collection_exclude_patterns(&scan_root, &config);
    let collected = collect_paths(&scan_root, 0, &exclude_patterns);

    assert!(
        collected
            .files
            .iter()
            .all(|(path, _)| !path.starts_with(&explicit_cache_dir))
    );
    assert!(collected.excluded_count >= 1);
}

#[test]
fn build_collection_exclude_patterns_does_not_exclude_scan_root_when_cache_root_matches_it() {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let scan_root = temp_dir.path().join("scan");
    fs::create_dir_all(scan_root.join("src")).unwrap();
    fs::write(scan_root.join("src").join("main.rs"), "fn main() {}").unwrap();

    let config = crate::cache::CacheConfig::new(scan_root.clone());
    let exclude_patterns = build_collection_exclude_patterns(&scan_root, &config);
    let collected = collect_paths(&scan_root, 0, &exclude_patterns);

    assert_eq!(collected.file_count(), 1);
    assert_eq!(collected.excluded_count, 0);
}
