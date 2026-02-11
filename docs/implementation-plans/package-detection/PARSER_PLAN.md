# Parser Parity Plan

> **Updated**: February 11, 2026
> **Status**: ~98% parity — only complex binary formats remain

## Current State

79 Rust parsers covering 31 ecosystems. Phases 1–4 complete. Only Phase 5 (complex/binary formats) remains.

---

## Implemented Ecosystems

All production handlers covered. Some consolidate multiple Python handlers into fewer Rust parsers (by design).

| Ecosystem | Python Handlers | Rust Parsers | Notes |
|-----------|----------------|--------------|-------|
| AboutCode | 1 | 1 | `AboutFileParser` |
| Alpine | 4 | 4 | Includes ⭐ APKBUILD (Python stub) |
| Autotools | 1 | 1 | `AutotoolsConfigureParser` |
| Bazel | 1 | 1 | `BazelBuildParser` |
| Bower | 1 | 1 | `BowerJsonParser` |
| Buck | 2 | 2 | `BuckBuildParser`, `BuckMetadataBzlParser` |
| Cargo/Rust | 2 | 2 | `CargoParser`, `CargoLockParser` |
| Chef | 3 | 3 | metadata.rb, metadata.json, ⭐ cookbook .tgz |
| CocoaPods | 4 | 4 | Podfile, Podfile.lock, .podspec, .podspec.json |
| Conan | 2 | 4 | ⭐ Beyond parity: added `conanfile.txt`, `conan.lock` |
| Conda | 3 | 3 | conda-meta JSON, environment.yaml, meta.yaml |
| CPAN | 2 | 2 | ⭐ Both beyond parity (Python has stubs only) |
| CRAN/R | 1 | 1 | `CranParser` |
| Dart/Pub | 2 | 2 | pubspec.yaml, pubspec.lock |
| Debian | 6 | 6 | Includes ⭐ .deb introspection, md5sums variants |
| FreeBSD | 1 | 1 | `FreebsdCompactManifestParser` |
| Go | 3 | 3 | go.mod, go.sum, Godeps.json |
| Gradle | 1 | 2 | ⭐ Beyond parity: added `GradleLockfileParser` |
| Haxe | 1 | 1 | `HaxeParser` |
| Linux Distro | 1 | 1 | `EtcOsReleaseParser` |
| Maven/Java | 3 | 3 | pom.xml, MANIFEST.MF, ⭐ OSGi metadata |
| npm/yarn/pnpm | 8 | 5 | Consolidates v1/v2 yarn, shrinkwrap variants |
| NuGet | 3 | 4 | ⭐ Beyond parity: added `PackagesConfigParser` |
| OCaml/opam | 1 | 1 | `OpamParser` |
| PHP/Composer | 2 | 2 | `ComposerJsonParser`, `ComposerLockParser` |
| Python/PyPI | ~13 | 6 | Consolidates many handlers, includes pip-inspect |
| README | 1 | 1 | `ReadmeParser` |
| RPM | 4 | 4 | ⭐ Specfile beyond parity, license files, Mariner |
| Ruby | 7 | 7 | Gemspec, Gemfile, lockfile, extracted gem variants |
| Swift | 3 | 3 | Package.resolved, Package.swift.json, deplock |
| Windows Update | 1 | 1 | `MicrosoftUpdateManifestParser` |
| misc.py recognizers | 23 | 19 | File-type-only (Python: `# TODO: parse me!!!`) |

---

## What's Left: Phase 5 — Complex/Binary Formats

These require substantial new infrastructure and have questionable ROI. Even Python doesn't fully parse most of them.

| Handler | Format | Challenge | Priority |
|---------|--------|-----------|----------|
| `PypiSdistArchiveHandler` | `*.tar.gz`, `*.zip` | Archive extraction + parse contents | Low |
| `MsiInstallerHandler` | `*.msi` | OLE Compound Document binary format | Low |
| `WindowsExecutableHandler` | `*.exe`, `*.dll` | PE binary format, VERSION_INFO resource | Low |
| Win Registry handlers (3) | Registry hive files | Binary registry format | Low |

**Skipped recognizers** (4 of 23): InstallShieldPackageHandler, NsisInstallerHandler, SquashfsImageHandler (need magic bytes), AndroidAppArchiveHandler (conflicts with AlpineApkParser).

**Recommendation**: Defer unless user demand. The binary Windows formats require specialized crates. `PypiSdistArchive` requires archive extraction infrastructure (same blocker as assembly Phase 4).

---

## Beyond Parity

Features where Rust exceeds the Python original. Documented in detail at `docs/improvements/`.

| Feature | Python equivalent |
|---------|-------------------|
| `ConanfileTxtParser`, `ConanLockParser` | None |
| `PackagesConfigParser` (NuGet) | None |
| `GradleLockfileParser` | None |
| `NpmWorkspaceParser` (pnpm-workspace.yaml) | NonAssemblable only |
| `DebianCopyrightParser` (standalone DEP-5) | Inline in assemble phase |
| CPAN full metadata extraction | Python has stub-only handlers |
| Alpine SHA1 Q1-prefixed base64 decoding | Python returns `null` (bug) |
| Alpine `p:` providers field | Python: "not used yet" |
| RPM full dependency extraction | Python: `# TODO: add dependencies!!!` |
| Debian .deb control.tar.gz extraction | Python: `# TODO: introspect archive` |
| RPM specfile full preamble parsing | Python: stub |
| OSGi MANIFEST.MF metadata extraction | Python: empty path_patterns |

---

## Quality Gates

Every new handler must satisfy:

1. **Code quality**: Zero clippy warnings, `cargo fmt` clean, no `.unwrap()` in library code
2. **Testing**: Unit tests covering happy path + edge cases + malformed input
3. **Registration**: Added to `register_package_handlers!` macro in `src/parsers/mod.rs`
4. **Documentation**: `SUPPORTED_FORMATS.md` regenerated (`cargo run --bin generate-supported-formats`)
5. **Parity validation**: Output compared against Python reference for same test files
6. **Beyond-parity**: If fixing Python bugs or implementing Python TODOs, document in `docs/improvements/`

---

## References

- **Python reference codebase**: `reference/scancode-toolkit/src/packagedcode/`
- **How to add a parser**: `docs/HOW_TO_ADD_A_PARSER.md`
- **Architecture**: `docs/ARCHITECTURE.md`
- **Beyond-parity docs**: `docs/improvements/`
