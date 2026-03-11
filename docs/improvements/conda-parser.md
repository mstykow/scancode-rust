# Conda Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode Conda handling in two concrete ways:

1. assembles `conda-meta/*.json` installed-package metadata together with sibling recipe `meta.yaml` data so installed files are assigned back to the Conda package
2. resolves channel-prefix ambiguity by keeping symbolic channel names as PURL namespace while preserving URL-like channel prefixes separately as `channel_url`

## Python Status

- Current Python ScanCode already has dedicated `conda-meta/*.json` handling and rootfs assembly logic to assign installed files from `files`/`extracted_package_dir`.
- Upstream still tracks two Conda gaps relevant here:
  - installed file assignment from `conda-meta/*.json`
  - ambiguity between symbolic channel namespace and URL-like channel prefixes

## Rust Improvements

### Installed file assignment from `conda-meta`

- Rust now assembles `conda-meta/*.json` with matching `pkgs/.../info/recipe/meta.yaml` recipe data using shared package identity.
- `conda-meta` parsing now emits `file_references` for:
  - installed file paths from `files[]`
  - extracted package directory under `pkgs/...`
  - package tarball path when present
- The generic file-reference resolver then assigns those installed files to the assembled Conda package.

### Channel namespace vs repository URL disambiguation

- Symbolic channel prefixes like `conda-forge::numpy` continue to become Conda PURL namespace.
- URL-like channel prefixes such as `https://...::flask=1.0.2` are no longer treated as namespace.
- Instead, URL-like prefixes are preserved in dependency `extra_data.channel_url`, while symbolic prefixes are preserved in `extra_data.channel`.

## Validation

- `cargo test conda --lib`
- `cargo test --features golden-tests conda_golden --lib`
- `cargo test test_assembly_conda_rootfs_assigns_meta_json_files --lib`

## Related Issues

- #195, #196
