# Bun Lock Parser: Text Lockfile Extraction and Legacy Binary Compatibility

## Summary

Rust now parses Bun's text-based `bun.lock` format, and it also adds a first static compatibility layer for legacy binary `bun.lockb` fixtures Bun still migrates.

This adds Bun lockfile dependency visibility that the Python reference does not currently provide, while keeping the still-unverified current binary format and any deeper Bun-specific binary sections as explicit follow-up work.

## Reference limitation

The Python reference does not currently parse Bun lockfiles, so Bun-managed dependency state is easy to miss during scans.

## Rust Improvements

### 1. Parse Bun text lockfiles as JSONC

Rust now reads `bun.lock` using JSONC-compatible parsing instead of assuming strict JSON.

This matters because Bun's text lockfile format includes trailing commas in real-world examples and is documented by Bun as a text/JSONC lockfile format rather than plain JSON.

### 2. Extract root and workspace dependency scopes

Rust now uses Bun's `workspaces` table to recover:

- root package identity when present
- direct `dependencies`
- direct `devDependencies`
- direct `optionalDependencies`
- direct `peerDependencies`
- workspace package versions for `workspace:` references

This means Bun lockfile dependencies are not treated as an undifferentiated flat list.

### 3. Extract resolved packages from Bun tuple entries

Rust now parses Bun's `packages` entries into resolved package records for:

- registry packages
- workspace packages
- file/link style packages
- git / GitHub / URL-like locators at the dependency level

For registry packages, Rust also preserves integrity-derived hashes and registry download URLs.

### 4. Preserve nested dependency metadata

Rust now extracts nested dependency edges from Bun package metadata objects, including:

- `dependencies`
- `optionalDependencies`
- `peerDependencies`

Workspace package tuples that omit inline metadata now fall back to the corresponding workspace entry, so workspace-local dependency data is not lost.

### 5. Integrate Bun into npm-family assembly

Rust now treats `bun.lock` as part of the npm-family sibling assembly path next to `package.json`.

This includes:

- sibling merge with `package.json`
- identity guarding so mismatched Bun lockfiles do not merge into the wrong npm package
- workspace hoisting for root Bun lockfile dependencies in npm-style monorepos

### 6. Static legacy `bun.lockb` v2 compatibility

Rust now statically parses Bun's official legacy `bun.lockb.v2` migration fixtures without shelling out to Bun.

The parser currently:

- validates Bun's binary magic header
- validates the legacy format version before decoding
- reconstructs root package identity
- reconstructs direct dependency scopes from dependency behavior flags
- reconstructs resolved package versions, resolved URLs, integrity, and nested dependency edges
- prefers sibling `bun.lock` when both text and binary lockfiles are present
- integrates `bun.lockb` into the same npm-family sibling/workspace assembly hooks as Bun text lockfiles

This stays inside the project's security boundary: no subprocess parsing, no execution of Bun against untrusted inputs, and fail-closed behavior on unsupported versions.

## Scope Boundary

This improvement now covers:

- the current text-based `bun.lock`
- legacy Bun binary lockfile **format v2** compatibility, using Bun's own committed migration fixtures as the oracle

Remaining Bun binary follow-up work is intentionally narrower than before because:

- Bun's current binary serializer version is newer than the legacy v2 fixtures we can verify today
- optional tagged binary sections and current-format parity still need source-coupled validation before they should be claimed as fully supported
- the migration path from `bun.lockb` to `bun.lock` remains the official Bun direction of travel

## Coverage

Coverage spans Bun text lockfile parsing, legacy `bun.lockb` compatibility boundaries, npm-family sibling assembly, and workspace dependency hoisting.
