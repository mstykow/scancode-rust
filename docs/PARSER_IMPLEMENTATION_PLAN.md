# Parser Implementation Plan: Achieving Full Feature Parity with ScanCode Toolkit

> **Status**: Active Development  
> **Last Updated**: 2025-02-05  
> **Current Progress**: 4/40+ ecosystems (10% coverage)  
> **Recent Work**: Comprehensive parser bug fixes and feature enhancements (Feb 2025)

## Executive Summary

This document outlines the comprehensive plan to achieve 100% parser feature parity between scancode-rust and ScanCode Toolkit. Currently, scancode-rust supports **12 package formats across 4 ecosystems** (npm, Python, Rust, Maven). ScanCode Toolkit supports **136+ formats across 40+ ecosystems**.

**Goal**: Implement all remaining parsers while maintaining Rust's safety, performance, and code quality advantages over the original Python implementation.

---

## Recent Improvements (February 2025)

### Phase 0.5: Parser Quality & Feature Enhancement (✅ Complete)

**Duration**: February 2025  
**Commits**: 17 new commits across 3 waves  
**Test Coverage**: 317 passing tests (up from 305)

#### Wave 1: Safety & Robustness (3 commits)
- **Eliminated panic-prone code**: Replaced all `.unwrap()` and `.expect()` calls with graceful error handling
- **Impact**: 11 potential panic points → graceful fallback with warning logs
- **Philosophy**: Match ScanCode Toolkit behavior ("extract what you can, log what you can't, continue scanning")

**Files Modified**:
- `src/parsers/pnpm_lock.rs` - 3 unwrap() calls removed
- `src/parsers/npm.rs` - 2 expect() calls removed
- `src/parsers/cargo.rs` - 4 expect() calls removed
- `src/parsers/python.rs` - 2 expect() calls removed (bundled with cargo.rs commit)

#### Wave 2: Data Consistency & Accuracy (9 commits)

**1. Namespace Standardization** (1 commit)
- **Problem**: Inconsistent handling of scoped package namespaces (@org vs org)
- **Solution**: Standardized all parsers to use `@org` format
- **Impact**: npm and pnpm parsers now consistently return `@babel` instead of `babel`

**2. Direct Dependency Detection** (3 commits)
- **Problem**: `is_direct` field hardcoded to `true` instead of analyzing dependency graph
- **Solution**: Implemented proper detection for 7 parsers
  - npm_lock: Track nesting depth (root level = direct)
  - pnpm_lock: Extract from `importers` section
  - yarn_lock: v1 limitation documented, v2+ workspace detection
  - poetry_lock: Match against pyproject.toml dependencies
  - pipfile_lock: All are direct (by design)
  - maven: All pom.xml dependencies are direct
  - python: All manifest dependencies are direct
- **Impact**: Accurate direct vs transitive dependency tracking

**3. Version Pinning Analysis** (2 commits)
- **Problem**: `is_pinned` field hardcoded instead of analyzing version specifiers
- **Solution**: Implemented version string analysis for 2 parsers
  - cargo: `"1.0.0"` = pinned, `"^1.0.0"` = not pinned (semver analysis)
  - maven: `"1.0.0"` = pinned, `"[1.0,2.0)"` = not pinned (range detection)
- **Impact**: Accurate pinning status for dependency resolution

**4. Hash Extraction Enhancement** (3 commits)
- **Problem**: Parsers only extracted 1-2 hash types instead of all available
- **Solution**: Enhanced 5 parsers to extract all 4 hash types (sha1, sha256, sha512, md5)
  - npm_lock: Added sha1, md5 extraction
  - npm: Added sha1, md5 from dist metadata
  - pnpm_lock: Added sha256, md5 fields to ResolvedPackage
  - poetry_lock: Extract sha256 from files array
  - yarn_lock: Updated struct for new hash fields
- **Impact**: Comprehensive integrity verification data

#### Wave 3: Advanced Features (5 commits)

**1. Archive Safety Checks** (1 commit)
- **Problem**: Python wheel/egg extraction vulnerable to zip bombs and DoS
- **Solution**: Added 4-level safety checks
  - Archive size limit: 100MB
  - Per-file size limit: 50MB
  - Compression ratio validation: 100:1 max
  - Total extracted size tracking
- **Impact**: Protection against malicious archives

**2. License Declaration Extraction** (4 commits)
- **Problem**: License information from manifests not extracted or normalized
- **Solution**: Integrated askalono for license normalization across 4 parsers
  - npm: Extract from package.json "license" field
  - cargo: Extract from Cargo.toml "license" field
  - python: Extract from pyproject.toml, setup.py classifiers, PKG-INFO
  - maven: Extract from pom.xml `<licenses>` section
- **Features**:
  - Raw license string preserved in `extracted_license_statement`
  - Normalized to SPDX with askalono (confidence ≥ 0.8)
  - Stored in `declared_license_expression` and `declared_license_expression_spdx`
  - Graceful fallback to raw strings when confidence is low
- **Impact**: Standardized, machine-readable license data across all parsers

#### Summary of Improvements

| Category | Metric | Before | After |
|----------|--------|--------|-------|
| **Safety** | Panic-prone calls | 11 | 0 |
| **Tests** | Passing tests | 305 | 317 |
| **Data Quality** | Parsers with `is_direct` detection | 0 | 7 |
| **Data Quality** | Parsers with `is_pinned` analysis | 0 | 2 |
| **Data Quality** | Hash types extracted | 1-2 | 4 (all) |
| **Features** | Parsers with license extraction | 0 | 4 |
| **Security** | Archive safety checks | No | Yes |
| **Commits** | Total feature commits | - | 17 |

---

## Table of Contents

1. [Guiding Principles](#guiding-principles)
2. [Current State](#current-state)
3. [Architectural Decisions](#architectural-decisions)
4. [Implementation Phases](#implementation-phases)
5. [Ecosystem Reference](#ecosystem-reference)
6. [Development Guidelines](#development-guidelines)
7. [Testing Strategy](#testing-strategy)
8. [Quality Gates](#quality-gates)
9. [Known Issues to Avoid](#known-issues-to-avoid)

---

## Guiding Principles

### 1. **Critical Analysis, Not Blind Translation**

- ⚠️ **DO NOT** blindly rewrite Python code in Rust
- ✅ **DO** analyze the Python implementation to understand:
  - What problem is being solved?
  - Why was this approach chosen?
  - Are there better ways to solve this in Rust?
  - What bugs or limitations exist in the original?

### 2. **Reference, Don't Replicate**

Use `reference/scancode-toolkit/` as:
- ✅ **Source of truth** for expected behavior and output formats
- ✅ **Documentation** for understanding format specifications
- ✅ **Test oracle** for verifying correctness
- ❌ **NOT** a template for implementation approach

### 3. **Rust-First Design**

Leverage Rust's strengths:
- **Type Safety**: Use strong types instead of runtime validation
- **Error Handling**: `Result<T, E>` instead of exception-based control flow
- **Ownership**: Avoid cloning where possible, use references
- **Iterators**: Lazy evaluation instead of eager list building
- **Pattern Matching**: Exhaustive matching instead of defensive checks

### 4. **Security & Robustness**

- **No Code Execution**: Never execute user-provided code (e.g., setup.py)
- **DoS Protection**: Implement limits on recursion, file size, memory
- **Input Validation**: Validate all external input with clear error messages
- **Fail Gracefully**: Parse errors should not crash the scanner

### 5. **Performance Optimization**

- **Lazy Parsing**: Only parse what's needed
- **Streaming**: Process large files without loading entirely into memory
- **Parallel Processing**: Use rayon for independent operations
- **Zero-Copy**: Use `&str` and `Cow` where appropriate

---

## Current State

### ✅ Implemented Ecosystems (4 ecosystems, 12 formats)

| Ecosystem | Formats | Status | Notes |
|-----------|---------|--------|-------|
| **npm** | 5 | ✅ Complete | package.json, package-lock.json, yarn.lock (v1/v2), pnpm-lock.yaml, pnpm-workspace.yaml |
| **Python** | 4 parsers, 11 formats | ✅ Complete | pyproject.toml, setup.py (AST), setup.cfg, PKG-INFO, METADATA, poetry.lock, Pipfile/Pipfile.lock, requirements.txt, .whl, .egg |
| **Rust** | 2 | ✅ Complete | Cargo.toml, Cargo.lock |
| **Maven** | 4 | ✅ Complete | pom.xml, pom.properties, MANIFEST.MF, .pom archives |

**Test Coverage**: 317 passing tests, golden test infrastructure in place.

### Recent Quality Improvements

- ✅ **Safety**: Zero `.unwrap()` or `.expect()` calls in parser code
- ✅ **Data Accuracy**: Proper `is_direct` and `is_pinned` analysis
- ✅ **Hash Extraction**: All 4 hash types (sha1, sha256, sha512, md5) extracted
- ✅ **License Detection**: SPDX normalization via askalono integration
- ✅ **Archive Security**: Size limits and compression ratio validation
- ✅ **Error Handling**: Graceful fallback with warning logs

---

## Architectural Decisions

### Core Architecture (Established)

#### 1. **Trait-Based Parser System**

```rust
pub trait PackageParser {
    const PACKAGE_TYPE: &'static str;
    
    fn is_match(path: &Path) -> bool;
    fn extract_package_data(path: &Path) -> PackageData;
}
```

**Rationale**: 
- Type-safe dispatch
- Compile-time guarantees
- Easy to test in isolation
- Clear contract for implementers

**Python Contrast**: Python uses runtime class inspection and dynamic dispatch.

#### 2. **Unified Data Model**

All parsers output `PackageData` struct:
- Normalizes differences between ecosystems
- SBOM-compliant output format
- Single source of truth for data structure

**Key Fields**:
- Package identity: name, version, namespace
- Metadata: description, homepage_url, parties
- Dependencies: with scope, requirements, resolved packages
- Licenses: detection with confidence scores, declared licenses with SPDX normalization
- Checksums: SHA256, SHA1, MD5, SHA512 for archives
- URLs: repository, download, API endpoints

#### 3. **Auto-Generated Documentation**

Using `inventory` crate + `register_parser!` macro:
- Runtime metadata collection
- Type-safe registration
- Automatic doc generation from code
- Pre-commit hook keeps docs in sync

**Rationale**: Documentation that can't go stale.

#### 4. **Golden Test Infrastructure**

- Reference outputs from ScanCode Toolkit
- Automated comparison
- Format-agnostic (JSON diffs)
- Regression prevention

#### 5. **Security-First Parsing**

- **AST-based parsing** for code files (setup.py, build.gradle)
- **No eval/exec**: Never execute user code
- **DoS limits**: Max file size, recursion depth, iteration count
- **Archive safety**: Size limits, compression ratio validation
- **Circular dependency detection**: Prevent infinite loops

#### 6. **Askalono Integration for License Normalization**

- **SPDX License Database**: ~600 licenses embedded at compile time
- **N-gram Analysis**: Fuzzy matching for license text
- **Confidence Threshold**: 0.8 (80%) for normalization
- **Graceful Fallback**: Use raw strings when confidence is low
- **Shared Utility**: `normalize_license()` function in `parsers/utils.rs`

**Rationale**: Standardized, machine-readable license data across all package formats.

---

## Implementation Phases

### Phase 0: Infrastructure (✅ Complete)

- [x] Core data models (PackageData, Dependency, etc.)
- [x] Trait system (PackageParser)
- [x] Golden test framework
- [x] Auto-documentation system
- [x] Pre-commit hooks (fmt, clippy, doc generation)

### Phase 0.5: Parser Quality Enhancement (✅ Complete - Feb 2025)

- [x] Safety: Remove all unwrap/expect calls (Wave 1)
- [x] Data Consistency: is_direct, is_pinned, namespace, hashes (Wave 2)
- [x] Advanced Features: Archive safety, license extraction (Wave 3)
- [x] Test Coverage: 305 → 317 tests
- [x] Askalono Integration: License normalization across 4 parsers

### Phase 1: Top-Tier Modern Ecosystems (HIGH PRIORITY)

**Goal**: Cover 80% of real-world usage  
**Complexity**: Varies by parser (see breakdown below)
**Target**: +30 formats across 5 ecosystems

#### 1.1 Ruby / RubyGems (HIGH IMPACT)

**Priority**: ⭐⭐⭐⭐⭐ (Very High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| Gemfile | Medium | Low-Medium |
| Gemfile.lock | Medium | Low-Medium |
| .gemspec | Medium | Low-Medium |
| .gem archives | High | Medium-High |



**Key Challenges**:
- Gemfile uses Ruby DSL (requires custom parser or leverage Ruby AST)
- .gem archives are tar.gz with specific structure
- Version constraints use custom syntax

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/rubygems.py`
- `reference/scancode-toolkit/src/packagedcode/gemfile_lock.py`

**Implementation Notes**:
- **DO NOT** execute Ruby code from Gemfile
- Use parser combinator (nom) or custom lexer for Gemfile DSL
- Study `gemfile_lock.py` for lockfile format
- .gemspec files are Ruby code - use AST parsing similar to setup.py approach

**Known Python Issues to Avoid**:
- Ruby execution in original (security risk)
- Poor error messages for malformed files
- No validation of version constraints

---

#### 1.2 Go (HIGH IMPACT)

**Priority**: ⭐⭐⭐⭐⭐ (Very High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| go.mod | Low | Low |
| go.sum | Low | Low |
| Godeps/Godeps.json | Medium | Low-Medium |
| vendor/vendor.json | Low | Low |
| glide.yaml | Low | Low |
| glide.lock | Low | Low |



**Key Challenges**:
- go.mod has custom format (not JSON/YAML/TOML)
- Multiple legacy formats (Godeps, glide, vendor)
- Module path resolution can be complex

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/go_mod.py`
- `reference/scancode-toolkit/src/packagedcode/godeps.py`
- `reference/scancode-toolkit/src/packagedcode/golang.py`

**Implementation Notes**:
- go.mod parser: use nom for custom syntax
- go.sum is simple: `<module> <version> <hash>` per line
- Godeps.json is straightforward JSON
- Consider using `serde` for JSON formats

**Known Python Issues to Avoid**:
- Regex-based parsing in original (fragile)
- Poor handling of replace directives in go.mod
- No validation of semantic versions

---

#### 1.3 PHP / Composer (HIGH IMPACT)

**Priority**: ⭐⭐⭐⭐⭐ (Very High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| composer.json | Low | Low |
| composer.lock | Medium | Low-Medium |



**Key Challenges**:
- composer.lock has nested dependency resolution
- Version constraints use custom syntax
- PSR-4 autoloading information

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/phpcomposer.py`

**Implementation Notes**:
- Both files are JSON (easy with serde_json)
- Focus on dependency graph extraction
- Parse autoload sections for code structure

**Known Python Issues to Avoid**:
- Incomplete autoload parsing
- Missing support for composer.json "extra" fields
- No validation of version constraints

---

#### 1.4 .NET / NuGet (HIGH IMPACT)

**Priority**: ⭐⭐⭐⭐⭐ (Very High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| packages.config | Low | Low |
| .nuspec | Medium | Low-Medium |
| packages.lock.json | Medium | Low-Medium |
| project.assets.json | High | Medium-High |
| .nupkg archives | High | Medium-High |



**Key Challenges**:
- Multiple format generations (.config vs .json)
- .nuspec is XML with NuGet-specific schema
- .nupkg are ZIP archives with metadata
- Complex dependency resolution in project.assets.json

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/nuget.py`

**Implementation Notes**:
- Use `quick-xml` for .nuspec parsing
- Use `zip` crate for .nupkg archives
- packages.lock.json is straightforward JSON
- project.assets.json has deeply nested structure

**Known Python Issues to Avoid**:
- Incomplete .nuspec parsing (missing metadata fields)
- No support for FrameworkReference elements
- Poor error handling for corrupted archives

---

#### 1.5 Dart / Flutter / Pub (HIGH IMPACT)

**Priority**: ⭐⭐⭐⭐ (High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| pubspec.yaml | Low | Low |
| pubspec.lock | Low | Low |



**Key Challenges**:
- YAML parsing with custom pub schema
- Flutter-specific dependencies

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/pubspec.py`

**Implementation Notes**:
- Both files are YAML (use serde_yaml)
- Simple structure, straightforward parsing
- Watch for Flutter SDK dependencies vs package dependencies

**Known Python Issues to Avoid**:
- Missing support for pub workspace format
- Incomplete handling of git dependencies

---

### Phase 2: Mobile & Build Systems (MEDIUM PRIORITY)

**Goal**: Cover mobile development ecosystems  
**Complexity**: Varies by parser (see breakdown below)
**Target**: +20 formats across 4 ecosystems

#### 2.1 Android / Gradle

**Priority**: ⭐⭐⭐⭐ (High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| build.gradle | Very High | Very High |
| build.gradle.kts (Kotlin) | Very High | Very High |
| gradle.lockfile | Low | Low |
| AndroidManifest.xml | Medium | Medium |
| .apk archives | High | High |
| .aar archives | High | Medium |



**Key Challenges**:
- Gradle files are Groovy/Kotlin DSL (requires AST parsing)
- AndroidManifest.xml has complex schema
- .apk/.aar are ZIP archives with multiple metadata sources

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/build_gradle.py`
- `reference/scancode-toolkit/src/packagedcode/jar_manifest.py`

**Implementation Notes**:
- **CRITICAL**: Do NOT execute Groovy/Kotlin code
- Consider using tree-sitter for Groovy/Kotlin AST parsing
- APK parsing requires ZIP + XML (AndroidManifest.xml) + binary parsing (resources.arsc)
- Focus on dependency declarations initially

**Known Python Issues to Avoid**:
- Python version attempts limited Groovy execution (security risk)
- Regex-based parsing (fragile and incomplete)
- Missing support for Kotlin DSL
- No handling of composite builds

---

#### 2.2 Swift / SwiftPM

**Priority**: ⭐⭐⭐⭐ (High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| Package.swift | High | High |
| Package.resolved | Low | Low |
| Package.swift.json (dump) | Low | Low |



**Key Challenges**:
- Package.swift is Swift code (DSL)
- Can leverage `swift package dump-package` output (JSON)

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/swift.py`

**Implementation Notes**:
- Parse Package.resolved (JSON) first - easiest
- For Package.swift: use AST parsing (tree-sitter-swift) or rely on JSON dump
- **DO NOT** execute Swift code

**Known Python Issues to Avoid**:
- Relies on Swift compiler being installed
- No fallback when Swift not available

---

#### 2.3 CocoaPods (iOS/macOS)

**Priority**: ⭐⭐⭐⭐ (High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| Podfile | High | Medium |
| Podfile.lock | Medium | Medium |
| .podspec | High | Medium |
| .podspec.json | Low | Low |



**Key Challenges**:
- Podfile is Ruby DSL
- .podspec is Ruby code
- Complex dependency resolution

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/cocoapods.py`

**Implementation Notes**:
- Similar challenges to Gemfile (Ruby DSL)
- .podspec.json is easier (JSON format)
- Podfile.lock is YAML-like format

**Known Python Issues to Avoid**:
- Ruby execution for Podfile/podspec (security risk)
- Incomplete parsing of dependency constraints

---

#### 2.4 Bower (Legacy JavaScript)

**Priority**: ⭐⭐ (Low - legacy)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| bower.json | Low | Low |



**Key Challenges**:
- Simple JSON format
- Mostly deprecated in favor of npm

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/bower.py`

**Implementation Notes**:
- Straightforward JSON parsing with serde
- Similar structure to package.json
- Low priority due to ecosystem decline

---

### Phase 3: Scientific & Specialized (MEDIUM PRIORITY)

**Goal**: Cover scientific computing and specialized ecosystems  
**Complexity**: Varies by parser (see breakdown below)
**Target**: +15 formats across 4 ecosystems

#### 3.1 Conda (Python/R Scientific)

**Priority**: ⭐⭐⭐⭐ (High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| meta.yaml | Medium | Medium |
| environment.yml | Low | Low |
| conda.yaml | Low | Low |



**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/conda.py`

**Implementation Notes**:
- All YAML formats (use serde_yaml)
- meta.yaml has Jinja2 templating (consider supporting or skip templated values)
- Focus on package and dependency information

**Known Python Issues to Avoid**:
- Python version attempts Jinja2 template evaluation (complex, error-prone)
- Missing validation of version selectors

---

#### 3.2 CRAN (R Language)

**Priority**: ⭐⭐⭐ (Medium)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| DESCRIPTION | Low | Low |



**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/cran.py`

**Implementation Notes**:
- Debian control file format (key: value pairs)
- Similar to PKG-INFO parsing we already have
- Reuse RFC822 metadata parser pattern

---

#### 3.3 Conan (C++)

**Priority**: ⭐⭐⭐ (Medium)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| conanfile.py | Very High | Very High |
| conanfile.txt | Low | Low |
| conan.lock | Medium | Medium |



**Key Challenges**:
- conanfile.py is Python code (requires AST parsing)
- Custom recipe format

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/conan.py`

**Implementation Notes**:
- **DO NOT** execute conanfile.py
- Use Python AST parsing (similar to setup.py approach)
- conanfile.txt is INI-like format
- conan.lock is JSON

**Known Python Issues to Avoid**:
- Python execution of conanfile.py (security risk)
- Incomplete AST extraction

---

#### 3.4 Haxe

**Priority**: ⭐⭐ (Low - niche)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| haxelib.json | Low | Low |



**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/haxe.py`

**Implementation Notes**:
- Simple JSON format
- Straightforward with serde_json

---

#### 3.5 OCaml / OPAM

**Priority**: ⭐⭐ (Low - niche)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| opam files | Medium | Medium |



**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/opam.py`

**Implementation Notes**:
- Custom format (key-value with S-expressions)
- Requires custom parser (nom)

---

### Phase 4: Linux Distribution Packages (MEDIUM PRIORITY)

**Goal**: Cover major Linux package managers  
**Complexity**: Varies by parser (see breakdown below)
**Target**: +20 formats across 3 ecosystems

#### 4.1 Debian / Ubuntu (.deb)

**Priority**: ⭐⭐⭐⭐ (High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| debian/control (source) | Low | Low |
| control (binary) | Low | Low |
| debian/copyright | High | High |
| .deb archives | High | High |
| dpkg status database | Medium | Medium |



**Key Challenges**:
- .deb archives are ar archives containing tar.gz archives
- debian/copyright uses machine-readable format (DEP-5)
- dpkg database is multi-record format

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/debian.py`
- `reference/scancode-toolkit/src/packagedcode/debian_copyright.py`

**Implementation Notes**:
- Control files use RFC822-like format
- .deb parsing requires ar + tar.gz extraction
- DEP-5 copyright format has specific schema

**Known Python Issues to Avoid**:
- Incomplete copyright parsing
- Poor handling of multiline fields
- No validation of versioned dependencies

---

#### 4.2 RPM / RedHat / Fedora

**Priority**: ⭐⭐⭐⭐ (High)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| .rpm archives | Very High | Very High |
| .spec files | High | High |
| RPM database (BDB) | Very High | Very High |
| RPM database (SQLite) | High | Medium |



**Key Challenges**:
- RPM format is complex binary format (cpio + header)
- .spec files are custom DSL with macros
- Multiple database formats (BDB, NDB, SQLite)

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/rpm.py`
- `reference/scancode-toolkit/src/packagedcode/spec.py`
- `reference/scancode-toolkit/src/packagedcode/pyrpm.py`

**Implementation Notes**:
- Consider using `rpm` crate if available, or parse manually
- .spec parser requires macro expansion (complex)
- RPM header parsing is well-documented but intricate

**Known Python Issues to Avoid**:
- Heavy reliance on external rpm tools
- Incomplete .spec parsing (macro expansion)
- Database format support is scattered

---

#### 4.3 Alpine (.apk)

**Priority**: ⭐⭐⭐ (Medium)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| APKBUILD | High | High |
| .apk archives | High | High |
| installed database | Medium | Medium |



**Key Challenges**:
- APKBUILD is shell script (requires bash parsing)
- .apk archives are tar.gz with specific structure

**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/alpine.py`

**Implementation Notes**:
- **DO NOT** execute APKBUILD shell scripts
- Use AST parsing for shell scripts (tree-sitter-bash)
- .apk format is simpler than .rpm

**Known Python Issues to Avoid**:
- Shell execution of APKBUILD (security risk)
- Regex-based parsing (fragile)

---

#### 4.4 FreeBSD

**Priority**: ⭐⭐ (Low - specialized)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| Package manifests | Medium | Medium |



**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/freebsd.py`

---

### Phase 5: Legacy & Specialized (LOW PRIORITY)

**Goal**: Comprehensive coverage of niche ecosystems  
**Complexity**: Varies by parser (see breakdown below)
**Target**: +40 formats across 20+ ecosystems

#### 5.1 Chef (DevOps)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| metadata.rb | High | Medium |
| metadata.json | Low | Low |
| Berksfile | High | Medium |
| Berksfile.lock | Low | Low |



**Reference Files**:
- `reference/scancode-toolkit/src/packagedcode/chef.py`

---

#### 5.2 CPAN (Perl)

| Format | Complexity | Complexity |
|--------|------------|------------------|
| META.json | Low | Low |
| META.yml | Low | Low |
| Makefile.PL | Very High | Very High |
| dist.ini | Medium | Medium |
| MANIFEST | Low | Low |



**Reference Files**: TBD (not in current reference)

---

#### 5.3 Build Systems & Misc

| Ecosystem | Formats | Priority | Effort |
|-----------|---------|----------|--------|
| Bazel | BUILD files | ⭐⭐ | High |
| Buck | BUCK files, metadata | ⭐ | High |
| Autotools | configure.ac, configure | ⭐ | Medium |
| Ivy (Java) | ivy.xml | ⭐⭐ | Medium |
| Meteor | package.js | ⭐ | Medium |

---

#### 5.4 Binary Formats & Archives

| Format | Complexity | Complexity |
|--------|------------|------------------|
| MSI (Windows) | Very High | Very High |
| Windows PE (.exe/.dll) | Very High | Very High |
| JAR (Java) | Medium | Medium |
| WAR (Java) | Medium | Medium |
| EAR (Java) | Medium | Medium |
| ISO images | High | High |
| DMG (macOS) | High | High |

**Note**: These require binary format parsing and are lower priority due to specialized use cases.

---

#### 5.5 AboutCode & Metadata

| Format | Complexity | Complexity |
|--------|------------|------------------|
| .ABOUT files | Low | Low |
| README parsers | Medium | Medium |

---

## Development Guidelines

### For Each New Parser Implementation

#### Step 1: Research & Analysis

1. **Understand the Ecosystem**
   - Read official documentation
   - Study format specifications
   - Identify common use cases

2. **Analyze Python Reference**
   ```bash
   cd reference/scancode-toolkit
   grep -r "class.*Handler" src/packagedcode/<ecosystem>.py
   ```
   - What does the Python code do?
   - Why does it make these choices?
   - What are the edge cases?
   - What tests exist?

3. **Identify Issues**
   - Security vulnerabilities (code execution)
   - Performance bottlenecks (O(n²) algorithms, excessive cloning)
   - Bugs (incorrect parsing, missing fields)
   - Missing features (incomplete format support)

4. **Design Improvements**
   - How can we do this better in Rust?
   - What types make this safer?
   - Where can we use iterators instead of vectors?
   - Can we parse streaming instead of loading entire file?

#### Step 2: Create Test Data

1. **Collect Real-World Examples**
   - Find popular open-source projects using this format
   - Include edge cases (empty files, unusual syntax, large files)
   - Add malformed examples for error handling

2. **Generate Golden Outputs**
   ```bash
   scancode -p <testfile> --json <output.json>
   ```

3. **Document Test Coverage**
   - What scenarios are tested?
   - What edge cases are covered?
   - What's intentionally not supported?

#### Step 3: Implement Parser

1. **Create Parser File**
   ```
   src/parsers/<ecosystem>.rs
   ```

2. **Implement PackageParser Trait**
   ```rust
   pub struct MyParser;
   
   impl PackageParser for MyParser {
       const PACKAGE_TYPE: &'static str = "ecosystem-name";
       
       fn is_match(path: &Path) -> bool {
           // File pattern matching
       }
       
       fn extract_package_data(path: &Path) -> PackageData {
           // Parsing logic
       }
   }
   ```

3. **Register Metadata**
   ```rust
   crate::register_parser!(
       "Description",
       &["**/pattern.json"],
       "package-type",
       "Language",
       Some("https://docs.url"),
   );
   ```

4. **Add Module Declaration**
   ```rust
   // In src/parsers/mod.rs
   pub mod my_parser;
   ```

#### Step 4: Write Tests

1. **Unit Tests**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_is_match() { /* ... */ }
       
       #[test]
       fn test_extract_basic() { /* ... */ }
       
       #[test]
       fn test_extract_complex() { /* ... */ }
       
       #[test]
       fn test_malformed_input() { /* ... */ }
   }
   ```

2. **Golden Tests**
   ```
   testdata/<ecosystem>/
     ├── simple/
     │   ├── manifest.json
     │   └── expected.json
     └── complex/
         ├── manifest.json
         └── expected.json
   ```

3. **Test Coverage Goals**
   - ✅ Basic functionality
   - ✅ Edge cases (empty, minimal, maximal)
   - ✅ Error handling (malformed, missing fields)
   - ✅ Performance (large files)
   - ✅ Security (malicious input)

#### Step 5: Documentation

1. **Module-Level Documentation**
   ```rust
   //! Parser for <ecosystem> package manifests.
   //!
   //! Supports the following formats:
   //! - Format 1: description
   //! - Format 2: description
   //!
   //! # Key Features
   //! - Feature 1
   //! - Feature 2
   //!
   //! # Implementation Notes
   //! - Note 1
   //! - Note 2
   ```

2. **Public API Documentation**
   ```rust
   /// Parser for <ecosystem> package manifests.
   ///
   /// # Examples
   ///
   /// ```no_run
   /// use scancode_rust::parsers::my_parser::MyParser;
   /// // ...
   /// ```
   ```

3. **Update SUPPORTED_FORMATS.md**
   ```bash
   cargo run --bin generate-supported-formats
   ```

#### Step 6: Review & Quality Gates

1. **Self-Review Checklist**
   - [ ] All tests pass (`cargo test`)
   - [ ] No clippy warnings (`cargo clippy`)
   - [ ] Code formatted (`cargo fmt`)
   - [ ] Documentation complete (`cargo doc`)
   - [ ] No unsafe code (unless absolutely necessary and documented)
   - [ ] No unwrap/expect in library code
   - [ ] Error messages are clear and actionable
   - [ ] Performance tested with large files

2. **Security Checklist**
   - [ ] No code execution (eval, exec, subprocess)
   - [ ] Input validation (file size, recursion depth)
   - [ ] DoS protection (limits on iterations, memory)
   - [ ] Safe parsing (no panic on malformed input)

3. **Comparison with Python**
   - [ ] Feature parity achieved
   - [ ] Known bugs NOT replicated
   - [ ] Performance improvements documented
   - [ ] Edge cases handled better

---

## Testing Strategy

### Test Pyramid

```
       /\
      /  \    Integration Tests (Golden Tests)
     /    \   - Compare output with ScanCode Toolkit
    /------\  - End-to-end format validation
   /        \
  /   Unit   \ Unit Tests
 /   Tests    \ - Parser functions
/______________\ - Helper utilities
```

### Golden Test Workflow

1. **Generate Reference Output**
   ```bash
   cd reference/scancode-toolkit
   scancode -p testdata/npm/package.json --json expected.json
   ```

2. **Run Rust Parser**
   ```rust
   #[test]
   fn test_golden_npm_simple() {
       let result = NpmParser::extract_package_data(Path::new("testdata/npm/package.json"));
       let expected = read_expected_json("testdata/npm/expected.json");
       assert_eq_ignore_order(result, expected);
   }
   ```

3. **Continuous Validation**
   - Pre-commit hook runs golden tests
   - CI runs full test suite
   - Regression detection automatic

### Performance Testing

For each parser:
- Benchmark with `criterion` crate
- Test with 1KB, 100KB, 10MB files
- Memory profiling with `valgrind`/`heaptrack`
- Ensure O(n) or better complexity

---

## Quality Gates

### Before Merging Parser Implementation

1. **Functionality**
   - [ ] All documented formats supported
   - [ ] Feature parity with ScanCode Toolkit
   - [ ] Edge cases handled gracefully

2. **Code Quality**
   - [ ] Zero clippy warnings
   - [ ] Code formatted with rustfmt
   - [ ] Documentation complete
   - [ ] No TODO/FIXME without GitHub issue

3. **Testing**
   - [ ] Unit test coverage >90%
   - [ ] Golden tests pass
   - [ ] Performance benchmarks acceptable
   - [ ] Security tests pass

4. **Security**
   - [ ] No code execution
   - [ ] Input validation present
   - [ ] DoS protection implemented
   - [ ] Security review documented

5. **Documentation**
   - [ ] Public API documented
   - [ ] Module-level documentation present
   - [ ] SUPPORTED_FORMATS.md updated
   - [ ] Known limitations documented
   - [ ] Migration notes (if applicable)

---

## Known Issues to Avoid (from Python Reference)

### Security Issues

1. **Code Execution**
   - ❌ Python's `eval()` / `exec()` in setup.py parsing
   - ❌ Ruby execution in Gemfile/Podfile parsing
   - ❌ Shell execution in APKBUILD parsing
   - ❌ Groovy execution in Gradle parsing
   - ✅ Solution: AST parsing only, never execute

2. **DoS Vulnerabilities**
   - ❌ No recursion depth limits
   - ❌ No file size limits
   - ❌ Quadratic complexity in nested loops
   - ✅ Solution: Explicit limits, O(n) algorithms

3. **Path Traversal**
   - ❌ Inadequate path sanitization in archive extraction
   - ✅ Solution: Strict path validation, use `Path::canonicalize()`

### Correctness Issues

1. **Incomplete Parsing**
   - ❌ Missing fields in .nuspec files
   - ❌ Partial support for PEP 508 markers
   - ❌ Incomplete go.mod directive support
   - ✅ Solution: Comprehensive format coverage, exhaustive testing

2. **Version Handling**
   - ❌ No validation of semantic versions
   - ❌ Incorrect range parsing (e.g., `^1.2.3`)
   - ✅ Solution: Use `semver` crate, validate all versions

3. **Encoding Issues**
   - ❌ Assumes UTF-8 everywhere
   - ❌ Poor handling of BOM markers
   - ✅ Solution: Explicit encoding detection/handling

### Performance Issues

1. **Memory Usage**
   - ❌ Loading entire large files into memory
   - ❌ Excessive cloning of strings
   - ✅ Solution: Streaming parsers, use `&str` and `Cow`

2. **Algorithmic Complexity**
   - ❌ O(n²) nested loops in dependency resolution
   - ❌ Repeated regex compilation
   - ✅ Solution: Use iterators, compile regex once with `lazy_static`

3. **Unnecessary Work**
   - ❌ Parsing entire file when only subset needed
   - ❌ Computing unused fields
   - ✅ Solution: Lazy evaluation, parse on demand

### Design Issues

1. **Error Handling**
   - ❌ Silent failures (returns empty result)
   - ❌ Generic error messages ("failed to parse")
   - ❌ Exceptions used for control flow
   - ✅ Solution: Detailed `Result<T, E>` errors with context

2. **Type Safety**
   - ❌ Runtime type checking with `isinstance()`
   - ❌ Optional fields as `None` vs absent
   - ✅ Solution: Strong typing with enums, `Option<T>`

3. **Modularity**
   - ❌ Large monolithic functions (500+ lines)
   - ❌ Tight coupling between parsers
   - ✅ Solution: Small focused functions, trait-based design

---

## Progress Tracking

### Implementation Dashboard

Track progress at: `docs/PARSER_IMPLEMENTATION_PROGRESS.md`

```markdown
# Parser Implementation Progress

Last Updated: 2025-02-05

## Overall Status
- Ecosystems: 4/40 (10%)
- Formats: 12/136 (9%)

## Recent Work (February 2025)
- ✅ Phase 0.5: Parser Quality Enhancement
  - 17 commits across 3 waves
  - Safety, data consistency, advanced features
  - Test coverage: 305 → 317

## Phase 1: Modern Ecosystems
- [ ] Ruby (0/4 formats)
- [ ] Go (0/6 formats)
- [ ] PHP (0/2 formats)
- [ ] .NET (0/5 formats)
- [ ] Dart (0/2 formats)

...
```

### Milestone Tracking

Use GitHub milestones:
- Milestone: "Phase 1 - Modern Ecosystems"
- Milestone: "Phase 2 - Mobile & Build"
- etc.

Each parser gets its own issue with:
- [ ] Research & design
- [ ] Implementation
- [ ] Testing
- [ ] Documentation
- [ ] Review & merge

---

## Conclusion

This plan provides a comprehensive roadmap to full parser parity with ScanCode Toolkit. By following these guidelines and prioritizing high-impact ecosystems first, scancode-rust can achieve:

1. **Better Security**: No code execution, robust input validation, archive safety
2. **Better Performance**: Streaming parsers, optimized algorithms
3. **Better Maintainability**: Type safety, clear error handling, comprehensive documentation
4. **Better Testability**: Comprehensive test coverage, golden tests
5. **Better Documentation**: Auto-generated, always in sync
6. **Better Data Quality**: Proper dependency tracking, version analysis, hash extraction, license normalization

**Recent Achievements** (February 2025):
- ✅ Eliminated all panic-prone code (11 instances)
- ✅ Implemented direct dependency detection (7 parsers)
- ✅ Implemented version pinning analysis (2 parsers)
- ✅ Enhanced hash extraction (5 parsers, all 4 hash types)
- ✅ Added archive safety checks (Python wheel/egg)
- ✅ Integrated askalono for license normalization (4 parsers)
- ✅ Test coverage increased: 305 → 317 tests

The journey from 4 to 40+ ecosystems is significant, but with disciplined execution and the strong foundation already in place (including recent quality improvements), it's entirely achievable.

**Total Scope**: 40+ ecosystems, 136+ formats



**Next Steps**:
1. Review and approve this updated plan
2. Create GitHub issues for Phase 1 parsers
3. Begin implementation with Ruby or Go parsers
4. Continue maintaining high quality standards established in Phase 0.5
