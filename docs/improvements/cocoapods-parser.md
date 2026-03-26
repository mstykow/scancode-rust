# CocoaPods Parser Improvements

## Summary

Rust now goes beyond the current Python ScanCode CocoaPods handling in two concrete ways:

1. refines dependency scope handling so runtime/development semantics are more honest across `.podspec`, `.podspec.json`, `Podfile`, and `Podfile.lock`
2. avoids duplicate package explosion for `RxDataSources.podspec`

## Python Status

- Current Python CocoaPods handling is split across `.podspec`, `.podspec.json`, `Podfile`, and `Podfile.lock` handlers.
- Upstream still has unresolved scope-semantics questions and a history of duplicate-output bugs around `RxDataSources.podspec`.

## Rust Improvements

### Refined scope handling

- `.podspec` parsing now distinguishes:
  - `add_dependency` / `add_runtime_dependency` → `scope = runtime`, `is_runtime = true`, `is_optional = false`
  - `add_development_dependency` → `scope = development`, `is_runtime = false`, `is_optional = true`
- `.podspec.json` dependencies now use the same runtime-oriented scope instead of the vague `dependencies` label.
- `Podfile` dependencies now use the more honest `scope = dependencies` with runtime/optional left unknown instead of forcing them to look like unconditional runtime dependencies.
- `Podfile.lock` dependencies now also use `scope = dependencies` and leave runtime/optional unset, because the lockfile does not encode enough information to prove those booleans safely.

### Duplicate-output protection

- Rust emits exactly one package with a bounded, non-duplicated dependency set for `RxDataSources.podspec`.
- This avoids the historical output blow-up seen around duplicate CocoaPods package expansion.

## Coverage

Coverage spans the refined scope semantics across CocoaPods manifests and lockfiles, including protection against historical duplicate-output regressions.
