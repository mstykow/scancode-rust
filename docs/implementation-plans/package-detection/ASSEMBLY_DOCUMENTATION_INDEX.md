# Assembly Documentation Index

This directory contains comprehensive documentation about Python ScanCode's assembly logic and the roadmap for implementing it in Rust.

## Documents

### 1. **ASSEMBLY_QUICK_REFERENCE.md** ⭐ START HERE

**Purpose**: Quick overview of assembly concepts and patterns
**Audience**: Developers starting assembly implementation
**Contents**:

- What is assembly?
- 20 ecosystems with assembly support (table)
- 6 assembly patterns with examples
- Implementation checklist
- Common pitfalls
- Testing strategy
- Recommended implementation order

**Read Time**: 10-15 minutes

---

### 2. **PYTHON_ASSEMBLERS_SUMMARY.md**

**Purpose**: High-level overview of Python ScanCode's assembly framework
**Audience**: Architects and lead developers
**Contents**:

- Assembly framework overview (models.py)
- 20 ecosystems with assembly support (detailed descriptions)
- 6 assembly patterns with use cases
- Key assembly concepts
- Ecosystems WITHOUT assembly support (17 total)
- Implementation notes

**Read Time**: 20-30 minutes

---

### 3. **PYTHON_ASSEMBLERS_DETAILED.md**

**Purpose**: Complete reference for each ecosystem's assembly logic
**Audience**: Developers implementing specific ecosystems
**Contents**:

- Complete assembler list (table)
- Detailed assembly logic for each of 20 ecosystems
- Assembly helper methods
- Scope terminology by ecosystem
- Key implementation patterns
- Testing considerations

**Read Time**: 45-60 minutes (reference document)

---

### 4. **ASSEMBLY_PARITY_ROADMAP.md**

**Purpose**: Implementation roadmap and prioritization
**Audience**: Project managers and developers
**Contents**:

- Executive summary
- 6 assembly patterns with complexity ratings
- Implementation priority (6 phases)
- Ecosystems without assembly support
- Key implementation insights
- Testing strategy
- Architecture recommendations
- Success criteria

**Read Time**: 30-40 minutes

---

## Quick Navigation

### By Role

**Project Manager**:

1. ASSEMBLY_QUICK_REFERENCE.md (overview)
2. ASSEMBLY_PARITY_ROADMAP.md (phases and timeline)

**Architect**:

1. PYTHON_ASSEMBLERS_SUMMARY.md (framework overview)
2. ASSEMBLY_PARITY_ROADMAP.md (architecture recommendations)

**Developer (Starting)**:

1. ASSEMBLY_QUICK_REFERENCE.md (concepts)
2. PYTHON_ASSEMBLERS_SUMMARY.md (patterns)

**Developer (Implementing Specific Ecosystem)**:

1. ASSEMBLY_QUICK_REFERENCE.md (pattern identification)
2. PYTHON_ASSEMBLERS_DETAILED.md (specific ecosystem details)
3. Reference Python code: `reference/scancode-toolkit/src/packagedcode/<ecosystem>.py`

---

### By Task

**Understanding Assembly Concepts**:
→ ASSEMBLY_QUICK_REFERENCE.md

**Identifying Assembly Pattern for Ecosystem**:
→ ASSEMBLY_QUICK_REFERENCE.md (6 patterns section)

**Implementing Specific Ecosystem**:
→ PYTHON_ASSEMBLERS_DETAILED.md (find ecosystem section)

**Planning Implementation Timeline**:
→ ASSEMBLY_PARITY_ROADMAP.md (phases section)

**Understanding Dependency Scopes**:
→ PYTHON_ASSEMBLERS_DETAILED.md (scope terminology section)

**Testing Assembly Implementation**:
→ ASSEMBLY_PARITY_ROADMAP.md (testing strategy section)

---

## Key Statistics

- **Total Ecosystems**: 37
  - With Assembly: 20
  - Without Assembly: 17

- **Assembly Patterns**: 6
  - Sibling-Merge: 8 ecosystems (⭐⭐)
  - Nested Sibling-Merge: 1 ecosystem (⭐⭐⭐)
  - Directory-Based: 3 ecosystems (⭐⭐⭐)
  - Archive Extraction: 3 ecosystems (⭐⭐⭐⭐)
  - Database-Based: 1 ecosystem (⭐⭐⭐⭐)
  - Multi-Format: 2 ecosystems (⭐⭐⭐⭐)

- **Implementation Phases**: 6
  - Phase 1 (Sibling-Merge): 2-3 weeks, 8 ecosystems
  - Phase 2 (Nested): 1-2 weeks, 1 ecosystem
  - Phase 3 (Directory-Based): 2-3 weeks, 3 ecosystems
  - Phase 4 (Archive): 3-4 weeks, 3 ecosystems
  - Phase 5 (Database): 2-3 weeks, 1 ecosystem
  - Phase 6 (Multi-Format): 2-3 weeks, 2 ecosystems

---

## 20 Ecosystems with Assembly Support

| # | Ecosystem | Pattern | Complexity |
|---|-----------|---------|------------|
| 1 | npm | Sibling-merge | ⭐⭐ |
| 2 | cargo | Sibling-merge + workspace | ⭐⭐⭐ |
| 3 | maven | Nested sibling-merge | ⭐⭐⭐ |
| 4 | cocoapods | Sibling-merge | ⭐⭐ |
| 5 | phpcomposer | Sibling-merge | ⭐⭐ |
| 6 | rubygems | Multi-file + archive | ⭐⭐⭐⭐ |
| 7 | golang | Sibling-merge | ⭐⭐ |
| 8 | pubspec | Sibling-merge | ⭐⭐ |
| 9 | swift | Multi-file merge | ⭐⭐⭐ |
| 10 | conda | Directory-based | ⭐⭐⭐ |
| 11 | debian | Archive extraction | ⭐⭐⭐⭐ |
| 12 | alpine | Archive + database | ⭐⭐⭐⭐ |
| 13 | rpm | Database-based | ⭐⭐⭐⭐ |
| 14 | pypi | Multi-format | ⭐⭐⭐⭐ |
| 15 | chef | Sibling-merge | ⭐⭐ |
| 16 | conan | Sibling-merge | ⭐⭐ |
| 17 | debian_copyright | Standalone | ⭐ |
| 18 | about | Standalone | ⭐ |
| 19 | build | Non-assembling | ⭐ |

---

## 6 Assembly Patterns

### Pattern 1: Sibling-Merge (⭐⭐)

Find related files in same directory and merge them.
**Ecosystems**: npm, cargo, cocoapods, phpcomposer, golang, pubspec, chef, conan

### Pattern 2: Nested Sibling-Merge (⭐⭐⭐)

Find related files in nested directory structures.
**Ecosystems**: maven

### Pattern 3: Directory-Based (⭐⭐⭐)

Scan directory for multiple related files and merge them.
**Ecosystems**: conda, alpine, debian

### Pattern 4: Archive Extraction (⭐⭐⭐⭐)

Extract archive files and parse metadata from contents.
**Ecosystems**: debian, alpine, rubygems

### Pattern 5: Database-Based (⭐⭐⭐⭐)

Parse system package databases directly.
**Ecosystems**: rpm

### Pattern 6: Multi-Format (⭐⭐⭐⭐)

Support multiple metadata file formats and merge them.
**Ecosystems**: pypi, rubygems

---

## Implementation Phases

### Phase 1: Sibling-Merge (Highest Priority)

**Effort**: 2-3 weeks | **Impact**: 8 ecosystems

- npm, cargo, golang, pubspec, cocoapods, phpcomposer, chef, conan

### Phase 2: Nested Sibling-Merge (High Priority)

**Effort**: 1-2 weeks | **Impact**: 1 ecosystem

- maven

### Phase 3: Directory-Based (Medium Priority)

**Effort**: 2-3 weeks | **Impact**: 3 ecosystems

- conda, alpine, debian

### Phase 4: Archive Extraction (Lower Priority)

**Effort**: 3-4 weeks | **Impact**: 3 ecosystems

- debian, alpine, rubygems

### Phase 5: Database-Based (Lower Priority)

**Effort**: 2-3 weeks | **Impact**: 1 ecosystem

- rpm

### Phase 6: Multi-Format (Lower Priority)

**Effort**: 2-3 weeks | **Impact**: 2 ecosystems

- pypi, rubygems

---

## Key Concepts

### Assembly

Process of combining data from multiple related manifest/lockfile datafiles into a single Package with its dependencies.

### Package

Top-level package instance with UUID, created from primary manifest file.

### Dependency

Top-level dependency instance with UUID, extracted from manifest or lockfile.

### Scope

Native ecosystem terminology for dependency type (e.g., npm's `devDependencies`, cargo's `dev-dependencies`).

### PURL

Package URL - unique identifier for a package (required to create Package object).

### Sibling Files

Related files in same directory (e.g., package.json and package-lock.json).

### Workspace

Multiple packages in single repository (only cargo supports this).

---

## References

### Python Implementation

- **Main Code**: `reference/scancode-toolkit/src/packagedcode/`
- **Models**: `reference/scancode-toolkit/src/packagedcode/models.py`
- **Test Data**: `reference/scancode-toolkit/tests/packagedcode/data/`

### Rust Implementation

- **Parsers**: `src/parsers/`
- **Models**: `src/models/`
- **Tests**: `src/parsers/*_test.rs`

---

## Document Versions

- **Created**: February 10, 2026
- **Last Updated**: February 10, 2026
- **Status**: Complete reference for Python ScanCode assembly logic

---

## Next Steps

1. **Read ASSEMBLY_QUICK_REFERENCE.md** for overview
2. **Identify target ecosystem** from 20 supported
3. **Find assembly pattern** in ASSEMBLY_QUICK_REFERENCE.md
4. **Review detailed implementation** in PYTHON_ASSEMBLERS_DETAILED.md
5. **Check Python code** in reference/scancode-toolkit/src/packagedcode/
6. **Implement in Rust** following the pattern
7. **Write golden tests** comparing with Python output
8. **Verify scope terminology** is preserved

---

## Questions?

Refer to the specific document for your question:

- **What is assembly?** → ASSEMBLY_QUICK_REFERENCE.md
- **How does [ecosystem] work?** → PYTHON_ASSEMBLERS_DETAILED.md
- **What's the implementation plan?** → ASSEMBLY_PARITY_ROADMAP.md
- **What patterns exist?** → ASSEMBLY_QUICK_REFERENCE.md or PYTHON_ASSEMBLERS_SUMMARY.md
