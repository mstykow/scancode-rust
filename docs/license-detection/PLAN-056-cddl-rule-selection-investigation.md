# PLAN-056: CDDL Rule Selection Investigation

## Status: FIXED

## Problem Statement

The CDDL 1.0 and CDDL 1.1 rule selection is incorrect. When scanning files that should match CDDL 1.0 rules, the Rust implementation incorrectly matches CDDL 1.1 rules instead.

### Manifestation in Golden Tests

| Test File | Expected Expression | Actual Expression |
|-----------|---------------------|-------------------|
| `cddl-1.0_or_gpl-2.0-glassfish.txt` | `cddl-1.0 OR gpl-2.0` | `cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0` |
| `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_2.txt` | `(cddl-1.0 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0` | `(cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0` |

---

## Root Cause Analysis

### The Problem

The issue is in the handling of `qspan_positions` when comparing matches with different position representation modes:

1. **CDDL 1.0 matches**: Have `qspan_positions: None` (contiguous range 18-270, 252 positions)
2. **CDDL 1.1 matches**: Have `qspan_positions: Some([...])` (174 scattered positions in range 0-270)

When comparing these matches in `filter_overlapping_matches`, the overlap ratio calculation was incorrect because:

1. **`qoverlap()`** was computing range overlap instead of actual position overlap
2. **`qcontains()`** was using range containment instead of set containment

### Key Metrics from Investigation

**CDDL 1.0 (correct match)**:
- Coverage: 96.2%
- matched_length: 252 tokens
- hilen: 51
- start_token: 18, end_token: 270
- qspan_positions: None (contiguous)

**CDDL 1.1 (incorrect match)**:
- Coverage: 59.0%
- matched_length: 174 tokens
- hilen: 35
- start_token: 0, end_token: 270
- qspan_positions: Some(174 positions)

### Bug in `qoverlap()` 

**Before fix**: Computed range overlap (252 tokens), treating all positions in [start, end) as matching.

**After fix**: Computes actual position overlap (164 tokens) by:
- Using set intersection when both have `qspan_positions`
- Checking each position against range when one has `qspan_positions` and the other doesn't

### Bug in `qcontains()`

**Before fix**: Used simple range containment (`start <= other.start && end >= other.end`), which incorrectly said CDDL 1.1 "contains" CDDL 1.0 because 0 <= 18 and 270 >= 270.

**After fix**: Uses set containment semantics matching Python's `Span.__contains__`:
- When one has positions and other has range, checks all positions against the range
- When both have positions, uses set intersection
- When both have ranges, uses range containment

---

## Final Fix

### Root Cause #2: Incorrect Merge in `surround()` Check

After fixing `qcontains()` and `qoverlap()`, CDDL 1.0 was still being lost. The final bug was in `merge_overlapping_matches()` at lines 251-257:

**Problem**: The `surround()` check only verified that bounds surrounded, not that positions actually overlapped. CDDL 1.1 had two matches:
- m1: start=0, end=270, 174 scattered positions
- m2: start=18, end=143, 81 scattered positions

When `m1.surround(m2)` returned true (0 <= 18 && 270 >= 143), the code combined them, creating a false inflated match with 255 positions and 86.4% coverage, which then beat CDDL 1.0 in `filter_overlapping_matches()`.

**Fix**: Added `qoverlap > 0` check before merging surrounded matches:

```rust
if current.surround(&next) {
    let qoverlap = current.qoverlap(&next);
    if qoverlap > 0 {  // NEW: Only merge if positions actually overlap
        let combined = combine_matches(&current, &next);
        if combined.qspan().len() == combined.ispan().len() {
            rule_matches[i] = combined;
            rule_matches.remove(j);
            continue;
        }
    }
}
```

This matches the Python FIXME comment at match.py:996: "qsurround is too weak. We want to check also isurround". The overlap check ensures we only merge when the matches actually share token positions.

---

## Files Modified

1. `src/license_detection/models.rs`:
   - Fixed `qcontains()` to handle mixed `qspan_positions` cases
   - Fixed `qoverlap()` to compute actual position overlap instead of range overlap
   - Fixed `qdistance_to()` to use saturating_sub for safety

2. `src/license_detection/match_refine.rs`:
   - Made `filter_contained_matches` pub(crate) for testing
   - **Added `qoverlap > 0` check in `surround()` merge condition**

3. `src/license_detection/cddl_investigation_test.rs`:
   - Created comprehensive investigation tests

---

## Resolution

All CDDL tests now pass:
- `test_cddl_10_detection_basic` - CDDL 1.0 correctly detected
- `test_cddl_11_detection_basic` - CDDL 1.1 correctly detected
- `test_detect_full_pipeline_matches` - Full pipeline validates CDDL 1.0 selection

---

## Success Criteria

1. CDDL 1.0 test file produces `cddl-1.0 OR gpl-2.0` expression
2. CDDL 1.1 test file continues to produce correct CDDL 1.1 expression
3. All existing tests pass
4. Golden tests for CDDL pass
