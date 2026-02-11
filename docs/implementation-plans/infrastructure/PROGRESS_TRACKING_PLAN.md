# Progress Tracking & Reporting Implementation Plan

> **Status**: 🟡 Planning Complete — Ready for Implementation
> **Priority**: P3 - Low Priority (UX Feature)
> **Estimated Effort**: 1-2 weeks
> **Dependencies**: None (integrates with existing scanner pipeline)

## Table of Contents

- [Overview](#overview)
- [Python Reference Analysis](#python-reference-analysis)
- [Current State in Rust](#current-state-in-rust)
- [Rust Architecture Design](#rust-architecture-design)
- [Implementation Phases](#implementation-phases)
- [Beyond-Parity Improvements](#beyond-parity-improvements)
- [Testing Strategy](#testing-strategy)
- [Success Criteria](#success-criteria)

---

## Overview

Enhanced progress reporting during scans: multi-phase progress bars, ETA, throughput metrics, real-time status updates, verbosity control, and scan summary statistics. The system must integrate cleanly with rayon parallel processing, avoid interfering with structured JSON output, and support quiet/verbose/default modes.

### Scope

**In Scope:**

- **Multi-phase progress bars**: File discovery, scanning, assembly, output writing
- **ETA calculation**: Estimated time to completion (built-in to indicatif)
- **Throughput metrics**: Files/second, bytes/second in scan summary
- **Verbosity control**: `--quiet` (suppress all), `--verbose` (file-by-file details)
- **Scan summary**: Statistics displayed at end of scan (counts, timings, speed)
- **Error display**: Real-time error reporting alongside progress bars
- **Logging integration**: Log messages printed above progress bars (not corrupting them)
- **Terminal detection**: Automatic TTY detection, graceful degradation when piped

**Out of Scope:**

- Plugin system for custom progress reporters (deferred to plugin system plan)
- Streaming JSON output during scan (deferred to output formats plan)
- Per-scanner timing (we have a monolithic pipeline, not a plugin-based one)
- Remote progress reporting (webhooks, etc.)

---

## Python Reference Analysis

### Source Files

| File | Lines | Role |
|------|-------|------|
| `reference/scancode-toolkit/src/scancode/cli.py` | 1670+ | Main CLI with all progress orchestration |
| `commoncode.cliutils` (external) | N/A | `progressmanager`, `path_progress_message` |
| `reference/scancode-toolkit/src/scancode/interrupt.py` | ~100 | Per-file timeout handling |
| `reference/scancode-toolkit/src/scancode/pool.py` | ~50 | Multiprocessing pool management |

### Python Architecture: 7 Scanning Phases

Python ScanCode runs through 7 sequential phases, each with its own progress reporting:

```text
1. Inventory     → "Collect file inventory..."  (no progress bar)
2. Setup         → Per-plugin setup messages    (verbose only)
3. Pre-Scan      → Stage/plugin messages        (verbose only)
4. Scan          → Progress bar OR file listing  (main phase)
5. Post-Scan     → Stage/plugin messages        (verbose only)
6. Output Filter → Stage/plugin messages        (verbose only)
7. Output        → "Save scan results as: ..."  (verbose only)
```

Only phase 4 (Scan) gets a progress bar. All other phases use simple text messages.

### Verbosity Modes

Python supports three mutually exclusive output modes:

| Mode | Flag | Behavior |
|------|------|----------|
| **Quiet** | `-q` / `--quiet` | No progress, no summary, no messages (only errors in red) |
| **Default** | (none) | Progress bar for scan phase + summary at end |
| **Verbose** | `-v` / `--verbose` | File-by-file path listing instead of bar + detailed messages |

Key detail: `--quiet` and `--verbose` are **mutually exclusive** (enforced by Click's `conflicting_options`).

### Progress Bar Implementation

Python uses `commoncode.cliutils.progressmanager` — a wrapper around Click's progress bar that:

1. Wraps an iterator of `(location, path, scan_errors, scan_time, scan_result, scan_timings)` tuples
2. Displays progress as items are consumed (one increment per file)
3. Uses `item_show_func=path_progress_message` to show current file in verbose mode
4. Outputs to **stderr** (never stdout, to avoid corrupting JSON output)
5. Supports context manager protocol (`__enter__`, `render_finish`)

### Scan Summary Statistics

At scan completion, Python displays:

```text
Summary:        licenses, copyrights with 4 process(es)
Errors count:   2
Scan Speed:     42.5 files/sec. 1.23 MB/sec.
Initial counts: 1500 resource(s): 1200 file(s) and 300 directorie(s) for 45.2 MB
Final counts:   1450 resource(s): 1150 file(s) and 300 directorie(s) for 44.8 MB
Timings:
  scan_start: 2024-01-15T10:30:00
  scan_end:   2024-01-15T10:30:35
  setup: 0.50s
  scan: 28.24s
  post-scan: 2.10s
  output: 0.15s
  total: 35.12s
```

### Error Collection

Python accumulates errors at two levels:

1. **Codebase-level errors**: `codebase.errors` list (global errors)
2. **Resource-level errors**: `resource.scan_errors` list (per-file errors)

At completion, `collect_errors()` gathers both into a combined list. Verbose mode includes stack traces; normal mode shows only the error message.

### Timing Infrastructure

Python tracks timing at multiple granularities:

| Scope | Storage | Contents |
|-------|---------|----------|
| Phase | `codebase.timings` | `{phase_name: seconds}` (setup, scan, post-scan, output, total) |
| Plugin | `codebase.timings` | `{stage:plugin_name: seconds}` |
| Per-file | `resource.scan_timings` | `{scanner_name: seconds}` (only if `--timing` flag) |

### Counters

Python tracks counts at multiple stages:

| Stage | Counter Keys |
|-------|-------------|
| Initial | `initial:files_count`, `initial:dirs_count`, `initial:size_count` |
| Pre-scan | `pre-scan-scan:files_count`, `pre-scan-scan:size_count` |
| Scan | `scan:files_count`, `scan:dirs_count`, `scan:size_count`, `scan:scanners` |
| Final | `final:files_count`, `final:dirs_count`, `final:size_count` |

### Human-Readable Size Formatting

Python formats sizes with a `format_size()` function: `0 Byte → 1 Byte → 123 Bytes → 1 KB → 2.45 MB → 1 GB → 12.30 TB`.

### Known Python Limitations

1. **No progress for most phases**: Only the scan phase gets a progress bar. Setup, pre-scan, post-scan, output filter, and output phases show only text messages (and many have `# TODO: add progress indicator` comments).
2. **No throughput in progress bar**: Files/sec and bytes/sec only appear in the final summary, not during scanning.
3. **Single progress bar**: No multi-bar showing different phases simultaneously.
4. **No ETA in verbose mode**: File-by-file listing has no ETA estimate.
5. **Coarse error display**: Errors shown at end, not inline during scanning.
6. **No color in progress bar**: Only summary and error messages use color (via Click's `secho`).

---

## Current State in Rust

### Already Implemented

The Rust codebase has basic progress tracking using `indicatif`:

**`src/main.rs`** — Progress bar creation:

```rust
fn create_progress_bar(total_files: usize) -> Arc<ProgressBar> {
    let progress_bar = ProgressBar::new(total_files as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files processed ({eta})")
            .expect("Failed to create progress bar style")
            .progress_chars("#>-"),
    );
    Arc::new(progress_bar)
}
```

**`src/scanner/process.rs`** — Per-file progress increment:

```rust
file_entries
    .par_iter()
    .map(|(path, metadata)| {
        let file_entry = process_file(path, metadata, scan_strategy);
        progress_bar.inc(1);
        file_entry
    })
    .collect()
```

**`src/main.rs`** — Simple `println!()` messages:

```rust
println!("Exclusion patterns: {:?}", cli.exclude);
println!("Found {} files in {} directories ({} items excluded)", ...);
println!("JSON output written to {}", cli.output_file);
```

### What's Missing

| Feature | Status |
|---------|--------|
| `--quiet` flag | ❌ Not implemented |
| `--verbose` flag | ❌ Not implemented |
| Multi-phase progress | ❌ Only scanning phase has a bar |
| Scan summary statistics | ❌ No summary at end |
| Throughput metrics | ❌ No files/sec or bytes/sec |
| Error display during scan | ❌ Errors only in JSON output |
| Logging integration | ❌ `log` crate available but not wired to progress |
| Assembly progress | ❌ Assembly phase is silent |
| Size formatting | ❌ No human-readable size display |
| Terminal detection | ❌ Progress bar always shown (no piped-output detection) |

### Existing Dependencies

Already in `Cargo.toml`:

```toml
indicatif = "0.18.3"    # Progress bars (already used)
rayon = "1.11.0"         # Parallel processing (already used)
clap = { version = "4.5.57", features = ["derive"] }  # CLI (already used)
log = "0.4.29"           # Logging facade (available but underused)
chrono = "0.4.43"        # Timestamps (already used)
```

In dev-dependencies:

```toml
env_logger = "0.11.8"   # Log implementation (dev only)
```

---

## Rust Architecture Design

### Design Decisions

#### D1: Use `indicatif` with `MultiProgress` for Multi-Phase Tracking

**Decision**: Use `MultiProgress` to manage phase-specific progress bars.

**Rationale**: indicatif is already a dependency, has native rayon support via `ParallelProgressIterator`, and `MultiProgress` supports hierarchical bar management. It is the de facto standard for Rust CLI progress (6.7M downloads/month, used in 5,190+ crates).

**Alternative considered**: Custom progress trait with atomic counters. Rejected because indicatif already handles rate limiting (20 Hz default), terminal detection, and thread-safe updates via `portable-atomic`.

#### D2: Three Verbosity Levels via `--quiet` and `--verbose`

**Decision**: Match Python's three modes — quiet, default, verbose — as mutually exclusive flags.

**Rationale**: Feature parity with Python ScanCode. Users expect the same CLI UX.

**Implementation**: Use `clap`'s conflict mechanism:

```rust
#[arg(short, long, conflicts_with = "verbose")]
pub quiet: bool,

#[arg(short, long, conflicts_with = "quiet")]
pub verbose: bool,
```

#### D3: Stderr for All Progress, Stdout Reserved for Structured Output

**Decision**: All progress bars, messages, and summaries go to stderr. Stdout is reserved for future stdout-based output modes.

**Rationale**: Python does this. It prevents progress from corrupting JSON when output goes to stdout. indicatif defaults to stderr already.

#### D4: Use `indicatif-log-bridge` for Log Integration

**Decision**: Wire `log` crate through `indicatif-log-bridge` so `warn!()` messages from parsers print above the progress bar cleanly.

**Rationale**: Parser code already uses `log::warn!()` for errors. Without the bridge, these messages would corrupt the progress bar display. The bridge intercepts log output and routes it through `MultiProgress::println()`.

**New dependency**: `indicatif-log-bridge = "0.2"`

#### D5: Centralized `ScanProgress` Struct — Not a Trait

**Decision**: Use a concrete `ScanProgress` struct rather than a `trait ProgressReporter`.

**Rationale**: We don't have a plugin system, and the progress reporting has exactly one consumer (the CLI). A trait would add abstraction without benefit. If a plugin system is added later, we can extract a trait then.

#### D6: Move `env_logger` to Regular Dependencies

**Decision**: Move `env_logger` from dev-dependencies to dependencies. It's lightweight and needed for `RUST_LOG` support in production.

**Rationale**: Enables `RUST_LOG=debug scancode-rust ...` for troubleshooting without recompilation. The bridge requires a concrete logger implementation at runtime.

### Data Structures

#### `ScanProgress` — Central Progress Manager

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

/// Manages all progress reporting for a scan.
///
/// Supports three modes:
/// - Quiet: All output suppressed (hidden draw target)
/// - Default: Progress bars on stderr
/// - Verbose: File-by-file messages on stderr (no bar)
pub struct ScanProgress {
    multi: MultiProgress,
    mode: ProgressMode,
    // Phase-specific bars (created lazily)
    discovery_bar: Option<ProgressBar>,
    scan_bar: Option<ProgressBar>,
    assembly_bar: Option<ProgressBar>,
    // Statistics tracking
    stats: ScanStats,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    Quiet,
    Default,
    Verbose,
}

/// Accumulated scan statistics for the final summary.
pub struct ScanStats {
    pub start_time: std::time::Instant,
    pub phase_timings: Vec<(String, f64)>,  // (phase_name, seconds)
    pub initial_files: usize,
    pub initial_dirs: usize,
    pub initial_size: u64,
    pub excluded_count: usize,
    pub final_files: usize,
    pub final_dirs: usize,
    pub error_count: usize,
    pub total_bytes_scanned: u64,
}
```

#### CLI Extensions

```rust
#[derive(Parser, Debug)]
pub struct Cli {
    // ... existing fields ...

    /// Do not print summary or progress
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Print progress as file-by-file path instead of a progress bar.
    /// Print verbose scan counters.
    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,
}
```

### Integration Points

The progress system hooks into the existing scanner pipeline at four points:

```text
main.rs::run()
│
├─ 1. DISCOVERY PHASE ──────────────────────────────────────────
│    count(&dir_path, max_depth, &exclude_patterns)
│    └─ ScanProgress::start_discovery()     ← spinner
│    └─ ScanProgress::finish_discovery()    ← records initial counts
│
├─ 2. LICENSE LOADING ──────────────────────────────────────────
│    load_license_database()
│    └─ ScanProgress::message("Loading SPDX data...")
│
├─ 3. SCAN PHASE ───────────────────────────────────────────────
│    process(&dir_path, ..., &progress)
│    └─ ScanProgress::scan_bar()            ← main progress bar
│    └─ progress_bar.inc(1)                 ← per-file (in rayon)
│    └─ ScanProgress::finish_scan()         ← records scan timing
│
├─ 4. ASSEMBLY PHASE ───────────────────────────────────────────
│    assembly::assemble(&mut files)
│    └─ ScanProgress::start_assembly()      ← assembly bar
│    └─ ScanProgress::finish_assembly()     ← records assembly timing
│
├─ 5. OUTPUT PHASE ─────────────────────────────────────────────
│    write_output(&output_file, &output)
│    └─ ScanProgress::message("Writing output...")
│
└─ 6. SUMMARY ──────────────────────────────────────────────────
     ScanProgress::display_summary()        ← final statistics
```

### Behavior by Mode

| Action | Quiet | Default | Verbose |
|--------|-------|---------|---------|
| Discovery spinner | Hidden | Shown | "Collecting files..." message |
| "Loading SPDX data..." | Hidden | Shown | Shown |
| "Found N files..." | Hidden | Shown | Shown with size details |
| Scan progress bar | Hidden | Shown | Hidden (replaced by per-file listing) |
| Per-file path | Hidden | Hidden | Shown on stderr |
| Per-file errors | Hidden | Shown via `println` on bar | Shown with path and detail |
| Assembly progress | Hidden | Shown (bar) | "Assembling packages..." message |
| "Writing output..." | Hidden | Shown | Shown |
| Scan summary | Hidden | Shown | Shown with extended detail |
| `log::warn!()` from parsers | Hidden | Shown above bar | Shown |

### Module Structure

```text
src/
├── progress.rs          ← NEW: ScanProgress struct, ScanStats, formatting
├── main.rs              ← MODIFIED: wire ScanProgress into run()
├── cli.rs               ← MODIFIED: add --quiet, --verbose flags
├── scanner/
│   └── process.rs       ← MODIFIED: accept ScanProgress instead of Arc<ProgressBar>
```

Single new file (`progress.rs`) plus modifications to three existing files. Minimal surface area.

### Summary Display Format

The scan summary matches Python's structure for familiarity:

```text
Summary:
  Scanned:    1200 files in 300 directories (45.2 MB)
  Excluded:   150 items
  Errors:     2
  Speed:      42.50 files/sec (1.23 MB/sec)
  Packages:   35 assembled from 78 manifests
Timings:
  scan_start: 2025-02-11T19:06:14+01:00
  scan_end:   2025-02-11T19:06:49+01:00
  discovery:  0.12s
  spdx_load:  0.28s
  scan:       28.24s
  assembly:   2.10s
  output:     0.15s
  total:      30.89s
```

### Human-Readable Size Formatting

A `format_size` utility function, matching Python's output format:

```rust
/// Format a byte count into a human-readable string.
///
/// # Examples
/// ```
/// assert_eq!(format_size(0), "0 Bytes");
/// assert_eq!(format_size(1), "1 Byte");
/// assert_eq!(format_size(1024), "1.00 KB");
/// assert_eq!(format_size(2567000), "2.45 MB");
/// ```
pub fn format_size(bytes: u64) -> String { ... }
```

---

## Implementation Phases

### Phase 1: CLI Flags and ScanProgress Skeleton (Day 1)

**Goal**: Add `--quiet`/`--verbose` flags and create the `ScanProgress` struct with mode-aware construction.

**Tasks**:

1. Add `quiet` and `verbose` fields to `Cli` struct with `conflicts_with`
2. Create `src/progress.rs` with:
   - `ProgressMode` enum
   - `ScanProgress::new(mode: ProgressMode)` constructor
   - `ScanProgress::message(&self, msg: &str)` — prints to stderr (respects mode)
   - `ScanProgress::scan_bar(&self) -> ProgressBar` — returns the scan progress bar
3. Wire `ScanProgress` into `main.rs::run()` replacing raw `println!()` calls
4. Replace `create_progress_bar()` with `ScanProgress::scan_bar()`
5. Replace `Arc<ProgressBar>` in `process()` signature with `&ProgressBar`

**Verification**: `cargo test`, `cargo clippy`, manual test with `--quiet` and `--verbose`

### Phase 2: Multi-Phase Progress Bars (Day 2)

**Goal**: Add discovery spinner, assembly progress, and output message.

**Tasks**:

1. Add `start_discovery()` / `finish_discovery()` methods:
   - Default mode: Spinner with "Collecting files..."
   - Verbose mode: Text message
   - Quiet mode: Hidden
2. Add `start_assembly()` / `finish_assembly()` methods:
   - Default mode: Progress bar (if assembly has meaningful count)
   - Verbose mode: "Assembling packages..." text
   - Quiet mode: Hidden
3. Add output phase messaging

**Verification**: Manual test of all three modes, verify no visual glitches

### Phase 3: Logging Integration (Day 2-3)

**Goal**: Wire `log` crate through `indicatif-log-bridge` so parser warnings display cleanly.

**Tasks**:

1. Move `env_logger` from dev-dependencies to dependencies
2. Add `indicatif-log-bridge` dependency
3. Initialize logging bridge in `main()`:

   ```rust
   let logger = env_logger::Builder::from_env(
       env_logger::Env::default().default_filter_or("warn")
   ).build();
   let level = logger.filter();
   LogWrapper::new(progress.multi().clone(), logger).try_init().unwrap();
   log::set_max_level(level);
   ```

4. Remove `println!("Loading SPDX data...")` and replace with `info!("Loading SPDX data...")`
5. Verify parser `warn!()` messages print above progress bar

**New dependency**: `indicatif-log-bridge = "0.2"`

**Verification**: Scan a directory with a malformed package manifest, verify warning appears above bar

### Phase 4: Scan Statistics and Summary (Day 3-4)

**Goal**: Track and display comprehensive scan statistics at completion.

**Tasks**:

1. Add `ScanStats` struct with timing and count fields
2. Track phase timings using `std::time::Instant`:
   - `discovery`, `spdx_load`, `scan`, `assembly`, `output`, `total`
3. Track counts: initial files/dirs/size, excluded, final files/dirs, errors, packages assembled
4. Implement `format_size()` utility
5. Implement `display_summary()` method matching the format shown above
6. Wire timing collection into each phase of `run()`

**Verification**: Compare summary output format with Python ScanCode for a reference scan

### Phase 5: Verbose Mode — File-by-File Listing (Day 4)

**Goal**: In verbose mode, replace the progress bar with per-file path listing on stderr.

**Tasks**:

1. In verbose mode, `scan_bar()` returns a hidden progress bar (tracking only, no display)
2. Add `ScanProgress::file_completed(&self, path: &str, had_error: bool)` method:
   - Verbose mode: Prints file path to stderr (errors in red if terminal supports color)
   - Default/quiet: No-op
3. Modify `process.rs` to call `file_completed()` after each file in the rayon loop
4. Verbose summary: Show extended detail (per-phase file counts, size counts)

**Verification**: Run with `--verbose`, verify file-by-file output matches Python behavior

### Phase 6: Error Display Enhancements (Day 5)

**Goal**: Surface scan errors during scanning, not just in JSON output.

**Tasks**:

1. In default mode: Use `progress_bar.println()` to show errors inline during scan
2. In verbose mode: Show error details per file
3. At end: Display error summary before statistics (matching Python's "Some files failed to scan properly:" format)
4. Color support: Errors in red, progress in cyan/green (only if terminal supports color)

**Verification**: Scan a directory with intentionally broken files, verify error display in all three modes

---

## Beyond-Parity Improvements

### B1: Multi-Phase Progress Bars (Python Has Single Bar)

Python only shows a progress bar for the scan phase. All other phases display text messages or nothing. Rust shows a spinner for discovery, a bar for scanning, and a bar for assembly — providing continuous visual feedback.

### B2: Throughput in Progress Bar Template (Python Shows Only at End)

Rust's progress bar template can include `{per_sec}` to show real-time files/second during scanning, not just in the final summary. indicatif computes this from its internal rate tracking.

### B3: Automatic Terminal Detection (Python Relies on Click)

indicatif + console automatically detect TTY, terminal width, color support, and the `NO_COLOR`/`TERM` environment variables. The progress bar gracefully degrades when piped, without any manual detection code.

### B4: Rate-Limited Updates (Python Updates Every File)

indicatif rate-limits redraws to 20 Hz by default, preventing terminal I/O from becoming a bottleneck on fast scans. Python's Click-based progress bar does not have built-in rate limiting.

### B5: Log Messages Above Progress Bar

With `indicatif-log-bridge`, any `warn!()` or `info!()` log message from parser code is automatically routed above the progress bar, maintaining clean display. Python has no equivalent — log messages would corrupt the progress bar if logging were enabled during a scan.

### B6: Real-Time Error Count in Progress Bar

The progress bar template can include a dynamic error counter:

```text
⠿ [00:02:15] [████████████░░░░░░░░░░░░] 450/1200 files (2 errors) (ETA: 3m12s)
```

Python only shows the error count in the final summary.

---

## Testing Strategy

### Unit Tests

1. **`format_size()`**: Property-based test that output is always ≤ 8 chars, round-trip consistency
2. **`ProgressMode` from CLI flags**: `(quiet=false, verbose=false)` → Default, `(true, false)` → Quiet, `(false, true)` → Verbose
3. **`ScanStats` accumulation**: Verify timing tracking produces reasonable values
4. **Summary formatting**: Snapshot test of summary output for known stats values

### Integration Tests

1. **Quiet mode**: Run scan with `--quiet`, verify stderr is empty
2. **Default mode**: Run scan, verify progress bar appeared on stderr (check for `\r` / ANSI escape sequences)
3. **Verbose mode**: Run scan with `--verbose`, verify file paths appear on stderr
4. **Piped output**: Run `scancode-rust dir -o out.json 2>/dev/null`, verify JSON is valid (no progress corruption)
5. **Error display**: Scan directory with malformed files, verify errors appear on stderr

### Manual Verification

1. Narrow terminal (< 40 cols): Verify bar doesn't overflow
2. Non-TTY (piped): Verify no ANSI escape codes in output
3. `NO_COLOR=1`: Verify monochrome output
4. `RUST_LOG=debug`: Verify log messages appear above progress bar
5. Large scan (10K+ files): Verify ETA is reasonable, no visual glitches

---

## Success Criteria

- [ ] `--quiet` suppresses all stderr output (progress + summary + messages)
- [ ] `--verbose` shows file-by-file paths and extended summary on stderr
- [ ] Default mode shows progress bar with ETA on stderr
- [ ] Progress bar never corrupts JSON output file
- [ ] Scan summary shows: file count, dir count, size, speed, errors, timings
- [ ] `log::warn!()` messages from parsers print above progress bar
- [ ] Progress bars auto-hide when stderr is not a terminal (piped)
- [ ] All existing tests pass without modification
- [ ] `cargo clippy` clean

---

## Dependency Summary

| Crate | Status | Purpose |
|-------|--------|---------|
| `indicatif` 0.18.3 | Already present | Progress bars, multi-bar, rayon integration |
| `clap` 4.5.57 | Already present | CLI argument parsing |
| `log` 0.4.29 | Already present | Logging facade |
| `rayon` 1.11.0 | Already present | Parallel processing |
| `chrono` 0.4.43 | Already present | Timestamps for summary |
| `env_logger` 0.11.8 | Move to deps | Logger implementation for `RUST_LOG` |
| `indicatif-log-bridge` ~0.2 | **New** | Route log messages above progress bars |

**Total new crates**: 1 (`indicatif-log-bridge`). Plus moving `env_logger` from dev to regular deps.

---

## Related Documents

- **Architecture**: [`docs/ARCHITECTURE.md`](../../ARCHITECTURE.md) — Scanner pipeline section
- **Caching Plan**: [`CACHING_PLAN.md`](CACHING_PLAN.md) — Cache loading will need progress reporting
- **Python Reference**: `reference/scancode-toolkit/src/scancode/cli.py` — Lines 256-268 (flags), 1178-1376 (scanner progress), 1476-1608 (summary display)
