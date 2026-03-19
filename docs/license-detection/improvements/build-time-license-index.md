# Build-Time License Index

**Status**: Planned
**Component**: `src/license_detection`
**Type**: Architecture / Performance / Packaging

This document is specifically about replacing runtime construction of the built-in
license index with a build-time generated artifact embedded in the binary.

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

Bake a generated license-index snapshot into the binary at build time.

For the first implementation, the snapshot should contain all precomputed Rust-side index data plus the byte patterns needed to rebuild the Aho-Corasick automatons quickly at runtime.

This is the recommended first step because:

- it removes filesystem and parsing requirements for end users,
- it eliminates almost all startup work,
- it avoids depending on private or unstable internal serialization details of `aho-corasick`,
- and it still leaves room for a future optimization that embeds fully serialized automatons if that later becomes practical.

### Build-Time Constraints

#### 1. Crates.io packaging

`reference/scancode-toolkit/...` is not included in the package manifest, so crates.io builds cannot rely on that directory existing.

That means the build pipeline must support:

- regenerating the embedded snapshot when the reference submodule is present,
- and falling back to a checked-in generated snapshot when it is not.

#### 2. Rule dataset size

The raw ScanCode dataset is large enough that embedding the full source tree is not a good default packaging strategy.

The repository currently contains roughly:

- `reference/scancode-toolkit/src/licensedcode/data/rules`: about 36k files,
- `reference/scancode-toolkit/src/licensedcode/data/licenses`: about 2.6k files,
- combined size: hundreds of MB.

So the binary should embed a generated artifact, not the raw dataset.

#### 3. Automaton serialization risk

The current `aho-corasick` crate usage does not expose an obvious stable, public serialization format for `AhoCorasick` itself.

So the safe first version is:

- serialize all deterministic precomputed data,
- store rule and unknown-pattern bytes,
- rebuild the two automatons from those bytes when constructing the embedded engine.

That still removes the expensive parts users care about most.

## Step-by-Step Implementation Plan

### Step 1: Split engine initialization by source

Refactor `LicenseDetectionEngine` so it can be created from multiple sources.

Target API shape:

```rust
impl LicenseDetectionEngine {
    pub fn from_embedded() -> anyhow::Result<Self>;
    pub fn from_directory(rules_path: &Path) -> anyhow::Result<Self>;
}
```

Implementation notes:

- Keep the current filesystem path behavior, but move it behind `from_directory()`.
- Make `from_embedded()` the normal CLI path.
- Keep the directory-based path for tests, parity work, debugging, and custom datasets.

### Step 2: Introduce a serializable snapshot type

Add a new module for build-time/runtime snapshot exchange, for example:

- `src/license_detection/embedded/mod.rs`
- `src/license_detection/embedded/schema.rs`

Define a snapshot that stores all data needed to reconstruct `LicenseIndex` without re-running rule loading or index building.

Suggested shape:

```rust
pub struct LicenseIndexSnapshot {
    pub version: u32,
    pub dictionary: TokenDictionarySnapshot,
    pub len_legalese: usize,
    pub rid_by_hash: Vec<([u8; 20], usize)>,
    pub rules_by_rid: Vec<Rule>,
    pub tids_by_rid: Vec<Vec<u16>>,
    pub sets_by_rid: Vec<(usize, Vec<u16>)>,
    pub msets_by_rid: Vec<(usize, Vec<(u16, usize)>)>,
    pub high_postings_by_rid: Vec<(usize, Vec<(u16, Vec<usize>)>)>,
    pub false_positive_rids: Vec<usize>,
    pub approx_matchable_rids: Vec<usize>,
    pub licenses_by_key: Vec<(String, License)>,
    pub pattern_id_to_rid: Vec<usize>,
    pub rid_by_spdx_key: Vec<(String, usize)>,
    pub unknown_spdx_rid: Option<usize>,
    pub rules_automaton_patterns: Vec<Vec<u8>>,
    pub unknown_automaton_patterns: Vec<Vec<u8>>,
}
```

Notes:

- Store token IDs as raw `u16` values in the snapshot and reconstruct `TokenId` on load.
- Prefer explicitly serializable collections over serializing `HashMap`/`HashSet` directly if deterministic output matters.
- Include a snapshot schema version from day one.

### Step 3: Make index building expose snapshot-ready data

Refactor `src/license_detection/index/builder/mod.rs` so the logic that currently builds a `LicenseIndex` can also return a snapshot-friendly intermediate form.

Recommended direction:

1. Extract a builder result struct, for example `BuiltLicenseIndexParts`.
2. Let the builder produce:
   - all precomputed maps/sets/vectors,
   - the rule automaton patterns,
   - the unknown automaton patterns.
3. Add two constructors on top of those parts:
   - `into_runtime_index()`
   - `into_snapshot()`

This avoids duplicating the indexing logic across runtime and build-time flows.

### Step 4: Add snapshot generation tooling

Add a generator that loads the ScanCode data, builds the index parts once, and writes a compressed snapshot artifact.

Possible locations:

- `build.rs` calling shared library code, or
- a dedicated helper binary used by `build.rs` in development workflows.

Expected behavior:

1. Discover the input data directory.
2. Load rules and licenses with the existing loader.
3. Build snapshot-ready index parts.
4. Serialize with `rmp-serde`.
5. Compress with `zstd`.
6. Write the artifact to `OUT_DIR`.

Also emit `cargo:rerun-if-changed=` lines for the generated snapshot input source that is actually being used.

### Step 5: Support both regeneration and fallback artifacts

Because crates.io builds do not have the reference submodule, support two build modes.

#### Mode A: maintainer/developer build

If `reference/scancode-toolkit/src/licensedcode/data` exists:

- regenerate the snapshot during build,
- write it to `OUT_DIR`,
- optionally provide a separate script or command to refresh the checked-in snapshot artifact.

#### Mode B: packaged build

If the reference directory does not exist:

- use a checked-in snapshot artifact from the repository,
- copy or re-expose it through `OUT_DIR` so the runtime include path stays consistent.

Recommended repository addition:

- `resources/license_detection/license_index_snapshot.msgpack.zst`

This keeps the package self-contained without vendoring the full raw rules tree.

### Step 6: Load the embedded snapshot at runtime

Implement `LicenseDetectionEngine::from_embedded()` to:

1. `include_bytes!` the compressed snapshot,
2. decompress it,
3. deserialize it,
4. rebuild the two automatons from baked pattern bytes,
5. reconstruct `LicenseIndex`,
6. build `SpdxMapping` from the reconstructed licenses.

This path should not touch the filesystem at all.

### Step 7: Switch the CLI default to the build-time index

Change `src/main.rs` and `src/cli.rs` so:

- the CLI uses the build-time embedded index by default,
- `--license-rules-path` becomes an explicit override for custom rule datasets,
- startup no longer fails just because the reference submodule is absent.

Recommended behavior:

- `None` => embedded build-time snapshot,
- `Some(path)` => custom directory load.

### Step 8: Keep custom datasets working

Do not remove directory-based loading.

Users and developers still need to be able to:

- test new or modified rules locally,
- compare Rust behavior against updated ScanCode rule trees,
- run targeted parity debugging.

So the embedded path should become the default, not the only path.

### Step 9: Add deterministic snapshot generation checks

Add tests or validation tools that ensure the generated snapshot is stable.

Suggested checks:

- generating the snapshot twice from the same input yields identical bytes,
- `from_embedded()` and `from_directory()` produce equivalent indexes for representative invariants,
- the snapshot schema version rejects incompatible data cleanly.

This will require deterministic ordering of serialized collections.

### Step 10: Add parity and regression tests

Before making the embedded path the default everywhere, add tests for:

#### Engine equivalence

- compare `from_embedded()` vs `from_directory()` on representative license texts,
- compare raw match outputs for a focused fixture set,
- compare final detection outputs for golden-style samples.

#### Startup behavior

- binary initializes license detection successfully when `reference/.../data` is absent,
- custom `--license-rules-path` still overrides the embedded dataset.

#### Failure handling

- corrupt snapshot bytes fail with a useful error,
- schema mismatch fails clearly,
- empty-pattern edge cases still behave the same after snapshot roundtrip.

### Step 11: Update developer workflows

Add a documented workflow for regenerating the checked-in snapshot.

Suggested commands:

```sh
./setup.sh
cargo run --bin generate-license-snapshot
```

If a dedicated generator binary is added, document that it is the canonical way to refresh the embedded artifact after updating the reference submodule.

### Step 12: Update user-facing docs

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

## Recommended Execution Order

Implement in this order to reduce risk:

1. split engine construction into embedded vs directory paths,
2. define snapshot schema,
3. refactor index builder to produce snapshot-ready parts,
4. add generator and build integration,
5. add runtime embedded loader,
6. add equivalence tests,
7. switch CLI default,
8. update docs and release workflow.

## Non-Goals for the First Version

These can wait until after the embedded snapshot path is working:

- serializing `AhoCorasick` internals directly,
- removing custom directory loading,
- minimizing binary size aggressively,
- adding snapshot delta updates,
- solving warm-cache and embedded-index loading in the same change.

## Expected Outcome

After this work:

- the released binary starts license detection without requiring external rule files,
- startup no longer reparses tens of thousands of rule files,
- crates.io and release builds remain self-contained,
- developers still retain the ability to test custom or updated rule datasets,
- and the embedded path creates a clean foundation for future startup optimizations.
