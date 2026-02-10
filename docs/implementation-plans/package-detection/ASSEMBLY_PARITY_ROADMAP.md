# Assembly Parity Roadmap for scancode-rust

## Executive Summary

Python ScanCode implements assembly logic for **20 package ecosystems**. Assembly is the process of combining data from multiple related manifest/lockfile datafiles into a single Package with its dependencies.

**Current Status**: Identify which ecosystems need assembly implementation in Rust.

---

## Assembly Patterns (6 Total)

### 1. **Sibling-Merge** (8 ecosystems)

Most common pattern. Find related files in same directory and merge them.

**Ecosystems**: npm, cargo, cocoapods, phpcomposer, golang, pubspec, chef, conan

**Implementation Complexity**: ⭐⭐ (Medium)

- Find sibling files by name
- Merge data from manifest + lockfile
- Create single Package with combined dependencies

**Key Challenge**: Handling missing lockfiles (manifest-only case)

---

### 2. **Nested Sibling-Merge** (1 ecosystem)

Find related files in nested directory structures.

**Ecosystems**: maven

**Implementation Complexity**: ⭐⭐⭐ (High)

- Navigate nested directory structure (META-INF/)
- Find pom.xml and MANIFEST.MF
- Order-dependent merging (pom.xml first, then MANIFEST.MF)

**Key Challenge**: Correct directory traversal and ordering

---

### 3. **Directory-Based** (3 ecosystems)

Scan directory for multiple related files and merge them.

**Ecosystems**: conda, alpine, debian

**Implementation Complexity**: ⭐⭐⭐ (High)

- Scan directory for multiple metadata files
- Merge all files into single Package
- Handle installation structure awareness

**Key Challenge**: Directory scanning and multi-file merging

---

### 4. **Archive Extraction** (3 ecosystems)

Extract archive files and parse metadata from contents.

**Ecosystems**: debian, alpine, rubygems

**Implementation Complexity**: ⭐⭐⭐⭐ (Very High)

- Extract archive files (tar, gzip, etc.)
- Parse metadata from extracted contents
- Merge with other metadata sources

**Key Challenge**: Archive handling and extraction

---

### 5. **Database-Based** (1 ecosystem)

Parse system package databases directly.

**Ecosystems**: rpm

**Implementation Complexity**: ⭐⭐⭐⭐ (Very High)

- Parse NDB database format (SUSE)
- Parse SQLite database format (RHEL/CentOS/Fedora)
- Extract package metadata from database

**Key Challenge**: Database format parsing

---

### 6. **Multi-Format** (2 ecosystems)

Support multiple metadata file formats and merge them.

**Ecosystems**: pypi, rubygems

**Implementation Complexity**: ⭐⭐⭐⭐ (Very High)

- Support multiple metadata formats
- Merge data from whichever format is present
- Handle different installation layouts

**Key Challenge**: Format detection and conditional merging

---

## Implementation Priority

### Phase 1: Sibling-Merge ✅ COMPLETED (Feb 10, 2026)

**Pattern**: Files in same directory merged into single package

**Ecosystems** (8 total):

- ✅ npm (package.json + lockfiles)
- ✅ cargo (Cargo.toml + Cargo.lock)
- ✅ cocoapods (*.podspec + Podfile.lock)
- ✅ phpcomposer (composer.json + composer.lock)
- ✅ golang (go.mod + go.sum)
- ✅ pubspec (pubspec.yaml + pubspec.lock)
- ✅ chef (metadata.json + metadata.rb)
- ✅ conan (conanfile.py + conandata.yml)

**Implementation**: src/assembly/mod.rs, src/assembly/sibling_merge.rs, src/assembly/assemblers.rs

**Golden Tests**: 4/8 (npm, cargo, go, composer) - testdata/assembly-golden/

---

### Phase 2: Nested Sibling-Merge (High Priority)

**Effort**: 1-2 weeks | **Impact**: 1 ecosystem (but important for Java)

1. **maven** - JAR-specific nested structure

**Deliverable**: Nested directory traversal and order-dependent merging

---

### Phase 3: Directory-Based (Medium Priority)

**Effort**: 2-3 weeks | **Impact**: 3 ecosystems

1. **conda** - Directory scanning + environment merging
2. **alpine** - Archive + database + build script
3. **debian** - Archive extraction + metadata merge

**Deliverable**: Directory scanning and multi-file merging framework

---

### Phase 4: Archive Extraction (Lower Priority)

**Effort**: 3-4 weeks | **Impact**: 3 ecosystems (but complex)

1. **debian** - .deb archive extraction
2. **alpine** - .apk archive extraction
3. **rubygems** - .gem archive extraction

**Deliverable**: Archive extraction and metadata parsing

---

### Phase 5: Database-Based (Lower Priority)

**Effort**: 2-3 weeks | **Impact**: 1 ecosystem (system packages)

1. **rpm** - NDB and SQLite database parsing

**Deliverable**: Database format parsing and metadata extraction

---

### Phase 6: Multi-Format (Lower Priority)

**Effort**: 2-3 weeks | **Impact**: 2 ecosystems

1. **pypi** - Multiple Python metadata formats
2. **rubygems** - Multiple gem formats

**Deliverable**: Format detection and conditional merging

---

## Ecosystems WITHOUT Assembly Support (17 total)

These have parsers but use default assembly (single-file only):

- bower
- cran
- freebsd
- gemfile_lock
- godeps
- haxe
- jar_manifest
- msi
- nevra
- nuget
- opam
- pyrpm
- readme
- spec
- win_pe
- win_reg
- windows

**Note**: These are lower priority as they don't require multi-file assembly logic.

---

## Key Implementation Insights

### 1. Sibling Finding

```rust
// Find related files in same directory
let siblings = resource.siblings(codebase);
let lockfile = siblings.iter()
    .find(|r| r.name == "package-lock.json");
```

### 2. Order Matters

In `assemble_from_many()`, the order of PackageData items is critical:

- First item creates the Package
- Subsequent items update it
- Packages must be yielded before Dependencies

### 3. Conditional Package Creation

```rust
// Only create package if PURL exists
if let Some(purl) = &package_data.purl {
    let package = Package::from_package_data(package_data, datafile_path);
    yield package;
} else {
    // No package, only yield dependencies
}
```

### 4. Scope Preservation

Each ecosystem has native scope terminology that must be preserved:

- npm: `dependencies`, `devDependencies`, `peerDependencies`, etc.
- cargo: `dependencies`, `dev-dependencies`, `build-dependencies`
- maven: `compile`, `test`, `provided`, `runtime`, `system`

### 5. Workspace Support

Cargo is unique in supporting workspaces:

- Multiple packages in single workspace
- Copy workspace-level metadata to members
- Handle glob patterns for member paths

---

## Testing Strategy

### Golden Tests

Compare Rust output with Python ScanCode reference:

```bash
# Run Python ScanCode on test data
python -m scancode --json-pp output.json testdata/

# Run Rust implementation
cargo run -- testdata/ -o output.json

# Compare outputs
diff output.json expected.json
```

### Edge Cases to Test

1. **Missing lockfiles** (manifest-only)
2. **Missing manifests** (lockfile-only)
3. **Workspace configurations** (cargo)
4. **Nested directory structures** (maven)
5. **Archive extraction** (debian, alpine, rubygems)
6. **Database parsing** (rpm)
7. **Multiple metadata formats** (pypi, rubygems)

### Scope Preservation Tests

- Verify native scope terminology is preserved
- Test scope-specific dependency handling
- Validate optional/runtime/dev distinctions

---

## Architecture Recommendations

### 1. Generic Sibling-Merge Framework

```rust
pub trait SiblingMergeAssembler {
    fn find_siblings(&self, resource: &Resource, codebase: &Codebase) -> Vec<Resource>;
    fn merge_package_data(&self, primary: PackageData, secondary: PackageData) -> PackageData;
}
```

### 2. Directory-Based Assembly Framework

```rust
pub trait DirectoryAssembler {
    fn scan_directory(&self, resource: &Resource, codebase: &Codebase) -> Vec<Resource>;
    fn merge_multiple(&self, datafiles: Vec<PackageData>) -> PackageData;
}
```

### 3. Archive Extraction Framework

```rust
pub trait ArchiveAssembler {
    fn extract_archive(&self, path: &Path) -> Result<Vec<u8>>;
    fn parse_extracted(&self, contents: &[u8]) -> Result<PackageData>;
}
```

### 4. Database Parsing Framework

```rust
pub trait DatabaseAssembler {
    fn parse_database(&self, path: &Path) -> Result<Vec<PackageData>>;
}
```

---

## Dependency Scope Reference

### npm

- `dependencies` - Runtime dependencies
- `devDependencies` - Development-only
- `peerDependencies` - Peer dependencies
- `optionalDependencies` - Optional runtime
- `bundledDependencies` - Bundled with package

### cargo

- `dependencies` - Runtime dependencies
- `dev-dependencies` - Development-only
- `build-dependencies` - Build-time only

### maven

- `compile` - Compile and runtime (default)
- `test` - Test-time only
- `provided` - Provided by runtime
- `runtime` - Runtime only
- `system` - System-provided

### python (pypi)

- `None` - Runtime dependencies
- `<extra_name>` - Optional dependency groups
- `dev` - Development dependencies (Poetry)

### golang

- `direct` - Direct dependencies
- `indirect` - Transitive dependencies

### ruby (gems)

- `runtime` - Runtime dependencies
- `development` - Development-only

---

## Success Criteria

**Phase 1 Success Criteria**:

- [x] Generic sibling-merge framework implemented
- [x] All 8 ecosystems have assemblers
- [x] Assembly integrated into scanner pipeline
- [x] Golden tests for at least 50% of ecosystems (4/8)

**Completion Date**: February 10, 2026
**Branch**: feat/package-assembly

### Phase 2 Complete

- [ ] Maven nested sibling-merge implemented
- [ ] JAR structure handling correct
- [ ] Order-dependent merging working
- [ ] Golden tests passing

### Phase 3 Complete

- [ ] Conda directory-based assembly working
- [ ] Alpine archive + database handling
- [ ] Debian archive extraction
- [ ] Golden tests passing

### Phase 4 Complete

- [ ] Archive extraction framework in place
- [ ] All archive-based ecosystems working
- [ ] Golden tests passing

### Phase 5 Complete

- [ ] Database parsing framework in place
- [ ] RPM NDB and SQLite support
- [ ] Golden tests passing

### Phase 6 Complete

- [ ] Multi-format detection working
- [ ] PyPI and RubyGems formats supported
- [ ] Golden tests passing

### Final

- [ ] 100% feature parity with Python ScanCode
- [ ] All 20 ecosystems with assembly support
- [ ] All golden tests passing
- [ ] Performance benchmarks showing improvement

---

## References

- **Python Implementation**: `reference/scancode-toolkit/src/packagedcode/`
- **Models**: `reference/scancode-toolkit/src/packagedcode/models.py`
- **Test Data**: `reference/scancode-toolkit/tests/packagedcode/data/`
- **Documentation**: `docs/PYTHON_ASSEMBLERS_SUMMARY.md`, `docs/PYTHON_ASSEMBLERS_DETAILED.md`
