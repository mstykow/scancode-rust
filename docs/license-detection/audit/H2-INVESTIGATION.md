# Hypothesis H2: Multi-Occurrence Deduplication Investigation

## Status: Under Investigation

## Problem Statement
~25 golden test cases have fewer matches than expected. The `flex-readme.txt` test is representative:
- **Expected**: 3 separate `flex-2.5` detections
- **Actual**: 1 detection covering all license text

## Investigation Summary

### Test File Analysis
File: `testdata/license-golden/datadriven/lic1/flex-readme.txt`

License text locations:
- Lines 27-28: "Note that flex is distributed under a copyright..."
- Line 30: "This file is part of flex."
- Lines 35-56: Full BSD-style license text

### Rust Behavior

**Phase 1 - Aho Matching**: Finds 3 exact matches with 100% coverage
- `flex-2.5_not_gpl.RULE` (lines 27-28, 100% coverage)
- `flex-2.5_10.RULE` (line 30, 100% coverage)
- `flex-2.5_4.RULE` (lines 35-56, 100% coverage)

**Issue**: There are 17 uncovered high-value positions on lines 6, 18-19, 24, 32, 58-79 (outside license text, in README content like "flex", "http", "lists")

**Phase 2 - Sequence Matching**: Because `is_matchable()` returns `true` (uncovered positions exist), sequence matching runs and finds:
- `flex-2.5_5.RULE` (lines 27-56, 100% coverage, seq matcher)

**Containment Filtering**: Removes Aho matches because they're contained in the larger seq match

**Final Result**: 1 detection with `flex-2.5_5.RULE`

### Python Behavior (Expected)
Expected: 3 separate detections of `flex-2.5`

## Root Cause Analysis

The issue appears to be in how Python groups matches into separate detections vs. Rust.

**Key Question**: Does Python:
1. Not produce the `flex-2.5_5.RULE` match at all?
2. Produce it but group it differently into 3 detections?
3. Have different containment filtering logic?

## Attempted Fixes

### Fix 1: Preserve Aho matches over seq matches when both have 100% coverage
**Result**: Caused 90 regressions (worse than 25 baseline)
**Reason**: This breaks legitimate deduplication in other test cases

## Next Steps

1. Run Python on `flex-readme.txt` with tracing to see all matches produced
2. Compare Python's detection grouping logic with Rust's
3. Check if Python's `is_matchable()` or `matched_qspans` computation differs
4. Investigate detection grouping thresholds (LINES_THRESHOLD, etc.)

## Relevant Files

- `src/license_detection/mod.rs` - Main detection engine
- `src/license_detection/match_refine/handle_overlaps.rs` - Containment filtering
- `src/license_detection/detection/grouping.rs` - Detection grouping
- `src/license_detection/query/mod.rs` - `is_matchable()` implementation
- `reference/scancode-toolkit/src/licensedcode/index.py` - Python reference
- `reference/scancode-toolkit/src/licensedcode/match.py` - Python match refinement
- `reference/scancode-toolkit/src/licensedcode/detection.py` - Python detection grouping

## Related Issues

- DIFFERENCES.md #1: QueryRun Splitting (FIXED)
- DIFFERENCES.md #6: MAX_DIST threshold (50 in Python vs 100 in Rust)
