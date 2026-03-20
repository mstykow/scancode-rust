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

## Reference limitation

The Python reference does not currently support `uv.lock`, which leaves a gap for uv-managed Python environments.

## Rust behavior

Rust parses `uv.lock` directly, recovers root-package identity and locked dependency data, and combines that lockfile evidence with sibling Python package metadata during scans.

## Impact

- Better Python dependency visibility for uv-managed projects
- Better root-package recovery when only `uv.lock` is available during scans
- Better alignment with the current Python packaging ecosystem
