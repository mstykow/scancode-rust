# scancode-rust Architecture

## Overview

scancode-rust is a complete rewrite of [ScanCode Toolkit](https://github.com/aboutcode-org/scancode-toolkit) in Rust, designed as a **drop-in replacement** with all features of the original, but with:

- **Zero bugs**: Leveraging Rust's type system and ownership model
- **Better performance**: Native code, parallel processing, zero-copy parsing
- **Enhanced security**: No code execution, comprehensive DoS protection
- **Feature parity or better**: 100% compatibility plus intentional improvements

**Current Status**: See [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md) for the full list of supported ecosystems and formats.

## Core Principles

### 1. Correctness Above All

> "always prefer correctness and full feature parity over effort/pragmatism"

- Every feature, edge case, and requirement from Python ScanCode must be preserved
- Zero tolerance for bugs - identify and fix issues from the original
- Comprehensive test coverage (unit + golden tests against Python reference)

### 2. Security First

- **No code execution**: AST parsing only, never eval/exec
- **DoS protection**: Explicit limits on file size, recursion, iterations
- **Archive safety**: Zip bomb prevention, compression ratio validation
- **Input validation**: Robust error handling, graceful degradation

See [ADR 0004: Security-First Parsing](adr/0004-security-first-parsing.md) for details.

### 3. Extraction vs Detection Separation

**Critical separation of concerns:**

- **Parsers extract** raw data from manifests
- **Detection engines** (future) normalize and analyze

Parsers NEVER:

- Normalize licenses to SPDX (detection engine's job)
- Extract copyright holders from file content (detection engine's job)
- Populate `declared_license_expression` (detection engine's job)

See [ADR 0002: Extraction vs Detection Separation](adr/0002-extraction-vs-detection.md) for details.

## System Architecture Overview

### Complete Processing Pipeline

scancode-rust implements a multi-phase processing pipeline based on Python ScanCode's architecture:

```text
┌─────────────────────────────────────────────────────────────────┐
│                    ScanCode Processing Pipeline                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase 1: Pre-Scan                                              │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • Archive extraction                                    │    │
│  │ • File type detection                                   │    │
│  │ • Pre-processing hooks                                  │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 2: Scanning                                              │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • Package manifest parsing (see SUPPORTED_FORMATS.md)   │    │
│  │ • License text detection                                │    │
│  │ • Copyright detection                                   │    │
│  │ • Email/URL extraction                                  │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 3: Post-Processing                                       │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • Package assembly (sibling, nested, file-ref, workspace)│    │
│  │ • Package consolidation/deduplication                   │    │
│  │ • License/copyright summarization                       │    │
│  │ • Tallies and facets                                    │    │
│  │ • Classification                                        │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 4: Filtering                                             │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • License policy filtering                              │    │
│  │ • Custom filter plugins                                 │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 5: Output                                                │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • JSON output (ScanCode-compatible)                     │    │
│  │ • SPDX, CycloneDX, CSV, YAML, HTML, JSONL              │    │
│  │ • HTML app and custom templates                         │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Component Inventory

- **Package Parsers**: See [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md) for complete list
- **Scanner Pipeline**: File discovery, parallel processing, progress tracking
- **Security Layer**: DoS protection, no code execution, archive safety
- **Package Assembly**: Sibling and nested merge strategies for combining related manifests
- **Text Detection**: License detection (n-gram matching), copyright detection (4-stage pipeline), email/URL extraction
- **Post-Processing**: Summarization, tallies, classification
- **Output**: JSON, SPDX (TV/RDF), CycloneDX (JSON/XML), CSV, YAML, JSON Lines, HTML, HTML app, custom templates
- **Testing Infrastructure**: Unit tests, doctests, golden tests, integration tests
- **Infrastructure**: Plugin system, caching, enhanced progress tracking

### Implementation Status

Implementation details in this document are intentionally architecture-focused.
For current capabilities and behavior, use:

- **[README.md](../README.md)** for user-facing features and usage
- **[SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md)** for currently supported formats and ecosystems
- **[TESTING_STRATEGY.md](TESTING_STRATEGY.md)** for verification and regression approach

### Plugin Architecture

Python ScanCode uses a plugin-based architecture with 5 plugin types:

1. **PreScan Plugins**: Archive extraction, file type detection
2. **Scan Plugins**: Package detection, license detection, copyright detection
3. **PostScan Plugins**: Package assembly, summarization, classification
4. **OutputFilter Plugins**: License policy filtering, custom filters
5. **Output Plugins**: Format-specific output (SPDX, CycloneDX, etc.)

The Rust implementation currently uses static, compile-time wiring (trait-based parsers and explicit pipeline stages).

## Architecture Components

### Trait-Based Parser System

**Core Abstraction:**

```rust
pub trait PackageParser {
    const PACKAGE_TYPE: &'static str;

    fn is_match(path: &Path) -> bool;
    fn extract_packages(path: &Path) -> Vec<PackageData>;
}
```

**Benefits:**

- Type-safe dispatch at compile time
- Zero runtime overhead
- Clear contract for all parsers
- Easy to test in isolation

**Implementation:**

```rust
pub struct NpmParser;

impl PackageParser for NpmParser {
    const PACKAGE_TYPE: &'static str = "npm";

    fn is_match(path: &Path) -> bool {
        matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some("package.json" | "package-lock.json")
        )
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        // Implementation
    }
}
```

See [ADR 0001: Trait-Based Parser Architecture](adr/0001-trait-based-parsers.md) for details.

### Parser Registration System

**How parsers are wired to the scanner:**

Parsers and recognizers are registered via the `register_package_handlers!` macro in `src/parsers/mod.rs`:

```rust
register_package_handlers! {
    parsers: [
        NpmParser,
        NpmLockParser,
        CargoParser,
        CargoLockParser,
        // ... more parsers ...
    ],
    recognizers: [
        JavaJarRecognizer,
        // ... file-type recognizers ...
    ],
}
```

**What this macro generates:**

1. **`try_parse_file(path: &Path) -> Option<Vec<PackageData>>`**
   - Called by scanner for every file
   - Tries each parser's `is_match()` in order
   - Returns first match's extracted data

2. **`parse_by_type_name(type_name: &str, path: &Path) -> Option<PackageData>`**
   - Used by test utilities for golden test generation
   - Allows direct parser invocation by name

3. **`list_parser_types() -> Vec<&'static str>`**
   - Returns all registered parser type names
   - Used by integration tests to verify registration

**Critical:** If a parser is implemented but not listed in this macro, it will **never be called** by the scanner, even if fully implemented and tested. The integration test `test_all_parsers_are_registered_and_exported` verifies this.

### Unified Data Model

All parsers output a single `PackageData` struct:

```rust
pub struct PackageData {
    // Identity
    pub package_type: Option<String>,
    pub namespace: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub purl: Option<String>,
    pub datasource_id: Option<DatasourceId>,

    // Metadata
    pub description: Option<String>,
    pub primary_language: Option<String>,
    pub release_date: Option<String>,
    pub homepage_url: Option<String>,
    pub parties: Vec<Party>,
    pub keywords: Vec<String>,

    // Dependencies
    pub dependencies: Vec<Dependency>,

    // Licenses (extraction only - detection is separate)
    pub extracted_license_statement: Option<String>,
    pub declared_license_expression: Option<String>,
    pub license_detections: Vec<LicenseDetection>,

    // Checksums & URLs
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
    pub download_url: Option<String>,
    pub vcs_url: Option<String>,
    pub repository_homepage_url: Option<String>,
    pub repository_download_url: Option<String>,
    pub api_data_url: Option<String>,

    // Additional data
    pub extra_data: Option<HashMap<String, serde_json::Value>>,
    pub source_packages: Vec<String>,
    pub file_references: Vec<FileReference>,
    pub is_private: bool,
    pub is_virtual: bool,
    // ... and more (see src/models/file_info.rs for complete definition)
}
```

**Rationale:**

- Normalizes differences across all supported ecosystems
- SBOM-compliant output format
- Single source of truth for structure

### Scanner Pipeline

```text
┌────────────────────────────────────────────────────────────┐
│                     scancode-rust                          │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  1. File Discovery           2. Parser Selection          │
│  ┌────────────────┐          ┌───────────────┐           │
│  │ Walk directory │─────────>│ Match file    │           │
│  │ Apply filters  │          │ to parser     │           │
│  └────────────────┘          └───────┬───────┘           │
│                                      │                     │
│  3. Extraction                       v                     │
│  ┌────────────────────────────────────────────┐           │
│  │ PackageParser::extract_packages()           │           │
│  │ ─ Read manifest                            │           │
│  │ ─ Parse structure                          │           │
│  │ ─ Extract metadata                         │           │
│  │ ─ Return PackageData                       │           │
│  └────────────────┬───────────────────────────┘           │
│                   │                                        │
│  4. Output        v                                        │
│  ┌─────────────────────────────────────┐                  │
│  │ Output format dispatch              │                  │
│  │ ─ JSON / YAML / CSV / JSONL         │                  │
│  │ ─ SPDX / CycloneDX / HTML / template│                  │
│  └─────────────────────────────────────┘                  │
│                                                            │
│  Detection Engines (Integrated)                           │
│  ┌───────────────────┐  ┌──────────────────┐             │
│  │ License Detection │  │ Copyright        │             │
│  │ ─ SPDX normalize  │  │ Detection        │             │
│  │ ─ Confidence      │  │ ─ Holder extract │             │
│  │ ─ Score threshold │  │ ─ Author extract │             │
│  └───────────────────┘  └──────────────────┘             │
└────────────────────────────────────────────────────────────┘
```

### Parallel Processing

Uses `rayon` for multi-threaded file scanning:

```rust
// Actual implementation in src/scanner/process.rs
files.par_iter()
    .map(|(path, metadata)| {
        // Each file processed in parallel
        let file_entry = process_file(path, metadata, scan_strategy);
        progress_bar.inc(1);
        file_entry
    })
    .collect()
```

Inside `process_file()`, the scanner calls `try_parse_file(path)` (generated by `register_package_handlers!` macro), then runs text detection engines on UTF-8 content:

```rust
// src/scanner/process.rs — simplified flow
if let Some(package_data) = try_parse_file(path) {
    file_info_builder.package_data(package_data);
}
// Text detection runs on all UTF-8 files
extract_license_information(&mut file_info_builder, text_content, scan_strategy)?;
extract_copyright_information(&mut file_info_builder, path, &text_content);
```

**Benefits:**

- Utilizes all CPU cores
- Maintains thread safety (Rust ownership guarantees)
- Progress tracking with atomic operations

### Package Assembly System

After scanning, the assembly system merges related manifests into logical packages using `DatasourceId`-based matching.

**Four assembly passes:**

- **SiblingMerge**: Combines sibling files in the same directory (e.g., `package.json` + `package-lock.json` → single npm package)
- **NestedMerge**: Combines parent/child manifests across directories (e.g., Maven parent POM + module POMs)
- **FileRefResolve**: Resolves `file_references` from package database entries (RPM/Alpine/Debian) against scanned files, sets `for_packages` on matched files, tracks missing references, and resolves RPM namespace from os-release
- **WorkspaceMerge**: Post-processing pass for monorepo workspaces (e.g., npm/pnpm/Cargo workspaces → separate Package per workspace member with shared resource assignment and `workspace:*` version resolution)

**How it works:**

1. Each `AssemblerConfig` declares which `DatasourceId` variants belong together and which file patterns to look for
2. After scanning, the assembler groups packages by directory
3. Packages whose `datasource_id` values match the same config are merged into a single logical package
4. Combined packages aggregate `datafile_paths` and `datasource_ids` from all contributing files
5. File reference resolution matches installed-package database entries to files on disk (e.g., Alpine `installed` DB lists files belonging to each package)
6. Workspace assembly runs as a final pass: detects workspace roots (npm/pnpm/Cargo), discovers members via glob patterns, creates per-member packages with full metadata inheritance (Cargo `[workspace.package]` and `workspace = true` resolution), hoists dependencies, and resolves `workspace:*` version references

Assembly is configurable via the `--no-assemble` CLI flag. See `src/assembly/` for implementation details.

### Security Architecture

```text
┌─────────────────────────────────────────────────────────┐
│                  Security Layers                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Layer 1: No Code Execution                             │
│  ┌────────────────────────────────────────────────┐    │
│  │ AST parsing only (setup.py, build.gradle)      │    │
│  │ Never eval/exec/subprocess                      │    │
│  │ Regex/token-based for DSLs                      │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
│  Layer 2: Resource Limits                               │
│  ┌────────────────────────────────────────────────┐    │
│  │ File size: 100MB max                            │    │
│  │ Recursion depth: 50 levels                      │    │
│  │ Iterations: 100,000 max                         │    │
│  │ String length: 10MB per field                   │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
│  Layer 3: Archive Safety                                │
│  ┌────────────────────────────────────────────────┐    │
│  │ Uncompressed size: 1GB max                      │    │
│  │ Compression ratio: 100:1 max (zip bomb detect)  │    │
│  │ Path traversal: Block ../ patterns              │    │
│  │ Temp cleanup: Automatic via TempDir             │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
│  Layer 4: Input Validation                              │
│  ┌────────────────────────────────────────────────┐    │
│  │ Result<T, E> error handling                     │    │
│  │ No .unwrap() in library code                    │    │
│  │ Graceful degradation on errors                  │    │
│  │ UTF-8 validation with lossy fallback            │    │
│  └────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

See [ADR 0004: Security-First Parsing](adr/0004-security-first-parsing.md) for comprehensive security analysis.

## Testing Strategy

### Four-Layer Test Pyramid

```text
         /\
        /  \    Integration Tests
       /    \   ─ End-to-end scanner pipeline
      /------\  ─ Full scan validation
     /        \
    / Golden   \ Golden Tests
   /  Tests     \ ─ Compare with Python ScanCode output
  /--------------\ ─ Real-world manifest files
 /                \
/    Unit Tests    \ Unit Tests + Doctests
/   + Doctests      \ ─ Parser functions, edge cases
/____________________\ ─ API documentation examples
```

**Four layers** (see [TESTING_STRATEGY.md](TESTING_STRATEGY.md) for full details):

1. **Doctests**: API documentation examples that run as tests
2. **Unit Tests**: Component-level tests for individual functions and edge cases
3. **Golden Tests**: Regression tests comparing output against Python ScanCode reference
4. **Integration Tests**: End-to-end tests validating the full scanner pipeline

See [ADR 0003: Golden Test Strategy](adr/0003-golden-test-strategy.md) for golden test details.

## Documentation Strategy

### Three-Layer Documentation

```text
┌─────────────────────────────────────────────────────────┐
│                 Documentation Sources                    │
└─────────────────────────────────────────────────────────┘
           │                    │                  │
           ▼                    ▼                  ▼
    ┌─────────────┐     ┌──────────────┐   ┌────────────┐
    │   Parser    │     │ Doc Comments │   │   Manual   │
    │  Metadata   │     │   (/// //!)  │   │ Markdown   │
    │   (code)    │     │              │   │   Files    │
    └──────┬──────┘     └──────┬───────┘   └──────┬─────┘
           │                   │                   │
           ▼                   ▼                   ▼
    ┌─────────────┐     ┌──────────────┐   ┌────────────┐
    │ Auto-Gen    │     │  cargo doc   │   │   GitHub   │
    │ Formats.md  │     │  (docs.rs)   │   │   README   │
    └─────────────┘     └──────────────┘   └────────────┘
```

**Auto-Generated**: `docs/SUPPORTED_FORMATS.md` (from parser metadata)  
**API Reference**: cargo doc (from `///` and `//!` comments)  
**Architecture**: ADRs, improvements, guides (manual Markdown)

See [ADR 0005: Auto-Generated Documentation](adr/0005-auto-generated-docs.md) for details.

## Beyond-Parity Improvements

We don't just match Python ScanCode - we improve it:

| Parser                  | Improvement                                                                                                                  | Type                                      |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| **Alpine**              | SHA1 checksums correctly decoded + Provider field extraction                                                                 | 🐛 Bug Fix + ✨ Feature                   |
| **RPM**                 | Full dependency extraction with version constraints                                                                          | ✨ Feature                                |
| **Debian**              | .deb archive introspection                                                                                                   | ✨ Feature                                |
| **Conan**               | conanfile.txt and conan.lock parsers (Python has neither)                                                                    | ✨ Feature                                |
| **Gradle**              | No code execution (token lexer vs Groovy engine)                                                                             | 🛡️ Security                               |
| **Gradle Lockfile**     | gradle.lockfile parser (Python has no equivalent)                                                                            | ✨ Feature                                |
| **Maven**               | SCM developerConnection separation, inception_year, renamed extra_data keys for consistency                                  | 🔍 Enhanced                               |
| **npm Workspace**       | pnpm-workspace.yaml extraction + workspace assembly with per-member packages (Python has stub parser + basic assembly)       | ✨ Feature                                |
| **Cargo Workspace**     | Full `[workspace.package]` metadata inheritance + `workspace = true` dependency resolution (Python has basic assembly)       | ✨ Feature                                |
| **Composer**            | Richer provenance metadata (7 extra fields)                                                                                  | 🔍 Enhanced                               |
| **Ruby**                | Semantic party model (unified name+email)                                                                                    | 🔍 Enhanced                               |
| **Dart**                | Proper scope handling + YAML preservation                                                                                    | 🔍 Enhanced                               |
| **CPAN**                | Full metadata extraction (Python has stubs only)                                                                             | ✨ Feature                                |
| **Copyright Detection** | Year range 2099 (was 2039), regex bug fixes, type-safe POS tags, thread-safe design, Unicode preservation, encoded-data skip | 🐛 Bug Fix + 🔍 Enhanced + ⚡ Performance |
| **Assembly**            | LazyLock static assembler lookup (zero allocation per call)                                                                  | ⚡ Performance                            |

See [docs/improvements/](improvements/) for detailed documentation of each improvement.

## Project Structure

The codebase follows a modular architecture:

- **`src/parsers/`** - Package manifest parsers (one per ecosystem)
- **`src/models/`** - Core data structures (PackageData, Dependency, DatasourceId, etc.)
- **`src/assembly/`** - Package assembly system (merging related manifests)
- **`src/scanner/`** - File system traversal and orchestration
- **`docs/`** - Architecture decisions, improvement docs, and guides
- **`testdata/`** - Test manifests for validation
- **`reference/`** - Python ScanCode Toolkit (reference submodule)

## Performance Characteristics

### Benchmarks

_(To be added: criterion benchmarks for parser performance)_

### Optimization Strategies

1. **Parallel Processing**: Uses all CPU cores via rayon
2. **Zero-Copy Parsing**: `&str` instead of `String` where possible
3. **Compile-Time Embedding**: License data embedded via `include_dir!`
4. **Lazy Evaluation**: Iterators instead of eager Vec building
5. **Efficient Parsers**: quick-xml, toml, serde_json (production-grade)

### Release Optimizations

```toml
[profile.release]
lto = true                # Link-time optimization
codegen-units = 1         # Single codegen unit for max optimization
strip = true              # Strip symbols for smaller binary
opt-level = 3             # Maximum optimization
```

## Extended Architecture

The following sections describe major architectural components in detail.

### Text Detection Engines

**License Detection**:

- License text matching using fingerprinting algorithms
- SPDX license expression generation
- Confidence scoring and multi-license handling
- Integration with existing SPDX license data

**Copyright Detection**:

The copyright detection engine extracts copyright statements, holder names, and author information from source files using a four-stage pipeline:

```text
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  1. Text     │───>│  2. Candidate│───>│  3. Lex +    │───>│  4. Tree     │
│  Preparation │    │  Selection   │    │  Parse       │    │  Walk +      │
│              │    │              │    │              │    │  Refinement  │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
```

1. **Text Preparation**: Normalizes copyright symbols (`©`, `(c)`, HTML entities), strips comment markers and markup, preserves Unicode (no ASCII transliteration)
2. **Candidate Selection**: Filters lines using hint markers (`opyr`, `auth`, `©`, year patterns), groups multi-line statements, and skips encoded or non-promising content early
3. **Lexing + Parsing**: POS-tags tokens using an ordered pattern set (type-safe `PosTag` enum), then applies grammar rules to build parse trees identifying `COPYRIGHT`, `AUTHOR`, `NAME`, `COMPANY` structures
4. **Tree Walk + Refinement**: Extracts `CopyrightDetection`, `HolderDetection`, `AuthorDetection` from parse trees, then applies cleanup (for example unbalanced parens, duplicate "Copyright" words, and junk patterns)

Key design decisions vs Python reference:

- **Type-safe POS tags**: Enum-based (not string-based) — compiler catches tag typos
- **Thread-safe**: No global mutable state (Python uses a singleton `DETECTOR`)
- **Sequential pattern matching**: `LazyLock<Vec<(Regex, PosTag)>>` with first-match-wins semantics (RegexSet cannot preserve match order)
- **Extended year range**: 1960-2099 (Python stops at 2039)
- **Bug fixes**: Fixed year-year separator bug, short-year typo, French/Spanish case-sensitivity, duplicate patterns

Special cases handled:

- Linux CREDITS files (structured `N:/E:/W:` format)
- SPDX-FileCopyrightText and SPDX-FileContributor
- "All Rights Reserved" in English, German, French, Spanish, Dutch
- Multi-line copyright statements spanning consecutive lines

Behavioral compatibility model:

- **Default expectation**: Follow Python ScanCode behavior closely for copyright, holder, and author extraction.
- **Intentional Rust differences**: Preserve Unicode names, apply correctness bug fixes from the Python reference, and keep detection thread-safe for parallel scans.
- **Known parity gaps**: Some edge-case files still differ from Python output; these are treated as targeted follow-up work with regression tests.
- **Fixture ownership**: Copyright golden fixtures in this repository are Rust-owned expectations; Python fixtures are a reference input, not the source of truth for local expected outputs.

Migration expectation:

- Most projects should observe equivalent results to Python ScanCode.
- Where differences exist, they are either intentional improvements (for example Unicode preservation) or explicitly tracked parity gaps.

Module location: `src/copyright/`

**Email/URL Detection**:

The email/URL detection engine is the simplest text detection feature — regex-based extraction with an ordered filter pipeline to remove junk results.

```text
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  1. Read     │───>│  2. Regex    │───>│  3. Filter   │───>│  4. Yield    │
│  Lines       │    │  Match       │    │  Pipeline    │    │  Results     │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
```

**Email detection**: RFC-ish regex (`[A-Z0-9._%-]+@[A-Z0-9.-]+\.[A-Z]{2,63}`) → 3-step filter pipeline (junk domain filter, uninteresting email filter, dedup).

**URL detection**: Three regex alternatives (scheme URLs, bare-domain URLs, git-style URLs) → 10-step filter pipeline:

1. CRLF cleanup → trailing junk stripping → empty URL filter → scheme addition → user/password stripping → invalid URL filter → canonicalization (via `url` crate) → junk host filter → junk URL filter → dedup

Both support configurable thresholds (`--max-email N`, `--max-url N`, default 50).

Golden regression coverage for this module uses local, repo-owned fixtures in
`testdata/plugin_email_url/` and test execution in `src/finder/golden_test.rs`.

Key design decisions vs Python reference:

- **`url` crate** for URL parsing/canonicalization (replaces `urlpy`)
- **`std::net`** for IP classification (replaces `ipaddress`)
- **Extended TLD support**: `{2,63}` per RFC 1035 (Python's `{2,4}` rejects `.museum`, `.technology`)
- **Fixed IPv6 private detection**: Python has assignment bug making IPv6 private detection non-functional
- **Proper error handling**: No silent exception swallowing in URL canonicalization

Junk classification data (~150 entries): example domains, private IPs, W3C/XML namespaces, DTD URLs, PKI/certificate URLs, CDN URLs, image file suffixes.

Module location: `src/finder/`

### Post-Processing Pipeline

**Package Consolidation**:

- Package deduplication across scan results
- Dependency graph resolution
- Transitive dependency handling

**Summarization**:

- License tallies and facets
- Copyright holder aggregation
- File classification (source, docs, data, etc.)
- Summary statistics

### Output Format Support

**Implementation and parity tracking:**

- Multi-format output layer is implemented in `src/output/mod.rs`
- CLI follows ScanCode-style output flags (for example `--json-pp FILE`,
  `--spdx-tv FILE`) and dispatches through `write_output_file`
- Format compatibility is verified through fixture-backed tests and documented
  in `docs/TESTING_STRATEGY.md`

**SBOM Formats**:

- SPDX: Tag-value and RDF/XML
- CycloneDX: JSON, XML
- Compatibility with SBOM tooling ecosystem

**Additional Formats**:

- CSV (tabular data export)
- YAML (human-readable)
- HTML report + HTML app
- Custom templates (user-defined formats)

#### Infrastructure Enhancements

**Plugin System**:

- Extensible plugin architecture
- Custom scan plugins
- Custom output formats
- Third-party integrations

**Caching**:

Two-layer caching system for scan performance optimization:

1. **License Index Cache**: Persists the compiled askalono `Store` (MessagePack + zstd) to avoid rebuilding from SPDX text on each run. Existing `Store::from_cache()`/`to_cache()` infrastructure handles serialization. Version-stamped with tool version + SPDX data version. This is designed to reduce startup overhead by reusing compiled store data.

2. **Scan Result Cache** (beyond-parity — Python has none): Content-addressed per-file cache keyed by SHA256 hash (already computed in `process_file()`). Cached data: package_data, license_detections, copyrights, programming_language. Path-dependent fields reconstructed at load time. Sharded directory layout (`ab/ab3f...postcard`) for filesystem scalability. This is intended to significantly improve repeated-scan performance.

3. **Incremental Scanning** (beyond-parity — Python has none): Scan manifest tracks `{path: (mtime, size, sha256)}` per directory. On re-scan, only files with changed mtime/size are re-hashed and re-scanned. Enables CI/CD integration (scan only changed files per commit).

Cache location: XDG-compliant (`~/.cache/scancode-rust/`), overridable via `SCANCODE_RUST_CACHE` env var or `--cache-dir` CLI flag. Multi-process safety via `fd-lock` file locking. Atomic writes (temp + rename) prevent corruption on crash.

Module location: `src/cache/`

**Progress Tracking**:

Centralized `ScanProgress` struct managing multi-phase progress bars via `indicatif::MultiProgress`:

1. **Discovery phase**: Spinner while counting files. Records initial file/dir/size counts.
2. **Scan phase**: Main progress bar with ETA, elapsed time, and file count. Integrates with rayon parallel processing via `Arc<ProgressBar>`. Rate-limited to 20 Hz (indicatif default).
3. **Assembly phase**: Progress bar for package assembly (sibling merge, workspace merge, etc.).
4. **Scan summary**: Files/sec, bytes/sec, error count, per-phase timings, initial/final counts.

Verbosity modes are part of ongoing CLI ergonomics work and are implemented in the command-line and progress modules.

Logging integration via `indicatif-log-bridge`: parser `warn!()` messages route above the progress bar without corrupting display. Runtime logging and output-writing behavior are implemented in `src/progress.rs`, `src/scanner.rs`, and output writers.

Module location: `src/progress.rs`

### Quality Enhancements

Ongoing quality improvements:

- Property-based testing with proptest
- Fuzzing with cargo-fuzz
- Performance benchmarks with criterion
- Memory profiling and optimization
- Continuous golden test expansion

## License Data Architecture

### How License Detection Works

This tool uses the [SPDX License List Data](https://github.com/spdx/license-list-data) for license detection. The license data is:

1. **Stored in a Git submodule** at `resources/licenses/` (sparse checkout of `json/details/` only)
2. **Embedded at compile time** using Rust's `include_dir!` macro (see `src/main.rs`)
3. **Built into the binary** - no runtime dependencies on external files

This means:

- **For users**: The binary is self-contained and portable
- **For developers**: The submodule must be initialized before building
- **Package size**: Only the needed JSON files are included in the published crate

### Updating the License Data

**For Releases:** The `release.sh` script automatically updates the license data to the latest version before publishing. No manual action needed.

**For Development:**

To initialize or update to the latest SPDX license definitions:

```sh
./setup.sh                  # Initialize/update license data to latest
cargo build --release       # Rebuild with updated data
```

The script will show if the license data was updated. If so, commit the change:

```sh
git add resources/licenses
git commit -m "chore: update SPDX license data"
```

The `setup.sh` script:

- Initializes the submodule with shallow clone (`--depth=1`)
- Configures sparse checkout to only include `json/details/` (saves ~90% disk space)
- Updates to the latest upstream version
- The build process then embeds these files directly into the compiled binary

## Related Documentation

- [README.md](../README.md) - User-facing overview, installation, and usage
- [AGENTS.md](../AGENTS.md) - Contributor guidelines and code style
- [ADRs](adr/) - Architectural decision records
- [Improvements](improvements/) - Beyond-parity features
- [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md) - Complete format list (auto-generated)
