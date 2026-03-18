# Ruby Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode Ruby handling in several concrete ways:

1. resolves gemspec constants from required local Ruby files instead of leaving all external constants unresolved
2. preserves Bundler `GIT` / `PATH` source metadata at parser level and proves it with parser goldens
3. merges extracted gem metadata layouts without duplicate package/dependency emission and assigns nested extracted files to the assembled gem package
4. tags nested Ruby legal/readme/manifest files as `key_file`, promotes package metadata from them, and computes a top-level `license_clarity_score`

## Python Status

- Python already strips many literal `.freeze` suffixes and parses Gemfile.lock source sections, but several upstream Ruby issues remain open around constant resolution, extracted-gem duplication, false dependency parsing, and key-file tagging.
- Upstream still leaves external constant references unresolved in gemspec output and does not have extracted-gem assembly regression coverage.
- Upstream issue `#3881` specifically tracks nested Ruby `LICENSE` files not being tagged as `key_file`, which in turn keeps package-level attribution and the codebase `license_clarity_score` at 0.

## Rust Improvements

### Required-file constant resolution

- Gemspec parsing now loads required local Ruby files when resolving constant-backed gemspec fields.
- This resolves values like:
  - `ProviderDSL::GemDescription::NAME`
  - `ProviderDSL::GemDescription::VERSION`
  - `ProviderDSL::GemDescription::AUTHORS`
  - `ProviderDSL::GemDescription::EMAIL`
  - `ProviderDSL::GemDescription::PAGE`
- The resolver stays intentionally narrow: it only looks at local required Ruby files adjacent to the gemspec, not arbitrary Ruby load paths.

### Gemfile.lock source metadata proof

- Bundler `GIT` and `PATH` metadata is now pinned with parser goldens for:
  - git `remote`
  - `revision`
  - `branch`
  - `ref`
  - source-type tagging
  - PATH primary-package identity behavior

### False-dependency protection

- Gemspec parser coverage now explicitly proves that description text mentioning `add_dependency`-like strings does not create fake dependencies.

### Extracted gem assembly

- `metadata.gz-extract` now assembles with sibling extracted gemspec/Gemfile/Gemfile.lock layouts instead of standing alone.
- Extracted gem dependency duplication is deduped during nested merge.
- Ruby package-root resource assignment now attaches nested files under the gem root — including subdirectory `LICENSE` files and Ruby source files — to the assembled gem package.

### Key-file tagging and summary scoring

- File-level classification now tags package-associated Ruby legal/readme/manifest files with:
  - `is_legal`
  - `is_manifest`
  - `is_readme`
  - `is_top_level`
  - `is_key_file`
- Nested files listed in Ruby package `file_references` are treated as top-level for that package even when they are not at filesystem depth 1.
- Package metadata is promoted from key files when missing, including:
  - `declared_license_expression`
  - `declared_license_expression_spdx`
  - `license_detections`
  - `copyright`
  - `holder`
- Output now includes a top-level `summary.license_clarity_score` block derived from key files, plus the combined summary declared license expression.

## Validation

- `cargo test ruby --lib`
- `cargo test --features golden-tests ruby_golden --lib`
- `cargo test --features golden-tests test_assembly_ruby_extracted_basic --lib`
- `cargo test --bin provenant`
- `cargo test --test output_format_golden`

## Related Issues

- Ruby batch: #151, #154, #156, #158, #160
- Follow-up resolved here: #161
