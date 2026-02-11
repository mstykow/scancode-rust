# CLI Implementation Plan

> **Status**: 🟡 5 parameters implemented, 30+ pending
> **Priority**: P1 - High (User-facing feature parity)
> **Dependencies**: Most flags depend on their underlying features (license detection, output formats, etc.)

## Overview

CLI parameter parity with Python ScanCode. Rust uses `clap`; Python uses `click`.

**Location**: `src/cli.rs`

## Parameter Mapping

### Implemented (5)

| Parameter | Notes |
|-----------|-------|
| `<dir_path>` | Positional argument |
| `-o, --output-file` | Default: "output.json" |
| `-m, --max-depth` | Default: 50 |
| `-e, --exclude` | Glob patterns |
| `--no-assemble` | Rust-specific |

### Core Parameters (pending)

| Parameter | Blocked By |
|-----------|------------|
| `-n, --processes` | — (thread pool control) |
| `--timeout` | — (per-file timeout) |
| `-q, --quiet` | PROGRESS_TRACKING_PLAN.md |
| `-v, --verbose` | PROGRESS_TRACKING_PLAN.md |
| `--strip-root` | — |
| `--full-root` | — |

### Output Format Flags (pending)

| Parameter | Blocked By |
|-----------|------------|
| `--yaml` | OUTPUT_FORMATS_PLAN.md |
| `--csv` | OUTPUT_FORMATS_PLAN.md |
| `--html` | OUTPUT_FORMATS_PLAN.md |
| `--spdx-tv` | OUTPUT_FORMATS_PLAN.md |
| `--spdx-rdf` | OUTPUT_FORMATS_PLAN.md |
| `--cyclonedx` | OUTPUT_FORMATS_PLAN.md |

### Scan Option Flags (pending)

| Parameter | Blocked By |
|-----------|------------|
| `--license` | LICENSE_DETECTION_PLAN.md |
| `--copyright` | COPYRIGHT_DETECTION_PLAN.md |
| `--email` | EMAIL_URL_DETECTION_PLAN.md |
| `--url` | EMAIL_URL_DETECTION_PLAN.md |
| `--license-score` | LICENSE_DETECTION_PLAN.md |
| `--license-text` | LICENSE_DETECTION_PLAN.md |
| `--classify` | SUMMARIZATION_PLAN.md |
| `--summary` | SUMMARIZATION_PLAN.md |

### Post-Scan Flags (pending)

| Parameter | Blocked By |
|-----------|------------|
| `--consolidate` | CONSOLIDATION_PLAN.md |
| `--filter-clues` | — |
| `--is-license-text` | LICENSE_DETECTION_PLAN.md |
| `--license-clarity-score` | LICENSE_DETECTION_PLAN.md |
| `--summary-key-files` | SUMMARIZATION_PLAN.md |

### Input/Output Control (pending)

| Parameter | Notes |
|-----------|-------|
| `--from-json` | Load from previous scan |
| `--include` | Include patterns |
| `--mark-source` | Mark source files |
| `--only-findings` | Filter output |

### Rust-Specific (planned)

| Parameter | Blocked By |
|-----------|------------|
| `--no-cache` | CACHING_PLAN.md |
| `--cache-dir` | CACHING_PLAN.md |

## Key Design Decisions

1. **Compile-time features over runtime plugins** — Rust's strength is compile-time optimization. No dynamic loading.
2. **Match Python CLI surface** — Same flags, same behavior. Consider `--format` enum for Rust v2.0.
3. **`--package` always on** — Package scanning runs by default (no flag needed).
4. **`--no-assemble` is Rust-specific** — Python always assembles.

## Differences from Python

- No plugin system (compile-time features instead)
- Thread pool via rayon instead of multiprocessing
- JSON output structure matches Python (`SCANCODE_OUTPUT_FORMAT_VERSION`)

## References

- **Python CLI**: `reference/scancode-toolkit/src/scancode/cli.py`
- **Clap docs**: https://docs.rs/clap/
