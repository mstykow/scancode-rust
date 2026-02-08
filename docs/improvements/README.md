# Beyond-Parity Parser Improvements

This directory documents features where scancode-rust **exceeds** the Python ScanCode Toolkit reference implementation.

## Philosophy

Our goal is **100% feature parity or better**. We:

- Fix bugs present in the Python implementation
- Implement features marked as TODO in Python
- Add missing functionality where it's low-hanging fruit
- Improve data quality and extraction accuracy

## Improvement Categories

### 🐛 Bug Fixes

Python implementation has incorrect behavior, we fix it.

### ✨ New Features

Python has TODO comments or placeholders, we implement the feature.

### 🔍 Enhanced Extraction

Python extracts some data, we extract more (additional fields, better parsing).

### 🛡️ Security Improvements

Python has unsafe patterns (code execution, DoS vulnerabilities), we use safe alternatives.

## Summary Table

| Parser | Improvement Type | Python Status | Our Status | Impact |
|--------|-----------------|---------------|------------|--------|
| [Alpine](alpine-parser.md) | 🐛 Bug Fix + ✨ Feature | SHA1 always `null` + Provider field TODO | SHA1 correctly decoded + Providers extracted | Critical for integrity verification |
| [RPM](rpm-parser.md) | ✨ New Feature | Multiple "add dependencies!!!" TODOs | Full dependency extraction with constraints | Essential for SBOM completeness |
| [Debian](debian-parser.md) | ✨ New Feature | TODO: "introspect archive" | Full .deb control.tar.gz extraction | Better metadata accuracy |
| [Composer](composer-parser.md) | 🔍 Enhanced | Basic extraction | Richer metadata (7 extra_data fields) | Improved package provenance tracking |
| [Ruby](ruby-parser.md) | 🔍 Enhanced | String-based party data | Semantic Party model | Structured author/maintainer data |
| [Dart](dart-parser.md) | 🔍 Enhanced | Scope always `null` + YAML lossy | Proper scope + YAML preservation | Correct dependency classification |
| [Gradle](gradle-parser.md) | 🛡️ Security | Groovy engine execution | Custom lexer (no execution) | No arbitrary code execution |
| All Parsers | 🛡️ Security | No DoS limits | File size + iteration limits | Protection against resource exhaustion |

## Per-Parser Documentation

Each parser with improvements has a dedicated document:

- **[alpine-parser.md](alpine-parser.md)** - 🐛 Bug Fix + ✨ Feature: SHA1 decoding fix + Provider field extraction
- **[rpm-parser.md](rpm-parser.md)** - ✨ New Feature: Dependency extraction with version constraints
- **[debian-parser.md](debian-parser.md)** - ✨ New Feature: .deb archive introspection
- **[composer-parser.md](composer-parser.md)** - 🔍 Enhanced Extraction: 7 additional extra_data fields for package provenance
- **[ruby-parser.md](ruby-parser.md)** - 🔍 Enhanced Extraction: Semantic Party model combining name and email
- **[dart-parser.md](dart-parser.md)** - 🔍 Enhanced Extraction: Proper scope handling + YAML trailing newline preservation
- **[gradle-parser.md](gradle-parser.md)** - 🛡️ Security Improvement: Token-based lexer instead of Groovy engine (no code execution)

## Verification

All improvements are:

- ✅ **Validated against real-world data** - Tested with actual package files
- ✅ **Covered by tests** - Unit tests + golden tests where applicable
- ✅ **Documented** - Code comments explain the improvement
- ✅ **Compared with Python** - Explicit comparison showing what changed

## Contributing Improvements

When implementing a parser, if you discover:

1. **A bug in Python**: Fix it in Rust, document here
2. **A TODO in Python**: Implement it in Rust, document here
3. **Missing extraction**: Add it in Rust, document here

**Template for new improvement docs**: See existing files for structure.

## Related Documentation

- [ADR 0002: Extraction vs Detection Separation](../adr/0002-extraction-vs-detection.md) - Why we separate concerns better than Python
- [ADR 0004: Security-First Parsing](../adr/0004-security-first-parsing.md) - Our security advantages over Python
- [SUPPORTED_FORMATS.md](../SUPPORTED_FORMATS.md) - Auto-generated list of all supported formats (run `cargo run --bin generate-supported-formats`)
