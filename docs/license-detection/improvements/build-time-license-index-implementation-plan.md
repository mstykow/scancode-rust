# Build-Time License Index - Implementation Plan

**Reference Document**: [build-time-license-index.md](./build-time-license-index.md)

This document breaks down the build-time license index implementation into manageable phases, each with clear deliverables and validation criteria.

## Overview

The goal is to make the default binary self-contained by embedding a build-time generated loader artifact, eliminating the runtime dependency on the ScanCode rules directory and removing YAML/frontmatter parsing at startup.

## Phase 1: Foundation - Engine Initialization and Loader-Stage Types

**Status**: Complete

### Objectives

1. Refactor `LicenseDetectionEngine` to support multiple initialization sources
2. Introduce explicit loader-stage type modules

### Deliverables

- [ ] Add `LicenseDetectionEngine::from_index(index: LicenseIndex) -> Result<Self>`
- [ ] Add `LicenseDetectionEngine::from_embedded() -> Result<Self>` (stub initially)
- [ ] Add `LicenseDetectionEngine::from_directory(rules_path: &Path) -> Result<Self>`
- [ ] Remove `LicenseDetectionEngine::new()` in favor of explicit constructors
- [ ] Create `src/license_detection/embedded/mod.rs`
- [ ] Create `src/license_detection/embedded/schema.rs`
- [ ] Create `src/license_detection/models/mod.rs` (if needed)
- [ ] Create `src/license_detection/models/loaded_rule.rs` (stub)
- [ ] Create `src/license_detection/models/loaded_license.rs` (stub)

### Validation

- All existing tests pass
- Clippy clean
- `from_directory()` works identically to current behavior

---

## Phase 2: Loader-Stage Models

**Status**: Pending

### Objectives

1. Define complete `LoadedRule` and `LoadedLicense` schemata
2. Implement all loader-stage normalization logic

### Deliverables

- [ ] Implement `LoadedRule` struct with all required fields
- [ ] Implement `LoadedLicense` struct with all required fields
- [ ] Implement `EmbeddedLoaderSnapshot` wrapper struct
- [ ] Add Serde derive for all loader-stage types
- [ ] Implement loader-stage normalization:
  - [ ] Derive `identifier` from filename
  - [ ] Derive `rule_kind` from source booleans with validation
  - [ ] Normalize `license_expression` (handle false-positive fallback)
  - [ ] Trim and normalize text fields
  - [ ] Handle URL merging for licenses
  - [ ] Validate file-local constraints

### Validation

- Unit tests for normalization logic
- Clippy clean
- Serde roundtrip tests

---

## Phase 3: Loader Refactoring

**Status**: Pending

### Objectives

1. Refactor existing loader to return `LoadedRule` and `LoadedLicense`
2. Separate loader-stage from build-stage concerns

### Deliverables

- [ ] Add `load_rules_from_directory(path) -> Result<Vec<LoadedRule>>`
- [ ] Add `load_licenses_from_directory(path) -> Result<Vec<LoadedLicense>>`
- [ ] Refactor `src/license_detection/rules/loader.rs` to use loader-stage types
- [ ] Extract content-based parsing helpers
- [ ] Keep text normalization and frontmatter interpretation behavior
- [ ] Move deprecated filtering out of loader (to build stage)

### Validation

- Existing loader tests pass
- New tests for `load_rules_from_directory` and `load_licenses_from_directory`
- Clippy clean

---

## Phase 4: Build-Stage Refactoring

**Status**: Pending

### Objectives

1. Create build stage that converts loaded types to runtime types
2. Move deprecated filtering to build stage
3. Implement license-derived rule synthesis in build stage

### Deliverables

- [ ] Add `build_index(loaded_rules, loaded_licenses, with_deprecated) -> LicenseIndex`
- [ ] Implement `LoadedRule -> Rule` conversion
- [ ] Implement `LoadedLicense -> License` conversion
- [ ] Move deprecated filtering to build stage with `with_deprecated: bool` parameter
- [ ] Ensure license-derived rule synthesis happens after deprecated filtering
- [ ] Update `LicenseDetectionEngine::from_directory()` to use new pipeline

### Validation

- `from_directory()` produces equivalent results to old pipeline
- Tests for deprecated filtering behavior
- Clippy clean

---

## Phase 5: Embedded Artifact Generation

**Status**: Pending

### Objectives

1. Create tooling to generate compressed loader artifacts
2. Implement deterministic serialization

### Deliverables

- [ ] Add `rmp-serde` and `zstd` dependencies
- [ ] Create generator binary: `src/bin/generate-license-loader-artifact.rs`
- [ ] Implement artifact generation:
  - [ ] Load rules and licenses from directory
  - [ ] Sort deterministically
  - [ ] Serialize with MessagePack
  - [ ] Compress with zstd
  - [ ] Write to output path
- [ ] Create maintainer script: `scripts/update-license-index-loader-artifact.sh`
- [ ] Generate initial artifact at `resources/license_detection/license_index_loader.msgpack.zst`

### Validation

- Generated artifact is deterministic (regenerate twice, compare bytes)
- Artifact can be deserialized back
- Clippy clean

---

## Phase 6: Runtime Embedded Loading

**Status**: Pending

### Objectives

1. Implement embedded artifact loading
2. Complete `LicenseDetectionEngine::from_embedded()`

### Deliverables

- [ ] Add embedded artifact bytes via `include_bytes!`
- [ ] Implement decompression (zstd)
- [ ] Implement deserialization (MessagePack)
- [ ] Validate `schema_version`
- [ ] Feed loaded data to build stage
- [ ] Complete `LicenseDetectionEngine::from_embedded()` implementation
- [ ] Update `Cargo.toml` to include artifact in package

### Validation

- `from_embedded()` initializes successfully
- Error handling for corrupt/invalid artifacts
- Clippy clean

---

## Phase 7: CLI Integration

**Status**: Pending

### Objectives

1. Switch CLI default to embedded index
2. Keep custom dataset support

### Deliverables

- [ ] Update `src/cli.rs`:
  - [ ] Default `license_rules_path` to `None`
  - [ ] `None` means use embedded index
  - [ ] `Some(path)` means use custom directory
- [ ] Update `src/main.rs`:
  - [ ] Use `from_embedded()` by default
  - [ ] Use `from_directory()` when path specified
  - [ ] Fail scan on initialization error (no silent skip)
- [ ] Remove default path to `reference/scancode-toolkit/src/licensedcode/data`

### Validation

- Binary runs without reference submodule
- `--license-rules-path` override works
- Initialization failure stops scan
- All existing tests pass
- Clippy clean

---

## Phase 8: Testing and Validation

**Status**: Pending

### Objectives

1. Add comprehensive equivalence tests
2. Add deterministic generation checks
3. Add packaging/build safeguards

### Deliverables

- [ ] Engine equivalence tests:
  - [ ] Compare `from_embedded()` vs `from_directory()` outputs
  - [ ] Compare deserialized vs filesystem loader outputs
  - [ ] Compare detection results for sample texts
- [ ] Determinism tests:
  - [ ] Regenerate artifact twice, verify identical bytes
  - [ ] Verify sorted output
- [ ] Failure handling tests:
  - [ ] Corrupt artifact bytes
  - [ ] Schema mismatch
  - [ ] Empty pattern edge cases
- [ ] Packaging tests:
  - [ ] Build without reference submodule
  - [ ] Verify artifact is loadable

### Validation

- All new tests pass
- Clippy clean

---

## Phase 9: Documentation

**Status**: Pending

### Objectives

1. Update developer documentation
2. Update user-facing documentation

### Deliverables

- [ ] Update `README.md`:
  - [ ] Mention built-in license index
  - [ ] Clarify reference dataset is optional
- [ ] Update `docs/ARCHITECTURE.md`:
  - [ ] Document loader/build stage separation
  - [ ] Document embedded artifact flow
- [ ] Update `docs/license-detection/ARCHITECTURE.md`
- [ ] Update `src/lib.rs` examples
- [ ] Update CLI help text in `src/cli.rs`
- [ ] Update `setup.sh` guidance

### Validation

- Documentation is accurate and complete
- Clippy clean

---

## Execution Notes

### Dependencies to Add

```toml
rmp-serde = "1.1"      # MessagePack serialization
zstd = "0.13"          # Compression
```

### File Structure

```
src/license_detection/
├── mod.rs
├── embedded/
│   ├── mod.rs
│   └── schema.rs
├── models/
│   ├── mod.rs
│   ├── loaded_rule.rs
│   └── loaded_license.rs
├── rules/
│   └── loader.rs      # Refactored to return LoadedRule
├── licenses/
│   └── loader.rs      # Refactored to return LoadedLicense
└── ...

resources/license_detection/
└── license_index_loader.msgpack.zst  # Generated artifact

scripts/
└── update-license-index-loader-artifact.sh
```

### Key Decisions

1. **No `build.rs`**: Use checked-in artifact, explicit regeneration
2. **Loader-stage normalization**: Single-file transformations only
3. **Build-stage policy**: Deprecated filtering, license synthesis
4. **Deterministic output**: Sorted collections, stable serialization
5. **Fail-fast defaults**: Initialization errors stop the scan

### Rollback Plan

Each phase is independently testable. If issues arise:

1. Phase 1-4: Can be developed behind feature flag
2. Phase 5-6: Artifact generation is separate from loading
3. Phase 7: CLI change is last, can be reverted independently
