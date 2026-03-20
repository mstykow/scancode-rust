# Composer Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode Composer handling in several concrete ways:

1. supports alternate Composer manifest/lockfile names such as `symfony.composer.json`, `composer.symfony.json`, and analogous lockfile names
2. assigns ordinary files under Composer package roots to the correct package, including nested Composer packages
3. keeps lockfile extraction lightweight by emitting one synthetic lock package plus dependency objects, while avoiding unnecessary array cloning on large `composer.lock` files
4. enriches manifest-level provenance and party typing beyond the earlier lock-only focus

## Python Status

- Python Composer support is split between `composer.json` and `composer.lock` handlers and only advertises glob-style `*composer.json` / `*composer.lock` recognition.
- Upstream explicitly tracks:
  - slow `composer.lock` scanning on very large lockfiles
  - nested Composer package/file assignment gaps
  - missing support for alternate Composer file names
- Python also maps manifest `source` and `dist` metadata but Rust previously left those fields empty for `composer.json`.

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

## Coverage

Coverage spans alternate Composer filenames, nested file assignment, lockfile handling, and richer manifest provenance behavior.
