# PLAN-0015: filter_dupes Regression Analysis

## Status: Investigation Complete

## Executive Summary

The `filter_dupes()` fix is **correct** - it aligns with Python's implementation. The regressions it introduces reveal **pre-existing issues** in other parts of the pipeline that were previously masked.

## Background

### The Fix
Added `filter_dupes()` to `src/license_detection/seq_match.rs` to deduplicate candidates by grouping them and keeping only the best from each group. This matches Python's behavior.

### Impact
- **Fixed**: 14 tests (including npruntime.h)
- **New failures**: 15 tests
- **Net**: -1 regression

## Root Cause Analysis

### Primary Issue: matched_length Precision Loss

**Location**: `src/license_detection/seq_match.rs:254`

**Problem**: The `DupeGroupKey.matched_length` uses integer rounding, losing precision compared to Python's 1-decimal-place rounding.

| License | matched_length | Python rounded | Rust rounded | Same Group? |
|---------|---------------|----------------|--------------|-------------|
| x11-dec1 | 138 | 6.9 | 7 | - |
| cmu-uc | 133 | 6.7 | 7 | **YES (wrong)** |

In Python, these are DIFFERENT groups (6.9 ≠ 6.7), so both candidates survive.
In Rust, they're the SAME group (7 = 7), so only one survives.

**Affected tests**:
- `MIT-CMU-style.txt` - Expected: x11-dec1, Actual: cmu-uc

**Fix**: Store the 1-decimal-place rounded value:
```rust
// Current (wrong):
matched_length: (candidate.score_vec_rounded.matched_length * 20.0).round() as i32,

// Should be:
matched_length: (candidate.score_vec_rounded.matched_length * 10.0).round() as i32,  // 69, 67
```

### Secondary Issues (Uncovered, Not Caused by filter_dupes)

#### 1. Overlapping Match Filtering Issue

**Location**: `src/license_detection/match_refine.rs`

**Problem**: `ar-ER.js.map` has two MIT matches at identical line boundaries (1-1), but both survive filtering. This should be caught by `filter_contained_matches` or `filter_overlapping_matches`.

**Affected tests**:
- `ar-ER.js.map` - Expected 1 "mit", Actual 2

#### 2. Missing License Reference Detection

**Location**: `src/license_detection/seq_match.rs` or `aho_match.rs`

**Problem**: Text like `"Re-licensed mDNSResponder daemon source code under Apache License, Version 2.0"` (changelog entries) isn't being detected.

**Affected tests**:
- `DNSDigest.c` - Expected 3 apache-2.0, Actual 2

#### 3. Dual-License Header Detection Issue

**Location**: `src/license_detection/seq_match.rs` or detection pipeline

**Problem**: Dual-license headers like MPL/GPL aren't being fully matched. Only short tags like `MODULE_LICENSE("Dual MPL/GPL")` are detected.

**Affected tests**:
- `sa11xx_base.c` - Expected 2 "mpl-1.1 OR gpl-2.0", Actual 1

#### 4. License Expression Combination Issue

**Location**: `src/license_detection/detection.rs`

**Problem**: When multiple overlapping matches are detected, the expression combination logic creates incorrect expressions like `lgpl-2.0-plus WITH wxwindows-exception-3.1 AND wxwindows-exception-3.1`.

**Affected tests**:
- `lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt` - Expected 1 expression, Actual 5

### Non-Issues (Already Working)

#### git.mk
- Expected: fsfap-no-warranty-disclaimer
- Status: **PASSING** - correctly detected after filter_dupes

#### lgpl-2.1_14.txt
- Expected: lgpl-2.1
- Status: **PASSING** - correctly detected after filter_dupes

## Test Case Analysis

| Test | Expected | Actual | Root Cause | Priority |
|------|----------|--------|------------|----------|
| MIT-CMU-style.txt | x11-dec1 | cmu-uc | matched_length precision loss | High |
| ar-ER.js.map | 1 mit | 2 mit | Overlapping match filtering | Medium |
| DNSDigest.c | 3 apache-2.0 | 2 apache-2.0 | License reference detection | Medium |
| sa11xx_base.c | 2 mpl/gpl | 1 mpl/gpl | Dual-license detection | Medium |
| lgpl-2.0-plus_wxwindows | 1 expr | 5 exprs | Expression combination | Medium |
| MIT.t21 | proprietary | mit | Needs investigation | Low |
| bsd.f | bsd-simplified | bsd-new | Needs investigation | Low |

## Recommended Fix Order

### Phase 1: Fix the precision issue (High Priority)

**File**: `src/license_detection/seq_match.rs:254`

Change the `DupeGroupKey.matched_length` calculation to use 1-decimal precision:
```rust
matched_length: (candidate.score_vec_rounded.matched_length * 10.0).round() as i32,
```

This should fix `MIT-CMU-style.txt` and may fix other cases.

### Phase 2: Fix overlapping match filtering (Medium Priority)

**File**: `src/license_detection/match_refine.rs`

Investigate why matches at identical line boundaries both survive filtering.

### Phase 3: Investigate remaining issues (Lower Priority)

- License reference detection for changelog entries
- Dual-license header detection
- Expression combination for WITH expressions

## Code Locations

| Component | File | Lines |
|-----------|------|-------|
| filter_dupes | `src/license_detection/seq_match.rs` | 130-180 |
| DupeGroupKey | `src/license_detection/seq_match.rs` | 27-35 |
| matched_length calculation | `src/license_detection/seq_match.rs` | 254 |
| Overlapping match filter | `src/license_detection/match_refine.rs` | filter_contained_matches, filter_overlapping_matches |
| Expression combination | `src/license_detection/detection.rs` | determine_license_expression |

## References

- Python filter_dupes: `reference/scancode-toolkit/src/licensedcode/match_set.py:467-485`
- Python ScoresVector: `reference/scancode-toolkit/src/licensedcode/match_set.py:440`
