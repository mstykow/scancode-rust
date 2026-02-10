# CLI Implementation Plan

> **Status**: 🟡 Planning - 5 parameters implemented, 30+ pending
> **Priority**: P1 - High (User-facing feature parity)
> **Estimated Effort**: 6-8 weeks (phased approach)
> **Dependencies**: Plugin system, output formats, scan options

## Overview

This plan tracks implementation of all CLI parameters from Python ScanCode to achieve feature parity. The Rust implementation uses `clap` for CLI parsing instead of Python's `click` framework.

## Current Implementation

**Location**: `src/cli.rs`

**Implemented Parameters** (5 total):

- `<dir_path>` - Positional argument for directory to scan
- `-o, --output-file` - Output JSON file path (default: "output.json")
- `-m, --max-depth` - Maximum recursion depth (default: 50)
- `-e, --exclude` - Glob patterns to exclude (comma-delimited)
- `--no-assemble` - Disable package assembly (Rust-specific)

## Python Reference Analysis

**Location**: `reference/scancode-toolkit/src/scancode/cli.py`

**Total Parameters**: 30+ (core + plugin-based)

### Parameter Categories

1. **Core Parameters** (10) - Basic scan control
2. **Information Flags** (5) - Help, version, examples
3. **Output Format Options** (8) - JSON, YAML, SPDX, CycloneDX, etc.
4. **Scan Options** (10+) - License, copyright, package, email, url, etc.
5. **Performance Options** (5) - Processes, timeout, memory
6. **Post-Scan Options** (5+) - Consolidate, classify, summarize

## Parameter Mapping

### Core Parameters

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `input` | ✅ | ✅ Implemented | cli.rs:7 | Positional `dir_path` |
| `-o, --output-file` | ✅ | ✅ Implemented | cli.rs:10 | Default: output.json |
| `--max-depth` | ✅ | ✅ Implemented | cli.rs:14 | Default: 50 |
| `--exclude` | ✅ | ✅ Implemented | cli.rs:18 | Glob patterns |
| `-n, --processes` | ✅ | ❌ Not started | — | Parallelization control |
| `--timeout` | ✅ | ❌ Not started | — | Scan timeout |
| `-q, --quiet` | ✅ | ❌ Planned | PROGRESS_TRACKING_PLAN.md | Suppress output |
| `-v, --verbose` | ✅ | ❌ Planned | PROGRESS_TRACKING_PLAN.md | Detailed output |
| `--strip-root` | ✅ | ❌ Not started | — | Path handling |
| `--full-root` | ✅ | ❌ Not started | — | Path handling |

### Information Flags

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `-h, --help` | ✅ | ✅ Auto (clap) | — | Clap generates automatically |
| `-V, --version` | ✅ | ✅ Auto (clap) | — | Clap generates automatically |
| `--about` | ✅ | ❌ Not started | — | Show about info |
| `--examples` | ✅ | ❌ Not started | — | Show usage examples |
| `--plugins` | ✅ | ❌ Not started | — | List available plugins |

### Output Format Options

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `--json` | ✅ | ✅ Implemented | — | Default output format |
| `--json-pp` | ✅ | ✅ Implemented | — | Pretty-printed (default) |
| `--yaml` | ✅ | ❌ Planned | OUTPUT_FORMATS_PLAN.md | YAML output |
| `--csv` | ✅ | ❌ Planned | OUTPUT_FORMATS_PLAN.md | CSV output |
| `--html` | ✅ | ❌ Planned | OUTPUT_FORMATS_PLAN.md | HTML output |
| `--spdx-tv` | ✅ | ❌ Planned | OUTPUT_FORMATS_PLAN.md | SPDX Tag/Value format |
| `--spdx-rdf` | ✅ | ❌ Planned | OUTPUT_FORMATS_PLAN.md | SPDX RDF format |
| `--cyclonedx` | ✅ | ❌ Planned | OUTPUT_FORMATS_PLAN.md | CycloneDX format |

### Scan Options (Plugin-Based in Python)

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `--package` | ✅ | ✅ Implemented | — | Always on (parsers run) |
| `--license` | ✅ | ❌ Not started | LICENSE_DETECTION_PLAN.md | License detection |
| `--copyright` | ✅ | ❌ Not started | COPYRIGHT_DETECTION_PLAN.md | Copyright detection |
| `--email` | ✅ | ❌ Not started | EMAIL_URL_DETECTION_PLAN.md | Email extraction |
| `--url` | ✅ | ❌ Not started | EMAIL_URL_DETECTION_PLAN.md | URL extraction |
| `--info` | ✅ | ✅ Implemented | — | File info (always on) |
| `--classify` | ✅ | ❌ Not started | SUMMARIZATION_PLAN.md | File classification |
| `--summary` | ✅ | ❌ Not started | SUMMARIZATION_PLAN.md | License/copyright tallies |
| `--license-score` | ✅ | ❌ Not started | LICENSE_DETECTION_PLAN.md | Minimum confidence threshold |
| `--license-text` | ✅ | ❌ Not started | LICENSE_DETECTION_PLAN.md | Include matched license text |
| `--license-url-template` | ✅ | ❌ Not started | LICENSE_DETECTION_PLAN.md | Template for license URLs |

### Performance Options

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `-n, --processes` | ✅ | ❌ Not started | — | Thread pool size |
| `--timeout` | ✅ | ❌ Not started | — | Per-file timeout |
| `--max-in-memory` | ✅ | ❌ Deferred | — | Memory management |
| `--timing` | ✅ | ❌ Not started | — | Performance metrics |
| `--max-depth` | ✅ | ✅ Implemented | cli.rs:14 | Recursion limit |

### Post-Scan Options

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `--consolidate` | ✅ | ❌ Not started | CONSOLIDATION_PLAN.md | Package deduplication |
| `--filter-clues` | ✅ | ❌ Not started | — | Filter low-confidence results |
| `--is-license-text` | ✅ | ❌ Not started | — | Detect license files |
| `--license-clarity-score` | ✅ | ❌ Not started | — | License clarity scoring |
| `--summary` | ✅ | ❌ Not started | SUMMARIZATION_PLAN.md | Tallies and facets |
| `--summary-key-files` | ✅ | ❌ Not started | SUMMARIZATION_PLAN.md | Key file tallies |

### Input Options

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `--from-json` | ✅ | ❌ Not started | — | Load from previous scan |
| `--include` | ✅ | ❌ Not started | — | Include patterns |
| `--ignore` | ✅ | ❌ Not started | — | Ignore patterns (deprecated) |

### Output Control Options

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `--strip-root` | ✅ | ❌ Not started | — | Strip root directory |
| `--full-root` | ✅ | ❌ Not started | — | Full absolute paths |
| `--mark-source` | ✅ | ❌ Not started | — | Mark source files |
| `--only-findings` | ✅ | ❌ Not started | — | Only files with findings |

### Rust-Specific Parameters

| Parameter | Python | Rust Status | Location | Notes |
|-----------|--------|-------------|----------|-------|
| `--no-assemble` | ❌ | ✅ Implemented | cli.rs:22 | Disable package assembly |
| `--no-cache` | ❌ | ❌ Planned | CACHING_PLAN.md | Disable caching |
| `--cache-dir` | ❌ | ❌ Planned | CACHING_PLAN.md | Cache directory |

## Architecture Design

### Plugin System vs. Compile-Time Features

**Decision**: Compile-time feature flags (not runtime plugins)

**Rationale**:

- Rust's strength is compile-time optimization
- No dynamic loading overhead
- Simpler dependency management
- Users can build minimal binaries

**Implementation**:

```rust
#[cfg(feature = "license-detection")]
mod license;

#[derive(Parser)]
struct Cli {
    #[cfg(feature = "license-detection")]
    #[arg(long)]
    license: bool,
}
```

### Output Format Selection

**Python Approach**: Separate flag per format (`--json`, `--yaml`, `--spdx-tv`, etc.)

**Rust Option 1**: Single `--format` flag with enum (cleaner UX, breaking change)

```rust
#[derive(ValueEnum, Clone)]
enum OutputFormat {
    Json,
    JsonPp,
    Yaml,
    Csv,
    Html,
    SpdxTv,
    SpdxRdf,
    CycloneDx,
}

#[arg(long, default_value = "json-pp")]
format: OutputFormat,
```

**Rust Option 2**: Match Python exactly (better compatibility, cluttered)

```rust
#[arg(long)]
json: Option<String>,

#[arg(long)]
yaml: Option<String>,

#[arg(long)]
spdx_tv: Option<String>,
```

**Recommendation**: Start with Option 2 for exact parity, consider Option 1 for Rust v2.0.

### Scan Option Flags

**Decision**: Individual boolean flags (match Python UX)

**Implementation**:

```rust
#[arg(long)]
license: bool,

#[arg(long)]
copyright: bool,

#[arg(long)]
email: bool,
```

## Implementation Phases

### Phase 1: Core UX (1-2 weeks) - MVP

**Goal**: Standard CLI user experience

**Tasks**:

1. Add `--version` flag (clap auto-generates)
2. Add `--quiet` / `--verbose` flags
3. Add `--processes` flag (thread pool control)
4. Add `--timeout` flag
5. Add `--strip-root` / `--full-root` flags
6. Update README with all current flags

**Success Criteria**:

- [ ] `--version` shows version
- [ ] `--quiet` suppresses progress output
- [ ] `--verbose` shows detailed logging
- [ ] `--processes` controls parallelization
- [ ] `--timeout` limits scan time
- [ ] `--strip-root` / `--full-root` adjust paths

### Phase 2: Output Formats (2-3 weeks)

**Goal**: Support multiple output formats

**Dependencies**: OUTPUT_FORMATS_PLAN.md

**Tasks**:

1. Add separate output format flags (`--yaml`, `--csv`, etc.)
2. Implement YAML serialization
3. Implement CSV serialization
4. Implement SPDX Tag/Value serialization
5. Implement SPDX RDF serialization
6. Implement CycloneDX serialization
7. Implement HTML template rendering

**Success Criteria**:

- [ ] `--yaml` produces YAML output
- [ ] `--csv` produces CSV output
- [ ] `--spdx-tv` produces SPDX Tag/Value
- [ ] `--spdx-rdf` produces SPDX RDF
- [ ] `--cyclonedx` produces CycloneDX JSON
- [ ] `--html` produces HTML report

### Phase 3: Scan Options (4-6 weeks)

**Goal**: Enable/disable scan features

**Dependencies**: LICENSE_DETECTION_PLAN.md, COPYRIGHT_DETECTION_PLAN.md, EMAIL_URL_DETECTION_PLAN.md

**Tasks**:

1. Add `--license` flag (requires license detection engine)
2. Add `--copyright` flag (requires copyright detection)
3. Add `--email` flag (requires email extraction)
4. Add `--url` flag (requires URL extraction)
5. Add `--license-score` flag (confidence threshold)
6. Add `--license-text` flag (include matched text)
7. Add `--license-url-template` flag
8. Add `--classify` flag (requires file classification)
9. Add `--summary` flag (requires summarization)

**Success Criteria**:

- [ ] `--license` enables license detection
- [ ] `--copyright` enables copyright detection
- [ ] `--email` enables email extraction
- [ ] `--url` enables URL extraction
- [ ] Flags can be combined (e.g., `--license --copyright`)

### Phase 4: Advanced Options (1-2 weeks)

**Goal**: Performance and post-processing options

**Dependencies**: CACHING_PLAN.md, CONSOLIDATION_PLAN.md

**Tasks**:

1. Add `--no-cache` / `--cache-dir` flags
2. Add `--consolidate` flag
3. Add `--timing` flag (performance metrics)
4. Add `--from-json` flag (load previous scan)
5. Add `--include` flag (include patterns)
6. Add `--filter-clues` flag
7. Add `--only-findings` flag
8. Add `--mark-source` flag

**Success Criteria**:

- [ ] `--no-cache` disables caching
- [ ] `--cache-dir` sets cache location
- [ ] `--consolidate` deduplicates packages
- [ ] `--timing` shows performance metrics
- [ ] `--from-json` loads previous results
- [ ] `--only-findings` filters output

## Testing Strategy

### Unit Tests

Test each CLI parameter:

```rust
#[test]
fn test_quiet_flag() {
    let cli = Cli::parse_from(&["scancode-rust", ".", "--quiet"]);
    assert!(cli.quiet);
}

#[test]
fn test_output_format_yaml() {
    let cli = Cli::parse_from(&["scancode-rust", ".", "--yaml", "output.yaml"]);
    assert_eq!(cli.yaml, Some("output.yaml".to_string()));
}
```

### Integration Tests

Test flag combinations:

```rust
#[test]
fn test_license_copyright_combined() {
    let cli = Cli::parse_from(&["scancode-rust", ".", "--license", "--copyright"]);
    assert!(cli.license);
    assert!(cli.copyright);
}

#[test]
fn test_conflicting_flags() {
    // Should fail - quiet and verbose are mutually exclusive
    let result = Cli::try_parse_from(&["scancode-rust", ".", "--quiet", "--verbose"]);
    assert!(result.is_err());
}
```

### Golden Tests

Compare output formats against Python reference:

```bash
# Python
scancode -p samples/ --json python-output.json

# Rust
scancode-rust samples/ -o rust-output.json

# Compare (ignoring UUIDs, timestamps)
diff <(jq 'del(.headers)' python-output.json) <(jq 'del(.headers)' rust-output.json)
```

## Documentation Requirements

### README.md

Update CLI options section with all implemented flags (already done for current 5 flags).

### --help Output

Clap auto-generates help text from doc comments:

```rust
/// Disable package assembly (merging related manifest/lockfiles into packages)
#[arg(long)]
pub no_assemble: bool,
```

### User Guide

Create `docs/CLI_REFERENCE.md` with:

- Detailed explanation of each flag
- Usage examples
- Common flag combinations
- Performance tuning guide

## Migration Notes

### Differences from Python

1. **No plugin system**: Compile-time features instead
2. **`--no-assemble` flag**: Rust-specific (Python always assembles)
3. **Output format flags**: Match Python exactly (separate flag per format)
4. **Default behavior**: Package scanning always on (no `--package` flag needed)
5. **Thread pool**: Rust uses rayon thread pool instead of multiprocessing

### Compatibility

- JSON output structure matches Python (SCANCODE_OUTPUT_FORMAT_VERSION)
- PURL format identical
- Dependency scopes use native ecosystem terminology (matches Python)
- Path handling (`--strip-root`, `--full-root`) must match Python exactly

## Success Criteria

- [ ] All core parameters implemented (10/10)
- [ ] All information flags implemented (5/5)
- [ ] All output formats implemented (8/8)
- [ ] All scan options implemented (11/11)
- [ ] All performance options implemented (5/5)
- [ ] All post-scan options implemented (6/6)
- [ ] All input options implemented (3/3)
- [ ] All output control options implemented (4/4)
- [ ] README documents all flags
- [ ] CLI_REFERENCE.md created
- [ ] Integration tests pass
- [ ] Golden tests pass against Python reference

## References

- **Python CLI**: `reference/scancode-toolkit/src/scancode/cli.py`
- **Python Plugins**: `reference/scancode-toolkit/src/*/plugin_*.py`
- **Clap Documentation**: https://docs.rs/clap/
- **Output Formats Plan**: `docs/implementation-plans/output/OUTPUT_FORMATS_PLAN.md`
- **License Detection Plan**: `docs/implementation-plans/text-detection/LICENSE_DETECTION_PLAN.md`
- **Copyright Detection Plan**: `docs/implementation-plans/text-detection/COPYRIGHT_DETECTION_PLAN.md`
- **Progress Tracking Plan**: `docs/implementation-plans/infrastructure/PROGRESS_TRACKING_PLAN.md`
- **Caching Plan**: `docs/implementation-plans/infrastructure/CACHING_PLAN.md`
- **Consolidation Plan**: `docs/implementation-plans/package-detection/CONSOLIDATION_PLAN.md`
