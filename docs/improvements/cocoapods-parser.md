# CocoaPods Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode CocoaPods handling in two concrete ways:

1. refines dependency scope handling so runtime/development semantics are more honest across `.podspec`, `.podspec.json`, `Podfile`, and `Podfile.lock`
2. adds an explicit `RxDataSources.podspec` regression proving the parser does not explode into duplicated package information

## Python Status

- Current Python CocoaPods handling is split across `.podspec`, `.podspec.json`, `Podfile`, and `Podfile.lock` handlers.
- Upstream explicitly tracks unresolved scope semantics in issue `#3835` and historically tracked a huge duplicate-output bug for `RxDataSources.podspec` in issue `#2915`.
- Current upstream fixtures now show normal `RxDataSources` output, so the old explosive behavior is a bug to avoid reproducing, not a parity target.

## Rust Improvements

### Refined scope handling

- `.podspec` parsing now distinguishes:
  - `add_dependency` / `add_runtime_dependency` → `scope = runtime`, `is_runtime = true`, `is_optional = false`
  - `add_development_dependency` → `scope = development`, `is_runtime = false`, `is_optional = true`
- `.podspec.json` dependencies now use the same runtime-oriented scope instead of the vague `dependencies` label.
- `Podfile` dependencies now use the more honest `scope = dependencies` with runtime/optional left unknown instead of forcing them to look like unconditional runtime dependencies.
- `Podfile.lock` dependencies now also use `scope = dependencies` and leave runtime/optional unset, because the lockfile does not encode enough information to prove those booleans safely.

### Duplicate-output protection

- Rust now carries an explicit regression test for `RxDataSources.podspec` proving the parser emits exactly one package with a bounded, non-duplicated dependency set.
- This matches the healthy current upstream fixture direction and guards against the old 940MB-style output blow-up.

## Validation

- `cargo test pod --lib`
- `cargo test --features golden-tests cocoapods_golden --lib`

## Related Issues

- #191, #192
