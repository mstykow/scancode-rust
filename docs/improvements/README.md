# Beyond-Parity Improvements

This directory documents features where scancode-rust **exceeds** the Python ScanCode Toolkit reference implementation.
It includes parser improvements and text-detection subsystem improvements.

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

| Area                                                    | Improvement Type                       | Python Status                                                                                                        | Our Status                                                                                  | Impact                                                                        |
| ------------------------------------------------------- | -------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [Alpine](alpine-parser.md)                              | 🐛 Bug Fix + ✨ Feature                | SHA1 always `null` + Provider field TODO                                                                             | SHA1 correctly decoded + Providers extracted                                                | Critical for integrity verification                                           |
| [RPM](rpm-parser.md)                                    | ✨ New Feature                         | Multiple "add dependencies!!!" TODOs                                                                                 | Full dependency extraction with constraints                                                 | Essential for SBOM completeness                                               |
| [Debian](debian-parser.md)                              | ✨ New Feature                         | TODO: "introspect archive"                                                                                           | Full .deb control.tar.gz extraction                                                         | Better metadata accuracy                                                      |
| [Conan](conan-parser.md)                                | ✨ New Feature                         | No conanfile.txt or conan.lock parser                                                                                | Full conanfile.txt + conan.lock extraction                                                  | C/C++ dependency visibility                                                   |
| [CPAN](cpan-parser.md)                                  | ✨ New Feature                         | Stub-only handlers (no parse method)                                                                                 | Full META.json, META.yml, MANIFEST parsing                                                  | Perl metadata extraction                                                      |
| [RPM Specfile](rpm-specfile-parser.md)                  | ✨ New Feature                         | Stub with TODO comment                                                                                               | Full preamble parsing                                                                       | RPM spec metadata extraction                                                  |
| [CPAN Makefile.PL](cpan-makefile-pl-parser.md)          | ✨ New Feature                         | Stub-only handler (no parse method)                                                                                  | WriteMakefile metadata extraction                                                           | Perl build metadata                                                           |
| [OSGi Manifest](osgi-manifest-parser.md)                | ✨ New Feature                         | Empty path_patterns (assembly only)                                                                                  | Full OSGi metadata extraction                                                               | Java bundle dependencies                                                      |
| [Gradle](gradle-parser.md)                              | 🛡️ Security                            | Groovy engine execution                                                                                              | Custom lexer (no execution)                                                                 | No arbitrary code execution                                                   |
| [Gradle Lockfile](gradle-lockfile-parser.md)            | ✨ New Feature                         | No gradle.lockfile parser                                                                                            | Full lockfile dependency extraction                                                         | Pinned dependency auditing                                                    |
| [npm Workspace](npm-workspace-parser.md)                | ✨ New Feature                         | NonAssemblable stub + basic assembly                                                                                 | Workspace extraction + improved assembly                                                    | Monorepo structure visibility + correct package counts                        |
| [Composer](composer-parser.md)                          | 🔍 Enhanced                            | Basic extraction                                                                                                     | Richer metadata (7 extra_data fields)                                                       | Improved package provenance tracking                                          |
| [Ruby](ruby-parser.md)                                  | 🔍 Enhanced                            | String-based party data                                                                                              | Semantic Party model                                                                        | Structured author/maintainer data                                             |
| [Dart](dart-parser.md)                                  | 🔍 Enhanced                            | Scope always `null` + YAML lossy                                                                                     | Proper scope + YAML preservation                                                            | Correct dependency classification                                             |
| [OS Release](os-release-parser.md)                      | 🐛 Bug Fix + 🔍 Enhanced               | Debian name logic bug + no URL extraction                                                                            | Fixed name logic + HOME/SUPPORT/BUG URLs                                                    | Correct distro identification + richer metadata                               |
| [Conan Data](conan-data-parser.md)                      | 🔍 Enhanced                            | Only extracts primary source URL                                                                                     | Patches metadata + mirror/fallback URLs                                                     | Complete source provenance tracking                                           |
| [CPAN dist.ini](cpan-dist-ini-parser.md)                | ✨ New Feature                         | Stub-only handler (returns empty)                                                                                    | Full INI parsing with dependency scopes                                                     | Perl Dist::Zilla metadata extraction                                          |
| [Swift Dependencies](swift-show-dependencies-parser.md) | 🔍 Enhanced                            | Only extracts root package name                                                                                      | Full dependency graph with versions + direct/transitive                                     | Complete Swift dependency visibility                                          |
| [Maven](maven-parser.md)                                | 🔍 Enhanced                            | SCM fields merged, no inception_year                                                                                 | SCM separation + inception_year + consistent keys                                           | Data preservation + SBOM completeness                                         |
| [npm Git URLs](npm-git-url-dependencies.md)             | 🐛 Bug Fix                             | Git URLs treated as pinned versions                                                                                  | Correct is_pinned=false for non-version deps                                                | Valid PURLs + correct dependency resolution status                            |
| [Gitmodules](gitmodules-parser.md)                      | ✨ New Feature                         | No .gitmodules parser                                                                                                | Full submodule dependency extraction                                                        | Complete dependency graphs for projects using submodules                      |
| [Copyright Detection](copyright-detection.md)           | 🐛 Bug Fix + 🔍 Enhanced + 🛡️ Security | Year range stops at 2039, short-year typo, French/Spanish case bugs, string-based POS tags, global mutable singleton | Year range 2099, all regex bugs fixed, type-safe enum POS tags, thread-safe `LazyLock`      | Correct year detection, reliable i18n, compile-time safety, parallel scanning |
| [Email/URL Detection](email-url-detection.md)           | 🐛 Bug Fix + 🔍 Enhanced + 🛡️ Security | TLD length too strict, IPv6/private-IP issues, less explicit URL handling                                            | Extended TLD support, robust host/IP filtering, credential stripping, local golden fixtures | Better extraction correctness and stable regression coverage                  |
| Cross-cutting (All Parsers)                             | 🛡️ Security                            | No DoS limits                                                                                                        | File size + iteration limits                                                                | Protection against resource exhaustion                                        |

## Per-Improvement Documentation

Each area with improvements has a dedicated document:

- **[alpine-parser.md](alpine-parser.md)** — 🐛 Bug Fix + ✨ Feature: SHA1 decoding fix + Provider field extraction
- **[rpm-parser.md](rpm-parser.md)** — ✨ New Feature: Dependency extraction with version constraints
- **[debian-parser.md](debian-parser.md)** — ✨ New Feature: .deb archive introspection
- **[conan-parser.md](conan-parser.md)** — ✨ New Feature: conanfile.txt and conan.lock parsers (Python has neither)
- **[cpan-parser.md](cpan-parser.md)** — ✨ New Feature: Full META.json, META.yml, MANIFEST parsing (Python has stubs only)
- **[gradle-parser.md](gradle-parser.md)** — 🛡️ Security Improvement: Token-based lexer instead of Groovy engine (no code execution)
- **[gradle-lockfile-parser.md](gradle-lockfile-parser.md)** — ✨ New Feature: gradle.lockfile dependency extraction (Python has no equivalent)
- **[npm-workspace-parser.md](npm-workspace-parser.md)** — ✨ New Feature: pnpm-workspace.yaml metadata extraction (Python has stub only)
- **[composer-parser.md](composer-parser.md)** — 🔍 Enhanced Extraction: 7 additional extra_data fields for package provenance
- **[ruby-parser.md](ruby-parser.md)** — 🔍 Enhanced Extraction: Semantic Party model combining name and email
- **[dart-parser.md](dart-parser.md)** — 🔍 Enhanced Extraction: Proper scope handling + YAML trailing newline preservation
- **[os-release-parser.md](os-release-parser.md)** — 🐛 Bug Fix + 🔍 Enhanced: Debian name logic fix + URL extraction (HOME, SUPPORT, BUG)
- **[conan-data-parser.md](conan-data-parser.md)** — 🔍 Enhanced Extraction: Patches metadata + mirror/fallback URL extraction
- **[cpan-dist-ini-parser.md](cpan-dist-ini-parser.md)** — ✨ New Feature: Full dist.ini parsing (Python has stub only)
- **[swift-show-dependencies-parser.md](swift-show-dependencies-parser.md)** — 🔍 Enhanced Extraction: Full dependency graph with versions and direct/transitive marking
- **[rpm-specfile-parser.md](rpm-specfile-parser.md)** — ✨ New Feature: Full RPM spec preamble parsing (Python is stub with TODO)
- **[cpan-makefile-pl-parser.md](cpan-makefile-pl-parser.md)** — ✨ New Feature: Makefile.PL WriteMakefile extraction (Python has no parse method)
- **[osgi-manifest-parser.md](osgi-manifest-parser.md)** — ✨ New Feature: OSGi bundle metadata extraction (Python has empty patterns)
- **[maven-parser.md](maven-parser.md)** — 🔍 Enhanced Extraction: SCM field separation, inception_year, consistent extra_data keys
- **[npm-git-url-dependencies.md](npm-git-url-dependencies.md)** — 🐛 Bug Fix: Correct handling of Git URLs, GitHub shortcuts, and local paths (Python treats them as pinned versions)
- **[gitmodules-parser.md](gitmodules-parser.md)** — ✨ New Feature: Git submodule dependency extraction (Python has no equivalent parser)
- **[copyright-detection.md](copyright-detection.md)** — 🐛 Bug Fix + 🔍 Enhanced + 🛡️ Security: Year range fix, regex typo fixes, type-safe POS tags, thread-safe design
- **[email-url-detection.md](email-url-detection.md)** — 🐛 Bug Fix + 🔍 Enhanced + 🛡️ Security: Email/URL extraction hardening, scanner/CLI integration, and local golden fixture ownership

## Verification

All improvements are:

- ✅ **Validated against real-world data** - Tested with actual package files
- ✅ **Covered by tests** - Unit tests + golden tests where applicable
- ✅ **Documented** - Code comments explain the improvement
- ✅ **Compared with Python** - Explicit comparison showing what changed

## Contributing Improvements

When implementing a parser or detection subsystem, if you discover:

1. **A bug in Python**: Fix it in Rust, document here
2. **A TODO in Python**: Implement it in Rust, document here
3. **Missing extraction**: Add it in Rust, document here

**Template for new improvement docs**: See existing files for structure.

## Related Documentation

- [ADR 0002: Extraction vs Detection Separation](../adr/0002-extraction-vs-detection.md) - Why we separate concerns better than Python
- [ADR 0004: Security-First Parsing](../adr/0004-security-first-parsing.md) - Our security advantages over Python
- [SUPPORTED_FORMATS.md](../SUPPORTED_FORMATS.md) - Auto-generated list of all supported formats (run `cargo run --bin generate-supported-formats`)
