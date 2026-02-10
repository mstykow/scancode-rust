# Package Assembly Implementation Plan

> **Status**: 🟢 Phase 1 Complete (Feb 10, 2026) | Phase 2-6 Pending
> **Priority**: P0 - Critical for Package Detection Completeness
> **Estimated Effort**: 4-6 weeks (Phase 1: 2-3 weeks)
> **Dependencies**: PARSER_PARITY_PLAN.md (parsers must exist first)

## Overview

Package assembly merges related manifest files into logical packages. For example, `package.json` + `package-lock.json` + `yarn.lock` are assembled into a single Package with complete dependency information.

## Scope

### What This Covers

- Sibling file merging (files in same directory)
- Nested file merging (e.g., Maven pom.xml + META-INF/MANIFEST.MF)
- Directory-based assembly (e.g., Debian installed packages)
- Archive extraction assembly (e.g., .gem files)
- UID generation for package instances
- Top-level `packages[]` and `dependencies[]` output arrays
- `for_packages` linking (files → packages)

### What This Doesn't Cover

- Package consolidation (deduplication) - see CONSOLIDATION_PLAN.md
- Package summarization (tallies, facets) - see SUMMARIZATION_PLAN.md

## Design Decisions (Finalized)

### 1. Assembly Activation

**Decision**: Always-on with `--no-assemble` opt-out flag

**Rationale**: Modern tool design, matches user expectations. Users expect complete package data by default.

### 2. UID Strategy

**Decision**: Random UUID (Python parity)

**Format**: `pkg:npm/lodash@4.17.21?uuid=a1b2c3d4-...`

**Rationale**:

- Enables traceability (file → package instance)
- Stable across file content changes
- Python compatibility
- Use fixed UUIDs in tests

### 3. Phase 1 Scope

**Decision**: Sibling-merge pattern (8 ecosystems)

**Ecosystems**: npm, cargo, cocoapods, phpcomposer, golang, pubspec, chef, conan

**Rationale**: Highest ROI (40% of ecosystems), simplest pattern, generic framework

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/packagedcode/`

**Key Files**:

- `models.py` - Assembly framework, `assemble()` method
- `build.py` - Package building and UID generation
- Individual parser files - Each has `assemble()` implementation

**Assembly Patterns**:

1. **Sibling-Merge** (8 ecosystems) - Files in same directory
2. **Nested Sibling-Merge** (1 ecosystem) - Maven with nested MANIFEST.MF
3. **Directory-Based** (3 ecosystems) - Conda, Alpine, Debian
4. **Archive Extraction** (3 ecosystems) - Debian, Alpine, RubyGems
5. **Database-Based** (1 ecosystem) - RPM
6. **Multi-Format** (2 ecosystems) - PyPI, RubyGems

**Total**: 20 ecosystems with assembly support

## Current State in Rust

### Phase 1 Complete ✅ (Feb 10, 2026)

- ✅ Individual file parsers (79 parsers, ~98% parity)
- ✅ PURL generation for packages and dependencies
- ✅ File-level package data extraction
- ✅ `package_uid`, `dependency_uid`, `for_package_uid` fields (commit 0d22687)
- ✅ Assembly framework with sibling-merge pattern (commit 0d22687)
- ✅ Assembly phase in scanner pipeline (commit 0d22687)
- ✅ Top-level `packages[]` and `dependencies[]` arrays (commit 0d22687)
- ✅ UID generation logic with UUID v4 (commit 0d22687)
- ✅ 8 ecosystem assemblers: npm, cargo, cocoapods, composer, golang, pubspec, chef, conan (commit 0d22687)
- ✅ `--no-assemble` CLI flag (commit 8cfc855)
- ✅ Golden tests for npm, cargo, go, composer (commit fce212c)

### Phase 2-6 Pending

- ❌ Nested sibling-merge (maven)
- ❌ Directory-based assembly (conda, alpine, debian)
- ❌ Archive extraction (debian, alpine, rubygems)
- ❌ Database-based assembly (rpm)
- ❌ Multi-format assembly (pypi, rubygems)
- ❌ Golden tests for cocoapods, pubspec, chef, conan

## Architecture Design

### New Data Structures

```rust
// Add to PackageData struct
pub package_uid: Option<String>,  // PURL + UUID qualifier

// Add to Dependency struct
pub dependency_uid: Option<String>,  // PURL + UUID qualifier
pub for_package_uid: Option<String>,  // Links to owning package

// New Package struct (composition over PackageData)
pub struct Package {
    pub package_data: PackageData,
    pub datafile_paths: Vec<String>,  // Which files define this package
    pub datasource_ids: Vec<String>,  // Which parsers extracted data
}

// New output structure
pub struct ScanOutput {
    pub packages: Vec<Package>,  // Top-level assembled packages
    pub dependencies: Vec<Dependency>,  // Top-level dependencies
    pub files: Vec<FileInfo>,  // File-level data with for_packages
}
```

### Assembler Trait

```rust
pub trait PackageAssembler {
    /// Ecosystem this assembler handles
    const ECOSYSTEM: &'static str;
    
    /// Find related files that should be assembled together
    fn find_related_files(&self, file: &Path, codebase: &Codebase) -> Vec<PathBuf>;
    
    /// Merge multiple PackageData into a single Package
    fn assemble(&self, package_data: Vec<PackageData>, paths: Vec<PathBuf>) -> Package;
}
```

### Scanner Pipeline Integration

```text
Current:
File Enumeration → Parser Selection → Package Extraction → JSON Output

With Assembly:
File Enumeration → Parser Selection → Package Extraction → Assembly Phase → JSON Output
                                                              ↓
                                                    Merge related files
                                                    Generate UIDs
                                                    Build packages[] array
```

## Implementation Phases

### Phase 1: Sibling-Merge ✅ COMPLETED (Feb 10, 2026)

**Goal**: Implement generic sibling-merge framework + 8 ecosystems

**Tasks**:

1. ✅ Add UID fields to structs (commit 0d22687)
2. ✅ Implement `build_package_uid()` function (commit 0d22687)
3. ✅ Create assembly framework (commit 0d22687)
4. ✅ Implement generic sibling-merge logic (commit 0d22687)
5. ✅ Add assembly phase to scanner (commit 0d22687)
6. ✅ Implement 8 assemblers (commit 0d22687):
   - ✅ npm
   - ✅ cargo
   - ✅ cocoapods
   - ✅ phpcomposer (composer)
   - ✅ golang
   - ✅ pubspec
   - ✅ chef
   - ✅ conan
7. ✅ Update output format (commit 0d22687)
8. ✅ Add `--no-assemble` flag (commit 8cfc855)
9. ✅ Add golden tests for 4 ecosystems (commit fce212c)

**Deliverables**:

- Generic assembly framework
- 8 ecosystem assemblers
- Updated output format
- CLI flag for opt-out
- Golden tests validating assembly

### Phase 2: Nested Sibling-Merge (1-2 weeks)

**Ecosystems**: Maven (1)

**Complexity**: Handles nested directory structures (pom.xml + META-INF/MANIFEST.MF)

### Phase 3: Directory-Based (2-3 weeks)

**Ecosystems**: Conda, Alpine, Debian (3)

**Complexity**: Scans directory trees to find package boundaries

### Phase 4: Archive Extraction (3-4 weeks)

**Ecosystems**: Debian, Alpine, RubyGems (3)

**Complexity**: Requires archive extraction and file introspection

### Phase 5: Database-Based (2-3 weeks)

**Ecosystems**: RPM (1)

**Complexity**: Queries system package databases

### Phase 6: Multi-Format (2-3 weeks)

**Ecosystems**: PyPI, RubyGems (2)

**Complexity**: Handles multiple file formats per ecosystem

**Total Timeline**: 14-18 weeks for complete parity

## Success Criteria

### Phase 1 (Sibling-Merge)

- [x] All 8 sibling-merge ecosystems have assemblers
- [x] Assembly runs by default, `--no-assemble` disables it
- [x] `packages[]` and `dependencies[]` arrays in JSON output
- [x] UIDs are stable and traceable
- [x] File-to-package linking via `for_packages` works
- [x] Tests pass for all 8 ecosystems
- [x] Golden tests exist for at least 4 ecosystems
- [x] Documentation updated

**Completion Summary**:

- **Branch**: feat/package-assembly
- **Commits**:
  - 27fb60e - Parser datasource_id fixes
  - 0d22687 - Assembly implementation
  - fce212c - Golden tests
  - 8cfc855 - Documentation
- **Test Coverage**: 10 unit tests + 4 golden tests
- **Status**: Ready for Phase 2 (Maven nested sibling-merge)

### Complete Parity (All Phases)

- [ ] All 20 Python assemblers ported
- [ ] All assembly patterns implemented
- [ ] Golden tests pass for all ecosystems
- [ ] Documentation complete

## Testing Strategy

1. **Unit Tests**: Test each assembler in isolation
2. **Golden Tests**: Compare assembled output against Python reference
3. **Integration Tests**: Test full scanner pipeline with assembly
4. **Performance Tests**: Measure assembly overhead

## Related Documents

- **Implementation**: `PARSER_PARITY_PLAN.md` (prerequisite)
- **Implementation**: `ASSEMBLY_PARITY_ROADMAP.md` (Python analysis)
- **Implementation**: `ASSEMBLY_QUICK_REFERENCE.md` (concepts)
- **Implementation**: `PYTHON_ASSEMBLERS_DETAILED.md` (reference)
- **Evergreen**: `ARCHITECTURE.md` (scanner pipeline)
- **Evergreen**: `TESTING_STRATEGY.md` (golden tests)

## Notes

- Assembly is independent of license/copyright detection
- Can be implemented in parallel with other features
- Phase 1 provides immediate value (npm + cargo = most common ecosystems)
- Later phases have diminishing returns (less common ecosystems)
