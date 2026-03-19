# Deno Parser

**Parsers**: `DenoParser`, `DenoLockParser`

## Why This Exists

Python ScanCode currently has no `deno.json`, `deno.jsonc`, or `deno.lock` support. Provenant now parses Deno configuration manifests and current v5 lockfiles directly.

## What We Extract

- publishable package identity from `name`, `version`, and `exports`,
- import-map dependency declarations from `imports`,
- Deno config metadata such as `scopes`, `links`, `tasks`, and `lock`,
- current `deno.lock` v5 `specifiers`, `jsr`, `npm`, `redirects`, `remote`, and `workspace.dependencies` sections,
- resolved JSR and npm packages with integrity data,
- direct remote module entries from `redirects` plus their locked remote hashes,
- sibling assembly between `deno.json(c)` and `deno.lock`.

## Reference limitation

The Python reference does not currently support Deno manifests or lockfiles, so modern Deno dependency data is easy to miss during scans.

## Rust behavior

Rust parses Deno configuration and lockfile inputs directly, recovers publishable package identity, keeps import and remote-module metadata, and assembles manifest plus lockfile evidence when both are present.

## Impact

- Better JS/TS dependency visibility for Deno projects
- Better support for modern `jsr:` / `npm:` / remote-import workflows
- Better lockfile-backed package evidence for reproducible Deno builds
