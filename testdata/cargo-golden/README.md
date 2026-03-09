# Cargo Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Test Status

**Currently Passing:** 8/8 tests

- ✅ `test_golden_clap`
- ✅ `test_golden_package`
- ✅ `test_golden_rustup`
- ✅ `test_golden_scan`
- ✅ `test_golden_single_file_scan`
- ✅ `test_golden_tauri`
- ✅ `test_golden_publish_false`
- ✅ `test_golden_cargo_lock_basic`

## Test Coverage

### Passing Tests

1. **clap** - Cargo.toml with authors, docs URL, readme, and mixed dependency styles
2. **package** - package metadata fixture covering readme/documentation extraction
3. **rustup** - Cargo.toml with docs URL and readme extraction
4. **scan** - real-world Cargo.toml with repository URL and readme extraction
5. **single-file-scan** - Cargo.toml parser-only coverage for docs/readme fields
6. **tauri** - workspace-marker manifest coverage with direct readme plus inherited fields
7. **publish-false** - Cargo.toml with `publish = false` and `readme`
8. **lock-basic** - Cargo.lock parser golden for root package, checksum, and pinned deps

## Test Data

Test files in this directory validate Cargo.toml parser output format and field extraction.
