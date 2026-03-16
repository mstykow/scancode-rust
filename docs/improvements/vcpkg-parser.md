# vcpkg Parser: Modern Manifest Support

## Summary

Rust now parses the primary modern vcpkg manifest surface, `vcpkg.json`, for both project manifests and port/library manifests.

This delivers the highest-value part of vcpkg manifest-mode support immediately: direct dependency extraction, package identity for named manifests, and preservation of versioning/configuration metadata that affects dependency resolution.

## Upstream / Reference Context

The current local roadmap issue grouped three related files together:

- `vcpkg.json`
- `vcpkg-configuration.json`
- `vcpkg-lock.json`

But Microsoft’s documentation makes it clear that `vcpkg.json` is the required manifest and direct dependency surface, while configuration and registry lock metadata are supporting layers.

That means the most valuable first step is the manifest itself.

## Rust Improvements

### 1. Strict-JSON `vcpkg.json` parsing

Rust now parses `vcpkg.json` as strict JSON, matching Microsoft’s documented format rules.

This covers both important manifest roles:

- top-level project manifests, where `name` and version can be omitted
- port/library manifests, where `name` and a version field are present

### 2. Version-field normalization for port manifests

Rust supports the documented manifest version fields and folds `port-version` into the final version string when present.

This means a vcpkg port manifest can produce a stable package version even when the packaging revision is tracked separately from the upstream version.

### 3. Direct dependency extraction from string and object forms

Rust now extracts dependencies from both supported dependency syntaxes:

- simple string entries such as `"fmt"`
- object entries with additional manifest metadata

For object dependencies, Rust preserves the most important vcpkg dependency metadata in dependency `extra_data`, including:

- `version>=`
- `features`
- `default-features`
- `host`
- `platform`

This gives the scan result the core modern vcpkg dependency graph without needing lock-state support first.

### 4. Preserve manifest-level resolution metadata

Rust now keeps top-level vcpkg manifest metadata that meaningfully affects dependency resolution or policy, including:

- `builtin-baseline`
- `overrides`
- `supports`
- `default-features`
- `features`

These are stored in `extra_data` so the scan preserves the context needed to understand how the manifest constrains dependency selection.

### 5. Embedded and sibling configuration awareness

When configuration is embedded in `vcpkg.json`, Rust preserves it directly.

When embedded configuration is absent, Rust also opportunistically reads a sibling `vcpkg-configuration.json` and stores it under manifest `extra_data` as configuration metadata.

This improves usefulness for real repositories today without yet claiming that standalone `vcpkg-configuration.json` is modeled as a separate first-class parser surface.

## Scope Boundary

This improvement intentionally covers:

- `vcpkg.json`
- embedded `configuration` / `vcpkg-configuration`
- sibling `vcpkg-configuration.json` ingestion into manifest metadata

This improvement intentionally does **not** yet claim first-class support for:

- standalone `vcpkg-configuration.json` provenance as its own parser/datasource
- `vcpkg-lock.json` registry lock-state parsing

Those are tracked separately as narrower follow-up work after the main manifest parser.

## Primary Areas Affected

- vcpkg project manifest parsing
- vcpkg port/library manifest parsing
- direct dependency extraction for modern vcpkg manifests
- manifest metadata preservation for baselines, overrides, and configuration

## Verification

This improvement is covered by:

- unit tests for project manifests
- unit tests for port/library manifests
- unit tests for project manifests without package identity
- unit tests for sibling configuration ingestion
- parser golden coverage for project manifests
- parser golden coverage for port/library manifests
