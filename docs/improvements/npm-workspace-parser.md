# npm Workspace Parser: Improvements Over Python

## Summary

Our Rust implementation improves on the Python reference by:

- ✨ **New Feature: pnpm-workspace.yaml metadata extraction** — Python recognizes the file but extracts no metadata (NonAssemblable handler)

## Problem in Python Reference

Python ScanCode has a `PnpmWorkspaceYamlHandler` in `packagedcode/npm.py`, but it is declared as `NonAssemblable` — meaning it only detects the file's presence without extracting any useful data from it.

The handler recognizes `pnpm-workspace.yaml` files but produces no package metadata, no workspace pattern extraction, and no structural information about the monorepo.

## Our Solution

We implemented `NpmWorkspaceParser` which extracts workspace configuration data from `pnpm-workspace.yaml` files, including:

- Workspace package glob patterns (e.g., `packages/*`, `apps/*`)
- Monorepo structure information
- Negation patterns for excluded packages

### Before/After Comparison

**Python Output** (stub — NonAssemblable):

```json
{
  "type": "npm",
  "name": null,
  "version": null,
  "extra_data": {}
}
```

**Rust Output** (real extraction):

```json
{
  "type": "npm-workspace",
  "extra_data": {
    "datasource_id": "pnpm_workspace_yaml",
    "workspaces": [
      "packages/*",
      "apps/*",
      "tools/*"
    ]
  }
}
```

## What Gets Extracted

| Field | Source | Description |
|-------|--------|-------------|
| `package_type` | hardcoded | `"npm-workspace"` |
| `extra_data.datasource_id` | hardcoded | `"pnpm_workspace_yaml"` |
| `extra_data.workspaces` | `packages` field | Array of glob patterns defining workspace package locations |

### Supported Patterns

The parser handles all pnpm workspace glob patterns:

- `"packages/*"` — Single-level wildcard
- `"**/components/*"` — Recursive wildcard
- `"!packages/excluded"` — Negation patterns
- `"*"` — Root-level wildcard
- Empty or missing `packages` field — Graceful fallback

## Impact

- **Monorepo visibility**: pnpm workspaces are increasingly common; extracting their structure provides context for dependency analysis
- **SBOM completeness**: Workspace configuration files are no longer opaque to the scanner

## References

### Python Reference

- `reference/scancode-toolkit/src/packagedcode/npm.py` — `PnpmWorkspaceYamlHandler` (NonAssemblable, no extraction)

### pnpm Documentation

- [pnpm-workspace.yaml](https://pnpm.io/pnpm-workspace_yaml)

## Status

- ✅ **Implementation**: Complete, validated, production-ready
- ✅ **Testing**: Unit tests covering all pattern types and edge cases
- ✅ **Documentation**: Complete
