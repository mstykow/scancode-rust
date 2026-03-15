# Parser Parity Plan

> **Updated**: March 15, 2026
> **Status**: 🟢 Planned production parser/recognizer coverage is implemented; current GitHub-driven follow-up opportunities are tracked below.

## Current State

Production parser coverage for the original parity scope is implemented. The active parser roadmap is now the **open GitHub issue backlog for net-new parsers and post-parity parser expansions**, not missing work from the original parity campaign.

This document keeps the high-level coverage map, but now also records which open parser issues appear to create the most value. GitHub remains the source of truth for issue state and closure; this document captures prioritization, overlap with current Rust coverage, and roadmap notes.

### Value Signals Used For Triage

An open parser issue creates more value when it:

- extends a **widely used ecosystem** already common in real-world scans,
- adds a **standard manifest / lockfile / workspace file** with clear semantics,
- builds on an **existing parser family** or assembly path already present in Rust,
- improves **package identity and dependency graph quality**, not just extra metadata, and
- shows **active upstream contributor demand**, such as open `aboutcode-org/scancode-toolkit` PRs for the same format,
- avoids narrow binary reverse-engineering work with low evidence of user demand.

### How To Use This Plan In Future PRs

This file is intended to work incrementally.

- Treat each roadmap row as a **work unit** for one PR or a small sequence of related PRs.
- If a PR fully closes the GitHub issues named in a row, **remove that row from the opportunity tables** and update any affected coverage notes elsewhere in this file.
- If a PR only closes part of a row, **shrink the issue set and notes to the remaining open work** rather than adding historical status text.
- Keep GitHub as the system of record for open/closed state; this plan should describe the **remaining backlog**, not a changelog of completed cleanup.
- If a PR adds beyond-parity behavior or fixes a Python bug, document that in `docs/improvements/` rather than expanding this plan.

---

## Implemented Ecosystems

All production handlers in the original plan scope are covered. Some ecosystems consolidate multiple Python handlers into fewer Rust parsers by design.

| Ecosystem             | Coverage       | Notes                                                                                    |
| --------------------- | -------------- | ---------------------------------------------------------------------------------------- |
| AboutCode             | ✅ Implemented | `AboutFileParser`                                                                        |
| Alpine                | ✅ Implemented | `.apk` archive + installed DB + `APKBUILD` recipe parsing                                |
| Autotools             | ✅ Implemented | `AutotoolsConfigureParser`                                                               |
| Bazel                 | ✅ Implemented | `BazelBuildParser`, `BazelModuleParser`                                                  |
| Bower                 | ✅ Implemented | `BowerJsonParser`                                                                        |
| Buck                  | ✅ Implemented | `BuckBuildParser`, `BuckMetadataBzlParser`                                               |
| Cargo/Rust            | ✅ Implemented | `CargoParser`, `CargoLockParser` + workspace assembly                                    |
| Chef                  | ✅ Implemented | `metadata.rb`, `metadata.json`. Cookbook `.tgz` not implemented (archive).               |
| CocoaPods             | ✅ Implemented | `Podfile`, `Podfile.lock`, `.podspec`, `.podspec.json`                                   |
| Conan                 | ✅ Implemented | ⭐ Beyond parity: added `conanfile.txt`, `conan.lock`, `conandata.yml`                   |
| Conda                 | ✅ Implemented | `conda-meta` JSON, `environment.yml`, `meta.yaml`                                        |
| CPAN                  | ✅ Implemented | ⭐ Beyond parity: `META.json`, `META.yml`, `MANIFEST`, `dist.ini`, `Makefile.PL`         |
| CRAN/R                | ✅ Implemented | `CranParser`                                                                             |
| Dart/Pub              | ✅ Implemented | `pubspec.yaml`, `pubspec.lock`                                                           |
| Deno                  | ✅ Implemented | `deno.json`, `deno.jsonc`, `deno.lock`                                                   |
| Debian                | ✅ Implemented | Includes ⭐ `.deb` introspection, copyright, distroless, `md5sums` variants              |
| Docker                | ✅ Implemented | `Dockerfile`, `Containerfile`, OCI label extraction                                      |
| FreeBSD               | ✅ Implemented | `FreebsdCompactManifestParser`                                                           |
| Git submodules        | ✅ Implemented | `GitmodulesParser`                                                                       |
| Go                    | ✅ Implemented | `go.mod`, `go.sum`, `Godeps.json`, `go.mod.graph`, `go.work`                             |
| Gradle                | ✅ Implemented | ⭐ Beyond parity: added `GradleLockfileParser` and `GradleModuleParser`                  |
| Haxe                  | ✅ Implemented | `HaxeParser`                                                                             |
| Linux Distro          | ✅ Implemented | `OsReleaseParser` ⭐ fixes name logic bug + extracts URLs                                |
| Maven/Java            | ✅ Implemented | `pom.xml`, `MANIFEST.MF`, ⭐ OSGi metadata + SCM/CI/issue management                     |
| npm/yarn/pnpm         | ✅ Implemented | Consolidates v1/v2 yarn, shrinkwrap variants + workspace assembly                        |
| NuGet                 | ✅ Implemented | ⭐ Beyond parity: added `PackagesConfigParser` and `DotNetDepsJsonParser`                |
| OCaml/opam            | ✅ Implemented | `OpamParser`                                                                             |
| PHP/Composer          | ✅ Implemented | `ComposerJsonParser`, `ComposerLockParser` ⭐ extra provenance fields                    |
| Python/PyPI           | ✅ Implemented | Consolidates many handlers, includes `pip-inspect.deplock`, `uv.lock`, and `pylock.toml` |
| README                | ✅ Implemented | `ReadmeParser`                                                                           |
| RPM                   | ✅ Implemented | ⭐ Specfile, DB (3 variants), license files, Mariner, archive                            |
| Ruby                  | ✅ Implemented | Gemspec, Gemfile, lockfile, `.gem` archive, extracted metadata                           |
| Swift                 | ✅ Implemented | `Package.resolved`, `Package.swift.json`, ⭐ full dependency graph                       |
| Windows Update        | ✅ Implemented | `MicrosoftUpdateManifestParser`                                                          |
| `misc.py` recognizers | ✅ Implemented | All recognizers implemented, including magic byte detection                              |

---

## Open Parser Opportunities From GitHub

The tables below classify the current open `package-parsing` / `new-parser` issues by **value created**, not by easiest implementation order. Open upstream Python-reference PR activity is also used as a demand signal when it materially changes priority. Issue bodies were used where they added important context such as standards, adjacent files, or explicit user impact.

### Highest-Value Opportunities

These issues now create the most value overall after balancing ecosystem reach, package/dependency signal quality, reuse of existing Rust parser families, and upstream implementation momentum. Not every row has equally strong upstream PR activity, but all of them now outrank the remaining medium set on overall impact.

| Issue Set  | Opportunity                                                               | Why this creates value                                                                                                                                        | Current overlap / notes                                                                                                                                                                                                                                                                   |
| ---------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #111, #112 | Installed Python distribution metadata (`WHEEL`, pip cache `origin.json`) | Improves installed-package identity, wheel-specific PURL reconstruction, and package-cache provenance in one of the broadest ecosystems scanned in practice.  | Strongest combined signal in the backlog: Python packaging standards make these files ubiquitous in installed environments, Rust already has deep Python parser/file-reference infrastructure, and open upstream PRs `#4776` (`WHEEL`) and `#4708` (`origin.json`) confirm active demand. |
| #333       | Bun lockfiles (`bun.lock`, `bun.lockb`)                                   | Adds resolved dependency evidence for a fast-growing JS/TS package manager while reusing the existing npm-family manifest and assembly model.                 | Promoted despite weak current upstream ScanCode momentum because ecosystem impact is already large and growing: `package.json` is already covered, `bun.lock` is Bun's text default, `bun.lockb` is legacy, and support fits naturally beside npm/yarn/pnpm lockfiles.                    |
| #113       | Arch Linux (`.SRCINFO`, `.PKGINFO`, `.AURINFO`)                           | Adds a high-volume distro package family with explicit metadata files and closes the concrete `alpm`-style gap with source-tree and package metadata signals. | Strong distro-family leverage from existing Alpine/Debian/RPM work, plus meaningful ecosystem size from AUR. Upstream PR `#4603` provides active but partial momentum because it currently covers `.SRCINFO` rather than the whole row.                                                   |

### Medium-Value Opportunities

These are still useful expansions with credible ecosystem reach, but they now sit below the top tier because they have narrower usage, weaker assembly leverage, more format variance, broader multi-format scope, or weaker current upstream momentum. Some grouped rows also likely need future splitting because their effort and payoff are not perfectly uniform.

| Issue Set      | Opportunity                                                                               | Why this creates value                                                                                                                      | Current overlap / notes                                                                                                                                                                                                               |
| -------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #69            | Scala SBT                                                                                 | Covers a common JVM build ecosystem that is currently invisible when projects contain Scala packaging but not enough Maven metadata.        | Duplicate issue #92 was closed; #69 remains the canonical tracker for concrete `.sbt` support.                                                                                                                                        |
| #73            | Meson (`meson.build`)                                                                     | Common in native-code projects and can expose project name, version, license, and dependency declarations in source trees.                  | Likely good ROI because the issue cites explicit `project()` metadata and dependency docs.                                                                                                                                            |
| #82, #83       | Nix / Guix package metadata                                                               | Covers important reproducible-build and Linux packaging ecosystems that show up in source distributions and infrastructure repos.           | High ecosystem reach, especially on the Nix side, but parsing semantics are broader and less standardized than lockfile-style formats; no open upstream PR was visible, and the grouped row likely hides Nix/Guix effort differences. |
| #65            | `symfony.lock`                                                                            | Adds framework-specific dependency state for a widely used PHP stack without requiring a whole new ecosystem parser family.                 | Best treated as Composer-adjacent enrichment; upstream PR `#4651` is open, and the issue notes it is likely very close to existing `composer.lock` semantics.                                                                         |
| #90, #109, #84 | Android metadata and package artifacts (`METADATA`, `.aab`, binary `AndroidManifest.xml`) | Improves Android package identity in source trees and shipped artifacts, especially when Java/Gradle manifests are not enough.              | Good ecosystem value, but this row spans multiple artifact/parser problems with uneven complexity, so sequencing is harder than the top-ranked medium items.                                                                          |
| #63            | vcpkg (`portfile.cmake`, `CONTROL`)                                                       | Expands C/C++ dependency coverage with concrete package-manager metadata used in Windows and cross-platform C++ projects.                   | Moderate leverage, but semantics span multiple file conventions.                                                                                                                                                                      |
| #62            | Yocto / BitBake (`*.bb`)                                                                  | Important for embedded Linux and distro build systems where package metadata lives in recipe files.                                         | Already anticipated in the old purl-gap section; open upstream PR `#3092` exists, but the format still spans high-variance recipe semantics and has stayed review-heavy upstream.                                                     |
| #106           | SPDX and CycloneDX SBOM ingestion                                                         | Lets scans treat already-produced SBOMs as package data, which is valuable for binary-only or artifact-only inputs.                         | Large cross-ecosystem strategic value, but broader and less parser-local than manifest/lockfile additions, and upstream demand is much clearer for SBOM output than SBOM-as-input parsing.                                            |
| #110           | AppStream (`appdata.xml`, `metainfo.xml`)                                                 | Adds package-like metadata common in Linux desktop and distro packaging workflows.                                                          | The issue provides concrete spec and example references, making this more actionable than many metadata-only formats.                                                                                                                 |
| #96            | `buildpack.toml`                                                                          | Adds structured metadata for buildpack ecosystems and platform packaging pipelines.                                                         | Clear format with an open upstream PR `#4031`, but still narrower than mainstream application package managers.                                                                                                                       |
| #64            | PlatformIO `library.json`                                                                 | Adds structured metadata for embedded/IoT dependency scanning.                                                                              | Narrower audience than mainstream package managers, but format is explicit.                                                                                                                                                           |
| #121           | Flatpak / Snap / AppImage-style package formats                                           | Captures package/container distribution formats that appear outside source manifests and can surface package identity in shipped artifacts. | The issue was narrowed on GitHub after confirming `Dockerfile` / `Containerfile` coverage already exists in the Rust codebase.                                                                                                        |
| #95            | Ivy `dependencies.properties`                                                             | Adds another JVM dependency metadata surface for legacy or enterprise Java builds.                                                          | More incremental than transformational because Maven/Gradle coverage already exists.                                                                                                                                                  |
| #103           | LuaRocks `rockspec`                                                                       | Closes a previously documented purl gap with a concrete manifest issue.                                                                     | Still likely lower demand than `hex` / `hackage`, but open upstream PR `#4743` confirms real implementation interest.                                                                                                                 |
| #107           | Cargo auditable metadata in Rust binaries                                                 | Enables package extraction from compiled Rust binaries, not just source manifests.                                                          | High artifact-scan value, but this is deeper binary parsing than ordinary manifest work.                                                                                                                                              |
| #71            | OpenWrt packages                                                                          | Adds package metadata for embedded/router firmware ecosystems.                                                                              | Niche but meaningful in firmware and distro-style scans; upstream PR `#2400` is still open, but the implementation signal is stale and partial.                                                                                       |
| #77            | Carthage `Cartfile`                                                                       | Fills a recognizable Apple ecosystem gap next to the existing Swift package support.                                                        | Lower leverage than SwiftPM because Carthage is narrower and more legacy-skewed now, but still useful.                                                                                                                                |
| #91            | illumos / OmniOS / NetBSD `pkgsrc`                                                        | Extends system-package coverage to additional Unix packaging families.                                                                      | Useful for breadth, but likely narrower demand than Debian/RPM/Alpine.                                                                                                                                                                |
| #76            | PEAR PHP (`package.xml`)                                                                  | Captures legacy PHP package metadata that still appears in older projects and archives.                                                     | Lower leverage than Composer-adjacent work, but the format is explicit.                                                                                                                                                               |
| #87            | Nimble                                                                                    | Adds package support for the Nim ecosystem.                                                                                                 | Useful ecosystem expansion, but smaller install base than the highest-value candidates.                                                                                                                                               |

### Lower-Value Or Opportunistic Opportunities

These issues may still be worth doing, but they are currently lower-value because they are niche, metadata-only, weakly standardized, duplicative of broader families, or likely better handled after higher-signal parser work lands.

| Issue Set            | Opportunity                                                                                              | Why it is lower-value today                                                                                                   | Current overlap / notes                                                                                                       |
| -------------------- | -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| #120                 | Raku packages                                                                                            | Net-new ecosystem, but smaller observed demand than the highest-priority language ecosystems.                                 | Worth keeping on the radar, but not above Python/JS/Go/JVM follow-ons.                                                        |
| #119                 | Doc tags (`phpdoc`, `JSDoc`, `Javadoc`, `Doxygen`)                                                       | These are source annotations, not package manifests, so they create weaker package-identity value than real manifest parsing. | May belong in a metadata-detection track more than the parser roadmap.                                                        |
| #115                 | `.npmrc`                                                                                                 | Useful for registry/config context, but it does not define package identity the way `package.json` or lockfiles do.           | Better treated as npm metadata enrichment than a top parser target, despite open upstream PRs `#4794` and `#4608`.            |
| #114, #72            | Java module/runtime descriptors (`.module`, `JMOD`, `JIMAGE`)                                            | Useful for shipped Java artifacts, but more binary/runtime-oriented than source package parsing.                              | Lower leverage than Maven/Gradle/Bazel/SBT source metadata.                                                                   |
| #108                 | `nx` workspaces                                                                                          | Adds workspace structure inside an ecosystem the repo already covers well via npm/yarn/pnpm.                                  | Helpful, but less valuable than new manifests or lockfiles with dependency semantics.                                         |
| #104, #85, #89       | Structured project metadata (`CITATION.cff`, `codemeta.json`, `publiccode.yml`)                          | These files improve package description and provenance, but usually add less dependency value than manifest/lockfile formats. | Good candidates once the higher-signal dependency formats are done; `CITATION.cff` has open upstream PRs `#4728` and `#3625`. |
| #86, #98, #100, #101 | Vendor / attestation metadata (`cgmanifest.json`, `.asf.yaml`, PostgreSQL `META.json`, `component.json`) | Useful provenance surfaces, but each is narrow and ecosystem-specific.                                                        | Best handled opportunistically when a broader ecosystem batch is active; `component.json` has open upstream PR `#4138`.       |
| #94, #88             | Ecosystem-specific JS/CMS manifests (Unity `package.json`, WordPress plugins)                            | Can add package metadata, but they are subsets or specializations of already-covered families.                                | Value is real but localized; Unity support has an open upstream PR (`#3597`).                                                 |
| #81                  | Scoop                                                                                                    | Adds Windows package-manager metadata, but user demand is likely narrower than cross-platform manifests.                      | Separate from Winget / Windows Package Manager manifests and still tracked independently.                                     |
| #68                  | Windows Package Manager / Winget manifests                                                               | Adds Windows package-manager metadata, but user demand is likely narrower than cross-platform manifests.                      | Duplicate issue #74 was closed after #68 was retitled as the canonical Winget tracker.                                        |
| #80                  | ROS packages                                                                                             | Useful for robotics ecosystems, but not broadly used outside that domain.                                                     | Likely better after stronger general-purpose ecosystems are complete.                                                         |
| #79                  | `datapackage.json`                                                                                       | Structured metadata, but narrower adoption and lighter dependency value.                                                      | More metadata-oriented than dependency-oriented.                                                                              |
| #70                  | DOAP RDF/XML                                                                                             | Rich project metadata, but niche and metadata-first.                                                                          | Better as enrichment work.                                                                                                    |
| #67                  | PEX Python binaries                                                                                      | Valuable for packaged Python artifacts, but artifact parsing is costlier than manifest follow-ons.                            | Lower priority than Python lockfile and metadata improvements.                                                                |
| #66                  | `qt_attribution.json`                                                                                    | Useful attribution data, but specialized to a narrower packaging workflow.                                                    | More provenance than dependency graph.                                                                                        |
| #61                  | SQLite amalgamation                                                                                      | Important for one widely reused library, but too narrow to outrank ecosystem-scale formats.                                   | Feels like a targeted recognizer, not a broad parser family.                                                                  |
| #59                  | `AssemblyInfo.cs`                                                                                        | Provides .NET project metadata, but weaker package/dependency value than `.nuspec`, project files, and `.deps.json`.          | Better as a NuGet/.NET enrichment pass.                                                                                       |
| #58                  | Linux kernel modules (`.ko`)                                                                             | Binary/package artifact parsing with specialized semantics and unclear general demand.                                        | Similar low-ROI profile to other deep binary formats.                                                                         |
| #75                  | Font files (`.ttf`, `.otf`)                                                                              | Broad file presence, but weak package semantics compared with real package manifests.                                         | Better framed as file metadata extraction than parser-priority work.                                                          |

## Deferred: Windows Binary Formats

These require specialized crates and have low ROI. Even Python does not fully parse most of them. Deferred unless user demand becomes concrete.

| Handler                    | Format              | Challenge                               | Priority |
| -------------------------- | ------------------- | --------------------------------------- | -------- |
| `MsiInstallerHandler`      | `*.msi`             | OLE Compound Document binary format     | Low      |
| `WindowsExecutableHandler` | `*.exe`, `*.dll`    | PE binary format, VERSION_INFO resource | Low      |
| Win Registry handlers      | Registry hive files | Binary registry format                  | Low      |

### Out of Scope

| Handler                      | Reason                                                                         |
| ---------------------------- | ------------------------------------------------------------------------------ |
| `PypiSdistArchiveHandler`    | Requires archive extraction, permanently out of scope (see `ASSEMBLY_PLAN.md`) |
| `ChefCookbookTarballHandler` | Requires archive extraction                                                    |

---

## Future: Missing purl-spec Ecosystems Not Yet Backed By Open GitHub Issues

The previous plan listed purl-spec types that neither Python nor Rust handled. Some of those are now covered by explicit GitHub issues and therefore moved into the issue-driven sections above:

- `alpm` → tracked by #113
- `yocto` → tracked by #62
- `luarocks` → tracked by #103

The remaining notable purl types without a dedicated open parser issue are:

| purl type          | Ecosystem     | Manifest files / detection signals              | Priority | Notes                                                                                                                                                     |
| ------------------ | ------------- | ----------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `hex`              | Elixir/Erlang | `mix.exs`, `mix.lock`                           | High     | Clear manifest and lockfile targets make this a strong candidate for a new GitHub issue.                                                                  |
| `hackage`          | Haskell       | `*.cabal`, `cabal.project`, `stack.yaml`        | High     | Clear manifest targets and ecosystem visibility make this a strong candidate for a new GitHub issue.                                                      |
| `swid`             | SWID tags     | `*.swidtag` (ISO 19770-2 XML)                   | Medium   | Strong metadata standard, but narrower real-world demand than `hex` / `hackage`.                                                                          |
| `julia`            | Julia         | `Project.toml`, `Manifest.toml`                 | Medium   | Worth promoting above some niche open issues if user demand appears.                                                                                      |
| `oci`              | OCI images    | OCI manifest and index JSON, image layout files | Medium   | `Dockerfile`/`Containerfile` metadata is implemented, but OCI manifest-style package ingestion no longer has a dedicated open issue after narrowing #121. |
| `huggingface`      | HuggingFace   | `config.json`, model cards, repository metadata | Low      | Manifest conventions are weaker than mainstream package ecosystems.                                                                                       |
| `bitnami`          | Bitnami       | Bitnami catalog metadata                        | Low      | Narrow packaging family.                                                                                                                                  |
| `mlflow`           | MLflow        | `MLmodel` files, registry metadata              | Low      | More model/artifact metadata than classic package parsing.                                                                                                |
| `otp`              | Erlang/OTP    | `*.app.src`, `rebar.config`                     | Low      | Overlaps with the likely higher-value `hex` work.                                                                                                         |
| `bitbucket`        | Bitbucket     | URL-based identification only                   | Low      | No clear manifest target.                                                                                                                                 |
| `generic`          | Generic       | Catch-all type                                  | N/A      | Not meaningfully parseable.                                                                                                                               |
| `qpkg`             | QNAP NAS      | Proprietary format                              | N/A      | Very niche and format-specific.                                                                                                                           |
| `vscode-extension` | VS Code       | `package.json` with `engines.vscode`            | N/A      | Effectively a subset of the existing npm parser family.                                                                                                   |

---

## Additional High-Signal Parser Gaps Not Yet Reflected In The GitHub Backlog

The review that promoted Bun also surfaced several **concrete manifest / lockfile gaps that are not currently represented in the open upstream issue backlog at all**. These are worth tracking here so they do not stay invisible simply because no canonical GitHub issue exists yet.

These candidates were screened to ensure they are:

- not already implemented in `src/parsers/` or `docs/SUPPORTED_FORMATS.md`,
- not already listed elsewhere in this plan,
- backed by a clear manifest or lockfile format rather than loose metadata, and
- large enough in OSS usage to merit at least **high-medium** attention.

### Strongest Newly Identified Gaps

| Opportunity                      | Manifest files / detection signals                          | Priority    | Why it matters now                                                                                                                                                                       | Current overlap / notes                                                                                                                                                                                                                                         |
| -------------------------------- | ----------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Modern vcpkg manifest mode       | `vcpkg.json`, `vcpkg-configuration.json`, `vcpkg-lock.json` | High        | vcpkg now recommends manifest mode for most users, and the JSON manifest family carries clear package identity, dependency, feature, and registry semantics for a large C/C++ ecosystem. | The existing GitHub-tracked vcpkg row only covers legacy `portfile.cmake` / `CONTROL` work. This newer manifest-mode family is a separate and likely higher-value gap that should get its own upstream issue.                                                   |
| NuGet Central Package Management | `Directory.Packages.props`                                  | High        | Central package management is now common in large modern .NET repos and materially improves dependency graph capture by centralizing PackageReference versions.                          | This is a genuine missing piece inside an otherwise strong NuGet family: Rust already supports `*.csproj`, `*.fsproj`, `*.vbproj`, `packages.lock.json`, `project.json`, and `project.lock.json`, but not the central version file that many repos now rely on. |
| Helm charts                      | `Chart.yaml`, `Chart.lock`                                  | High-Medium | Helm charts are first-class packages in the Kubernetes ecosystem, and `Chart.lock` adds explicit dependency-state value rather than just descriptive metadata.                           | Treat this as package parsing, not generic deployment config. The lack of a purl-specific row or upstream issue should not hide its real ecosystem reach across many major OSS repos.                                                                           |
| Pixi                             | `pixi.toml`, `pixi.lock`                                    | High-Medium | Pixi is a fast-growing reproducible environment manager with a crisp manifest+lockfile model and strong adjacency to already-covered Python/Conda package detection work.                | This is the same kind of modern ecosystem follow-on that Bun represented: easy to miss in parity-focused planning even though it is clearly becoming part of real-world repos.                                                                                  |
| Clojure manifests                | `deps.edn`, `project.clj`                                   | High-Medium | These are bona fide dependency manifests for an established JVM ecosystem and would close a JVM-family blind spot not covered by Maven/Gradle/SBT.                                       | Best tracked as one ecosystem row even though it spans the Clojure CLI and Leiningen toolchains. The current roadmap already recognizes JVM follow-ons such as SBT and Ivy, making this omission more noticeable.                                               |

### Next Tier From The Same Review

These also appear to be real parser gaps, but they currently rank below the group above on combined ecosystem size, adjacency, or clarity of package semantics.

| Opportunity       | Manifest files / detection signals | Priority | Notes                                                                                                                                     |
| ----------------- | ---------------------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| R `renv` lockfile | `renv.lock`                        | Medium   | Real reproducible-environment lockfile with clear structure, but narrower and more lockfile-only than the top group.                      |
| Paket             | `paket.dependencies`, `paket.lock` | Medium   | Valid .NET dependency workflow, but secondary in impact compared with `Directory.Packages.props` and the rest of the modern NuGet family. |

When one of these gaps becomes active work, prefer creating a dedicated upstream issue first so the issue-driven sections above can absorb it cleanly in the next backlog refresh.

---

## Quality Gates

Every new handler must satisfy:

1. **Code quality**: Zero clippy warnings, `cargo fmt` clean, no `.unwrap()` in library code
2. **Testing**: Unit tests covering happy path + edge cases + malformed input
3. **Registration**: Added to `register_package_handlers!` macro in `src/parsers/mod.rs`
4. **Documentation**: `register_parser!` macro in parser file for `SUPPORTED_FORMATS.md` auto-generation
5. **Parity validation**: Output compared against Python reference for the same test files
6. **Beyond parity**: If fixing Python bugs or implementing Python TODOs, document in `docs/improvements/`

---

## References

- **GitHub backlog refresh command**: `gh issue list --state open --label package-parsing --limit 200`
- **Python reference codebase**: `reference/scancode-toolkit/src/packagedcode/`
- **How to add a parser**: `docs/HOW_TO_ADD_A_PARSER.md`
- **Architecture**: `docs/ARCHITECTURE.md`
- **Beyond-parity docs**: `docs/improvements/`
