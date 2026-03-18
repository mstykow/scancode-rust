# UV Lock Parser

**Parser**: `UvLockParser`

## Why This Exists

Python ScanCode currently has no `uv.lock` support. Provenant now parses uv lockfiles directly, which closes a modern Python packaging gap without waiting for upstream reference support.

## What We Extract

- root project identity from the local `virtual` or `editable` package entry,
- direct runtime and development dependencies from root-package dependency groups,
- resolved package versions for all locked packages,
- dependency markers and source provenance in preserved extra data,
- artifact provenance from `sdist` / `wheels` entries,
- lockfile metadata such as format version, revision, and `requires-python`.

## Why It Is Beyond Parity

- **Python status**: no `uv.lock` handler
- **Rust status**: dedicated parser, tests, golden fixture, datasource ID, and PyPI sibling assembly support

## Impact

- Better Python dependency visibility for uv-managed projects
- Better root-package recovery when only `uv.lock` is available during scans
- Better alignment with the current Python packaging ecosystem
