# Go Parser Improvements

## Summary

Rust now goes beyond the released Python ScanCode Go handling in several concrete ways:

1. preserves fallback datasource IDs across Go parser error paths
2. captures `replace` coverage in parser goldens instead of leaving directive support only unit-tested
3. adds a dedicated `go.mod graph` parser so direct vs transitive module relationships are modeled separately from `go.sum`
4. categorizes `_test.go` and `//go:build test` files as non-production source for source-directory heuristics

## Python Status

- Python Go support is centered on `go.mod` and `go.sum` with documented upstream gaps for directives, dependency granularity, build-constraint categorization, and module graph support.
- Upstream explicitly tracks `replace`/directive support, `go.sum`/`go.mod` granularity, build-constraint categorization, and module graph support as open issues.

## Rust Improvements

### Directive and fallback correctness

- Existing Rust support for `replace`, `retract`, and `toolchain` is now backed by a real parser golden using the upstream `opencensus-service` fixture.
- Fallback `PackageData` for `go.mod`, `go.sum`, and `Godeps.json` now keep the correct `datasource_id`, which is important for assembly/accounting consistency.

### Module graph support

- Rust adds a dedicated `go.mod.graph` / `go.modgraph` parser for checked-in `go mod graph` output.
- The graph parser models:
  - direct module edges from the main module
  - transitive module edges from dependency modules
  - pinned versions from the graph artifact itself
- This keeps graph semantics distinct from `go.sum`, which remains checksum-focused.

### Build-constraint-aware categorization

- Scanner-side Go categorization now treats these files as non-production source for directory `is_source` heuristics:
  - `_test.go`
  - files with `//go:build test`
  - files with `// +build test`
- This prevents ordinary Go package directories from being penalized in source-count heuristics just because they contain test-only files.

## Validation

- `cargo test go --lib`
- `cargo test --features golden-tests go_golden --lib`
- `cargo test --features golden-tests test_assembly_go_basic --lib`
- `cargo test --features golden-tests test_assembly_go_graph_basic --lib`

## Related Issues

- #152, #153, #155, #218
