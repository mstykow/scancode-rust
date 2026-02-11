# Package Assembly Implementation Plan

> **Status**: 🟢 **COMPLETE** — All phases done (Feb 11, 2026). Package assembly is feature-complete.
> **Priority**: P0 - Critical for Package Detection Completeness
> **Dependencies**: PARSER_PLAN.md (parsers must exist first)

## Overview

Package assembly merges related manifest files into logical packages. For example, `package.json` + `package-lock.json` + `yarn.lock` are assembled into a single Package with complete dependency information.

### Scope

**Covers**: Sibling file merging, nested file merging, directory-based assembly, archive extraction assembly, UID generation, top-level `packages[]` and `dependencies[]` output arrays, `for_packages` linking.

**Does NOT cover**: Package consolidation (see `../post-processing/CONSOLIDATION_PLAN.md`), package summarization (see `../post-processing/SUMMARIZATION_PLAN.md`).

---

## Current State (Accurate as of Feb 11, 2026)

### ✅ Phase 1: Sibling-Merge — COMPLETE (Feb 10, 2026)

Generic sibling-merge framework + 8 ecosystems.

- Assembly framework with sibling-merge pattern
- Assembly phase in scanner pipeline
- Top-level `packages[]` and `dependencies[]` arrays in JSON output
- UID generation logic with UUID v4
- `--no-assemble` CLI flag
- 8 ecosystem assemblers: npm, cargo, cocoapods, composer, golang, pubspec, chef, conan
- Golden tests for 4 ecosystems (npm, cargo, go, composer)

**Key commits**: 0d22687 (assembly implementation), fce212c (golden tests), 8cfc855 (`--no-assemble` flag)

### ✅ Phase 2: Nested Sibling-Merge — COMPLETE (Feb 10, 2026)

Nested directory patterns for ecosystems where related files live in different subdirectories.

- Maven nested sibling-merge (pom.xml + META-INF/MANIFEST.MF)
- Debian source nested merge (debian/ directory)
- Generalized `find_package_root()` for nested anchor directories
- Maven golden test passing

**Implementation**: `src/assembly/nested_merge.rs`

### ✅ Phase 3: Comprehensive Assembler Configs — COMPLETE (Feb 10, 2026)

Every parser's datasource_id now has an assembly config.

- `AssemblyMode` enum: `SiblingMerge` + `OnePerPackageData`
- `OnePerPackageData` mode for database files (Alpine, RPM, Debian installed DBs)
- 26 assembler configs covering all parser ecosystems
- 10 datasource IDs explicitly documented as intentionally unassembled
- All phantom datasource IDs fixed

**Key commits**: 8f2465e (Alpine/RPM parser fixes), 70335a5 (OnePerPackageData), 34c4f37 (Debian source nested merge)

### ✅ Phase 4a: npm Workspace Assembly — COMPLETE (Feb 11, 2026)

npm/pnpm workspace support: creates separate Package objects per workspace member, resolves `workspace:*` version references, and properly assigns shared resources.

- Workspace root detection (package.json `workspaces` field + pnpm-workspace.yaml)
- Member discovery via three-tier glob matching (simple paths, single-star, complex globs)
- Member Package creation with proper UID assignment
- Root dependency hoisting (workspace-level, or to root if pnpm)
- `workspace:*`, `workspace:^`, `workspace:~` version resolution against member versions
- `for_packages` assignment (member files → member UID; shared files → all members or root only)
- pnpm variant handling (non-private root kept as separate package)
- Sibling-merge cleanup (removes duplicate packages created by earlier assembly phases)
- Exclusion pattern support for workspace member discovery

**Implementation**: `src/assembly/workspace_merge.rs` (859 lines)
**Key commits**: 55cac94 (workspace assembly implementation)

### ✅ Phase 4b: File Reference Resolution — COMPLETE (Feb 11, 2026)

Resolves `file_references` from package database entries (RPM/Alpine/Debian) against scanned files on disk.

- Database path detection for Alpine (`lib/apk/db/installed`), RPM (BDB/NDB/SQLite), Debian (`var/lib/dpkg/status`), Debian Distroless
- Root path computation from datafile path (e.g., `rootfs/var/lib/rpm/Packages` → root `rootfs/`)
- File reference resolution via HashMap index (O(1) lookup per reference)
- `for_packages` assignment on matched files
- Missing reference tracking in `package.extra_data["missing_file_references"]`
- RPM namespace resolution from `etc/os-release` or `usr/lib/os-release`
- Namespace propagation to package dependencies
- Per-package file reference collection (purl-matched, not all-packages-in-file)

**Implementation**: `src/assembly/file_ref_resolve.rs` (750 lines)

**Bug fixes during implementation:**

- Fixed Alpine parser case-sensitivity bug: `T:`/`t:` and `C:`/`c:` keys were colliding due to rfc822 parser lowercasing all keys. Replaced with Alpine-specific case-sensitive parser.
- Fixed file reference collection: was returning ALL file_references from a DB file for every package instead of only the matching package's references (purl-based filtering).

### ✅ Phase 4c: Cargo Workspace Assembly — COMPLETE (Feb 11, 2026)

Cargo workspace support: creates separate Package objects per workspace member, resolves `[workspace.package]` inheritance and `workspace = true` dependencies.

- Workspace root detection via `[workspace]` section with `members` array
- Member discovery via glob pattern matching (simple paths, single-star, complex globs)
- Workspace metadata extraction (`[workspace.package]` and `[workspace.dependencies]`)
- Full field inheritance: version, license, homepage, repository, categories, edition, rust-version, authors
- Dependency version resolution for `{ workspace = true }` dependencies
- Member Package creation with inherited metadata
- Root package removal (workspace manifests don't publish)
- `for_packages` assignment (member files → member UID; shared files → all members; `target/` excluded)

**Implementation**: `src/assembly/cargo_workspace_merge.rs` (524 lines)

### Golden Tests (8 total)

| Test | Ecosystem | Status |
|------|-----------|--------|
| npm-basic | npm | ✅ Pass |
| cargo-basic | cargo | ✅ Pass |
| go-basic | golang | ✅ Pass |
| composer-basic | phpcomposer | ✅ Pass |
| maven-basic | maven | ✅ Pass |
| npm-workspace | npm (workspace) | ✅ Pass |
| pnpm-workspace | npm (pnpm workspace) | ✅ Pass |
| alpine-file-refs | alpine (file reference resolution) | ✅ Pass |
| cargo-workspace | cargo (workspace) | ✅ Pass |

---

## Completed — No Remaining Work

All package assembly features are implemented. The assembly pipeline is feature-complete.

### ✅ Archive Extraction Assembly — OUT OF SCOPE

Archive extraction is **permanently out of scope** for scancode-rust. The Python ScanCode ecosystem uses a separate tool called [ExtractCode](https://github.com/aboutcode-org/extractcode) that users run as a preprocessing step before scanning. ScanCode's core pipeline never extracts archives — it only scans pre-existing files on disk.

Users of scancode-rust can use ExtractCode before scanning, just like with Python ScanCode. A future `extractcode-rust` tool could be created as a separate project, but that is entirely outside the scope of scancode-rust.

**Note**: Our `.deb` and `.apk` parsers can read metadata directly from archive files without extraction, which is an improvement over Python (documented in `docs/improvements/debian-parser.md` and `docs/improvements/alpine-parser.md`). This means users get package metadata from these archives without needing ExtractCode at all.

### ExtractCode Naming Convention Compatibility

When ExtractCode extracts archives, it creates directories with a `-extract` suffix (e.g., `control.tar.gz-extract/`, `data.gz-extract/`, `metadata.gz-extract/`). For scancode-rust to be a drop-in replacement, parsers must also match these `-extract` paths.

**Current coverage**:

| ExtractCode Pattern | Python Parser | Rust Status |
|---|---|---|
| `*/control.tar.{gz,xz}-extract/control` | `DebianControlFileInExtractedDebHandler` | ✅ Supported |
| `*/control.tar.{gz,xz}-extract/md5sums` | `DebianMd5sumsInDebHandler` | ✅ Supported |
| `*/metadata.gz-extract` | `GemArchiveHandler` | ✅ Supported |
| `*/data.gz-extract/*.gemspec` | `GemspecInExtractedGemHandler` | ✅ Supported |
| `*/data.gz-extract/Gemfile` | `GemfileInExtractedGemHandler` | ✅ Supported |
| `*/data.gz-extract/Gemfile.lock` | `GemfileLockInExtractedGemHandler` | ✅ Supported |
| `info/recipe.tar-extract/recipe/meta.yaml` | `CondaMetaYamlHandler` | ✅ Supported (by filename match) |

### What Python's "Consolidation" Does (NOT Assembly)

Consolidation is a **separate post-scan plugin** (`plugin_consolidate.py`), not part of assembly. It:

- Groups resources by package or copyright holder
- Creates `ConsolidatedPackage` and `ConsolidatedComponent` objects
- Combines declared licenses with discovered licenses in files
- This is tracked separately in [CONSOLIDATION_PLAN.md](../post-processing/CONSOLIDATION_PLAN.md)

---

## Success Criteria

- [x] npm workspace assembly creates separate packages per workspace member
- [x] Database assembly resolves file references for RPM/Alpine/Debian
- [x] Golden tests for workspace and database assembly scenarios
- [x] ExtractCode `-extract` path patterns supported for drop-in compatibility
- [x] Cargo workspace assembly with metadata inheritance and dependency resolution
- [x] Debian extracted control file parser for ExtractCode compatibility

---

## Architecture Reference

### Data Structures

```rust
// PackageData fields for assembly
pub package_uid: Option<String>,     // PURL + UUID qualifier
pub dependency_uid: Option<String>,  // PURL + UUID qualifier
pub for_package_uid: Option<String>, // Links dependency to owning package

// Package struct (assembled result)
pub struct Package {
    pub package_data: PackageData,
    pub datafile_paths: Vec<String>,   // Which files define this package
    pub datasource_ids: Vec<String>,   // Which parsers extracted data
}
```

### Assembly Pipeline

```text
File Enumeration → Parser Selection → Package Extraction → Assembly Phase → JSON Output
                                                               ↓
                                                     Phase 1: Group by directory → merge siblings
                                                     Phase 2: Find nested patterns → merge into root
                                                     Phase 3: OnePerPackageData → each file's packages
                                                     Phase 4: File ref resolution → for_packages linking
                                                     Phase 5: Workspace assembly → per-member packages
```

### Assembly Modes

| Mode | Behavior | Used By |
|------|----------|---------|
| `SiblingMerge` | Merge related files in same/nested directory | npm, cargo, maven, golang, etc. (23 configs) |
| `OnePerPackageData` | Each file becomes independent packages | Alpine DB, RPM DB, Debian installed DB (3 configs) |
| File ref resolution | Resolve file_references → `for_packages` linking | Alpine, RPM, Debian installed DBs |
| Workspace assembly | Post-processing pass creating per-member packages | npm/pnpm workspaces, Cargo workspaces |

### Performance Optimizations

- **Static assembler lookup via `LazyLock`**: The `ASSEMBLER_LOOKUP` HashMap (mapping every `DatasourceId` to its config key) is built once on first access using `std::sync::LazyLock` and stored as a `static`. Previously, this map was rebuilt on every `assemble()` call. Now there is zero allocation per call and O(1) config lookup by `DatasourceId`. See `src/assembly/mod.rs` lines 18–32.

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/assembly/mod.rs` | Core pipeline (713 lines) |
| `src/assembly/assemblers.rs` | 26 assembler configs (310 lines) |
| `src/assembly/sibling_merge.rs` | Sibling pattern matching (102 lines) |
| `src/assembly/nested_merge.rs` | Nested pattern matching (356 lines) |
| `src/assembly/workspace_merge.rs` | npm/pnpm workspace assembly (859 lines) |
| `src/assembly/cargo_workspace_merge.rs` | Cargo workspace assembly (524 lines) |
| `src/assembly/file_ref_resolve.rs` | File reference resolution for DB packages (750 lines) |
| `src/assembly/assembly_golden_test.rs` | 9 golden tests |

---

## Python Reference

| Resource | Location |
|----------|----------|
| Assembly framework | `reference/scancode-toolkit/src/packagedcode/models.py` |
| npm assembly | `reference/scancode-toolkit/src/packagedcode/npm.py` |
| Cargo assembly | `reference/scancode-toolkit/src/packagedcode/cargo.py` |
| Consolidation plugin | `reference/scancode-toolkit/src/summarycode/plugin_consolidate.py` |
| Test data | `reference/scancode-toolkit/tests/packagedcode/data/` |

### Python Helper Methods (for reference)

- `assemble()` — Main method, yields Package/Dependency/Resource
- `assemble_from_many()` — Combine multiple PackageData into single Package
- `assemble_from_many_datafiles()` — Find files by pattern and merge
- `assign_package_to_parent_tree()` — Associate package to directory tree
- `get_ancestor(levels_up=N)` — Walk up N directory levels

---

## Related Documents

- **Post-assembly**: [CONSOLIDATION_PLAN.md](../post-processing/CONSOLIDATION_PLAN.md) — Resource grouping and package enrichment (separate concern)
- **Parsers**: [PARSER_PLAN.md](PARSER_PLAN.md) — Parser implementations (prerequisite)
- **Evergreen**: [ARCHITECTURE.md](../../ARCHITECTURE.md) — Scanner pipeline architecture
- **Evergreen**: [TESTING_STRATEGY.md](../../TESTING_STRATEGY.md) — Golden test methodology
