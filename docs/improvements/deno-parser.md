# Deno Parser

**Parsers**: `DenoParser`, `DenoLockParser`

## Why This Exists

Python ScanCode currently has no `deno.json`, `deno.jsonc`, or `deno.lock` support. scancode-rust now parses Deno configuration manifests and current v5 lockfiles directly.

## What We Extract

- publishable package identity from `name`, `version`, and `exports`,
- import-map dependency declarations from `imports`,
- Deno config metadata such as `scopes`, `links`, `tasks`, and `lock`,
- current `deno.lock` v5 `specifiers`, `jsr`, `npm`, `redirects`, `remote`, and `workspace.dependencies` sections,
- resolved JSR and npm packages with integrity data,
- direct remote module entries from `redirects` plus their locked remote hashes,
- sibling assembly between `deno.json(c)` and `deno.lock`.

## Why It Is Beyond Parity

- **Python status**: no Deno parser
- **Rust status**: dedicated manifest and lockfile parsers, tests, generated supported-format docs, datasource IDs, and sibling assembly support

## Impact

- Better JS/TS dependency visibility for Deno projects
- Better support for modern `jsr:` / `npm:` / remote-import workflows
- Better lockfile-backed package evidence for reproducible Deno builds
