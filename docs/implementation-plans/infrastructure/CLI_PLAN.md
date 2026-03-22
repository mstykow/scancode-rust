# CLI Implementation Plan

> **Status**: 🟡 Core CLI parity implemented; cache controls are now wired, with remaining feature-blocked flags still pending
> **Priority**: P1 - High (user-facing drop-in replacement parity)
> **Dependencies**: Some flags depend on underlying features (license detection, post-scan processing, caching)

## Overview

CLI parameter parity with Python ScanCode. Rust uses `clap`; Python uses
`click` + plugin-provided options.

This plan tracks progress toward a **drop-in replacement CLI surface**.

**Location**: [`src/cli.rs`](../../../src/cli.rs)

## Parameter Mapping

### Implemented (drop-in-oriented core)

| Parameter                  | Notes                                                                                                             |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `<dir_path>`               | Positional argument                                                                                               |
| `--json <FILE>`            | Compact JSON output                                                                                               |
| `--json-pp <FILE>`         | Pretty JSON output                                                                                                |
| `--json-lines <FILE>`      | JSON Lines output                                                                                                 |
| `--yaml <FILE>`            | YAML output                                                                                                       |
| `--csv <FILE>`             | CSV output (deprecated upstream but supported)                                                                    |
| `--html <FILE>`            | HTML report output                                                                                                |
| `--spdx-tv <FILE>`         | SPDX Tag/Value output                                                                                             |
| `--spdx-rdf <FILE>`        | SPDX RDF/XML output                                                                                               |
| `--cyclonedx <FILE>`       | CycloneDX JSON output                                                                                             |
| `--cyclonedx-xml <FILE>`   | CycloneDX XML output                                                                                              |
| `--custom-output <FILE>`   | Custom template output file                                                                                       |
| `--custom-template <FILE>` | Required with `--custom-output`                                                                                   |
| `-m, --max-depth`          | Default: 0 (no depth limit)                                                                                       |
| `-n, --processes`          | Compatible with `0` and `-1` values                                                                               |
| `--timeout`                | Per-file timeout option                                                                                           |
| `-q, --quiet`              | Quiet mode                                                                                                        |
| `-v, --verbose`            | Verbose path mode                                                                                                 |
| `--strip-root`             | Relative path normalization                                                                                       |
| `--full-root`              | Absolute path reporting                                                                                           |
| `--exclude` / `--ignore`   | Glob patterns (`--ignore` for ScanCode parity)                                                                    |
| `--from-json`              | Load previous JSON scan(s) from positional input (`<dir_path>...`); incompatible with `--copyright/--email/--url` |
| `--include`                | Include path patterns                                                                                             |
| `--mark-source`            | Mark source-heavy files/directories                                                                               |
| `--only-findings`          | Filter output to files with findings                                                                              |
| `--filter-clues`           | Filter redundant clues                                                                                            |
| `-c, --copyright`          | Copyright/holder/author detection toggle                                                                          |
| `-e, --email`              | Enable email detection                                                                                            |
| `-u, --url`                | Enable URL detection                                                                                              |
| `--no-assemble`            | Rust-specific                                                                                                     |
| `--max-email`              | Threshold (default 50, requires `--email`)                                                                        |
| `--max-url`                | Threshold (default 50, requires `--url`)                                                                          |

### Core Parameters (partial)

| Parameter         | Status                                                                   |
| ----------------- | ------------------------------------------------------------------------ |
| `--max-in-memory` | Parsed and exposed; full parity memory/disk-spill behavior still pending |

### Scan Option Flags (pending)

| Parameter                   | Blocked By                                                                     |
| --------------------------- | ------------------------------------------------------------------------------ |
| `--license`                 | [`LICENSE_DETECTION_ARCHITECTURE.md`](../../LICENSE_DETECTION_ARCHITECTURE.md) |
| `--license-score`           | [`LICENSE_DETECTION_ARCHITECTURE.md`](../../LICENSE_DETECTION_ARCHITECTURE.md) |
| `--license-text`            | [`LICENSE_DETECTION_ARCHITECTURE.md`](../../LICENSE_DETECTION_ARCHITECTURE.md) |
| `--classify`                | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |
| `--facet <facet>=<pattern>` | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |
| `--generated`               | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |
| `--summary`                 | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |

Runtime dependency notes:

- `--summary` requires `--classify` for parity-compatible behavior.
- `--facet <facet>=<pattern>` defines per-facet glob rules; files not matched by any facet default to `core` in the reference implementation.

### Post-Scan Flags (pending)

| Parameter                 | Blocked By                                                                     |
| ------------------------- | ------------------------------------------------------------------------------ |
| `--is-license-text`       | [`LICENSE_DETECTION_ARCHITECTURE.md`](../../LICENSE_DETECTION_ARCHITECTURE.md) |
| `--license-clarity-score` | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |
| `--tallies`               | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |
| `--tallies-with-details`  | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |
| `--tallies-key-files`     | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |
| `--tallies-by-facet`      | [`SUMMARIZATION_PLAN.md`](../post-processing/SUMMARIZATION_PLAN.md)            |

Runtime dependency notes:

- `--license-clarity-score` requires `--classify` for parity-compatible behavior.
- `--tallies-key-files` requires `--classify` and `--tallies`.
- `--tallies-by-facet` requires `--facet <facet>=<pattern>` and `--tallies`.

### Explicitly Deferred / Not Planned

| Parameter       | Decision                                                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `--consolidate` | Intentionally not planned for current Provenant scope; see [`CONSOLIDATION_PLAN.md`](../post-processing/CONSOLIDATION_PLAN.md) |

### Input/Output Control (implemented)

| Parameter         | Notes                                                                        |
| ----------------- | ---------------------------------------------------------------------------- |
| `--from-json`     | Load from previous scan JSON input(s); disallows `--copyright/--email/--url` |
| `--include`       | Include patterns                                                             |
| `--mark-source`   | Mark source files                                                            |
| `--only-findings` | Filter output                                                                |

### Cache Control (partial)

| Parameter         | Positioning / Status                                                                            |
| ----------------- | ----------------------------------------------------------------------------------------------- |
| `--cache-dir`     | Implemented and wired in runtime startup cache bootstrap                                        |
| `--cache-clear`   | Implemented and wired to clear persisted cache before directory scan starts                     |
| `--max-in-memory` | Implemented as CLI surface; parity-equivalent spill behavior remains pending in caching roadmap |
| `--no-cache`      | Optional Rust-specific convenience; not parity-required                                         |
| `--incremental`   | CACHING_PLAN.md (beyond parity; defer until robust model)                                       |

## Key Design Decisions

1. **Compile-time features over runtime plugins** — Rust prioritizes
   compile-time optimization.
2. **Match Python CLI surface for drop-in replacement** — preserve canonical
   ScanCode option names and argument shape (especially output options).
3. **Avoid parallel output-spec APIs** — do not expose a second primary output
   selection mechanism that diverges from ScanCode usage.
4. **`--package` always on** — package scanning runs by default (no flag needed).
5. **`--no-assemble` is Rust-specific** — Python always assembles.

## Differences from Python (current intentional)

- No plugin runtime architecture (compile-time wiring instead)
- `--consolidate` is intentionally not planned because it is compatibility-oriented and upstream-deprecated
- Thread pool via rayon instead of multiprocessing
- JSON output structure matches Python (`OUTPUT_FORMAT_VERSION`)
- `--no-cache` is not a parity requirement (upstream removed it); if retained, it is Rust-specific

## References

- **Python CLI**:
  [`reference/scancode-toolkit/src/scancode/cli.py`](../../../reference/scancode-toolkit/src/scancode/cli.py)
- **Output plugins (reference CLI options)**:
  [`reference/scancode-toolkit/src/formattedcode/`](../../../reference/scancode-toolkit/src/formattedcode/)
- **Clap docs**: https://docs.rs/clap/
