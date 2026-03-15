# Bun Lock Parser: Text Lockfile Dependency Extraction

## Summary

Rust now parses Bun's text-based `bun.lock` format and integrates it into the existing npm-family assembly flow.

This adds Bun lockfile dependency visibility that the Python reference does not currently provide, while intentionally leaving legacy binary `bun.lockb` support as separate follow-up work.

## Python Reference Status

- The Python reference does not have Bun lockfile parser coverage today.
- The local parser roadmap previously tracked Bun lockfiles as the highest-value remaining JavaScript-family gap.
- Upstream ScanCode now has a tracking issue for Bun lockfiles, but no reference implementation PR was available when this parser work started.

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

## Scope Boundary

This improvement intentionally covers the **current Bun text lockfile** only.

Legacy binary `bun.lockb` remains follow-up work because:

- Bun itself treats the text lockfile as the current default
- the binary format is much less reviewable and less well documented publicly
- the migration path from `bun.lockb` to `bun.lock` is already an official Bun workflow

## Primary Areas Affected

- Bun lockfile parsing
- npm-family sibling assembly
- npm-family workspace dependency hoisting
- parser and assembly regression coverage for Bun lockfiles

## Verification

This improvement is covered by:

- Bun parser-focused unit tests
- Bun parser golden coverage
- Bun sibling assembly golden coverage
- Bun workspace assembly regression coverage
- Bun lockfile mismatch assembly coverage
