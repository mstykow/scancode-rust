# PLAN-051: Fix Hispan Reconstruction in combine_matches()

## Status: NOT IMPLEMENTED

## Summary

Rust stores only `hilen` (count of high-value tokens) in `LicenseMatch`, reconstructing hispan positions from `rule_start_token`. If the original hispan wasn't a contiguous range, reconstruction produces incorrect positions after merge.

---

## Problem Statement

**Python**: Stores `hispan` as actual `Span` (set of token positions).

**Rust** (match_refine.rs:119-126):

```rust
let a_hispan: HashSet<usize> = (a.rule_start_token..a.rule_start_token + a.hilen)
    .filter(|&p| a.ispan().contains(&p))
    .collect();
```

This assumes hispan is always a contiguous range starting at `rule_start_token`. If the original match had a non-contiguous hispan (gaps in high-value token positions), the reconstructed positions will be wrong.

---

## Impact

Edge cases where hispan is non-contiguous may have incorrect positions after merge, affecting:
- Match quality scoring
- Coverage calculations
- Detection categorization

---

## Root Cause

During `combine_matches()`, Rust computes the union of hispan positions. But since `hilen` only stores the count, the reconstruction assumes contiguity.

---

## Solution Options

### Option 1: Store hispan positions explicitly

Add field to `LicenseMatch`:

```rust
pub hispan_positions: Option<Vec<usize>>,
```

Update during match creation and merge.

### Option 2: Validate hispan is contiguous before reconstruction

Add assertion or graceful handling for non-contiguous hispans.

### Option 3: Use same reconstruction logic as Python

Verify Python's approach and match it exactly.

---

## Priority: MEDIUM

This is a correctness issue but may only affect edge cases with non-contiguous hispans.

---

## Related Files

- `src/license_detection/models.rs` - LicenseMatch struct
- `src/license_detection/match_refine.rs` - combine_matches()
- `src/license_detection/aho_match.rs` - Match creation
- `src/license_detection/seq_match.rs` - Match creation

---

## Verification

1. Add unit test with non-contiguous hispan
2. Run golden tests
3. Compare hispan values with Python for test cases

---

## Reference

- PLAN-048: P4 - Original finding
- PLAN-014: Related to hispan/ispan position tracking
