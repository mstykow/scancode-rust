# Go Workspace Parser

**Parser**: `GoWorkParser`

## Why This Exists

Python ScanCode currently has only an open attempt at `go.work` support. Provenant now parses Go workspace files directly using the official `go.work` grammar.

## What We Extract

- workspace-level `go` and `toolchain` directives,
- `use` workspace member paths,
- local workspace member module identities by reading the referenced `go.mod` files,
- `replace` directives with old/new module or local-path metadata,
- sibling assembly between a root `go.mod` and `go.work` when both exist in the same directory.

## Reference limitation

The Python reference does not currently provide merged `go.work` support, so Go workspace structure remains easy to miss during scans.

## Rust behavior

Rust parses `go.work` directly, recovers module identities from `use` entries, and assembles workspace-level metadata with a sibling root `go.mod` when both are present.

## Impact

- Better Go monorepo/workspace visibility
- Better workspace-member ownership signals during package detection
- Better support for modern Go multi-module development layouts
