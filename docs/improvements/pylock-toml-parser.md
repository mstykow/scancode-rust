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

## Why It Is Beyond Parity

- **Python status**: no `pylock.toml` handler
- **Rust status**: dedicated parser, tests, golden fixture, datasource ID, and sibling assembly support with `pyproject.toml`

## Impact

- Better Python dependency visibility for standards-based lockfiles
- Better interoperability with emerging PyPA lockfile tooling
- Better long-term coverage for modern Python packaging workflows
