# Package Assembly Implementation Plan

> **Status**: 🟢 Phase 1-3 Complete (Feb 10, 2026) | Phase 4 Pending
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

### Golden Tests (5 total)

| Test | Ecosystem | Status |
|------|-----------|--------|
| npm-basic | npm | ✅ Pass |
| cargo-basic | cargo | ✅ Pass |
| go-basic | golang | ✅ Pass |
| composer-basic | phpcomposer | ✅ Pass |
| maven-basic | maven | ✅ Pass |

---

## What's Left: Phase 4

### Python Reference Audit Findings

The Python ScanCode reference implementation has **three assembly patterns** beyond what we've implemented. These are all complex and involve capabilities that scancode-rust doesn't currently have (archive extraction, codebase tree walking).

#### Pattern A: Archive Extraction Assembly

**Ecosystems**: RubyGems (.gem), Debian (.deb), Alpine (.apk)

**What Python does**:

- Extracts archive contents (tar, gzip, etc.)
- Parses metadata from extracted files
- Uses `get_ancestor(levels_up=N)` to navigate up extracted directory trees
- Calls `assemble_from_many_datafiles()` to merge extracted manifests

**Example** (RubyGems):

```text
my-gem.gem (extracted) →
├── metadata.gz-extract/metadata.gz-extract  → gem metadata
├── data.gz-extract/*.gemspec                → gemspec
├── data.gz-extract/Gemfile                  → Gemfile
└── data.gz-extract/Gemfile.lock             → lockfile
```

**Rust gap**: We don't extract archives during scanning. Parsers only see pre-existing files on disk.

#### Pattern B: Database + File Reference Resolution

**Ecosystems**: RPM (NDB, SQLite, BDB), Alpine (installed DB), Debian (installed DB)

**What Python does**:

- Parses package database entries
- Each entry contains `file_references` (list of installed file paths)
- Walks up to filesystem root, then resolves each file reference path
- Associates resolved files with the package (`package_adder`)
- Tracks `missing_file_references` in `extra_data`
- Uses `os-release` to determine distro namespace (RPM)

**Rust gap**: Our `OnePerPackageData` mode creates packages from database entries, but doesn't resolve file references. The `for_packages` linking for database-sourced packages is incomplete.

#### Pattern C: Workspace Support

**Ecosystems**: npm (npm/pnpm workspaces), Cargo (workspaces)

**What Python does** (npm):

- Reads `workspaces` field from package.json
- Finds `pnpm-workspace.yaml` if present
- Creates separate Package for each workspace member
- Uses `walk_npm()` to assign resources, skipping `node_modules`

**Rust gap**: Cargo workspaces are partially handled by our sibling merge (Cargo.toml + Cargo.lock), but npm workspaces are not. Neither implementation creates separate packages per workspace member.

### What Python's "Consolidation" Does (NOT Assembly)

Consolidation is a **separate post-scan plugin** (`plugin_consolidate.py`), not part of assembly. It:

- Groups resources by package or copyright holder
- Creates `ConsolidatedPackage` and `ConsolidatedComponent` objects
- Combines declared licenses with discovered licenses in files
- This is tracked separately in [CONSOLIDATION_PLAN.md](../post-processing/CONSOLIDATION_PLAN.md)

---

## Phase 4: Remaining Assembly Work

### Priority Assessment

| Feature | Effort | Impact | Priority |
|---------|--------|--------|----------|
| File reference resolution (RPM/Alpine/Debian) | 2-3 weeks | Medium (installed pkg scanning) | P2 |
| npm workspace support | 1-2 weeks | High (monorepo scanning) | P1 |
| Archive extraction assembly | 3-4 weeks | Low (requires archive extraction infrastructure) | P3 |

### Recommended Order

1. **npm workspace support** — High impact, moderate effort. Many real-world codebases use npm/pnpm workspaces.
2. **File reference resolution** — Completes the database assembly story for installed package scanning.
3. **Archive extraction** — Requires building archive extraction infrastructure first. Lower priority since users typically scan source code, not archives.

### Success Criteria

- [ ] npm workspace assembly creates separate packages per workspace member
- [ ] Database assembly resolves file references for RPM/Alpine/Debian
- [ ] Golden tests for workspace and database assembly scenarios
- [ ] Archive extraction framework in place (stretch goal)

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
```

### Assembly Modes

| Mode | Behavior | Used By |
|------|----------|---------|
| `SiblingMerge` | Merge related files in same/nested directory | npm, cargo, maven, golang, etc. (23 configs) |
| `OnePerPackageData` | Each file becomes independent packages | Alpine DB, RPM DB, Debian installed DB (3 configs) |

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/assembly/mod.rs` | Core pipeline (709 lines) |
| `src/assembly/assemblers.rs` | 26 assembler configs (310 lines) |
| `src/assembly/sibling_merge.rs` | Sibling pattern matching (102 lines) |
| `src/assembly/nested_merge.rs` | Nested pattern matching (356 lines) |
| `src/assembly/assembly_golden_test.rs` | 5 golden tests (352 lines) |

---

## Python Reference

| Resource | Location |
|----------|----------|
| Assembly framework | `reference/scancode-toolkit/src/packagedcode/models.py` |
| npm assembly | `reference/scancode-toolkit/src/packagedcode/npm.py` |
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
