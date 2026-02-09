# Feature Parity Roadmap

> **Updated**: February 10, 2026
> **Status**: Active — living document tracking remaining work toward 100% parser parity

## Current State

| Metric | Value |
|--------|-------|
| Rust parsers registered | 74 structs |
| Formats covered | 60+ file patterns |
| Ecosystems | 28 |
| Tests passing | 1,051 (48 ignored) |
| Clippy warnings | 0 |

## Gap Analysis

Cross-reference of every concrete Python `DatafileHandler` / `NonAssemblableDatafileHandler` against our Rust `PackageParser` implementations. Source: `reference/scancode-toolkit/src/packagedcode/`.

### Legend

- ✅ Implemented in Rust
- 🟡 Partially implemented (some handlers missing)
- ❌ Not implemented
- ⭐ Beyond parity (Rust has features Python lacks)

---

### Fully Implemented Ecosystems (✅)

These ecosystems have all production handlers covered. Some consolidate multiple Python handlers into fewer Rust parsers (by design).

| Ecosystem | Python Handlers | Rust Parsers | Notes |
|-----------|----------------|--------------|-------|
| AboutCode | 1 | 1 | `AboutFileParser` ✅ |
| Autotools | 1 | 1 | `AutotoolsConfigureParser` ✅ |
| Bazel | 1 | 1 | `BazelBuildParser` ✅ |
| Bower | 1 | 1 | `BowerJsonParser` ✅ |
| Buck | 2 | 2 | `BuckBuildParser`, `BuckMetadataBzlParser` ✅ |
| Cargo/Rust | 2 | 2 | `CargoParser`, `CargoLockParser` ✅ |
| CocoaPods | 4 | 4 | Podfile, Podfile.lock, .podspec, .podspec.json ✅ |
| CRAN/R | 1 | 1 | `CranParser` ✅ |
| Dart/Pub | 2 | 2 | pubspec.yaml, pubspec.lock ✅ |
| FreeBSD | 1 | 1 | `FreebsdCompactManifestParser` ✅ |
| Go | 3 | 3 | go.mod, go.sum, Godeps.json ✅ |
| Gradle | 1 | 2 | ⭐ Beyond parity: added `GradleLockfileParser` |
| Haxe | 1 | 1 | `HaxeParser` ✅ |
| npm/yarn/pnpm | 8 | 5 | Consolidates v1/v2 yarn, shrinkwrap variants ✅ |
| NuGet | 3 | 4 | ⭐ Beyond parity: added `PackagesConfigParser` |
| OCaml/opam | 1 | 1 | `OpamParser` ✅ |
| PHP/Composer | 2 | 2 | `ComposerJsonParser`, `ComposerLockParser` ✅ |
| Python/PyPI | ~13 | 5 | Consolidates many handlers into `PythonParser` ✅ |
| Swift | 2 | 2 | Package.resolved, Package.swift.json ✅ |

### Partially Implemented Ecosystems (🟡)

| Ecosystem | Python Handler | Pattern | Rust Status | Effort | Priority |
|-----------|---------------|---------|-------------|--------|----------|
| **Alpine** | `AlpineApkbuildHandler` | `*APKBUILD` | ❌ Missing | High (bash DSL) | Low |
| **Chef** | `ChefCookbookHandler` | `*.tgz` | ❌ Missing | Medium (archive) | Low |
| **Conan** | `ConanDataHandler` | `*/conandata.yml` | ❌ Missing | Low (YAML) | Medium |
| **Conda** | `CondaMetaJsonHandler` | `*conda-meta/*.json` | ❌ Missing | Low (JSON) | Medium |
| **CPAN** | `CpanMakefilePlHandler` | `*/Makefile.PL` | ❌ Missing | High (Perl DSL) | Low |
| **CPAN** | `CpanDistIniHandler` | `*/dist.ini` | ❌ Missing | Low (INI) | Low |
| **Debian** | `DebianMd5sumFilelistInPackageHandler` | md5sums in package | ❌ Missing | Low | Low |
| **Maven** | `JavaOSGiManifestHandler` | OSGi `MANIFEST.MF` | ❌ Missing | Low (extend existing) | Low |
| **PyPI** | `PipInspectDeplockHandler` | `*pip-inspect.deplock` | ❌ Missing | Low (JSON) | Medium |
| **PyPI** | `PypiSdistArchiveHandler` | `*.tar.gz, *.tar.bz2, *.zip` | ❌ Missing | High (archive) | Low |
| **RPM** | `RpmSpecfileHandler` | `*.spec` | ❌ Missing | High (spec DSL) | Medium |
| **RPM** | `RpmMarinerContainerManifestHandler` | `*container-manifest-2` | ❌ Missing | Low (JSON) | Low |
| **RPM** | `RpmLicenseFilesHandler` | license file patterns | ❌ Missing | Low | Low |
| **Ruby** | `GemMetadataArchiveExtractedHandler` | `*/metadata.gz-extract` | ❌ Missing | Medium | Low |
| **Ruby** | `GemspecInExtractedGemHandler` | `*/data.gz-extract/*.gemspec` | ❌ Missing | Low | Low |
| **Ruby** | `GemspecInInstalledVendorBundleSpecificationsHandler` | `*/specifications/*.gemspec` | ❌ Missing | Low | Low |
| **Ruby** | `GemfileInExtractedGemHandler` | `*/data.gz-extract/Gemfile` | ❌ Missing | Low | Low |
| **Ruby** | `GemfileLockInExtractedGemHandler` | `*/data.gz-extract/Gemfile.lock` | ❌ Missing | Low | Low |
| **Swift** | `SwiftShowDependenciesDepLockHandler` | `*swift-show-dependencies.deplock` | ❌ Missing | Low | Low |

### Not Implemented Ecosystems (❌)

| Ecosystem | Handler(s) | Pattern(s) | Effort | Priority |
|-----------|-----------|------------|--------|----------|
| **Linux Distro** | `EtcOsReleaseHandler` | `*etc/os-release`, `*usr/lib/os-release` | Low | High |
| **README** | `ReadmeHandler` | `*README.android`, `*README.chromium`, etc. | Medium | Medium |
| **MSI** | `MsiInstallerHandler` | `*.msi` | Very High (OLE binary) | Low |
| **Windows PE** | `WindowsExecutableHandler` | `*.exe`, `*.dll`, etc. | Very High (PE binary) | Low |
| **Windows Registry** | 3 Docker registry handlers | Registry hive files | Very High (binary) | Low |
| **Windows Update** | `MicrosoftUpdateManifestHandler` | `*.mum` | Low (XML) | Low |

### Not Implemented: `misc.py` NonAssemblable Recognizers (❌)

These are **file-type recognizers only** — Python marks them all `# TODO: parse me!!!`. They tag files as a package type but extract **no metadata**. The Python source says:

> "Various package data file formats to implement."

They are the lowest-priority items since even Python doesn't parse them.

| Handler | Pattern | Type |
|---------|---------|------|
| `JavaJarHandler` | `*.jar` | Java archive |
| `IvyXmlHandler` | `*/ivy.xml` | Java dependency |
| `JavaWarHandler` | `*.war` | Java web archive |
| `JavaWarWebXmlHandler` | `*/WEB-INF/web.xml` | Java web config |
| `JavaEarHandler` | `*.ear` | Java enterprise archive |
| `JavaEarAppXmlHandler` | `*/META-INF/application.xml` | Java enterprise config |
| `Axis2MarModuleXmlHandler` | `*/meta-inf/module.xml` | Axis2 module |
| `Axis2MarArchiveHandler` | `*.mar` | Axis2 archive |
| `JBossSarHandler` | `*.sar` | JBoss service archive |
| `JBossServiceXmlHandler` | `*/meta-inf/jboss-service.xml` | JBoss config |
| `MeteorPackageHandler` | `*/package.js` | Meteor.js |
| `AndroidAppArchiveHandler` | `*.apk` | Android APK |
| `AndroidLibraryHandler` | `*.aar` | Android library |
| `MozillaExtensionHandler` | `*.xpi` | Firefox extension |
| `ChromeExtensionHandler` | `*.crx` | Chrome extension |
| `IosAppIpaHandler` | `*.ipa` | iOS app |
| `CabArchiveHandler` | `*.cab` | Windows cabinet |
| `InstallShieldPackageHandler` | `*.exe` | InstallShield installer |
| `NsisInstallerHandler` | `*.exe` | NSIS installer |
| `SharArchiveHandler` | `*.shar` | Shell archive |
| `AppleDmgHandler` | `*.dmg`, `*.sparseimage` | macOS disk image |
| `IsoImageHandler` | `*.iso`, `*.udf`, `*.img` | Disk image |
| `SquashfsImageHandler` | squashfs | Linux filesystem image |

---

## Beyond Parity (Rust extras Python lacks)

| Rust Parser | What it does | Python equivalent |
|-------------|-------------|-------------------|
| `ConanfileTxtParser` | Parses `conanfile.txt` | None |
| `ConanLockParser` | Parses `conan.lock` | None |
| `PackagesConfigParser` | Parses NuGet `packages.config` | None |
| `GradleLockfileParser` | Parses `gradle.lockfile` | None |
| `NpmWorkspaceParser` | Parses `pnpm-workspace.yaml` with metadata | Python has NonAssemblable only |
| `DebianCopyrightParser` | Standalone DEP-5 parser | Inline in assemble phase |
| CPAN parsers | Full metadata extraction | Python has stub-only handlers |
| Alpine SHA1 | Correctly decodes Q1-prefixed base64 | Python returns `null` (bug) |
| Alpine providers | Extracts `p:` field | Python: "not used yet" |
| RPM dependencies | Full dependency extraction | Python: `# TODO: add dependencies!!!` |
| Debian .deb introspection | Full control.tar.gz extraction | Python: `# TODO: introspect archive` |

---

## Implementation Phases

### Phase 1: Quick Wins — Simple Format Gaps (est. 1–2 days)

Low-effort handlers with straightforward parsing. Each is a small, self-contained task.

| # | Handler | Pattern | Format | Est. Hours |
|---|---------|---------|--------|------------|
| 1 | `EtcOsReleaseHandler` | `*etc/os-release` | key=value pairs | 2–3 |
| 2 | `ConanDataHandler` | `*/conandata.yml` | YAML | 2–3 |
| 3 | `CondaMetaJsonHandler` | `*conda-meta/*.json` | JSON | 2–3 |
| 4 | `PipInspectDeplockHandler` | `*pip-inspect.deplock` | JSON | 2–3 |
| 5 | `RpmMarinerContainerManifestHandler` | `*container-manifest-2` | text/JSON | 2–3 |
| 6 | `SwiftShowDependenciesDepLockHandler` | `*swift-show-dependencies.deplock` | text | 1–2 |
| 7 | `MicrosoftUpdateManifestHandler` | `*.mum` | XML | 2–3 |
| 8 | `CpanDistIniHandler` | `*/dist.ini` | INI | 2–3 |

**Total**: ~16–23 hours

**Acceptance criteria**: Each handler has unit tests, matches Python's `path_patterns`, extracts at least the fields Python extracts (or documents why not), passes clippy, registered in `define_parsers!`.

---

### Phase 2: Medium Effort — DSL & Metadata Gaps (est. 3–5 days)

Handlers requiring custom parsing logic or heuristics.

| # | Handler | Pattern | Challenge | Est. Hours |
|---|---------|---------|-----------|------------|
| 1 | `ReadmeHandler` | `*README.android`, `*README.chromium`, etc. | Heuristic text extraction | 4–6 |
| 2 | `RpmSpecfileHandler` | `*.spec` | RPM spec DSL, macro expansion | 8–12 |
| 3 | `JavaOSGiManifestHandler` | OSGi `MANIFEST.MF` | Extend existing `MavenParser` | 3–4 |
| 4 | `RpmLicenseFilesHandler` | `/usr/share/licenses/*` patterns | Path-based recognition | 2–3 |
| 5 | `CpanMakefilePlHandler` | `*/Makefile.PL` | Perl DSL regex extraction | 4–6 |

**Total**: ~21–31 hours

**Note on `ReadmeHandler`**: Python only handles 5 specific README variants (`README.android`, `README.chromium`, `README.facebook`, `README.google`, `README.thirdparty`) — these are third-party attribution files, not general READMEs. Each has structured key-value content.

**Note on `RpmSpecfileHandler`**: Python marks this as `NonAssemblable` — it recognizes `.spec` files but does minimal extraction. We could match that minimal behavior first and enhance later.

---

### Phase 3: Extracted Archive Variants (est. 2–3 days)

These handle files found inside extracted archives (e.g., extracted `.gem` files). They reuse existing parser logic with different path patterns.

| # | Handler | Pattern | Based On |
|---|---------|---------|----------|
| 1 | `GemMetadataArchiveExtractedHandler` | `*/metadata.gz-extract` | New (YAML metadata) |
| 2 | `GemspecInExtractedGemHandler` | `*/data.gz-extract/*.gemspec` | `GemspecParser` |
| 3 | `GemspecInInstalledVendorBundleSpecificationsHandler` | `*/specifications/*.gemspec` | `GemspecParser` |
| 4 | `GemfileInExtractedGemHandler` | `*/data.gz-extract/Gemfile` | `GemfileParser` |
| 5 | `GemfileLockInExtractedGemHandler` | `*/data.gz-extract/Gemfile.lock` | `GemfileLockParser` |
| 6 | `DebianMd5sumFilelistInPackageHandler` | md5sums variant | `DebianInstalledMd5sumsParser` |

**Total**: ~12–16 hours

**Implementation approach**: Most of these just need expanded `is_match()` patterns on existing parsers, or thin wrappers that delegate to existing extraction logic.

---

### Phase 4: NonAssemblable Recognizers (est. 2–3 days)

The 23 handlers from `misc.py`. Python marks all of them `# TODO: parse me!!!` — they only recognize file types by extension/pattern with no metadata extraction. Implementation is trivial: each is an `is_match()` + a `PackageData` with just `package_type` set.

**Implementation approach**: A single batch implementation. Could use a table-driven approach:

```rust
// Example: define all recognizers from a static table
struct FileTypeRecognizer {
    package_type: &'static str,
    patterns: &'static [&'static str],
    description: &'static str,
}
```

**Total**: ~12–16 hours for all 23

---

### Phase 5: Complex/Binary Formats (est. weeks, optional)

These require substantial new infrastructure and have questionable ROI.

| Handler | Format | Challenge | Est. Hours |
|---------|--------|-----------|------------|
| `AlpineApkbuildHandler` | `*APKBUILD` | Bash variable expansion, needs bash parser | 16–24 |
| `ChefCookbookHandler` | `*.tgz` | Archive extraction + metadata.json inside | 8–12 |
| `PypiSdistArchiveHandler` | `*.tar.gz`, `*.zip` | Source dist archives, extract setup.py/pyproject.toml | 12–16 |
| `MsiInstallerHandler` | `*.msi` | OLE Compound Document binary format | 20–30 |
| `WindowsExecutableHandler` | `*.exe`, `*.dll` | PE binary format, VERSION_INFO resource | 20–30 |
| Win Registry handlers (3) | Registry hive files | Binary registry format | 30–40 |

**Total**: ~106–152 hours

**Recommendation**: Defer unless user demand. The binary Windows formats (`MSI`, `PE`, `Registry`) require specialized crates and extensive testing. `APKBUILD` requires a bash parser. `PypiSdistArchive` requires extracting archives and then parsing the files inside (Python does this in the assembly phase, not the parser).

---

## Effort Summary

| Phase | Handlers | Est. Hours | Cumulative Parity |
|-------|----------|------------|-------------------|
| Current state | 74 parsers | — | ~63% of Python handlers |
| Phase 1: Quick Wins | +8 | 16–23 | ~70% |
| Phase 2: Medium Effort | +5 | 21–31 | ~74% |
| Phase 3: Archive Variants | +6 | 12–16 | ~79% |
| Phase 4: Recognizers | +23 | 12–16 | ~99% |
| Phase 5: Complex/Binary | +9 | 106–152 | 100% |
| **Total to 99%** | **+42** | **~61–86 hours** | |
| **Total to 100%** | **+51** | **~167–238 hours** | |

**Key insight**: Phases 1–4 get us to ~99% parity in ~61–86 hours. The last 1% (Phase 5 binary formats) costs more than all other phases combined. These binary formats have no metadata extraction even in Python (`# TODO: parse me!!!` or require external libraries like `container_inspector`).

---

## Quality Gates

Every new handler must satisfy:

1. **Code quality**: Zero clippy warnings, `cargo fmt` clean, no `.unwrap()` in library code
2. **Testing**: Unit tests covering happy path + edge cases + malformed input
3. **Registration**: Added to `define_parsers!` macro in `src/parsers/mod.rs`
4. **Documentation**: `SUPPORTED_FORMATS.md` regenerated (`cargo run --bin generate-supported-formats`)
5. **Parity validation**: Output compared against Python reference for same test files
6. **Beyond-parity**: If fixing Python bugs or implementing Python TODOs, document in `docs/improvements/`

---

## References

- **Python reference codebase**: `reference/scancode-toolkit/src/packagedcode/`
- **How to add a parser**: [`docs/HOW_TO_ADD_A_PARSER.md`](HOW_TO_ADD_A_PARSER.md)
- **Architecture**: [`docs/ARCHITECTURE.md`](ARCHITECTURE.md)
- **ADRs**: [`docs/adr/`](adr/)
- **Beyond-parity docs**: [`docs/improvements/`](improvements/)
