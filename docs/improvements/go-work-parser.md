# Go Workspace Parser

**Parser**: `GoWorkParser`

## Why This Exists

Python ScanCode currently has only an open attempt at `go.work` support. scancode-rust now parses Go workspace files directly using the official `go.work` grammar.

## What We Extract

- workspace-level `go` and `toolchain` directives,
- `use` workspace member paths,
- local workspace member module identities by reading the referenced `go.mod` files,
- `replace` directives with old/new module or local-path metadata,
- sibling assembly between a root `go.mod` and `go.work` when both exist in the same directory,
- parser goldens and assembly golden coverage for workspace scenarios.

## Why It Is Beyond Parity

- **Python status**: no merged `go.work` handler
- **Rust status**: dedicated parser, real fixtures, local module-path recovery from `use` entries, and assembly coverage

## Impact

- Better Go monorepo/workspace visibility
- Better workspace-member ownership signals during package detection
- Better support for modern Go multi-module development layouts
