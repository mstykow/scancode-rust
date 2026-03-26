# Hex Lock Parser: Static `mix.lock` Support

## Summary

Rust now parses Hex `mix.lock` files statically and extracts locked dependency information without executing Elixir code.

This delivers exact package versions, nested dependency requirements, repo provenance, and checksum metadata from the structured lockfile, while keeping executable `mix.exs` parsing out of scope.

## Upstream / Reference Context

The Python reference does not ship a Hex `mix.lock` parser, and the two Mix surfaces differ sharply in risk and parseability:

- `mix.lock` is structured lock data
- `mix.exs` is executable Elixir and commonly contains computed values, helper calls, module attributes, and environment-dependent logic

That makes `mix.lock` the safest static Hex surface to support.

## Rust Improvements

### 1. Static `mix.lock` parsing with no code execution

Rust now parses the subset of Elixir term syntax actually used in `mix.lock`:

- maps
- tuples
- lists
- keyword lists
- strings
- atoms
- booleans
- integers

This is enough to decode real Hex lock entries without invoking Mix or evaluating Elixir code.

### 2. Locked Hex dependency extraction

For `:hex` entries, Rust now extracts:

- package/app key
- Hex package name
- exact locked version
- repository name
- manager list
- inner checksum
- outer checksum
- nested dependency requirements
- nested dependency `optional` flags

That gives scans concrete locked-package inventory instead of only source-manifest intent.

### 3. Nested dependency reconstruction

Rust now reconstructs nested dependency edges from lock entries, preserving:

- dependency app name
- package alias from `hex:` when present
- requirement string
- repository value
- optional flag

This makes the lockfile useful not just for package inventory but for dependency graph recovery too.

### 4. Safe, honest scope boundary

Rust intentionally ignores non-`:hex` lock entries instead of pretending broader Mix support.

Rust also intentionally does **not** attempt `mix.exs` parsing.

That keeps the implementation honest:

- no Elixir execution
- no `mix` subprocess usage
- no brittle partial evaluation of project manifests

## Scope Boundary

This improvement intentionally covers:

- static `mix.lock` parsing for Hex dependency data

This improvement intentionally does **not** yet claim support for:

- `mix.exs` project identity extraction
- direct dependency declaration extraction from executable Elixir manifests
- umbrella/project metadata from Mix source files

Those remain outside the supported scope described here.

## Primary Areas Affected

- Hex locked dependency extraction
- nested dependency graph recovery for Elixir projects
- safe parsing of Elixir-term lockfile syntax

## Coverage

Coverage includes:

- unit tests for basic Hex lockfiles
- unit tests for alias/nested dependency cases
- unit tests for malformed/non-Hex entries
- parser goldens for a real `mix.lock` fixture
