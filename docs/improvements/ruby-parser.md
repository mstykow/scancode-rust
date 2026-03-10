# Ruby Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode Ruby handling in several concrete ways:

1. resolves gemspec constants from required local Ruby files instead of leaving all external constants unresolved
2. preserves Bundler `GIT` / `PATH` source metadata at parser level and now proves it with parser goldens
3. merges extracted gem metadata layouts without duplicate package/dependency emission and assigns nested extracted files to the assembled gem package

## Python Status

- Python already strips many literal `.freeze` suffixes and parses Gemfile.lock source sections, but several upstream Ruby issues remain open around constant resolution, extracted-gem duplication, false dependency parsing, and key-file tagging.
- Upstream still leaves external constant references unresolved in gemspec output and does not have extracted-gem assembly regression coverage.
- Upstream also tracks nested Ruby LICENSE/key-file tagging as a separate unresolved ecosystem/datafile-handler problem.

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

- Bundler `GIT` and `PATH` metadata was already partially present in unit coverage; this batch adds parser goldens that pin:
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

## Deferred / Not in Scope Yet

- **Key-file tagging / `license_clarity_score` infrastructure** is still not implemented generically in this repo.
- So while nested Ruby files are now associated to the correct package, the specific upstream-style `key_file` / `license_clarity_score` problem remains a separate follow-up.

## Validation

- `cargo test ruby --lib`
- `cargo test --features golden-tests ruby_golden --lib`
- `cargo test --features golden-tests test_assembly_ruby_extracted_basic --lib`

## Related Issues

- Fixed in this batch: #151, #154, #156, #158, #160
- Deferred follow-up: #161
