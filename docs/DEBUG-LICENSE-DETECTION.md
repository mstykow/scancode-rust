# Debug License Detection Pipeline

This document describes how to debug the license detection pipeline in both Python (reference) and Rust implementations.

## Overview

License detection is a complex multi-stage pipeline. Debugging tools are essential for understanding why licenses are (or aren't) detected, and for ensuring parity between Python and Rust implementations.

---

## Rust Debug Pipeline

### Location

`src/bin/debug_license_detection.rs`

### Usage

```bash
# Build and run with debug-pipeline feature
cargo run --features debug-pipeline --bin debug_license_detection -- <file_path>

# Example
cargo run --features debug-pipeline --bin debug_license_detection -- testdata/mit.txt
```

### Requirements

- The `debug-pipeline` feature must be enabled
- Reference license data must be available at `reference/scancode-toolkit/src/licensedcode/data`

### Pipeline Stages

The Rust debug script instruments these stages:

| Stage | Description | Key Output |
|-------|-------------|------------|
| 1. Query Building | Tokenizes input, creates query runs | Token count, matchables |
| 2. Hash Matching | SHA1 exact match | Hash matches or "none" |
| 3. SPDX-LID | SPDX-License-Identifier detection | SPDX tag matches |
| 4. Aho-Corasick | Multi-pattern exact matching | Exact matches |
| 5a. Candidate Selection | Set-similarity ranking | Top candidates with scores |
| 5. Sequence Matching | Approximate alignment | Sequence matches |
| 6. Match Merging | Combines adjacent matches | Before/after counts |
| 7. Refinement | Individual filter stages | Per-filter statistics |
| 8. Detection Assembly | Creates final detections | License expressions |

### Example Output

```
================================================================================
 STAGE 7: MATCH REFINEMENT
================================================================================

=== Individual Filter Stages ===

--------------------------------------------------------------------------------
 filter_too_short_matches
--------------------------------------------------------------------------------
  Before: 60, After: 60

--------------------------------------------------------------------------------
 filter_below_rule_minimum_coverage
--------------------------------------------------------------------------------
  Before: 60, After: 53

--------------------------------------------------------------------------------
 filter_contained_matches
--------------------------------------------------------------------------------
  Kept: 2, Discarded: 44

After full refinement:
  Good matches: 2
  Weak matches: 0

================================================================================
 STAGE 8: DETECTION ASSEMBLY
================================================================================
Groups: 1

FINAL DETECTIONS: 1

Detection 1:
  License expression: mit
  SPDX expression: mit
  Matches: 2
```

### Debug Wrapper Functions

The `debug-pipeline` feature exposes internal filter functions via `*_debug_only` wrappers:

| Function | Description |
|----------|-------------|
| `filter_contained_matches_debug_only` | Removes matches contained in larger matches |
| `filter_too_short_matches_debug_only` | Removes matches below minimum length |
| `filter_false_positive_matches_debug_only` | Removes detected false positives |
| `filter_spurious_matches_debug_only` | Removes spurious low-quality matches |
| `filter_below_rule_minimum_coverage_debug_only` | Enforces rule coverage thresholds |
| `filter_short_matches_scattered_on_too_many_lines_debug_only` | Filters scattered matches |
| `filter_matches_missing_required_phrases_debug_only` | Enforces required phrase rules |
| `filter_matches_to_spurious_single_token_debug_only` | Filters single-token matches |
| `filter_invalid_matches_to_single_word_gibberish_debug_only` | Removes gibberish matches |

These are defined in `src/license_detection/debug_pipeline.rs`.

---

## Python Debug Pipeline

### Location

`reference/scancode-playground/debug_license_detection.py`

### Usage

```bash
cd reference/scancode-playground
venv/bin/python debug_license_detection.py <file_path>

# Example
venv/bin/python debug_license_detection.py apache-2.0.LICENSE
```

### Options

- `--min-score=N` - Set minimum score threshold (default: 0)
- `--no-approximate` - Disable approximate matching stage

### Pipeline Stages

The Python script mirrors the Rust stages. See the Python section below for detailed stage descriptions.

---

## Comparing Python vs Rust

When debugging license detection issues, run both scripts on the same file:

```bash
# Python
cd reference/scancode-playground
venv/bin/python debug_license_detection.py /path/to/file.txt

# Rust
cargo run --features debug-pipeline --bin debug_license_detection -- /path/to/file.txt
```

Compare the output at each stage to identify where behavior diverges.

### Common Differences

| Aspect | Python | Rust |
|--------|--------|------|
| Tokenization | Python tokenizer | Rust tokenizer (should match) |
| Match counts | May differ due to merging | Should match after merge |
| Filter thresholds | Same constants | Same constants |
| Final detections | Reference behavior | Should match |

---

## Architecture Notes

### Why Debug-Only Functions?

The `*_debug_only` wrapper functions are compiled only when the `debug-pipeline` feature is enabled. This:

1. Keeps the public API clean for production use
2. Adds zero overhead when debugging is not needed
3. Makes it easy to find and remove debug code (`grep debug_only`)
4. Ensures debug functions stay in sync with internal implementations

### Adding New Debug Functions

When adding new internal filter functions that should be debuggable:

1. Make the internal function `pub(crate)` in `match_refine.rs`
2. Add a wrapper in `debug_pipeline.rs` with `*_debug_only` suffix
3. Add the re-export in `mod.rs` under `#[cfg(feature = "debug-pipeline")]`

---

## Related Files

### Rust
- `src/license_detection/mod.rs` - Pipeline orchestration
- `src/license_detection/debug_pipeline.rs` - Debug wrapper functions
- `src/license_detection/match_refine.rs` - Filter implementations
- `src/bin/debug_license_detection.rs` - Debug CLI tool

### Python
- `reference/scancode-toolkit/src/licensedcode/index.py` - Main matching
- `reference/scancode-toolkit/src/licensedcode/match.py` - Filtering
- `reference/scancode-playground/debug_license_detection.py` - Debug script
