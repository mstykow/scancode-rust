# Package Parser Enhancement Plan

> **Status**: 🟡 Active — ecosystem-by-ecosystem enhancement backlog and execution tracker
> **Updated**: March 12, 2026
> **Dependencies**: [PARSER_PLAN.md](PARSER_PLAN.md), [ASSEMBLY_PLAN.md](ASSEMBLY_PLAN.md), [HOW_TO_ADD_A_PARSER.md](../../HOW_TO_ADD_A_PARSER.md), [TESTING_STRATEGY.md](../../TESTING_STRATEGY.md)

## Purpose

This document is the durable workboard for open package-parser enhancement work. It exists so future sessions can resume the effort without rebuilding the issue inventory, sequencing, or testing expectations from scratch.

## How to Use This Plan

At the start of each session:

1. Read this file first.
2. Pick the next ecosystem marked `Planned` unless priorities changed.
3. Before coding, confirm the issue set is still open with GitHub.
   - Read the full issue body for every issue in the active ecosystem, not just the title or earlier notes.
4. After finishing an ecosystem PR, update this file:
   - mark the ecosystem `Done` once the implementation work, tests, docs, and PR are complete
   - use the detailed scope section to note whether the PR is still open, merged, or followed by later cleanup
   - add any follow-up issues created or intentionally deferred
5. If sequencing changes, update the order here instead of relying on chat history.

## Ground Rules

- One PR per ecosystem family.
- Multiple commits inside a PR are fine if they are atomic and reviewable.
- Do not mix unrelated ecosystems in one PR.
- Do not hide generic refactors inside ecosystem PRs.
- Use Python ScanCode as a behavior/spec reference, not as an implementation blueprint.
- Use the relevant parts of [`docs/HOW_TO_ADD_A_PARSER.md`](../../HOW_TO_ADD_A_PARSER.md) as the checklist for registration, golden tests, assembly, validation, and documentation, even when enhancing an existing parser.
- Add parser golden tests when parser-level parity/regression coverage is meaningful.
- Add assembly golden tests when assembly, workspace, or file-reference behavior changes.
- Every bug fix claim must be backed by tests; prefer a focused failing regression test first, then the code fix, then the relevant verification rerun.
- Before opening an ecosystem PR, re-audit the active issue set against the latest GitHub issue text and confirm each issue has concrete code/test/golden/doc evidence.
- When parser work fixes Python bugs or adds beyond-parity behavior, document it in `docs/improvements/`.
- PRs that fully resolve issues should close them via GitHub closing keywords in the PR body (for example: `Closes #122`).
- Snapshot/expected-output changes require explicit rationale in the PR body.

## Required Touchpoints Per Ecosystem PR

- Parser implementation for the active ecosystem
- Parser registration when new formats or handlers are introduced
- Focused unit tests for the changed parser behavior
- Parser-level golden coverage when output parity/regression coverage is meaningful
- Real fixtures under `testdata/` that exercise the changed behavior
- Datasource coverage when new file-format identities are introduced
- Assembly configuration and golden coverage when package grouping, ownership, workspace, or file-reference behavior changes
- Supported-format documentation when parser coverage or advertised metadata changes
- Improvement documentation when Rust behavior intentionally exceeds or corrects Python behavior

## Session Refresh Commands

Refresh the enhancement backlog:

```bash
gh issue list --state open --label package-parsing --label enhancement --limit 200
```

Refresh one ecosystem only:

```bash
gh issue list --state open --label package-parsing --label enhancement --search '"Improve Maven:" in:title'
```

## Validation Baseline For Every Ecosystem PR

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo test assembly::assemblers::tests::test_every_datasource_id_is_accounted_for --lib
```

```bash
cargo run --quiet --bin generate-supported-formats && git diff --exit-code docs/SUPPORTED_FORMATS.md
```

## Execution Learnings From Completed Batches

- Issue titles are not enough. For the active ecosystem, read every current issue body with `gh issue view <id>` before coding and again before PR creation, because the detailed findings and reference links may have changed.
- Keep an explicit issue-to-evidence matrix while working. An issue is not ready to close unless it maps to exact code paths plus concrete tests, goldens, and docs when applicable.
- Use the smallest reproducible parser fixture possible for bug work. For manifest/lockfile semantics, start with a focused unit test or tiny synthetic fixture; reserve assembly goldens for ownership, merge, workspace, or file-assignment behavior.
- Treat parser gaps and assembly gaps separately. If the issue audit shows the remaining problem is parser-only, do not broaden the fix into assembly refactors just because those files are nearby.
- When a bug fix is believed complete, verify it in two steps: (1) the new targeted regression test proves the behavior, and (2) the ecosystem validation suite still passes.

- Cross-ecosystem parser-golden cleanup PRs are allowed when the work is specifically about removing stale non-license-engine `#[ignore]` coverage gaps. Treat those PRs as coverage-restoration work, not as completion of the ecosystem issue batches below.
- When such a cleanup PR lands, update ecosystem rows only if the underlying GitHub issue set changed; do not mark an ecosystem `Done` merely because its parser goldens were re-enabled.

## Ecosystem Workboard

| Order | Ecosystem  | Status  | Issue Set                                                        | Primary Validation                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ----- | ---------- | ------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | Maven      | Done    | #122, #124, #126, #128, #131, #135, #207, #208, #211, #214       | `cargo test maven`; `cargo test --features golden-tests maven_golden`; `cargo test --features golden-tests test_assembly_maven_basic`; targeted nested `META-INF/maven/**` regression coverage and datasource-accounting validation                                                                                                                                                                                                                                       |
| 2     | npm + Yarn | Done    | #123, #125, #127, #129, #133, #197, #198, #205, #206             | `cargo test npm`; `cargo test yarn`; `cargo test --features golden-tests npm_golden`; `cargo test --features golden-tests test_assembly_npm_basic`; `cargo test --features golden-tests test_assembly_npm_workspace`; `cargo test --features golden-tests test_assembly_pnpm_workspace`; `cargo test --features golden-tests test_assembly_npm_nested_packages`                                                                                                           |
| 3     | NuGet      | Done    | #157, #159, #162, #163, #165, #215, #216                         | `cargo test nuget`; `cargo test --features golden-tests nuget_golden`; `cargo test --features golden-tests test_assembly_nuget_basic`                                                                                                                                                                                                                                                                                                                                     |
| 4     | RPM        | Done    | #164, #166, #167, #168, #169, #170, #171                         | `cargo test rpm`; `cargo test --features golden-tests rpm_golden`; `cargo test test_resolve_rpm_namespace --lib`; `cargo test test_merge_rpm_yumdb_metadata --lib`                                                                                                                                                                                                                                                                                                        |
| 5     | Cargo      | Done    | #184, #189, #217                                                 | `cargo test cargo`; `cargo test --features golden-tests cargo_golden`; `cargo test --features golden-tests test_assembly_cargo_basic`; `cargo test --features golden-tests test_assembly_cargo_workspace`                                                                                                                                                                                                                                                                 |
| 6     | Go         | Done    | #152, #153, #155, #218                                           | `cargo test go`; `cargo test --features golden-tests go_golden`; `cargo test --features golden-tests test_assembly_go_basic`; `cargo test --features golden-tests test_assembly_go_graph_basic`                                                                                                                                                                                                                                                                           |
| 7     | Gradle     | Done    | #130, #132, #134, #137                                           | `cargo test gradle`; `cargo test --features golden-tests gradle_golden`; `cargo test test_cyclonedx_json_includes_component_license_expression --lib`; `cargo test test_skip_template_placeholder_pom_coordinates --lib`                                                                                                                                                                                                                                                  |
| 8     | Ruby       | Done    | #151, #154, #156, #158, #160, #161                               | `cargo test ruby`; `cargo test --features golden-tests ruby_golden`; `cargo test --features golden-tests test_assembly_ruby_extracted_basic`; `cargo test --bin scancode-rust`; `cargo test --test output_format_golden`                                                                                                                                                                                                                                                  |
| 9     | Composer   | Done    | #187, #188, #190                                                 | `cargo test composer`; `cargo test --features golden-tests composer_golden`; `cargo test --features golden-tests test_assembly_composer_basic`; `cargo test --features golden-tests test_assembly_composer_nested`                                                                                                                                                                                                                                                        |
| 10    | Conda      | Done    | #195, #196                                                       | `cargo test conda`; `cargo test --features golden-tests conda_golden`; `cargo test test_assembly_conda_rootfs_assigns_meta_json_files --lib`                                                                                                                                                                                                                                                                                                                              |
| 11    | CocoaPods  | Done    | #191, #192                                                       | `cargo test pod`; `cargo test --features golden-tests cocoapods_golden`                                                                                                                                                                                                                                                                                                                                                                                                   |
| 12    | Alpine     | Done    | #172, #173, #174, #175                                           | `cargo test alpine`; `cargo test --features golden-tests alpine_golden`; `cargo test --features golden-tests test_assembly_alpine_file_refs`                                                                                                                                                                                                                                                                                                                              |
| 13    | ABOUT      | Done    | #201, #202, #203, #204                                           | `cargo test about`; `cargo test --features golden-tests about --lib`; `cargo test about_scan_promotes_packages_and_assigns_referenced_files --bin scancode-rust`; `cargo test about_scan_tracks_missing_file_references --bin scancode-rust`                                                                                                                                                                                                                              |
| 14    | Swift      | Done    | #193                                                             | `cargo test swift`; `cargo test --bin scancode-rust swift_scan_`; `cargo test --features golden-tests swift_golden`                                                                                                                                                                                                                                                                                                                                                       |
| 15    | Conan      | Done    | #194                                                             | `cargo test conan`; `cargo test --features golden-tests conan_golden`                                                                                                                                                                                                                                                                                                                                                                                                     |
| 16    | Docker     | Done    | #199, #200                                                       | `cargo test docker --lib`; `cargo test containerfile_scan_keeps_package_data_unassembled --bin scancode-rust`; `cargo test --features golden-tests docker_golden --lib`                                                                                                                                                                                                                                                                                                   |
| 17    | Python     | Done    | #136, #138, #139, #140, #143, #144, #147, #148, #149, #150, #209 | `cargo test python`; `cargo test requirements_txt`; `cargo test pipfile_lock`; `cargo test poetry_lock`; `cargo test pip_inspect_deplock`; `cargo test --features golden-tests python_golden`; `cargo test --features golden-tests requirements_txt_golden`; `cargo test --features golden-tests pipfile_lock_golden`; `cargo test --features golden-tests poetry_lock_golden`; `cargo test assembly::assemblers::tests::test_every_datasource_id_is_accounted_for --lib` |
| 18    | Debian     | Planned | #176, #177, #178, #179, #180, #181, #182, #183, #185, #186, #219 | `cargo test debian`; `cargo test --features golden-tests debian_golden`                                                                                                                                                                                                                                                                                                                                                                                                   |

## Detailed Scoping Rules

### Maven Pilot PR

Start here unless priorities change.

Issues:

- #122 POM license normalization
- #124 dependencyManagement field support
- #126 PURL qualifiers processing
- #128 duplicated name in description
- #131 relocations metadata support
- #135 JAR META-INF POM handling validation
- #207 treat META-INF as top-level directory for Java JARs
- #208 fix extracted license statement display in POM
- #211 fix typo in party role field
- #214 add support for POM 4.1.0 model

Likely touchpoints:

- Maven parser implementation and registration
- Maven-focused unit and golden coverage
- Maven parser fixtures and expected outputs
- Assembly golden coverage if Maven package ownership or merging changes

Current status:

- The Maven ecosystem batch now covers the full listed issue set with parser, unit-test, parser-golden, assembly-golden, and improvement-doc updates.
- The final follow-up added top-level XML-comment license capture, explicit developer-role regression coverage, nested `META-INF/maven/<group>/<artifact>/pom.xml` assembly coverage, and a multi-nested-POM safety regression.
- Issue-closing keywords belong in the PR body only for issues whose final branch state is fully evidenced by code and tests.

### npm + Yarn PR Scope

Issues:

- #123 npm overrides field support
- #125 package.json ↔ package-lock.json name/version consistency validation
- #127 avoid dummy npm URLs for missing metadata
- #129 scoped npm API URL regression coverage
- #133 latest package-lock / hidden lockfile handling
- #197 Yarn plug-and-play related package-root coverage
- #198 infer Yarn dependency scope from sibling package.json
- #205 large/uncommon npm layout coverage
- #206 nested bundled package ownership / purl attribution

Likely touchpoints:

- npm and Yarn parser implementation, registration, and metadata shaping
- npm/Yarn-focused unit and parser-golden coverage
- npm/Yarn assembly behavior for sibling merge, workspace handling, and nested package ownership
- Representative parser and assembly fixtures under `testdata/`
- Improvement documentation for intentional Rust-vs-Python behavior differences

Current status (March 9, 2026):

- Local work now preserves npm `overrides`, avoids synthetic URLs for empty npm metadata, and adds scoped API URL regression coverage.
- Scoped npm fallback URLs now use the correct registry/tarball shape, while invalid homepage arrays and blank bugs URLs are normalized away.
- npm lockfile handling now falls back to `packages[""]` for root identity, preserves `link: true` and other non-version dependency specs, records lockfile version metadata, and correctly keeps nested duplicate packages transitive unless they are truly root-direct.
- Yarn lock parsing now infers direct dependency scope from a sibling `package.json`.
- npm/pnpm assembly now assigns package-root files while skipping first-level `node_modules`, preserves unattached lockfile dependencies when a sibling manifest is not packageable, and emits deterministic package/file ordering.
- Workspace assembly now accepts array, string, and object-style workspace declarations, with coverage in npm workspace, pnpm workspace, and nested package assembly goldens.
- Additional regression coverage now exists for npm lockfile `file:`, `git+...`, tarball URL, and `npm:` alias cases, plus the nested duplicate directness bug.
- PR #297 (`fix(npm): complete the npm and yarn enhancement batch`) has merged, so this ecosystem row is now `Done`.

### NuGet PR Scope

Issues:

- #157 nuspec license collection
- #159 package license detection
- #162 missing party types
- #163 modern nuspec metadata structure
- #165 license detection from nuspec files
- #215 extra NuGet manifests
- #216 Visual Studio / NuGet project manifest support

Likely touchpoints:

- NuGet parser implementation and registration for `.nuspec`, `.nupkg`, `packages.config`, `packages.lock.json`, `project.json`, `project.lock.json`, and PackageReference project files
- NuGet-focused unit coverage for modern license metadata, archive-backed license files, project manifests, and party typing
- Parser goldens for legacy nuspecs plus modern `.nuspec`, legacy `project.json`, and PackageReference `.csproj`
- Assembly coverage for sibling merge of package metadata project files plus dependency-only NuGet manifests
- Improvement documentation for beyond-parity project-manifest support and archive-backed license extraction

Current status (March 9, 2026):

- Local work now preserves NuGet party `type` as `person` for nuspec- and project-derived author/owner data.
- Modern NuGet license metadata now records `license_type` and `license_file` hints in parser `extra_data`, while `.nuspec` parsing keeps file-based `<license>` entries ahead of deprecated `licenseUrl` placeholders.
- `.nupkg` extraction now reads the referenced license file contents when a nuspec declares `<license type="file">...`, giving the package parser a real extracted license statement instead of only the placeholder path.
- New parser support exists for legacy `project.json`, legacy `project.lock.json`, and PackageReference `.csproj`/`.vbproj`/`.fsproj` manifests.
- Parser goldens now cover the Fizzler modern nuspec fixture, a legacy `project.json` fixture, and a PackageReference `.csproj` fixture.
- Assembly golden coverage now exists for a `.csproj` + `packages.config` sibling merge in `test_assembly_nuget_basic`.
- PR #299 (`fix(nuget): complete the NuGet enhancement batch`) captures the completed implementation batch.

### RPM PR Scope

Issues:

- #164 missing distro namespace in RPM PURLs
- #166 yumdb metadata collection from installed RPM rootfs
- #167 missing RPM metadata fields
- #168 Fedora source/VCS extra data extraction
- #169 safe handling of missing/invalid database path data
- #170 hash-named source RPM detection
- #171 full RPM version preservation in container/rootfs scans

Likely touchpoints:

- RPM archive parsing, installed DB parsing, and file-reference post-processing
- RPM-focused unit coverage for source RPM identity, installed EVR handling, YumDB metadata parsing, and namespace/PURL propagation
- Parser goldens for richer RPM archive expectations where local fixtures already exist
- Improvement documentation for beyond-parity YumDB support and content-based source RPM detection

Current status (March 9, 2026):

- Local work now recognizes hash-named source RPMs by RPM magic bytes instead of extension-only matching.
- RPM archives now preserve source qualifiers, richer party/keyword/build metadata, VCS hints, and source URLs where available.
- Installed RPM namespace propagation now rewrites package and dependency PURLs/UIDs after `os-release` inference instead of only filling the separate `namespace` field.
- Installed RPM version regressions now have focused coverage proving `version-release` is preserved.
- YumDB `from_repo` package sidecars now parse sibling YumDB keys and merge them back onto the matching installed RPM package under `extra_data.yumdb`.
- Parser golden coverage now includes a real source RPM fixture (`setup-2.5.49-b1.src.rpm`) with the richer archive metadata contract.
- PR #300 (`fix(rpm): complete the RPM enhancement batch`) captures the completed implementation batch.

### Cargo PR Scope

Issues:

- #184 crate files not assigned to package
- #189 workspace member file detection
- #217 complete Rust Cargo support

Likely touchpoints:

- Cargo.toml and Cargo.lock parser behavior for filename matching and manifest field extraction
- Cargo-focused unit coverage for lowercase manifests plus readme/publish metadata
- Parser goldens for Cargo.toml and Cargo.lock cases where parity/regression coverage is meaningful
- Assembly coverage for plain crate file ownership and Cargo workspace member file ownership
- Improvement documentation for beyond-parity file assignment and parser parity fixes

Current status (March 9, 2026):

- Local work now accepts lowercase `cargo.toml` and `cargo.lock` filenames in direct parser matching, bringing parser behavior into line with the registered metadata patterns and Python reference tests.
- Cargo.toml parsing now preserves `readme` and `publish` metadata in parser `extra_data`, and workspace readme inheritance resolves into `readme_file` for member packages when applicable.
- Cargo parser fallbacks now retain `package_type` and `datasource_id` on error paths for both `Cargo.toml` and `Cargo.lock`.
- Assembly now assigns plain crate files under a Cargo package root to that package while still skipping `target/` and leaving nested package roots to their own package assignments.
- Workspace member fixtures now prove non-manifest files like `crates/cli/LICENSE` and `crates/core/README.md` are associated with the correct member package.
- Parser golden coverage now includes a `publish = false` Cargo.toml fixture and a Cargo.lock fixture, while existing Cargo.toml goldens now capture `readme_file` extraction.
- PR #304 (`fix(cargo): complete the Cargo enhancement batch`) captures the completed implementation batch.

### Go PR Scope

Issues:

- #152 support go.mod directives
- #153 improve granularity of detection within go.sum/go.mod
- #155 use Go build naming conventions and directives for file categorization and dependency scopes
- #218 support Go module graph

Likely touchpoints:

- Go parser behavior for fallback datasource coverage and generated module-graph artifacts
- Go-focused unit coverage for graph parsing and scanner-side source categorization
- Parser goldens for `replace` directives and graph output
- Assembly coverage for sibling merge of `go.mod`, `go.sum`, and `go.mod.graph`
- Improvement documentation for graph support and Go test/build file categorization

Current status (March 9, 2026):

- Local work now keeps `datasource_id` populated on Go parser fallback/error paths for `go.mod`, `go.sum`, and `Godeps.json`.
- Go parser goldens now cover a real upstream `replace` directive fixture (`opencensus-service`) plus a checked-in `go.mod graph` artifact.
- A new `go.mod.graph` parser now models direct vs transitive module relationships separately from `go.sum`, keeping graph semantics out of the checksum parser.
- Go assembly coverage now includes a sibling merge case for `go.mod`, `go.sum`, and `go.mod.graph` together.
- Scanner-side Go source categorization now treats `_test.go` files and `//go:build test` / `// +build test` files as non-production source for `is_source` directory heuristics.
- PR #305 (`fix(go): complete the Go enhancement batch`) captures the completed implementation batch.

### Gradle PR Scope

Issues:

- #130 version catalog template POM detection
- #132 runtime dependency scope classification
- #134 SBOM component license extraction
- #137 correct package identifiers from build.gradle

Likely touchpoints:

- Gradle build script parser behavior for scope classification, catalog alias resolution, project references, and POM license metadata
- Focused unit coverage for `compileOnly`, version catalogs, project references, and Gradle license extraction
- Parser goldens for version-catalog aliases and Groovy/Kotlin license-bearing publishing metadata
- Output regression coverage for CycloneDX license emission
- Small Maven guardrail for placeholder-only template coordinates because `#130` is locally grouped here even though its upstream reference is Maven-specific

Current status (March 9, 2026):

- Local work now classifies `compileOnly`, `compileOnlyApi`, `annotationProcessor`, `kapt`, and `ksp` as non-runtime Gradle scopes while keeping `test*` scopes optional.
- Gradle parser output now extracts `pom { licenses { ... } }` metadata and promotes recognizable SPDX-like values into declared license fields consumed by CycloneDX output.
- TOML-backed `libs.versions.toml` aliases now resolve to real Maven package identifiers from nearby version catalogs.
- Local project references like `project(":libs:download")` now preserve parent path segments in the emitted package identifier.
- A small Maven guardrail now skips placeholder-only `${groupId}` / `${artifactId}` / `${version}` template coordinates so the misbucketed `#130` issue is resolved without reopening a full Maven batch.
- PR #306 (`fix(gradle): complete the Gradle enhancement batch`) captures the completed implementation batch.

### Ruby PR Scope

Issues:

- #151 strip `.freeze` from gemspec values
- #154 resolve gemspec version/constants
- #156 preserve GIT/PATH info from `Gemfile.lock`
- #158 avoid repeated package/dependency results on extracted rubygems
- #160 avoid false dependency parsing from gemspec description text
- #161 tag nested key files / license clarity correctly

Likely touchpoints:

- Ruby parser behavior for gemspec constants, false dependency extraction, and Bundler lockfile source metadata
- Parser goldens for gemspec constants and Gemfile.lock GIT/PATH coverage
- Assembly coverage for extracted gem metadata + extracted gemspec merge/dedupe and nested file assignment
- Improvement documentation for required-file constant resolution, extracted-gem merge dedupe, and Bundler source provenance

Current status (March 10, 2026):

- Local work now resolves gemspec constants from required local Ruby files for name/version/authors/email/homepage-style metadata.
- Ruby parser goldens now cover an upstream-style `with_variables.gemspec` case plus `Gemfile.lock` GIT/PATH source metadata behavior.
- Extracted gem layouts now assemble `metadata.gz-extract` together with sibling `data.gz-extract/*.gemspec` content instead of emitting repeated package/dependency results.
- Ruby package-root resource assignment now associates nested extracted files such as `LICENSE.txt` and Ruby source files with the assembled gem package.
- A targeted regression now proves description text does not become a fake dependency.
- Follow-up work now tags nested Ruby legal/readme/manifest files as `key_file`, promotes package metadata from those files, and computes a top-level `summary.license_clarity_score`.
- PR #307 (`fix(ruby): complete the Ruby enhancement batch`) captures the completed main Ruby batch; the stacked follow-up PR for `#161` builds on top of it.
- PR #308 (`fix(ruby): tag nested key files and score license clarity`) captures the completed `#161` follow-up.

### Composer PR Scope

Issues:

- #187 slow composer.lock scanning performance
- #188 nested PHP package detection
- #190 support alternate file names

Likely touchpoints:

- Composer parser behavior for alternate file names, manifest provenance fields, and large lockfile extraction
- Composer-focused unit coverage for alternate names, manifest `source`/`dist`, large lockfiles, and package-root assignment
- Parser goldens for `composer.json` metadata in addition to the existing lockfile golden
- Assembly coverage for nested Composer packages and file ownership
- Improvement documentation for alternate-name support, nested file assignment, and lighter lockfile handling

Current status (March 10, 2026):

- Local work now recognizes both suffix-style and prefix-style alternate Composer manifest/lock names (for example `php.composer.json` and `composer.symfony.json`).
- Composer manifest parsing now preserves `vcs_url`, `download_url`, and typed author/vendor parties from `source`/`dist` metadata.
- Composer lock extraction now avoids cloning full package arrays before dependency extraction and keeps the current lightweight “one synthetic lock package plus dependencies” design.
- Composer package-root resource assignment now associates ordinary files to the correct Composer package, including nested Composer packages under a parent project tree.
- Parser golden coverage now includes a `composer.json` manifest fixture (`a-timer`) alongside the existing lockfile golden.
- Assembly golden coverage now includes a nested Composer package fixture with alternate file names and nested file assignment.
- PR #310 (`fix(composer): complete the Composer enhancement batch`) captures the completed implementation batch.

### Conda PR Scope

Issues:

- #195 parse conda-meta files for installed package files
- #196 resolve namespace and repo URL ambiguity

Likely touchpoints:

- Conda parser behavior for `conda-meta` package identity and channel-prefix handling
- Conda-focused unit coverage for URL-like channel prefixes and symbolic channel namespace retention
- Parser goldens for `conda-meta/*.json` installed-package metadata
- Rootfs assembly coverage for `conda-meta/*.json` + recipe `meta.yaml` merged package/file assignment
- Improvement documentation for installed file assignment and channel disambiguation

Current status (March 11, 2026):

- Local work now emits `conda-meta` `file_references` for installed files, extracted package directories, and package tarball paths.
- Rootfs Conda assembly now merges `conda-meta/*.json` installed metadata with the matching recipe `meta.yaml` package and assigns the installed files back to that assembled package.
- URL-like channel prefixes are no longer overloaded into namespace; they are preserved separately as `channel_url`, while symbolic channel names remain namespace-like and are also preserved as `channel` metadata.
- Parser golden coverage now includes the existing local `tzdata` `conda-meta` fixture.
- PR #311 (`fix(conda): complete the Conda enhancement batch`) captures the completed implementation batch.

### CocoaPods PR Scope

Issues:

- #191 refine CocoaPods scope handling
- #192 duplicated package info in podspec

Likely touchpoints:

- CocoaPods parser behavior for `.podspec`, `.podspec.json`, `Podfile`, and `Podfile.lock` scope semantics
- CocoaPods-focused unit coverage for runtime vs development dependency handling
- Parser goldens for `.podspec.json`, `Podfile`, and `Podfile.lock` after the refined scope contract
- Explicit regression proof that `RxDataSources.podspec` stays bounded and non-duplicated
- Improvement documentation for scope semantics and duplication protection

Current status (March 11, 2026):

- Local work now distinguishes runtime vs development dependency scope in `.podspec` parsing instead of flattening everything to runtime.
- `.podspec.json` dependencies now use the same runtime-oriented scope contract as `.podspec`.
- `Podfile` and `Podfile.lock` dependencies now use the more honest generic `dependencies` scope and leave runtime/optional unset where CocoaPods does not encode enough information to prove those booleans safely.
- CocoaPods parser goldens have been refreshed to the new scope contract.
- A targeted `RxDataSources.podspec` regression now proves the parser emits a single bounded package representation instead of duplicated package information.
- PR #312 (`fix(cocoapods): complete the CocoaPods enhancement batch`) captures the completed implementation batch.

### Alpine PR Scope

Issues:

- #172 implement APKBUILD parser
- #173 fix APKBUILD matched license text
- #174 handle packages with no files
- #175 fix Alpine VCS URL scheme

Likely touchpoints:

- Alpine parser behavior for APKBUILD recipe variables, sources, checksums, and license handling
- Alpine-focused unit coverage for `custom:multiple`, fileless installed packages, and HTTPS VCS URL generation
- Parser goldens for installed DB plus representative APKBUILD fixtures
- Existing installed-db file-reference assembly coverage (kept unchanged as the parser-side proof already exists)
- Improvement documentation for APKBUILD parsing and current-vs-upstream Alpine differences

Current status (March 11, 2026):

- Local work now includes a static `APKBUILD` parser with local fixture-backed coverage for `icu` and `linux-firmware`.
- The `custom:multiple` APKBUILD license case now preserves raw `matched_text` while still normalizing declared license fields.
- Installed package commit metadata now emits `git+https://git.alpinelinux.org/aports/commit/?id=...` VCS URLs.
- Fileless Alpine packages are now explicitly covered by regression tests so they are not silently dropped.
- Alpine parser golden coverage now includes both installed-db and APKBUILD fixtures.
- PR #314 (`fix(alpine): complete the Alpine enhancement batch`) captures the completed implementation batch.

### ABOUT PR Scope

Issues:

- #201 improve ABOUT fully qualified PURLs using `download_url`
- #202 stop reporting invalid `pkg:about` PURLs
- #203 handle partial/incomplete ABOUT files gracefully
- #204 fix ABOUT package root / file-reference dependency on direct file access

Likely touchpoints:

- ABOUT parser behavior for explicit PURL aliases, `download_url`-based type inference, and graceful fallback metadata retention
- ABOUT-focused unit coverage for PyPI/GitHub PURL inference, invalid YAML fallback, and extra preserved ABOUT fields
- Parser goldens for local `apipkg.ABOUT` and `appdirs.ABOUT` fixtures
- Scan-time package promotion and file-reference resolution for `about_resource`, `license_file`, and `notice_file`
- Improvement documentation for invalid `pkg:about` suppression and path-based file-reference resolution

Current status (March 11, 2026):

- ABOUT files now infer real ecosystem PURLs from recognized `download_url` hosts (currently PyPI wheel URLs and GitHub raw/source URLs) when no explicit PURL is present.
- Rust no longer synthesizes invalid `pkg:about/...` URLs just because the metadata file itself is an `.ABOUT` file.
- Invalid YAML and other malformed ABOUT files now keep `package_type = about` and `datasource_id = about_file` on fallback.
- `about_resource`, `license_file`, and `notice_file` references are now resolved relative to the ABOUT file path during scan-time package promotion, and unresolved entries are recorded in `extra_data.missing_file_references`.
- Local parser golden coverage now exists for `apipkg.ABOUT` and `appdirs.ABOUT`, and scan-level tests prove package promotion plus missing file reference behavior.
- PR #315 (`fix(about): complete the ABOUT enhancement batch`) captures the completed implementation batch.

### Swift PR Scope

Issues:

- #193 determine correct top-level Swift PURLs

Likely touchpoints:

- Swift assembly behavior for `Package.swift.json` / `Package.swift.deplock` / `Package.resolved` / `.package.resolved` / `swift-show-dependencies.deplock`
- Swift-focused scan-level regression coverage for manifest-owned root packages, show-dependencies precedence, resolved fallback, and resolved-only package emission
- Focused parser test updates only where scan-level parity requires distinct top-level package shaping from parser-internal data

Current status (March 11, 2026):

- Swift assembly now uses manifest data to own the top-level package when `Package.swift.json` or `Package.swift.deplock` is present, instead of incorrectly trying to derive that root package from resolved dependency artifacts.
- `swift-show-dependencies.deplock` now supersedes the assembled dependency graph without replacing the manifest-owned root package, matching the upstream ScanCode precedence contract.
- `Package.resolved` / `.package.resolved` now act as fallback dependency enrichment when a manifest is present and emit one top-level package per resolved dependency when no manifest or show-dependencies file exists.
- New scan-level regression tests now cover the upstream-style `fastlane_resolved_v1`, `mapboxmaps_manifest_and_resolved`, `vercelui`, and `vercelui_show_dependencies` fixtures.

### Conan PR Scope

Issues:

- #194 collect all download URLs from `conandata.yml`

Likely touchpoints:

- Conan `conandata.yml` parser behavior for multi-URL `sources.<version>.url` entries
- Real Conan recipe regression coverage for Boost and libzip multi-URL fixtures
- Conan parser-golden wiring for existing `conandata.yml` expected outputs while preserving the Rust-only `mirror_urls` enhancement in `extra_data`

Current status (March 12, 2026):

- Rust already preserves all download URLs from multi-URL `conandata.yml` entries under `extra_data.mirror_urls`, while still using the primary URL as `download_url`.
- Conan parser regression coverage now uses the real Boost and libzip recipe fixtures to prove all source URLs are retained.
- Conan parser golden coverage is now wired for the existing Boost, libgettext, and libzip `conandata.yml` expected outputs, giving this beyond-parity behavior concrete fixture-backed evidence.

### Docker PR Scope

Issues:

- #199 support `Containerfile` as an alternative Dockerfile name
- #200 treat Dockerfile and Containerfile as non-assembled package data and collect OCI labels

Likely touchpoints:

- new Docker parser for Dockerfile-like files
- parser registration plus package-type / datasource-id plumbing
- intentionally unassembled datasource accounting
- real fixture coverage for Dockerfile and Containerfile OCI label extraction

Current status (March 12, 2026):

- Rust now recognizes `Dockerfile`, `Containerfile`, and `Containerfile.core` as Docker package-data files.
- OCI labels under `org.opencontainers.image.*` are mapped into package metadata and also preserved in `extra_data.oci_labels`.
- Dockerfile and Containerfile outputs are intentionally left non-assembled, with scan-level regression coverage proving they remain file-level package data.

### Python Batch Status

Implemented in PR #319:

- #136 richer `setup.cfg` metadata
- #138 imported-module fallback for unresolved setup.py dunder version metadata
- #139 imported sibling dunder metadata (`__version__`, `__author__`, `__license__`)
- #140 `setup.py` `project_urls` with `OrderedDict`
- #143 mapped `setup.cfg` project URLs
- #147 PKG-INFO / METADATA license-file references
- #148 private-package classifier support
- #209 exact-filename `pypi.json` parser support

Closed after audit as already covered or nonblocking:

- #141 setup.cfg requirement parsing
- #142 setup.cfg dependency scopes
- #145 wheel metadata description handling
- #146 setup.cfg requirement field semantics
- #210 broad PyPI package-handling refactor umbrella
- #212 `setuptools.setup()` detection in setup.py
- #213 requirements-driven dependency inclusion

Current status (March 12, 2026):

- PR #319 (`fix(python): complete the Python enhancement batch`) covers the concrete parser and metadata gaps that still reproduced locally.
- The closed issues above were re-audited against current local behavior and did not remain as distinct Rust parser defects after the batch verification.
- The remaining open Python issues were narrowed to follow-up candidates after the parser batch; `#150`, `#144`, and `#149` are now each addressed in dedicated follow-up PRs.

### Python Follow-up PR Scope (#150)

Issue:

- #150 Python file-to-package assignment during assembled scans

Narrow related fallout:

- parser-side sibling `RECORD` / `installed-files.txt` collection needed so installed metadata files actually expose assignable file references

Likely touchpoints:

- standalone `METADATA` / `PKG-INFO` ancillary file-reference collection in the Python parser
- Python-specific file-reference resolution for installed metadata under `site-packages/` and `dist-packages/`
- scan-level regression coverage for `.dist-info` and `.egg-info` installed layouts

Current status (March 12, 2026):

- Standalone Python `METADATA` now collects sibling `RECORD` file references.
- Standalone Python `PKG-INFO` now collects sibling `installed-files.txt` file references.
- Python file-reference resolution now assigns referenced files back to the assembled package for installed metadata under `site-packages/` and `dist-packages/`.
- The follow-up deliberately stays file-reference-driven and does not broaden into generic whole-tree Python file harvesting.

### Python Follow-up PR Scope (#144)

Issue:

- #144 ancillary package file collection for assembled Python packages

Narrowed implementation target:

- sibling `SOURCES.txt` next to source-layout `.egg-info/PKG-INFO`
- file-reference-driven assignment of the listed source-package files during scans

Likely touchpoints:

- standalone `PKG-INFO` ancillary `SOURCES.txt` collection in the Python parser
- Python source-layout file-reference resolution rooted above the `.egg-info` directory
- scan-level regression coverage for source-package layouts that list package files in `SOURCES.txt`

Current status (March 12, 2026):

- Standalone source-package `.egg-info/PKG-INFO` now collects sibling `SOURCES.txt` entries as `file_references`.
- Source-layout Python scans now assign `SOURCES.txt`-listed files such as `setup.py` and package modules back to the assembled Python package.
- This follow-up deliberately stays `SOURCES.txt`-driven and does not broaden into generic whole-tree Python file harvesting.

### Python Follow-up PR Scope (#149)

Issue:

- #149 remaining wheel vs sdist metadata-quality deltas after the current parser batch

Narrowed implementation target:

- RFC822 dependency extraction from wheel `METADATA` and source `PKG-INFO`
- extra-scoped dependency shaping from `extra == ...` markers and sibling `.egg-info/requires.txt`
- simple structured marker preservation (for example `python_version == '2.7'`)
- pinned `==` / `===` dependency detection
- sibling `.egg-info/requires.txt` recovery where source metadata needs extra dependency evidence

Likely touchpoints:

- `build_package_data_from_rfc822()` dependency population
- source-package dependency recovery from sibling `.egg-info/requires.txt`
- focused parser regression tests for wheel-vs-sdist pairs and metadata/v20 extras

Current status (March 12, 2026):

- Wheel `METADATA` no longer drops `Requires-Dist` dependencies.
- Extra-scoped wheel dependencies are now preserved with the extra name as scope and `is_optional = true`.
- Marker-bearing dependencies now preserve structured fields like `python_version` in dependency `extra_data`.
- Single `==` / `===` dependency specifiers are now treated as pinned and embedded in the emitted dependency PURL.
- Source-package metadata can recover dependency evidence from sibling `.egg-info/requires.txt` when plain `PKG-INFO` alone does not carry that information.

### Debian PR Scope Rule

Include only:

- `debian/` directory detection
- control-file dependency parsing
- copyright parsing and detection behavior
- `.deb` package and container-image Debian metadata covered by the listed issues

Exclude from the Debian PR:

- cross-ecosystem license engine refactors
- non-Debian distro work
- generic file-reference or scanner refactors unless they are strictly necessary to close a listed Debian issue

## PR Template Checklist

Each ecosystem PR should state:

- issues covered
- issue-closing keywords for fully resolved issues
- explicit exclusions
- intentional differences from Python
- follow-up issues created or intentionally deferred

## Completion Tracking

When an ecosystem batch state changes, update the row above to one of:

- `In progress` — active implementation work still underway
- `Done` — implementation work is complete, even if the PR is still awaiting merge
- `Deferred`

GitHub remains the source of truth for merged PR numbers and issue-closure links; this document tracks status and scope only.
