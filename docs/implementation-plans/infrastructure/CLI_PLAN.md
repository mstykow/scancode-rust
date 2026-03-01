# CLI Implementation Plan

> **Status**: 🟡 Drop-in output flag surface implemented; broader CLI parity remains
> **Priority**: P1 - High (user-facing drop-in replacement parity)
> **Dependencies**: Some flags depend on underlying features (license detection, post-scan processing, caching)

## Overview

CLI parameter parity with Python ScanCode. Rust uses `clap`; Python uses
`click` + plugin-provided options.

This plan tracks progress toward a **drop-in replacement CLI surface**.

**Location**: [`src/cli.rs`](../../../src/cli.rs)

## Parameter Mapping

### Implemented (drop-in-oriented core)

| Parameter                  | Notes                                          |
| -------------------------- | ---------------------------------------------- |
| `<dir_path>`               | Positional argument                            |
| `--json <FILE>`            | Compact JSON output                            |
| `--json-pp <FILE>`         | Pretty JSON output                             |
| `--json-lines <FILE>`      | JSON Lines output                              |
| `--yaml <FILE>`            | YAML output                                    |
| `--csv <FILE>`             | CSV output (deprecated upstream but supported) |
| `--html <FILE>`            | HTML report output                             |
| `--spdx-tv <FILE>`         | SPDX Tag/Value output                          |
| `--spdx-rdf <FILE>`        | SPDX RDF/XML output                            |
| `--cyclonedx <FILE>`       | CycloneDX JSON output                          |
| `--cyclonedx-xml <FILE>`   | CycloneDX XML output                           |
| `--custom-output <FILE>`   | Custom template output file                    |
| `--custom-template <FILE>` | Required with `--custom-output`                |
| `-m, --max-depth`          | Default: 50                                    |
| `-e, --exclude`            | Glob patterns                                  |
| `--no-assemble`            | Rust-specific                                  |
| `--email`                  | Enable email detection                         |
| `--max-email`              | Threshold (default 50, requires `--email`)     |
| `-u, --url`                | Enable URL detection                           |
| `--max-url`                | Threshold (default 50, requires `--url`)       |

### Core Parameters (pending)

| Parameter         | Blocked By                |
| ----------------- | ------------------------- |
| `-n, --processes` | — (thread pool control)   |
| `--timeout`       | — (per-file timeout)      |
| `-q, --quiet`     | PROGRESS_TRACKING_PLAN.md |
| `-v, --verbose`   | PROGRESS_TRACKING_PLAN.md |
| `--strip-root`    | —                         |
| `--full-root`     | —                         |

### Scan Option Flags (pending)

| Parameter         | Blocked By                  |
| ----------------- | --------------------------- |
| `--license`       | LICENSE_DETECTION_PLAN.md   |
| `--copyright`     | COPYRIGHT_DETECTION_PLAN.md |
| `--license-score` | LICENSE_DETECTION_PLAN.md   |
| `--license-text`  | LICENSE_DETECTION_PLAN.md   |
| `--classify`      | SUMMARIZATION_PLAN.md       |
| `--summary`       | SUMMARIZATION_PLAN.md       |

### Post-Scan Flags (pending)

| Parameter                 | Blocked By                |
| ------------------------- | ------------------------- |
| `--consolidate`           | CONSOLIDATION_PLAN.md     |
| `--filter-clues`          | —                         |
| `--is-license-text`       | LICENSE_DETECTION_PLAN.md |
| `--license-clarity-score` | LICENSE_DETECTION_PLAN.md |
| `--summary-key-files`     | SUMMARIZATION_PLAN.md     |

### Input/Output Control (pending)

| Parameter         | Notes                   |
| ----------------- | ----------------------- |
| `--from-json`     | Load from previous scan |
| `--include`       | Include patterns        |
| `--mark-source`   | Mark source files       |
| `--only-findings` | Filter output           |

### Rust-Specific (planned)

| Parameter     | Blocked By      |
| ------------- | --------------- |
| `--no-cache`  | CACHING_PLAN.md |
| `--cache-dir` | CACHING_PLAN.md |

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
- Thread pool via rayon instead of multiprocessing
- JSON output structure matches Python (`SCANCODE_OUTPUT_FORMAT_VERSION`)

## References

- **Python CLI**:
  [`reference/scancode-toolkit/src/scancode/cli.py`](../../../reference/scancode-toolkit/src/scancode/cli.py)
- **Output plugins (reference CLI options)**:
  [`reference/scancode-toolkit/src/formattedcode/`](../../../reference/scancode-toolkit/src/formattedcode/)
- **Clap docs**: https://docs.rs/clap/
