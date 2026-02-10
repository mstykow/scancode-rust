use glob::Pattern;
use indicatif::ProgressBar;
use scancode_rust::askalono::{ScanStrategy, Store};
use scancode_rust::parsers::list_parser_types;
use scancode_rust::{FileType, process};
use std::sync::Arc;

/// Helper to create a minimal Store for testing
/// This creates an empty store - tests don't need actual license data
fn create_test_store() -> Store {
    Store::new()
}

/// Helper to create a ScanStrategy for testing
fn create_test_strategy(store: &Store) -> ScanStrategy<'_> {
    ScanStrategy::new(store)
        .optimize(false)
        .confidence_threshold(0.9)
}

#[test]
fn test_scanner_discovers_all_registered_parsers() {
    let test_dir = "testdata/integration/multi-parser";
    let progress = Arc::new(ProgressBar::hidden());
    let patterns: Vec<Pattern> = vec![];
    let store = create_test_store();
    let strategy = create_test_strategy(&store);

    let result =
        process(test_dir, 50, progress, &patterns, &strategy).expect("Scan should succeed");

    // Should find 3 files with package data (npm, python, cargo)
    let package_files: Vec<_> = result
        .files
        .iter()
        .filter(|f| f.file_type == FileType::File && !f.package_data.is_empty())
        .collect();

    assert_eq!(
        package_files.len(),
        3,
        "Should find all 3 package manifests, found: {:?}",
        package_files.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Verify each parser was invoked
    let has_npm = package_files
        .iter()
        .any(|f| f.package_data[0].package_type == Some("npm".to_string()));
    let has_pypi = package_files
        .iter()
        .any(|f| f.package_data[0].package_type == Some("pypi".to_string()));
    let has_cargo = package_files
        .iter()
        .any(|f| f.package_data[0].package_type == Some("cargo".to_string()));

    assert!(has_npm, "NpmParser should be invoked");
    assert!(has_pypi, "PythonParser should be invoked");
    assert!(has_cargo, "CargoParser should be invoked");
}

#[test]
fn test_full_output_format_structure() {
    let test_dir = "testdata/integration/multi-parser";
    let progress = Arc::new(ProgressBar::hidden());
    let patterns: Vec<Pattern> = vec![];
    let store = create_test_store();
    let strategy = create_test_strategy(&store);

    let result =
        process(test_dir, 50, progress, &patterns, &strategy).expect("Scan should succeed");

    // Verify basic structure
    assert!(!result.files.is_empty(), "Should have files in result");

    // Verify each file has required fields
    for file in &result.files {
        if file.file_type == FileType::File {
            assert!(!file.name.is_empty(), "File should have name");
            assert!(!file.path.is_empty(), "File should have path");
            assert!(file.sha1.is_some(), "File should have SHA1 hash");
            assert!(file.md5.is_some(), "File should have MD5 hash");
            assert!(file.sha256.is_some(), "File should have SHA256 hash");
            assert!(
                file.mime_type.is_some(),
                "File should have mime type for: {}",
                file.name
            );
            assert!(file.size > 0, "File should have size for: {}", file.name);
        }
    }

    // Verify package files have package_data
    let package_files: Vec<_> = result
        .files
        .iter()
        .filter(|f| {
            matches!(
                f.name.as_str(),
                "package.json" | "pyproject.toml" | "Cargo.toml"
            )
        })
        .collect();

    assert_eq!(package_files.len(), 3, "Should find all 3 manifest files");

    for file in package_files {
        assert!(
            !file.package_data.is_empty(),
            "Manifest file {} should have package_data",
            file.name
        );
        let pkg = &file.package_data[0];
        assert!(pkg.package_type.is_some(), "Should have package type");
        assert!(pkg.name.is_some(), "Should have package name");
        assert!(pkg.version.is_some(), "Should have package version");
    }
}

#[test]
fn test_scanner_handles_empty_directory() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_path = temp_dir.path();

    let progress = Arc::new(ProgressBar::hidden());
    let patterns: Vec<Pattern> = vec![];
    let store = create_test_store();
    let strategy = create_test_strategy(&store);

    let result = process(test_path, 50, progress, &patterns, &strategy)
        .expect("Scan should succeed on empty directory");

    // Should have no files (only the directory entry might be present)
    let file_count = result
        .files
        .iter()
        .filter(|f| f.file_type == FileType::File)
        .count();
    assert_eq!(file_count, 0, "Empty directory should have no files");
}

#[test]
fn test_scanner_handles_parse_errors_gracefully() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_path = temp_dir.path();

    let malformed_json = test_path.join("package.json");
    fs::write(&malformed_json, "{ this is not valid json }").expect("Failed to write test file");

    let progress = Arc::new(ProgressBar::hidden());
    let patterns: Vec<Pattern> = vec![];
    let store = create_test_store();
    let strategy = create_test_strategy(&store);

    // Scan should complete without crashing
    let result = process(test_path, 50, progress, &patterns, &strategy)
        .expect("Scan should not crash on malformed files");

    // Should find the file
    let json_file = result
        .files
        .iter()
        .find(|f| f.name == "package.json")
        .expect("Should find package.json file");

    assert!(
        json_file.package_data.is_empty()
            || json_file.package_data[0].name.is_none()
            || !json_file.scan_errors.is_empty(),
        "Malformed file should have empty/invalid package data or scan errors"
    );
}

#[test]
fn test_exclusion_patterns_filter_correctly() {
    let test_dir = "testdata/integration/multi-parser";
    let progress = Arc::new(ProgressBar::hidden());

    let patterns: Vec<Pattern> = vec![Pattern::new("*.toml").expect("Invalid pattern")];
    let store = create_test_store();
    let strategy = create_test_strategy(&store);

    let result =
        process(test_dir, 50, progress, &patterns, &strategy).expect("Scan should succeed");

    // Should not find any .toml files
    let toml_files: Vec<_> = result
        .files
        .iter()
        .filter(|f| f.name.ends_with(".toml"))
        .collect();

    assert!(
        toml_files.is_empty(),
        "Should not find .toml files, but found: {:?}",
        toml_files.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Should still find .json file
    let has_json = result.files.iter().any(|f| f.name == "package.json");
    assert!(has_json, "Should still find package.json");

    // Check excluded count
    assert!(
        result.excluded_count > 0,
        "Should have excluded at least one file"
    );
}

#[test]
fn test_max_depth_limits_traversal() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_path = temp_dir.path();

    let level1 = test_path.join("level1");
    let level2 = level1.join("level2");
    fs::create_dir_all(&level2).expect("Failed to create nested dirs");

    let deep_file = level2.join("package.json");
    fs::write(&deep_file, r#"{"name": "deep", "version": "1.0.0"}"#)
        .expect("Failed to write test file");

    let progress = Arc::new(ProgressBar::hidden());
    let patterns: Vec<Pattern> = vec![];
    let store = create_test_store();
    let strategy = create_test_strategy(&store);

    // Scan with max_depth=1 (should not reach level2)
    let result =
        process(test_path, 1, progress, &patterns, &strategy).expect("Scan should succeed");

    // Should not find the deep package.json
    let has_deep_json = result.files.iter().any(|f| f.name == "package.json");
    assert!(!has_deep_json, "Should not find package.json at depth > 1");
}

/// Regression test: Verify that all parsers in register_package_handlers! macro are actually
/// exported and accessible. This catches bugs where parsers are implemented but
/// not registered in the macro (like CargoLockParser was before being fixed).
#[test]
fn test_all_parsers_are_registered_and_exported() {
    // Get list of all parser types from the macro
    let parser_types = list_parser_types();

    // This test verifies that list_parser_types() returns a non-empty list
    // If a parser is implemented but not in register_package_handlers!, it won't appear here
    assert!(
        !parser_types.is_empty(),
        "Should have at least one parser registered"
    );

    // Known parsers that should be present (sample check)
    let expected_parsers = vec![
        "NpmParser",
        "NpmLockParser",
        "CargoParser",
        "CargoLockParser", // This was missing before the fix
        "PythonParser",
        "ComposerLockParser",
        "YarnLockParser",
        "PnpmLockParser",
        "PoetryLockParser",
    ];

    for expected in expected_parsers {
        assert!(
            parser_types.contains(&expected),
            "Parser '{}' should be registered in register_package_handlers! macro",
            expected
        );
    }

    // Verify we have a reasonable number of parsers (40+ formats supported)
    // If this number is suspiciously low, it indicates missing registrations
    assert!(
        parser_types.len() >= 40,
        "Expected at least 40 parsers, found {}. Some parsers may not be registered.",
        parser_types.len()
    );
}
