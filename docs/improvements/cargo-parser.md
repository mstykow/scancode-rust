# Cargo Parser Improvements

## Summary

Rust now goes beyond the released Python ScanCode Cargo handling in several concrete ways:

1. assigns ordinary crate files to their Cargo package instead of limiting ownership to `Cargo.toml` and `Cargo.lock`
2. proves workspace member file ownership for non-manifest files such as `LICENSE` and `README.md`
3. closes parser parity gaps around lowercase Cargo filenames and missing manifest fields like `readme` and `publish`

## Python Status

- Upstream Cargo support exists, but the open issue set still tracks crate-wide file assignment, workspace member file assignment, and broader Cargo manifest/lockfile completeness.
- Upstream issues explicitly document missing `for_packages` ownership for both plain Cargo crates and workspace member files.
- Python reference tests already accept lowercase `cargo.toml` / `cargo.lock`, but the current Rust direct parser matching did not.

## Rust Improvements

### Cargo package file ownership

- Plain Cargo crates now assign files under the crate root to the crate package, not only the manifest and lockfile.
- The assignment logic still skips `target/` and avoids stealing files from nested Cargo package roots.

### Workspace member ownership

- Cargo workspace member files like `crates/cli/LICENSE` and `crates/core/README.md` are now explicitly covered in assembly regression fixtures.
- This proves the member package assignment logic works for real non-manifest member files, not only `Cargo.toml`.

### Parser parity fixes

- `CargoParser::is_match()` now accepts lowercase `cargo.toml`.
- `CargoLockParser::is_match()` now accepts lowercase `cargo.lock`.
- Cargo.toml parsing now preserves:
  - `readme` as `extra_data.readme_file`
  - `publish` as `extra_data.publish`
- Cargo workspace readme inheritance now resolves to `readme_file` when workspace metadata provides it.
- Error-path fallback package data now keeps the correct Cargo `package_type` and `datasource_id` for both manifest and lockfile parsing.

## Validation

- `cargo test cargo --lib`
- `cargo test --features golden-tests cargo_golden --lib`
- `cargo test --features golden-tests test_assembly_cargo_basic --lib`
- `cargo test --features golden-tests test_assembly_cargo_workspace --lib`

## Related Issues

- #184, #189, #217
