# RPM Parser Improvements

## Summary

Rust now goes beyond the released Python ScanCode RPM handling in several concrete ways:

1. keeps full EVR identity in installed RPM database parsing instead of dropping the `Release` portion
2. recognizes hash-named source RPM files by RPM magic bytes, not only by filename extension
3. preserves richer archive metadata such as packager parties, keywords, build metadata, source URLs, and VCS hints
4. collects YumDB sidecar metadata from installed RPM rootfs scans and merges it back onto the matching installed package

## Python Status

- Python RPM archive handling still leaves multiple metadata/dependency TODOs in `rpm.py`.
- Python installed RPM parsing in `rpm_installed.py` still contains the `FIXME` about EVRA handling and emits truncated versions in container/rootfs scans.
- Python does not detect extensionless hash-named source RPM files until dispatch happens elsewhere.
- Python tracks YumDB as a desired enhancement, but released behavior does not merge that metadata into installed RPM packages.

## Rust Improvements

### Archive metadata and identity

- Archive parsing now keeps richer metadata from RPM headers:
  - packager party data
  - group as keywords
  - build host/time in `extra_data`
  - source URLs in `extra_data`
  - VCS URLs from `Vcs` tags or git-like source URLs
- Source RPMs now add a `source=true` qualifier in both `qualifiers` and `purl`.
- Hash-named source RPM files are now recognized by RPM magic bytes instead of extension-only matching.

### Installed database behavior

- Installed RPM versions keep the full `version-release` string.
- RPM namespace propagation from nearby `os-release` now rewrites package and dependency PURLs/UIDs, not just a separate `namespace` field.
- Invalid or incomplete file-reference tuples are skipped safely during installed DB file-reference construction.

### YumDB enrichment

- Rust adds a dedicated YumDB parser for canonical `from_repo` entries under `var/lib/yum/yumdb/`.
- The parser reads sibling YumDB keys from the same package directory.
- Post-assembly merge logic attaches these YumDB keys under `extra_data.yumdb` on the matching installed RPM package and removes the standalone YumDB fragment.

## Validation

- `cargo test rpm --lib`
- `cargo test --features golden-tests rpm_golden --lib`
- `cargo test test_resolve_rpm_namespace --lib`
- `cargo test test_merge_rpm_yumdb_metadata --lib`

## Related Issues

- #164, #166, #167, #168, #169, #170, #171
