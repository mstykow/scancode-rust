# pylock.toml Parser

**Parser**: `PylockTomlParser`

## Why This Exists

Python ScanCode currently has no `pylock.toml` support. Provenant now parses the standardized lockfile format defined by PEP 751 and the PyPA `pylock.toml` specification.

## What We Extract

- lockfile-level metadata such as `lock-version`, `created-by`, `requires-python`, environments, extras, dependency groups, and default groups,
- all `[[packages]]` entries as locked Python package dependencies,
- package-level dependency edges from `packages.dependencies`,
- source provenance from `vcs`, `directory`, `archive`, `sdist`, and `wheels` records,
- artifact hashes and download locations,
- group/extra-aware runtime and optional classification for package roots and their transitive dependencies.

## Reference limitation

The Python reference does not currently support `pylock.toml`, which leaves a gap for standards-based Python lockfile workflows.

## Rust behavior

Rust parses `pylock.toml` directly, preserves lockfile and artifact provenance, and assembles the lockfile with sibling Python project metadata when both inputs are available.

## Impact

- Better Python dependency visibility for standards-based lockfiles
- Better interoperability with emerging PyPA lockfile tooling
- Better long-term coverage for modern Python packaging workflows
