# Build-Time License Index

**Status**: Planned
**Component**: `src/license_detection`
**Type**: Architecture / Performance / Packaging

This document is specifically about replacing runtime construction of the built-in
license index with a build-time generated artifact embedded in the binary.

### Required Behavioral Change

The current CLI path can silently disable license detection when engine
initialization fails. That is acceptable for an optional external dataset, but it
is not acceptable once the binary ships with a built-in index.

As part of this plan, default engine initialization should become fail-fast:

- embedded-index initialization failure should fail the scan,
- custom `--license-rules-path` failures should also be surfaced clearly,
- and the scanner should stop treating license detection as silently optional in
  the default path.

### Problem

License detection currently depends on a runtime rules directory.

- `src/main.rs` initializes `LicenseDetectionEngine` from `--license-rules-path`.
- `src/cli.rs` defaults that path to `reference/scancode-toolkit/src/licensedcode/data`.
- `src/license_detection/mod.rs` then loads all `.RULE` and `.LICENSE` files and rebuilds the full `LicenseIndex` on every startup.

This causes two UX and packaging problems:

1. end users need the ScanCode data tree available next to the binary,
2. every cold start pays the full rule-loading and index-construction cost.

### Goal

Make the default binary self-contained:

- no runtime dependency on the ScanCode rules directory,
- no runtime YAML/frontmatter parsing for the built-in dataset,
- no runtime recomputation of token dictionaries, thresholds, hash maps, or rule metadata for the built-in dataset,
- keep an escape hatch for developers and advanced users to load a custom rules directory.

### Recommended Design

Bake the output of the loader stage into the binary at build time.

For the first implementation, the embedded artifact should contain serialized
`LoadedRule` and `LoadedLicense` values: parsed and normalized representations of
the `.RULE` and `.LICENSE` files.

At runtime, the engine should:

1. deserialize the embedded loader outputs,
2. feed them into the normal build stage,
3. construct runtime `Rule` values, `LicenseIndex`, automatons, and supporting maps.

This is the recommended first step because:

- it removes the runtime filesystem dependency on the ScanCode dataset,
- it removes runtime YAML/frontmatter parsing,
- it requires only a modest refactor of the current pipeline,
- it creates a clean separation between loader-stage and build-stage data,
- and it leaves room for a future optimization that embeds more of the built index if startup still needs improvement.

The first version should be described primarily as a packaging fix and a partial
startup improvement. Index construction and automaton compilation will still run
at startup in this design.

### Build-Time Constraints

#### 1. Crates.io packaging

`reference/scancode-toolkit/...` is not included in the package manifest, so crates.io builds cannot rely on that directory existing.

That means the build pipeline must support:

- consuming a checked-in generated loader artifact in normal builds,
- regenerating that loader artifact in an explicit maintainer workflow,
- and building successfully when the reference submodule is absent.

If a checked-in loader artifact is added, `Cargo.toml` must also include it in
the packaged crate contents.

#### 2. Rule dataset size

The raw ScanCode dataset is large enough that embedding the full source tree is not a good default packaging strategy.

The repository currently contains roughly:

- `reference/scancode-toolkit/src/licensedcode/data/rules`: about 36k files,
- `reference/scancode-toolkit/src/licensedcode/data/licenses`: about 2.6k files,
- combined size: hundreds of MB.

So the binary should embed a generated artifact, not the raw dataset.

#### 3. Runtime build work still remains

This design does not eliminate startup index construction.

The build stage still needs to:

- synthesize license-derived rules,
- filter deprecated entries according to build policy,
- tokenize rule texts,
- compute thresholds and derived flags,
- and compile the Aho-Corasick automatons.

That is acceptable for the first version because the main goals are:

- self-contained binaries,
- no runtime dependency on the reference dataset,
- and a cleaner pipeline boundary.

#### 4. Deterministic artifact generation

The generated loader artifact should be byte-stable for the same input dataset.

That means loader outputs must be serialized in a stable order, especially the
top-level rule and license lists.

#### 5. Build system boundaries

Version 1 should avoid `build.rs` unless it becomes necessary later.

Chosen v1 packaging path:

- store the checked-in loader artifact at
  `resources/license_detection/license_index_loader.msgpack.zst`,
- embed it directly with `include_bytes!`,
- regenerate it only through an explicit maintainer script,
- do not regenerate it automatically during ordinary `cargo build`.

This keeps the implementation simple and avoids unnecessary build-system coupling.

## Step-by-Step Implementation Plan

### Step 1: Make engine initialization shared and fail-fast

Refactor `LicenseDetectionEngine` so it can be created from multiple sources.

Target API shape:

```rust
impl LicenseDetectionEngine {
    fn from_index(index: LicenseIndex) -> anyhow::Result<Self>;
    pub fn from_embedded() -> anyhow::Result<Self>;
    pub fn from_directory(rules_path: &Path) -> anyhow::Result<Self>;
}
```

Implementation notes:

- Add a shared internal constructor so embedded and directory-backed flows do not
  duplicate final assembly logic.
- Remove `LicenseDetectionEngine::new()` in favor of explicit constructors.
- Keep the current filesystem path behavior behind `from_directory()`.
- Make `from_embedded()` the normal CLI path.
- Keep the directory-based path for tests, parity work, debugging, and custom datasets.
- Make `from_directory()` strict and all-or-nothing: any filesystem, parse, or
  validation error in the supplied dataset returns `Err` instead of yielding a
  partial engine.
- Change the default CLI path to return a real error instead of silently skipping
  license detection.

### Step 2: Introduce explicit loader-stage types

Add explicit loader-stage models for parsed rule and license data, for example:

- `src/license_detection/embedded/mod.rs`
- `src/license_detection/embedded/schema.rs`
- `src/license_detection/models/loaded_rule.rs`
- `src/license_detection/models/loaded_license.rs`

These types should represent parsed and normalized file content, not runtime
index state.

Recommended type split:

- `LoadedRule` and `LoadedLicense` are loader-stage outputs,
- `Rule` remains a build-stage/runtime type,
- `LicenseIndex` remains the final built runtime structure.

Important note:

- `Rule` now uses `RuleKind` as an enum,
- and `LoadedRule` should also store the derived `RuleKind` directly,
- invalid source flag combinations should fail during loading.

### Step 3: Define `LoadedRule` and `LoadedLicense` schemata

The loader-stage types should include exactly the parsed and normalized data
needed by the build stage.

These types are intended to be canonical parsed-and-normalized loader outputs,
not minimally parsed syntax trees. Any transformation that depends only on one
file's contents and filename should happen in the loader stage.

Examples of loader-stage responsibilities:

- text trimming and normalization,
- fallback/default handling derived only from one file,
- empty-vector to `None` cleanup,
- merged URL collection for licenses,
- file-local validation,
- and false-positive handling for missing rule `license_expression`.

Examples of build-stage responsibilities:

- deprecated filtering policy,
- license-derived rule synthesis,
- tokenization and threshold computation,
- and all index and automaton construction.

Suggested shape:

```rust
pub struct LoadedRule {
    pub identifier: String,
    pub license_expression: String,
    pub text: String,
    pub rule_kind: RuleKind,
    pub is_false_positive: bool,
    pub is_required_phrase: bool,
    pub relevance: Option<u8>,
    pub minimum_coverage: Option<u8>,
    pub has_stored_minimum_coverage: bool,
    pub is_continuous: bool,
    pub referenced_filenames: Option<Vec<String>>,
    pub ignorable_urls: Option<Vec<String>>,
    pub ignorable_emails: Option<Vec<String>>,
    pub ignorable_copyrights: Option<Vec<String>>,
    pub ignorable_holders: Option<Vec<String>>,
    pub ignorable_authors: Option<Vec<String>>,
    pub language: Option<String>,
    pub notes: Option<String>,
    pub is_deprecated: bool,
}

pub struct LoadedLicense {
    pub key: String,
    pub name: String,
    pub spdx_license_key: Option<String>,
    pub other_spdx_license_keys: Vec<String>,
    pub category: Option<String>,
    pub text: String,
    pub reference_urls: Vec<String>,
    pub notes: Option<String>,
    pub is_deprecated: bool,
    pub replaced_by: Vec<String>,
    pub minimum_coverage: Option<u8>,
    pub ignorable_copyrights: Option<Vec<String>>,
    pub ignorable_holders: Option<Vec<String>>,
    pub ignorable_authors: Option<Vec<String>>,
    pub ignorable_urls: Option<Vec<String>>,
    pub ignorable_emails: Option<Vec<String>>,
}

pub struct EmbeddedLoaderSnapshot {
    pub schema_version: u32,
    pub rules: Vec<LoadedRule>,
    pub licenses: Vec<LoadedLicense>,
}
```

Notes:

- `LoadedRule.rule_kind` is derived by the loader from the source rule-kind
  booleans.
- Invalid rule-kind flag combinations fail during loading.
- `LoadedRule.license_expression` should already reflect current single-file
  interpretation rules:
  - explicit expressions are normalized in the loader,
  - false-positive rules with no source expression get the loader-time fallback
    value of `"unknown"`,
  - non-false-positive rules without a source expression fail during loading.
- `EmbeddedLoaderSnapshot` is the top-level serialized artifact wrapper and is
  responsible for decompression, deserialization, and `schema_version`
  validation.
- Deprecated entries should still be loaded into the artifact; filtering belongs
  in the build stage.
- Prefer dedicated loader structs over serializing runtime `Rule` directly.
- Keep artifact metadata minimal in v1 so output stays deterministic.

### Conversion Contract

The conversion boundary should be:

- loader stage returns canonical `LoadedRule` / `LoadedLicense` values,
- build stage converts those into runtime `Rule` and runtime `License` values,
- runtime-only derived state is created only in the build stage.

Recommended concrete entrypoints:

- `load_rules_from_directory(...) -> Result<Vec<LoadedRule>>`
- `load_licenses_from_directory(...) -> Result<Vec<LoadedLicense>>`
- `build_index(loaded_rules, loaded_licenses, with_deprecated) -> LicenseIndex`
- `LicenseDetectionEngine::from_directory(...)`
- `LicenseDetectionEngine::from_embedded()`

Required loader-stage normalization:

#### `LoadedRule`

- derive `identifier` from filename,
- derive `rule_kind` from source rule-kind booleans,
- reject invalid source flag combinations,
- normalize trivial outer parentheses in `license_expression`,
- if `is_false_positive` is true and the file omits `license_expression`, store
  `"unknown"` in `LoadedRule.license_expression`,
- default relevance from missing source values using current loader behavior,
- record `has_stored_minimum_coverage`,
- trim and normalize text and optional metadata fields,
- reject other invalid file-local combinations.

#### `LoadedLicense`

- derive `key` from filename and validate it against frontmatter,
- derive `name` using the current fallback chain,
- merge `reference_urls` from the current set of source URL fields in loader order,
- parse and normalize `minimum_coverage`,
- trim and normalize text and optional metadata fields,
- preserve deprecation metadata without filtering.

Required build-stage conversion:

- convert `LoadedRule` into initial runtime `Rule` values with only non-indexed
  fields populated,
- convert `LoadedLicense` into runtime `License` values,
- then continue with the existing build/index pipeline.

### Step 4: Refactor the loader around these types

Refactor `src/license_detection/rules/loader.rs` so the loader returns
`LoadedRule` and `LoadedLicense` values.

Recommended direction:

1. Extract content-based parsing helpers instead of path-only parsers.
2. Parse `.RULE` files into `LoadedRule`.
3. Parse `.LICENSE` files into `LoadedLicense`.
4. Keep the current text normalization and frontmatter interpretation behavior.
5. Keep duplicate-text validation in the loader if useful, but move deprecated
   filtering out of the loader.

This creates the stable serialization boundary for the embedded artifact.

### Step 5: Add loader-artifact generation tooling

Add a generator that loads the ScanCode data and writes a compressed loader artifact.

Recommended shape:

- a dedicated helper binary, such as `generate-license-loader-artifact`, backed by
  shared helper code,
- plus a maintainer script that invokes it in the canonical way.

Recommended default:

- loader-artifact regeneration is explicit and maintainer-driven,
- normal `cargo build` does not parse the full ScanCode dataset.

Expected behavior:

1. Discover the input data directory.
2. Load rules and licenses with the existing loader.
3. Sort them deterministically.
4. Serialize with `rmp-serde`.
5. Compress with `zstd`.
6. Write the artifact to a checked-in path such as `resources/license_detection/license_index_loader.msgpack.zst`.

The maintainer command should be the canonical way to refresh the embedded
artifact after updating the reference submodule.

Recommended maintainer entry point:

- `./scripts/update-license-index-loader-artifact.sh`

That script should:

1. verify the reference dataset is present,
2. invoke the generator,
3. write the checked-in artifact,
4. optionally run a deterministic regeneration check,
5. print a short reminder to commit the updated artifact.

### Step 6: Support normal builds, maintainer refreshes, and packaged builds

Because crates.io builds do not have the reference submodule, support three build modes.

#### Mode A: normal build

Default behavior for regular `cargo build`:

- consume the checked-in loader artifact,
- embed it directly with `include_bytes!`,
- avoid regenerating the loader artifact automatically.

#### Mode B: maintainer refresh workflow

If `reference/scancode-toolkit/src/licensedcode/data` exists:

- run an explicit maintainer command or script to regenerate the checked-in loader artifact,
- validate deterministic output,
- commit the refreshed artifact when the dataset intentionally changes.

Suggested workflow:

```sh
./setup.sh
./scripts/update-license-index-loader-artifact.sh
```

or equivalent via a dedicated generator binary.

#### Mode C: packaged build

If the reference directory does not exist:

- use a checked-in loader artifact from the repository,
- embed it directly from the checked-in path.

Recommended repository addition:

- `resources/license_detection/license_index_loader.msgpack.zst`

This keeps the package self-contained without vendoring the full raw rules tree.

Also update `Cargo.toml` package inclusion so `resources/license_detection/license_index_loader.msgpack.zst`
is present in crates.io builds.

### Step 7: Load the embedded loader artifact at runtime

Implement `LicenseDetectionEngine::from_embedded()` to:

1. `include_bytes!("../resources/license_detection/license_index_loader.msgpack.zst")`
   or equivalent workspace-relative path,
2. decompress it,
3. deserialize it,
4. feed `LoadedRule` and `LoadedLicense` into the normal build stage,
5. construct `LicenseIndex`,
6. build `SpdxMapping` from the built licenses/index.

This path should not touch the filesystem at all.

The runtime loader should validate `schema_version` before constructing the
engine and return a clear error on mismatch.

### Step 8: Move deprecated filtering into the build stage

Adjust the build stage so deprecated filtering is a build policy, not a loader
policy.

Recommended direction:

- the loader always returns the full parsed dataset,
- the build stage accepts a plain `with_deprecated: bool` parameter,
- normal CLI builds exclude deprecated rules and licenses there,
- tests can still build with or without deprecated entries from the same loader artifact.

To preserve current behavior, deprecated filtering should happen before
license-derived rule synthesis.

Required build-stage order:

1. start from all loaded rules and loaded licenses,
2. apply deprecated filtering according to build policy,
3. synthesize license-derived rules only from the filtered license set,
4. continue normal runtime-rule building and indexing.

This keeps the embedded loader output faithful to the source dataset and makes
policy decisions explicit.

### Step 9: Switch the CLI default to the build-time index

Change `src/main.rs` and `src/cli.rs` so:

- the CLI uses the build-time embedded index by default,
- `--license-rules-path` becomes an explicit override for custom rule datasets,
- `license_rules_path` defaults to `None` in the CLI,
- startup no longer depends on the reference submodule being present,
- default initialization failure stops the scan instead of silently disabling license detection.

Recommended behavior:

- `None` => embedded build-time loader artifact,
- `Some(path)` => custom directory load.

### Step 10: Keep custom datasets working

Do not remove directory-based loading.

Users and developers still need to be able to:

- test new or modified rules locally,
- compare Rust behavior against updated ScanCode rule trees,
- run targeted parity debugging.

So the embedded path should become the default, not the only path.

### Step 11: Add deterministic loader-artifact generation checks

Add tests or validation tools that ensure the generated loader artifact is stable.

Suggested checks:

- generating the loader artifact twice from the same input yields identical bytes,
- loader outputs are sorted deterministically before serialization,
- `from_embedded()` and `from_directory()` produce equivalent built indexes for representative invariants,
- the loader-artifact schema version rejects incompatible data cleanly.

This will require deterministic ordering of serialized collections and a stable
serialization of `LoadedRule` and `LoadedLicense`.

### Step 12: Add parity and regression tests

Before making the embedded path the default everywhere, add tests for:

#### Engine equivalence

- compare `from_embedded()` vs `from_directory()` on representative license texts,
- compare deserialized loader outputs vs filesystem loader outputs,
- compare raw match outputs for a focused fixture set,
- compare final detection outputs for golden-style samples.

#### Startup behavior

- binary initializes license detection successfully when `reference/.../data` is absent,
- custom `--license-rules-path` still overrides the embedded dataset.

#### Failure semantics

- embedded initialization failure fails the scan,
- custom rules-path initialization failure also fails the scan,
- `from_directory()` is all-or-nothing and does not tolerate partial dataset loading,
- no default path silently downgrades to "scan without licenses".

#### Failure handling

- corrupt loader-artifact bytes fail with a useful error,
- schema mismatch fails clearly,
- empty-pattern edge cases still behave the same after loader-artifact roundtrip.

### Step 13: Add build and release safeguards

Add CI or release checks for:

- building without the reference submodule present,
- building from the packaged crate contents,
- ensuring the checked-in loader artifact is included and loadable,
- ensuring stdout output modes do not get polluted by engine-init logging.

### Step 14: Update developer workflows

Add a documented workflow for regenerating the checked-in loader artifact.

Suggested commands:

```sh
./setup.sh
./scripts/update-license-index-loader-artifact.sh
```

The script can call a dedicated generator binary internally, but the script
should be the canonical maintainer entry point.

Also update developer-facing documentation so it explains:

- when the loader artifact must be regenerated,
- which script regenerates it,
- that normal builds consume the checked-in artifact,
- and that updating the reference dataset is a two-step process: refresh the
  dataset, then refresh the loader artifact.

### Step 15: Update user-facing docs

After the implementation lands, update:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/license-detection/ARCHITECTURE.md`
- `src/lib.rs` examples
- CLI help text in `src/cli.rs`

The docs should clearly say:

- normal binaries already contain the built-in license index,
- the reference dataset is only needed for development and custom-rule workflows,
- `--license-rules-path` is now an override, not a requirement.

Also remove or adjust any success logging on engine initialization that would
contaminate stdout-based report output.

In addition to the license-detection docs, update setup/install guidance so it
mentions the maintainer loader-artifact refresh step. In particular, `setup.sh`
currently already prints guidance about updating embedded license data; after
this work it should also hint that maintainers may need to rebuild the checked-in
license loader artifact.

## Recommended Execution Order

Implement in this order to reduce risk:

1. add a shared internal engine constructor and fail-fast default init,
2. introduce `LoadedRule` and `LoadedLicense`,
3. refactor the loader to return those types,
4. move deprecated filtering into the build stage,
5. define the embedded loader-artifact schema,
6. add the explicit maintainer loader-artifact generation script and generator,
7. add minimal build integration that consumes the checked-in artifact,
8. add runtime embedded loading and validation,
9. add equivalence, packaging, and no-submodule tests,
10. switch CLI default,
11. update docs and release workflow.

## Non-Goals for the First Version

These can wait until after the embedded loader-artifact path is working:

- embedding fully prebuilt runtime indexes,
- serializing `AhoCorasick` internals directly,
- removing custom directory loading,
- minimizing binary size aggressively,
- adding artifact delta updates,
- solving warm-cache and embedded-index loading in the same change.

## Expected Outcome

After this work:

- the released binary starts license detection without requiring external rule files,
- startup no longer reparses tens of thousands of rule files from YAML/frontmatter,
- crates.io and release builds remain self-contained,
- developers still retain the ability to test custom or updated rule datasets,
- the pipeline gains a clean separation between loader-stage and build-stage data,
- and the embedded path creates a clean foundation for future startup optimizations.
