# Assembly Quick Reference

## What is Assembly?

Assembly is the process of combining data from multiple related manifest/lockfile datafiles into a single Package with its dependencies.

**Example**: npm's `package.json` + `package-lock.json` → single Package with merged dependencies

---

## 20 Ecosystems with Assembly Support

| # | Ecosystem | Pattern | Files | Complexity |
|---|-----------|---------|-------|------------|
| 1 | npm | Sibling-merge | package.json + lockfiles | ⭐⭐ |
| 2 | cargo | Sibling-merge + workspace | Cargo.toml + Cargo.lock | ⭐⭐⭐ |
| 3 | maven | Nested sibling-merge | pom.xml + MANIFEST.MF | ⭐⭐⭐ |
| 4 | cocoapods | Sibling-merge | .podspec + Podfile.lock | ⭐⭐ |
| 5 | phpcomposer | Sibling-merge | composer.json + composer.lock | ⭐⭐ |
| 6 | rubygems | Multi-file + archive | .gemspec + Gemfile.lock + .gem | ⭐⭐⭐⭐ |
| 7 | golang | Sibling-merge | go.mod + go.sum | ⭐⭐ |
| 8 | pubspec | Sibling-merge | pubspec.yaml + pubspec.lock | ⭐⭐ |
| 9 | swift | Multi-file merge | Package.swift + Package.resolved | ⭐⭐⭐ |
| 10 | conda | Directory-based | conda-meta/*.json + environment.yaml | ⭐⭐⭐ |
| 11 | debian | Archive extraction | .deb + .debian.tar.xz | ⭐⭐⭐⭐ |
| 12 | alpine | Archive + database | .apk + installed DB + APKBUILD | ⭐⭐⭐⭐ |
| 13 | rpm | Database-based | Packages.db or Packages.sqlite | ⭐⭐⭐⭐ |
| 14 | pypi | Multi-format | PKG-INFO + setup.py + pyproject.toml | ⭐⭐⭐⭐ |
| 15 | chef | Sibling-merge | metadata.rb + metadata.json | ⭐⭐ |
| 16 | conan | Sibling-merge | conanfile.py + conandata.yml | ⭐⭐ |
| 17 | debian_copyright | Standalone | debian/copyright | ⭐ |
| 18 | about | Standalone | *.ABOUT | ⭐ |
| 19 | build | Non-assembling | configure, BUILD, etc. | ⭐ |

---

## 6 Assembly Patterns

### Pattern 1: Sibling-Merge ⭐⭐ (8 ecosystems)

Find related files in same directory and merge them.

**Ecosystems**: npm, cargo, cocoapods, phpcomposer, golang, pubspec, chef, conan

**Algorithm**:

```text
1. Parse primary manifest (e.g., package.json)
2. Find sibling lockfile (e.g., package-lock.json)
3. Merge dependencies from both
4. Create single Package with combined data
```

**Edge Cases**:

- Missing lockfile (manifest-only) → create package from manifest
- Missing manifest (lockfile-only) → yield dependencies only
- Multiple lockfiles → merge all

---

### Pattern 2: Nested Sibling-Merge ⭐⭐⭐ (1 ecosystem)

Find related files in nested directory structures.

**Ecosystems**: maven

**Algorithm**:

```text
1. Find pom.xml in META-INF/maven/**/
2. Find MANIFEST.MF in META-INF/
3. Create package from pom.xml
4. Update package with MANIFEST.MF data
5. Order matters: pom.xml first, then MANIFEST.MF
```

**Key Challenge**: Correct directory traversal and ordering

---

### Pattern 3: Directory-Based ⭐⭐⭐ (3 ecosystems)

Scan directory for multiple related files and merge them.

**Ecosystems**: conda, alpine, debian

**Algorithm**:

```text
1. Scan directory for metadata files
2. Parse each file
3. Merge all into single Package
4. Handle installation structure
```

**Example**: conda-meta/*.json → multiple packages

---

### Pattern 4: Archive Extraction ⭐⭐⭐⭐ (3 ecosystems)

Extract archive files and parse metadata from contents.

**Ecosystems**: debian, alpine, rubygems

**Algorithm**:

```text
1. Extract archive (.deb, .apk, .gem)
2. Parse metadata from extracted contents
3. Merge with other metadata sources
4. Create Package
```

**Key Challenge**: Archive handling and extraction

---

### Pattern 5: Database-Based ⭐⭐⭐⭐ (1 ecosystem)

Parse system package databases directly.

**Ecosystems**: rpm

**Algorithm**:

```text
1. Parse database (NDB or SQLite)
2. Extract package metadata
3. Create Package for each entry
```

**Key Challenge**: Database format parsing

---

### Pattern 6: Multi-Format ⭐⭐⭐⭐ (2 ecosystems)

Support multiple metadata file formats and merge them.

**Ecosystems**: pypi, rubygems

**Algorithm**:

```text
1. Detect available metadata formats
2. Parse each format
3. Merge data from all formats
4. Create Package
```

**Key Challenge**: Format detection and conditional merging

---

## Implementation Checklist

### For Each Ecosystem

- [ ] Identify assembly pattern
- [ ] Find sibling/related files
- [ ] Parse primary manifest
- [ ] Parse secondary files (lockfile, metadata, etc.)
- [ ] Merge data (dependencies, metadata, etc.)
- [ ] Create Package with merged data
- [ ] Yield Package and Dependencies
- [ ] Assign package to resources
- [ ] Write golden tests
- [ ] Verify scope terminology preserved

---

## Key Concepts

### 1. Package Creation

- Created from primary manifest file
- Must have PURL (Package URL) to create Package
- If no PURL, only yield dependencies

### 2. Dependency Merging

- Manifest: version requirements
- Lockfile: pinned/resolved versions
- Merge both into single dependency list

### 3. Resource Assignment

- Assign package to all files in its tree
- Skip certain directories (e.g., node_modules for npm)
- Use parent tree assignment for some ecosystems

### 4. Scope Preservation

Each ecosystem has native scope terminology:

- npm: `dependencies`, `devDependencies`, `peerDependencies`, `optionalDependencies`
- cargo: `dependencies`, `dev-dependencies`, `build-dependencies`
- maven: `compile`, `test`, `provided`, `runtime`, `system`

**Important**: Preserve native terminology for semantic fidelity

### 5. Workspace Support

Only cargo supports workspaces:

- Multiple packages in single workspace
- Copy workspace-level metadata to members
- Handle glob patterns for member paths

---

## Common Pitfalls

1. **Forgetting lockfile-only case** - Some ecosystems can have lockfile without manifest
2. **Wrong merge order** - Maven requires pom.xml first, then MANIFEST.MF
3. **Not preserving scope** - Use native scope terminology, not normalized
4. **Missing edge cases** - Test with missing files, multiple files, etc.
5. **Resource assignment** - Assign package to correct files/directories
6. **Dependency deduplication** - Avoid duplicate dependencies when merging

---

## Testing Strategy

### Golden Tests

```bash
# Compare Rust output with Python ScanCode
python -m scancode --json-pp expected.json testdata/
cargo run -- testdata/ -o actual.json
diff expected.json actual.json
```

### Edge Cases

- [ ] Missing lockfiles
- [ ] Missing manifests
- [ ] Workspace configurations
- [ ] Nested directory structures
- [ ] Archive extraction
- [ ] Database parsing
- [ ] Multiple metadata formats

### Scope Tests

- [ ] Verify native scope terminology
- [ ] Test scope-specific handling
- [ ] Validate optional/runtime/dev distinctions

---

## Quick Links

- **Summary**: `docs/PYTHON_ASSEMBLERS_SUMMARY.md`
- **Detailed**: `docs/PYTHON_ASSEMBLERS_DETAILED.md`
- **Roadmap**: `docs/ASSEMBLY_PARITY_ROADMAP.md`
- **Python Code**: `reference/scancode-toolkit/src/packagedcode/`
- **Test Data**: `reference/scancode-toolkit/tests/packagedcode/data/`

---

## Implementation Order (Recommended)

### Phase 1: Sibling-Merge (2-3 weeks)

1. npm
2. cargo (with workspace)
3. golang
4. pubspec
5. cocoapods
6. phpcomposer
7. chef
8. conan

### Phase 2: Nested Sibling-Merge (1-2 weeks)

1. maven

### Phase 3: Directory-Based (2-3 weeks)

1. conda
2. alpine
3. debian

### Phase 4: Archive Extraction (3-4 weeks)

1. debian
2. alpine
3. rubygems

### Phase 5: Database-Based (2-3 weeks)

1. rpm

### Phase 6: Multi-Format (2-3 weeks)

1. pypi
2. rubygems

---

## Success Metrics

- [ ] All 20 ecosystems with assembly support
- [ ] 100% feature parity with Python ScanCode
- [ ] All golden tests passing
- [ ] Scope terminology preserved correctly
- [ ] Performance benchmarks showing improvement
