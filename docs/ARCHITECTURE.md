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
│  Phase 1: Pre-Scan (Planned)                                    │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • Archive extraction                                    │    │
│  │ • File type detection                                   │    │
│  │ • Pre-processing hooks                                  │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 2: Scanning (Partially Implemented)                      │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ ✅ Package manifest parsing (see SUPPORTED_FORMATS.md) │    │
│  │ ❌ License text detection                               │    │
│  │ ❌ Copyright detection                                  │    │
│  │ ❌ Email/URL extraction                                 │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 3: Post-Processing (Planned)                             │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • Package assembly (design complete)                    │    │
│  │ • Package consolidation/deduplication                   │    │
│  │ • License/copyright summarization                       │    │
│  │ • Tallies and facets                                    │    │
│  │ • Classification                                        │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 4: Filtering (Planned)                                   │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ • License policy filtering                              │    │
│  │ • Custom filter plugins                                 │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│                           ▼                                      │
│  Phase 5: Output (Partially Implemented)                        │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ ✅ JSON output (ScanCode-compatible)                    │    │
│  │ ❌ SPDX (RDF, JSON, YAML, tag-value)                    │    │
│  │ ❌ CycloneDX (JSON, XML)                                │    │
│  │ ❌ CSV, YAML, HTML                                      │    │
│  │ ❌ Custom templates                                     │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Component Inventory

**Implemented Components** (✅):

- **Package Parsers**: See [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md) for complete list
- **Scanner Pipeline**: File discovery, parallel processing, progress tracking
- **Security Layer**: DoS protection, no code execution, archive safety
- **JSON Output**: ScanCode Toolkit-compatible format
- **Testing Infrastructure**: Unit tests, golden tests, integration tests

**Planned Components** (❌):

- **Text Detection**: License detection, copyright detection, email/URL extraction
- **Package Assembly**: Merge related manifests into logical packages
- **Post-Processing**: Summarization, tallies, classification
- **Output Formats**: SPDX, CycloneDX, CSV, YAML, HTML
- **Infrastructure**: Plugin system, caching, enhanced progress tracking

### Implementation Status

For current implementation status, priorities, and effort estimates, see:

- **[implementation-plans/README.md](implementation-plans/README.md)** - Overview of all implementation plans
- **[implementation-plans/package-detection/](implementation-plans/package-detection/)** - Package parsing and assembly
- **[implementation-plans/text-detection/](implementation-plans/text-detection/)** - License, copyright, email/URL detection
- **[implementation-plans/post-processing/](implementation-plans/post-processing/)** - Summarization and tallies
- **[implementation-plans/output/](implementation-plans/output/)** - Output format support
- **[implementation-plans/infrastructure/](implementation-plans/infrastructure/)** - Plugin system, caching, progress tracking

Each plan includes detailed status, priorities (P0-P3), effort estimates, and implementation phases.

### Plugin Architecture (Planned)

Python ScanCode uses a plugin-based architecture with 5 plugin types:

1. **PreScan Plugins**: Archive extraction, file type detection
2. **Scan Plugins**: Package detection, license detection, copyright detection
3. **PostScan Plugins**: Package assembly, summarization, classification
4. **OutputFilter Plugins**: License policy filtering, custom filters
5. **Output Plugins**: Format-specific output (SPDX, CycloneDX, etc.)

The Rust implementation will adopt a similar architecture using Rust traits and dynamic dispatch, with compile-time plugin registration for zero runtime overhead.

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

Parsers are registered via the `define_parsers!` macro in `src/parsers/mod.rs`:

```rust
define_parsers! {
    NpmParser,
    NpmLockParser,
    CargoParser,
    CargoLockParser,
    // ... more parsers ...
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
    pub name: Option<String>,
    pub version: Option<String>,
    pub namespace: Option<String>,
    
    // Metadata
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub parties: Vec<Party>,
    
    // Dependencies
    pub dependencies: Vec<Dependency>,
    
    // Licenses (extraction only - detection is separate)
    pub extracted_license_statement: Option<String>,
    
    // Checksums & URLs
    pub sha256: Option<String>,
    pub repository_homepage_url: Option<String>,
    
    // Additional data
    pub extra_data: serde_json::Value,
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
│  │ JSON serialization                  │                  │
│  │ ─ ScanCode Toolkit compatible       │                  │
│  │ ─ SBOM-ready structure              │                  │
│  └─────────────────────────────────────┘                  │
│                                                            │
│  Future: Detection Engines (Post-Parser)                  │
│  ┌───────────────────┐  ┌──────────────────┐             │
│  │ License Detection │  │ Copyright        │             │
│  │ ─ SPDX normalize  │  │ Detection        │             │
│  │ ─ Confidence      │  │ ─ Holder extract │             │
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

Inside `process_file()`, the scanner calls `try_parse_file(path)` (generated by `define_parsers!` macro):

```rust
// src/scanner/process.rs, line 148
if let Some(package_data) = try_parse_file(path) {
    file_info_builder.package_data(package_data);
    Ok(())
} else {
    // Not a package manifest, try license detection
    extract_license_information(...)
}
```

**Benefits:**

- Utilizes all CPU cores
- Maintains thread safety (Rust ownership guarantees)
- Progress tracking with atomic operations

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

### Three-Layer Test Pyramid

```text
       /\
      /  \    Golden Tests (Integration)
     /    \   ─ Compare with Python ScanCode output
    /------\  ─ Real-world manifest files
   /        \
  /   Unit   \ Unit Tests
 /   Tests    \ ─ Parser functions
/______________\ ─ Edge cases
```

**Golden Tests** validate feature parity:

- Reference outputs from Python ScanCode Toolkit
- Automated JSON comparison
- Regression detection
- Run `cargo test golden` to see current pass rates

See [ADR 0003: Golden Test Strategy](adr/0003-golden-test-strategy.md) for details.

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

| Parser | Improvement | Type |
|--------|-------------|------|
| **Alpine** | SHA1 checksums correctly decoded + Provider field extraction | 🐛 Bug Fix + ✨ Feature |
| **RPM** | Full dependency extraction with version constraints | ✨ Feature |
| **Debian** | .deb archive introspection | ✨ Feature |
| **Conan** | conanfile.txt and conan.lock parsers (Python has neither) | ✨ Feature |
| **Gradle** | No code execution (token lexer vs Groovy engine) | 🛡️ Security |
| **Gradle Lockfile** | gradle.lockfile parser (Python has no equivalent) | ✨ Feature |
| **npm Workspace** | pnpm-workspace.yaml metadata extraction (Python has stub only) | ✨ Feature |
| **Composer** | Richer provenance metadata (7 extra fields) | 🔍 Enhanced |
| **Ruby** | Semantic party model (unified name+email) | 🔍 Enhanced |
| **Dart** | Proper scope handling + YAML preservation | 🔍 Enhanced |
| **CPAN** | Full metadata extraction (Python has stubs only) | ✨ Feature |

See [docs/improvements/](improvements/) for detailed documentation of each improvement.

## Project Structure

The codebase follows a modular architecture:

- **`src/parsers/`** - Package manifest parsers (one per ecosystem)
- **`src/models/`** - Core data structures (PackageData, Dependency, etc.)
- **`src/scanner/`** - File system traversal and orchestration
- **`docs/`** - Architecture decisions, improvement docs, and guides
- **`testdata/`** - Test manifests for validation
- **`reference/`** - Python ScanCode Toolkit (reference submodule)

## Performance Characteristics

### Benchmarks

*(To be added: criterion benchmarks for parser performance)*

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

## Future Architecture

### Planned Features

The following sections describe major architectural components planned for implementation. See [implementation-plans/](implementation-plans/) for detailed plans.

#### Text Detection Engines

**License Detection**:

- License text matching using fingerprinting algorithms
- SPDX license expression generation
- Confidence scoring and multi-license handling
- Integration with existing SPDX license data

**Copyright Detection**:

- Copyright statement extraction from file content
- Copyright holder identification
- Year range parsing
- Statement normalization

**Email/URL Detection**:

- Email address extraction
- URL detection and validation
- Author contact information

#### Package Assembly System

**Assembly**:

- Merge related manifests into logical packages
- Example: `package.json` + `package-lock.json` → single npm package
- Multiple assemblers for different ecosystems (npm, Maven, Python, etc.)

**Consolidation**:

- Package deduplication across scan results
- Dependency graph resolution
- Transitive dependency handling

#### Post-Processing Pipeline

**Summarization**:

- License tallies and facets
- Copyright holder aggregation
- File classification (source, docs, data, etc.)
- Summary statistics

#### Output Format Support

**SBOM Formats**:

- SPDX: RDF, JSON, YAML, tag-value
- CycloneDX: JSON, XML
- Compatibility with SBOM tooling ecosystem

**Additional Formats**:

- CSV (tabular data export)
- YAML (human-readable)
- HTML (interactive reports)
- Custom templates (user-defined formats)

#### Infrastructure Enhancements

**Plugin System**:

- Extensible plugin architecture
- Custom scan plugins
- Custom output formats
- Third-party integrations

**Caching**:

- Scan result caching
- Incremental scanning
- Cache invalidation strategies

**Progress Tracking**:

- Enhanced progress reporting
- Per-phase progress indicators
- Estimated time remaining

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
- [Implementation Plans](implementation-plans/) - Feature implementation status and roadmap
