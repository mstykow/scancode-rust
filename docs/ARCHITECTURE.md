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

## Future Work

### Remaining Parsers

See [NEXT_PHASE_PLAN.md](NEXT_PHASE_PLAN.md) for the current roadmap of remaining ecosystems and parsers.

### Detection Engines

Post parser implementation:

- **License detection** - SPDX normalization, confidence scoring
- **Copyright detection** - Copyright holder extraction from file content
- **Author extraction** - Email and author pattern detection

### Quality Enhancements

- Property-based testing with proptest
- Fuzzing with cargo-fuzz
- Performance benchmarks with criterion
- Memory profiling

## Related Documentation

- [README.md](../README.md) - User-facing overview and quick start
- [ADRs](adr/) - Architectural decision records
- [Improvements](improvements/) - Beyond-parity features
- [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md) - Complete format list (auto-generated)

## Contributing

See [AGENTS.md](../AGENTS.md) for guidelines on:

- Adding new parsers
- Parser implementation philosophy
- Testing requirements
- Code style and patterns

## License

Apache License 2.0
