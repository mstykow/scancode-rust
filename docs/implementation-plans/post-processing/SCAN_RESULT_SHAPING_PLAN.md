# Scan Result Shaping Implementation Plan

> **Status**: 🟡 Active — the core shaping pipeline is implemented in `src/scan_result_shaping.rs`, and this document records the remaining shaping-specific compatibility and scaling work
> **Priority**: P2 - Medium (important user-facing output semantics, but downstream of core scan correctness)
> **Dependencies**: [CLI_PLAN.md](../infrastructure/CLI_PLAN.md), [ASSEMBLY_PLAN.md](../package-detection/ASSEMBLY_PLAN.md), [SUMMARIZATION_PLAN.md](SUMMARIZATION_PLAN.md)

## Overview

This plan covers the **output-shaping** steps that happen after scanning and before final output serialization, but are not part of summary/tally analysis:

- `--filter-clues`
- `--include`
- `--only-findings`
- `--strip-root`
- `--full-root`
- `--mark-source`

On the current branch these behaviors live in [`src/scan_result_shaping.rs`](../../../src/scan_result_shaping.rs) and are orchestrated from [`src/main.rs`](../../../src/main.rs) before assembly and summary/tally output construction.

## Why This Needs Its Own Plan

These steps are not summary generation.

- They reshape the scanned file tree or mutate per-resource presentation fields.
- They run before `create_output(...)` in `src/post_processing/mod.rs`.
- They affect what downstream assembly/summary logic sees, but they are not part of the summarycode-style aggregation layer.

Keeping them in a separate plan prevents `SUMMARIZATION_PLAN.md` from becoming a catch-all post-scan bucket and makes the performance tradeoffs easier to reason about.

## Scope

### What This Covers

- Include-filtering the file tree while preserving required parent directories
- Filtering to only files with findings while preserving required parent directories
- Deduplicating redundant clue entries on individual files
- Path normalization for relative (`--strip-root`) and absolute (`--full-root`) output paths
- Source-tree marking (`is_source`, `source_count`) for files and directories

### What This Doesn't Cover

- Summary, tallies, score, classify, facets, generated detection (covered by [`SUMMARIZATION_PLAN.md`](SUMMARIZATION_PLAN.md))
- Package assembly (covered by [`ASSEMBLY_PLAN.md`](../package-detection/ASSEMBLY_PLAN.md))
- Output format rendering (covered by the output plans)

## Current Rust Design

**Location**: [`src/scan_result_shaping.rs`](../../../src/scan_result_shaping.rs)

Current shaping steps:

- `apply_include_filter()` — keeps matching files and the directory chain needed to retain a valid tree
- `apply_only_findings_filter()` — drops files without findings while retaining necessary ancestor directories
- `filter_redundant_clues()` — deduplicates identical copyrights/holders/authors/emails/URLs by value and line span
- `normalize_paths()` — applies `--strip-root` / `--full-root`
- `apply_mark_source()` — marks source-heavy files/directories and computes `source_count`

These functions operate directly on the in-memory `Vec<FileInfo>` produced by the scanner.

## Performance Principles

This layer should stay cheap and predictable.

- Prefer in-place filtering/mutation of the existing `Vec<FileInfo>`.
- Avoid repeated full-tree cloning or extra serialization steps.
- Preserve directory ancestry with one pass over kept file paths instead of rebuilding the tree repeatedly.
- Keep `mark_source` bottom-up and count-based rather than rescanning file contents.

This already differs favorably from Python's plugin-oriented summarycode approach because there is no per-resource persistence round-trip here.

## Current State in Rust

### Implemented

- ✅ Include filtering with ancestor-directory preservation
- ✅ Only-findings filtering with ancestor-directory preservation
- ✅ Redundant-clue deduplication by exact value/line-span identity
- ✅ Relative and absolute path normalization
- ✅ Explicit rejection of ambiguous `--from-json` + `--strip-root` / `--full-root` combinations
- ✅ Source-heavy directory marking using descendant source ratios
- ✅ Expanded invariant coverage in `src/scan_result_shaping_test.rs` for `full_root`, broader findings retention, clue dedupe, nested `mark_source`, and normalized package file-reference paths

### Remaining / Watch Items

- ❌ Full ScanCode compatibility review for path-pattern edge cases and root-handling nuances
- ❌ Clear documentation of the ordering contract relative to assembly and summary generation
- ❌ Performance review for large include-pattern sets if real-world profiling shows pressure

## Architectural Boundary

This layer should remain a **presentation shaper**, not a data enricher.

- It may remove or relabel existing output-facing fields.
- It should not invent new summary/package conclusions.
- It should not become the place where package ownership or declared-license provenance is inferred.

## Success Criteria

- [ ] Output-shaping flags remain clearly separated from summary/tally computation
- [ ] Filtering preserves required ancestor directories deterministically
- [ ] Path normalization behavior is explicit and tested
- [ ] `mark_source` remains count-based and does not introduce rescans
- [ ] Large scans do not incur Python-style repeated persistence/copy overhead in this layer

## Related Documents

- **Sibling**: [`SUMMARIZATION_PLAN.md`](SUMMARIZATION_PLAN.md)
- **Sibling**: [`CLI_PLAN.md`](../infrastructure/CLI_PLAN.md)
- **Prerequisite**: [`ASSEMBLY_PLAN.md`](../package-detection/ASSEMBLY_PLAN.md)

## Notes

- The current branch already implements the core shaping pipeline; this document exists to make that scope explicit and keep it from being conflated with summarycode parity work.
- If future ScanCode-compatible shaping flags land, add them here unless they directly feed summary/tally semantics.
