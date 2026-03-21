# Composer Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode Composer handling in several concrete ways:

1. supports alternate Composer manifest/lockfile names such as `symfony.composer.json`, `composer.symfony.json`, and analogous lockfile names
2. assigns ordinary files under Composer package roots to the correct package, including nested Composer packages
3. keeps lockfile extraction lightweight by emitting one synthetic lock package plus dependency objects, while avoiding unnecessary array cloning on large `composer.lock` files
4. enriches manifest-level provenance and party typing beyond the earlier lock-only focus
5. normalizes trustworthy `composer.json` license metadata into declared license fields while preserving the raw manifest statement

## Python Status

- Python Composer support is split between `composer.json` and `composer.lock` handlers and only advertises glob-style `*composer.json` / `*composer.lock` recognition.
- Upstream explicitly tracks:
  - slow `composer.lock` scanning on very large lockfiles
  - nested Composer package/file assignment gaps
  - missing support for alternate Composer file names
- Python also maps manifest `source` and `dist` metadata but Rust previously left those fields empty for `composer.json`.
- Upstream Composer handling also treats manifest `license` as expression-oriented package metadata and promotes recognized SPDX-style values into declared license fields.

## Rust Improvements

### Alternate file names

- Rust now recognizes:
  - `composer.json`
  - `symfony.composer.json`
  - `php.composer.json`
  - `composer.symfony.json`
  - and the analogous `*.lock` variants
- Assembly config now uses matching glob patterns so alternate-name manifests participate in sibling merge instead of only standalone parsing.

### Nested Composer package assignment

- Composer package-root resource assignment now associates ordinary files under a Composer package root with that package.
- Nested Composer packages no longer lose file ownership to a parent package just because they live under the parent directory tree.
- The new nested assembly fixture proves:
  - root package files remain on the root package
  - nested package files like `packages/plugin/README.md` stay with the nested package

### Large lockfile handling

- Rust keeps Composer lock parsing lightweight by:
  - emitting one synthetic lock package plus dependency edges instead of materializing every locked package as a top-level package
  - iterating lockfile package arrays by reference instead of cloning them first
  - pre-reserving dependency capacity for the combined `packages` + `packages-dev` lists
- This does not replicate Python’s slow-path object expansion, which is the right behavior because the upstream issue is a performance bug, not a semantic contract.

### Manifest provenance and party typing

- Composer manifest parsing now extracts:
  - `vcs_url` from manifest `source`
  - `download_url` from manifest `dist`
  - `Party.type = person` for authors and vendor parties

### Declared license normalization

- Rust now keeps a parser-level declared-license statement in `extracted_license_statement` and promotes parseable SPDX-style Composer metadata into:
  - `declared_license_expression`
  - `declared_license_expression_spdx`
  - `license_detections`
- For string inputs, the extracted statement matches the manifest value directly. For Composer array inputs, the parser preserves the declared choice semantics by joining entries into a deterministic `OR` statement string.
- This covers:
  - single SPDX identifiers such as `MIT`
  - SPDX expression strings such as `MIT OR Apache-2.0`
  - Composer array form, normalized as disjunctive `OR`
  - the documented Composer `proprietary` special case
- For private Composer manifests that omit both `name` and `license`, Rust now matches upstream ScanCode’s proprietary fallback behavior.
- Non-SPDX custom strings remain preserved as raw extracted manifest text instead of being over-normalized into misleading declared values.

## Coverage

Coverage spans alternate Composer filenames, nested file assignment, lockfile handling, richer manifest provenance behavior, SPDX license normalization, and a parser golden for Composer manifest declared-license output.
