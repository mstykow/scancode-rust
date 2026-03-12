# Swift Ecosystem: Root Package and Assembly Improvements

**Area**: Swift package detection and assembly  
**Files**: `src/assembly/swift_merge.rs`, `src/main_test.rs`, `src/parsers/swift_show_dependencies.rs`, `src/parsers/swift_manifest_json.rs`, `src/parsers/swift_resolved.rs`  
**Python Reference**: `reference/scancode-toolkit/src/packagedcode/swift.py`  
**Related Issue**: `aboutcode-org/scancode-toolkit#3793`

## Summary

**🐛 Bug Fix + 🔍 Enhanced Extraction**: Rust now assembles Swift package metadata with artifact-aware precedence instead of over- or under-asserting top-level package identity from whichever file happens to be present.

## Problem in the Reference Behavior Surface

Swift package metadata can come from multiple adjacent artifacts:

- `Package.swift.json`
- `Package.swift.deplock`
- `Package.resolved`
- `.package.resolved`
- `swift-show-dependencies.deplock`

The recent Swift batch showed that parser-level extraction alone was not enough. The important remaining behavior lived at **scan/assembly time**:

1. the manifest should own the root package when manifest data exists,
2. `swift-show-dependencies.deplock` should replace the dependency graph without overwriting manifest-owned root metadata,
3. `Package.resolved` should enrich dependency versions when show-dependencies data is missing,
4. resolved-only scans should still emit useful top-level packages, and
5. nested Swift roots should not accidentally inherit parent package ownership.

Without this logic, Swift scans could miss top-level packages entirely or attach the wrong root package identity.

## Rust Improvement

Rust now has a dedicated Swift assembly pass in `src/assembly/swift_merge.rs` with Swift-specific precedence rules.

### Implemented behavior

1. **Manifest-owned root package**
   - `Package.swift.json` has highest priority
   - `Package.swift.deplock` is the next fallback
   - `Package.swift` is the final manifest fallback

   This keeps the top-level package tied to the declaring manifest instead of inferring it from dependency lock data.

2. **Show-dependencies supersedes only the dependency graph**
   - `swift-show-dependencies.deplock` contributes the dependency graph
   - the manifest still owns the root package metadata
   - show-dependencies data is recorded in `datafile_paths` / `datasource_ids` without replacing root metadata

3. **Resolved fallback when show-dependencies data is absent**
   - `Package.resolved` / `.package.resolved` can replace manifest dependency versions with pinned resolved versions
   - this keeps root package ownership stable while still improving dependency precision

4. **Resolved-only package emission**
   - if a scan only has resolved data, Rust emits one top-level package per resolved dependency
   - this prevents resolved-only Swift projects from disappearing from assembled package output

5. **Nested-root resource isolation**
   - parent Swift package assignment skips nested Swift roots
   - files under a nested package are not incorrectly attached to the parent package

## Why this matters

- **Correct top-level package identity**: Swift scans now emit the intended root package instead of dropping it or deriving it from the wrong artifact.
- **Better dependency fidelity**: show-dependencies and resolved files contribute where they are strongest, without corrupting manifest metadata.
- **Safer package ownership**: nested Swift packages no longer inherit the wrong `for_packages` links.
- **More useful scan output**: resolved-only repositories still produce package-level results.

## Test Coverage

This Swift improvement is covered at both scan and unit levels.

### Scan-level regression tests

In `src/main_test.rs`:

- `swift_scan_uses_show_dependencies_only_fixture`
- `swift_scan_uses_resolved_only_fixture`
- `swift_scan_prefers_show_dependencies_over_manifest_dependencies`
- `swift_scan_falls_back_to_resolved_when_show_dependencies_missing`

These exercise the upstream-style fixtures:

- `fastlane_resolved_v1`
- `mapboxmaps_manifest_and_resolved`
- `vercelui`
- `vercelui_show_dependencies`

### Assembly unit tests

In `src/assembly/swift_merge.rs`:

- `build_swift_outputs_keeps_manifest_root_metadata`
- `assign_swift_resources_skips_nested_swift_roots`

### Existing parser golden coverage still applies

The parser layer remains covered by `src/parsers/swift_golden_test.rs`, including:

- manifest fixtures,
- resolved fixtures, and
- show-dependencies fixtures.

## Relationship to `swift-show-dependencies-parser.md`

`swift-show-dependencies-parser.md` documents the **parser-level** enhancement that extracts the full dependency graph from `swift-show-dependencies.deplock`.

This document covers the broader **ecosystem-level** Swift improvement added later: how manifest, show-dependencies, and resolved artifacts are assembled into correct scan output.

## References

- Reference implementation: `reference/scancode-toolkit/src/packagedcode/swift.py`
- Rust assembly implementation: `src/assembly/swift_merge.rs`
- Rust scan regressions: `src/main_test.rs`
- Rust parser goldens: `src/parsers/swift_golden_test.rs`
