# ABOUT Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode ABOUT handling in four concrete ways:

1. derives real ecosystem PURLs from `download_url` metadata when an ABOUT file lacks an explicit PURL
2. avoids emitting invalid `pkg:about/...` PURLs when the file only describes another ecosystem package
3. handles partial or malformed ABOUT files gracefully while preserving ABOUT parser identity on fallback paths
4. resolves `about_resource`, `license_file`, and `notice_file` references during scan-time package promotion without depending on reopening ABOUT files

## Python Status

- Current Python ABOUT handling still defaults to package type `about` and emits `pkg:about/...` package URLs when no explicit package URL is present.
- Upstream explicitly tracks four ABOUT gaps:
  - deriving fully qualified PURLs from `download_url`
  - avoiding invalid `pkg:about`
  - graceful handling of partial ABOUT files
  - `get_package_root()` / file-access coupling in ABOUT package promotion

## Rust Improvements

### PURL/type derivation from `download_url`

- ABOUT files without an explicit PURL now infer package type and PURL from recognized download hosts.
- Current fixture-backed inference includes:
  - PyPI wheel URLs → `pkg:pypi/...`
  - GitHub raw/source URLs → `pkg:github/...`

### No invalid `pkg:about`

- `PackageType::About` is now treated as parser metadata only.
- Rust will not synthesize a `pkg:about/...` PURL just because the file itself is an ABOUT file.
- If Rust cannot infer or parse a real package URL, the ABOUT record remains an ABOUT package with no PURL instead of emitting an invalid PURL type.

### Graceful partial ABOUT handling

- Invalid YAML and non-mapping ABOUT roots now fall back to a default `PackageData` that still preserves:
  - `package_type = about`
  - `datasource_id = about_file`
- Partial ABOUT files with only `download_url` no longer lose parser identity and can still infer a real ecosystem PURL when the URL is recognizable.

### Scan-time package promotion and file-reference resolution

- ABOUT files are now promoted into top-level packages during assembly when they have a real PURL.
- File references from:
  - `about_resource`
  - `license_file`
  - `notice_file`
    are resolved relative to the ABOUT file directory.
- Missing references are recorded in `extra_data.missing_file_references` rather than causing crashes or silent loss.
- This also resolves the underlying ABOUT `get_package_root()` problem in a Rust-native way: resolution is path-based and does not depend on reopening described resources.

## Coverage

Coverage spans ABOUT package promotion, path-based referenced-file resolution, and preservation of missing referenced files in structured metadata.
