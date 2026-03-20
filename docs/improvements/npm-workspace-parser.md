# npm Workspace: Improvements Over Python

## Summary

Our Rust implementation improves on the Python reference in two areas:

- ✨ **New Feature: pnpm-workspace.yaml metadata extraction** — Python recognizes the file but extracts no metadata (NonAssemblable handler)
- ✨ **Improved Assembly: Workspace assembly with per-member packages** — Python has basic workspace assembly; Rust adds exclusion patterns, sibling-merge cleanup, and more robust member discovery

## Part 1: Parser Improvements

### Problem in Python Reference

Python ScanCode has a `PnpmWorkspaceYamlHandler` in `packagedcode/npm.py`, but it is declared as `NonAssemblable` — meaning it only detects the file's presence without extracting any useful data from it.

The handler recognizes `pnpm-workspace.yaml` files but produces no package metadata, no workspace pattern extraction, and no structural information about the monorepo.

### Our Solution

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
  "type": "npm",
  "extra_data": {
    "datasource_id": "pnpm_workspace_yaml",
    "workspaces": ["packages/*", "apps/*", "tools/*"]
  }
}
```

### What Gets Extracted

| Field                      | Source           | Description                                                 |
| -------------------------- | ---------------- | ----------------------------------------------------------- |
| `package_type`             | hardcoded        | `"npm"` (consistent with ecosystem)                         |
| `extra_data.datasource_id` | hardcoded        | `"pnpm_workspace_yaml"`                                     |
| `extra_data.workspaces`    | `packages` field | Array of glob patterns defining workspace package locations |

### Supported Patterns

The parser handles all pnpm workspace glob patterns:

- `"packages/*"` — Single-level wildcard
- `"**/components/*"` — Recursive wildcard
- `"!packages/excluded"` — Negation patterns
- `"*"` — Root-level wildcard
- Empty or missing `packages` field — Graceful fallback

## Part 2: Assembly Improvements

### What Python Does

The Python reference handles workspace assembly by:

- Reads `workspaces` from package.json
- Reads `pnpm-workspace.yaml` if present
- Creates separate Package for each workspace member
- Uses `walk_npm()` to assign resources, skipping `node_modules`
- Resolves `workspace:*` version references

### What Rust Improves

Rust achieves feature parity with that assembly behavior and adds several improvements:

| Feature                                  | Python | Rust | Improvement                                                |
| ---------------------------------------- | ------ | ---- | ---------------------------------------------------------- |
| Workspace root detection                 | ✅     | ✅   | Equivalent                                                 |
| Member discovery via globs               | ✅     | ✅   | Three-tier matching (simple, single-star, complex)         |
| Per-member Package creation              | ✅     | ✅   | Equivalent                                                 |
| `workspace:*` version resolution         | ✅     | ✅   | Equivalent                                                 |
| `workspace:^` / `workspace:~` resolution | ✅     | ✅   | Equivalent                                                 |
| `for_packages` assignment                | ✅     | ✅   | Equivalent                                                 |
| pnpm variant handling                    | ✅     | ✅   | Equivalent                                                 |
| **Exclusion patterns**                   | ❌     | ✅   | 🆕 Respects `!pattern` negation in workspace globs         |
| **Sibling-merge cleanup**                | ❌     | ✅   | 🆕 Removes duplicate packages from earlier assembly phases |
| **Explicit dependency cleanup**          | ❌     | ✅   | 🆕 Cleans up root dependencies after hoisting to members   |

### Bugs Fixed from Python

1. **No exclusion pattern support**: Python ignores `!pattern` entries in workspace globs; Rust filters them out during member discovery
2. **Duplicate packages**: Python doesn't clean up packages created by sibling-merge before workspace assembly, leading to duplicates; Rust explicitly removes them
3. **Version resolution timing**: Python resolves workspace versions during parsing; Rust defers to assembly phase where all member versions are known
4. **Root package cleanup**: Python keeps private root packages in output; Rust removes them when all content is assigned to members
5. **Member validation**: Python doesn't validate that discovered members actually have package.json files; Rust verifies before creating packages

## Impact

- **Monorepo visibility**: pnpm workspaces are increasingly common; extracting their structure provides context for dependency analysis
- **SBOM completeness**: Workspace configuration files are no longer opaque to the scanner
- **Correct package counts**: No duplicate packages from assembly phase interactions
- **Accurate dependency graphs**: `workspace:*` references resolved to actual versions

## References

### pnpm Documentation

- [pnpm-workspace.yaml](https://pnpm.io/pnpm-workspace_yaml)

The parser and assembly behavior above describe the stable workspace improvements that matter to users.
