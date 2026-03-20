# Bazel Module Parser

**Parser**: `BazelModuleParser`

## Why This Exists

Python ScanCode currently has only an open attempt at `MODULE.bazel` support. Provenant now parses Bazel Bzlmod module manifests directly.

## What We Extract

- Bazel module identity from `module(name=..., version=...)`
- dependency declarations from `bazel_dep(...)`
- dev/runtime distinction from `dev_dependency = True`
- compatibility metadata such as `compatibility_level` and `bazel_compatibility`
- dependency-side metadata such as `repo_name`, `registry`, and `max_compatibility_level`
- common override declarations such as `archive_override`, `git_override`, and `local_path_override`
- parser goldens and a standalone assembly golden for `MODULE.bazel`

## Reference limitation

The Python reference does not currently provide merged `MODULE.bazel` support, so Bazel module metadata is easy to miss as package evidence.

## Rust behavior

Rust parses Bzlmod manifests directly and preserves module identity, dependency declarations, override metadata, and dev-scope information as structured package data.

## Impact

- Better Bazel/Bzlmod dependency visibility
- Better support for modern Bazel module-based dependency management
- Better alignment with purl-spec’s Bazel package type
