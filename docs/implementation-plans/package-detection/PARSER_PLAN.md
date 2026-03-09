# Parser Parity Plan

> **Updated**: March 4, 2026
> **Status**: 🟢 **COMPLETE** — Planned production parser/recognizer coverage is implemented. Deferred and future-scope items are documented below.

## Current State

Production parser coverage is complete for this plan's scope. Deferred/future-scope items are currently limited to low-priority Windows binary deep parsing.

---

## Implemented Ecosystems

All production handlers in plan scope are covered. Some ecosystems consolidate multiple Python handlers into fewer Rust parsers by design.

| Ecosystem           | Coverage       | Notes                                                                                          |
| ------------------- | -------------- | ---------------------------------------------------------------------------------------------- |
| AboutCode           | ✅ Implemented | `AboutFileParser`                                                                              |
| Alpine              | ✅ Implemented | .apk archive + installed DB. APKBUILD not implemented (Python is stub only).                   |
| Autotools           | ✅ Implemented | `AutotoolsConfigureParser`                                                                     |
| Bazel               | ✅ Implemented | `BazelBuildParser`                                                                             |
| Bower               | ✅ Implemented | `BowerJsonParser`                                                                              |
| Buck                | ✅ Implemented | `BuckBuildParser`, `BuckMetadataBzlParser`                                                     |
| Cargo/Rust          | ✅ Implemented | `CargoParser`, `CargoLockParser` + workspace assembly                                          |
| Chef                | ✅ Implemented | metadata.rb, metadata.json. Cookbook .tgz not implemented (archive).                           |
| CocoaPods           | ✅ Implemented | Podfile, Podfile.lock, .podspec, .podspec.json                                                 |
| Conan               | ✅ Implemented | ⭐ Beyond parity: added `conanfile.txt`, `conan.lock`, `conandata.yml`                         |
| Conda               | ✅ Implemented | conda-meta JSON, environment.yaml, meta.yaml                                                   |
| CPAN                | ✅ Implemented | ⭐ Beyond parity: META.json, META.yml, MANIFEST, dist.ini, Makefile.PL (Python has stubs only) |
| CRAN/R              | ✅ Implemented | `CranParser`                                                                                   |
| Dart/Pub            | ✅ Implemented | pubspec.yaml, pubspec.lock                                                                     |
| Debian              | ✅ Implemented | Includes ⭐ .deb introspection, copyright, distroless, md5sums variants                        |
| FreeBSD             | ✅ Implemented | `FreebsdCompactManifestParser`                                                                 |
| Git submodules      | ✅ Implemented | `GitmodulesParser`                                                                             |
| Go                  | ✅ Implemented | go.mod, go.sum, Godeps.json                                                                    |
| Gradle              | ✅ Implemented | ⭐ Beyond parity: added `GradleLockfileParser`                                                 |
| Haxe                | ✅ Implemented | `HaxeParser`                                                                                   |
| Linux Distro        | ✅ Implemented | `OsReleaseParser` ⭐ fixes name logic bug + extracts URLs                                      |
| Maven/Java          | ✅ Implemented | pom.xml, MANIFEST.MF, ⭐ OSGi metadata + SCM/CI/issue management                               |
| npm/yarn/pnpm       | ✅ Implemented | Consolidates v1/v2 yarn, shrinkwrap variants + workspace assembly                              |
| NuGet               | ✅ Implemented | ⭐ Beyond parity: added `PackagesConfigParser`                                                 |
| OCaml/opam          | ✅ Implemented | `OpamParser`                                                                                   |
| PHP/Composer        | ✅ Implemented | `ComposerJsonParser`, `ComposerLockParser` ⭐ extra provenance fields                          |
| Python/PyPI         | ✅ Implemented | Consolidates many handlers, includes pip-inspect                                               |
| README              | ✅ Implemented | `ReadmeParser`                                                                                 |
| RPM                 | ✅ Implemented | ⭐ Specfile, DB (3 variants), license files, Mariner, archive                                  |
| Ruby                | ✅ Implemented | Gemspec, Gemfile, lockfile, .gem archive, extracted metadata                                   |
| Swift               | ✅ Implemented | Package.resolved, Package.swift.json, ⭐ full dependency graph                                 |
| Windows Update      | ✅ Implemented | `MicrosoftUpdateManifestParser`                                                                |
| misc.py recognizers | ✅ Implemented | All recognizers implemented, including magic byte detection                                    |

---

## Deferred: Windows Binary Formats

These require specialized crates and have low ROI. Even Python doesn't fully parse most of them. Deferred unless user demand.

| Handler                    | Format              | Challenge                               | Priority |
| -------------------------- | ------------------- | --------------------------------------- | -------- |
| `MsiInstallerHandler`      | `*.msi`             | OLE Compound Document binary format     | Low      |
| `WindowsExecutableHandler` | `*.exe`, `*.dll`    | PE binary format, VERSION_INFO resource | Low      |
| Win Registry handlers      | Registry hive files | Binary registry format                  | Low      |

### Out of Scope

| Handler                      | Reason                                                                       |
| ---------------------------- | ---------------------------------------------------------------------------- |
| `PypiSdistArchiveHandler`    | Requires archive extraction, permanently out of scope (see ASSEMBLY_PLAN.md) |
| `ChefCookbookTarballHandler` | Requires archive extraction                                                  |
| `AlpineApkbuildHandler`      | Python implementation is a stub only                                         |

---

## Beyond Parity

Features where Rust exceeds the Python original. Documented in detail at `docs/improvements/`.

| Feature                                                             | Python equivalent                     |
| ------------------------------------------------------------------- | ------------------------------------- |
| `ConanfileTxtParser`, `ConanLockParser`                             | None                                  |
| `ConanDataParser` patch/mirror metadata                             | Only primary source URL               |
| `PackagesConfigParser` (NuGet)                                      | None                                  |
| `GradleLockfileParser`                                              | None                                  |
| `NpmWorkspaceParser` (pnpm-workspace.yaml)                          | NonAssemblable only                   |
| npm/pnpm workspace exclusion patterns + sibling cleanup             | No exclusion support                  |
| Cargo workspace `[workspace.package]` inheritance                   | Basic assembly                        |
| `DebianCopyrightParser` (standalone DEP-5)                          | Inline in assemble phase              |
| CPAN full metadata (META.json/yml, MANIFEST, dist.ini, Makefile.PL) | Python has stub-only handlers         |
| Alpine SHA1 Q1-prefixed base64 decoding                             | Python returns `null` (bug)           |
| Alpine `p:` providers field                                         | Python: "not used yet"                |
| RPM full dependency extraction                                      | Python: `# TODO: add dependencies!!!` |
| Debian .deb control.tar.gz extraction                               | Python: `# TODO: introspect archive`  |
| RPM specfile full preamble parsing                                  | Python: stub                          |
| OSGi MANIFEST.MF metadata extraction                                | Python: empty path_patterns           |
| OS release name logic fix + URL extraction                          | Python: bug + no URLs                 |
| Composer extra provenance fields (7 fields)                         | Basic extraction                      |
| Ruby semantic Party model (name+email)                              | String-based                          |
| Dart proper scope handling + YAML preservation                      | Scope always null                     |
| Swift full dependency graph with versions                           | Root package name only                |
| Gradle: custom lexer (no code execution)                            | Groovy engine                         |
| npm `is_private` from `private` field                               | Supported (was missing, now fixed)    |

---

## Future: Missing purl-spec Ecosystems

The following ecosystems are defined in the [purl-spec types index](https://github.com/package-url/purl-spec/blob/main/purl-types-index.json) but are not handled by either ScanCode Python or Rust. These represent potential future improvements.

| purl type          | Ecosystem     | Manifest files / detection signals                   | Priority |
| ------------------ | ------------- | ---------------------------------------------------- | -------- |
| `docker`           | Docker/OCI    | `Dockerfile`, `docker-compose.yml`, image manifests  | High     |
| `hex`              | Elixir/Erlang | `mix.exs`, `mix.lock`                                | Medium   |
| `hackage`          | Haskell       | `*.cabal`, `cabal.project`, `stack.yaml`             | Medium   |
| `swid`             | SWID tags     | `*.swidtag` (ISO 19770-2 XML)                        | Medium   |
| `julia`            | Julia         | `Project.toml`, `Manifest.toml`                      | Low      |
| `luarocks`         | Lua           | `*.rockspec`, `.luarocks/config.lua`                 | Low      |
| `alpm`             | Arch Linux    | `PKGBUILD`, pacman DB entries                        | Low      |
| `yocto`            | Yocto/OE      | BitBake recipes (`*.bb`, `*.bbappend`)               | Low      |
| `huggingface`      | HuggingFace   | `config.json` (model cards), no standard manifest    | Low      |
| `oci`              | OCI images    | Image manifests (overlaps with `docker`)             | Low      |
| `bitnami`          | Bitnami       | Bitnami catalog metadata                             | Low      |
| `mlflow`           | MLflow        | MLmodel files, model registry API                    | Low      |
| `otp`              | Erlang/OTP    | `*.app.src`, `rebar.config` (overlaps with `hex`)    | Low      |
| `bitbucket`        | Bitbucket     | URL-based identification only (no manifest)          | Low      |
| `generic`          | Generic       | Catch-all type, not parseable                        | N/A      |
| `qpkg`             | QNAP NAS      | Proprietary format, very niche                       | N/A      |
| `vscode-extension` | VS Code       | `package.json` with `engines.vscode` (subset of npm) | N/A      |

High-priority candidates (`docker`, `hex`, `hackage`) have well-defined manifest formats and broad adoption.

---

## Quality Gates

Every new handler must satisfy:

1. **Code quality**: Zero clippy warnings, `cargo fmt` clean, no `.unwrap()` in library code
2. **Testing**: Unit tests covering happy path + edge cases + malformed input
3. **Registration**: Added to `register_package_handlers!` macro in `src/parsers/mod.rs`
4. **Documentation**: `register_parser!` macro in parser file for SUPPORTED_FORMATS.md auto-generation
5. **Parity validation**: Output compared against Python reference for same test files
6. **Beyond-parity**: If fixing Python bugs or implementing Python TODOs, document in `docs/improvements/`

---

## References

- **Python reference codebase**: `reference/scancode-toolkit/src/packagedcode/`
- **How to add a parser**: `docs/HOW_TO_ADD_A_PARSER.md`
- **Architecture**: `docs/ARCHITECTURE.md`
- **Beyond-parity docs**: `docs/improvements/`
