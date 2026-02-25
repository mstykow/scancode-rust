# PLAN-051: Fix Hispan Reconstruction in combine_matches()

## Status: IMPLEMENTED

## Summary

Rust stores only `hilen` (count of high-value tokens) in `LicenseMatch`, reconstructing hispan positions from `rule_start_token`. If the original hispan wasn't a contiguous range, reconstruction produces incorrect positions after merge.

---

## Problem Statement

**Python** (match.py:680): Stores `hispan` as actual `Span` (set of token positions):
```python
combined = LicenseMatch(
    ...
    hispan=Span(self.hispan | other.hispan),  # Union of position sets
    ...
)
```

**Rust** (match_refine.rs:125-132):
```rust
let a_hispan: HashSet<usize> = (a.rule_start_token..a.rule_start_token + a.hilen)
    .filter(|&p| a.ispan().contains(&p))
    .collect();
let b_hispan: HashSet<usize> = (b.rule_start_token..b.rule_start_token + b.hilen)
    .filter(|&p| b.ispan().contains(&p))
    .collect();
let combined_hispan: HashSet<usize> = a_hispan.union(&b_hispan).copied().collect();
let hilen = combined_hispan.len();
```

This reconstruction assumes hispan is always a contiguous range starting at `rule_start_token`. If the original match had a non-contiguous hispan (gaps in high-value token positions), the reconstructed positions will be wrong.

### Example of the Bug

Consider a rule with tokens: `[HIGH, LOW, HIGH, LOW, HIGH]` (where HIGH = legalese token):
- Correct hispan: `{0, 2, 4}` (3 positions, non-contiguous)
- Rust stores: `hilen=3, rule_start_token=0`
- Rust reconstruction: `range(0, 3) = {0, 1, 2}` filtered by ispan → **WRONG**

The bug manifests when:
1. A rule has non-legalese tokens interspersed with legalese tokens
2. Two matches to the same rule are merged via `combine_matches()`
3. The combined hispan has incorrect positions, affecting `hilen()` comparisons in filtering

---

## Root Cause Analysis

### How hispan is Created

**hash_match.rs:85:**
```rust
let hispan = (0..rule_length).filter(|&p| itokens[p] < index.len_legalese as u16);
// Note: Only hilen (count) is stored, not the actual positions:
// hilen: hispan.count()
```

**aho_match.rs:133:**
```rust
let hispan_count = (0..matched_length)
    .filter(|&p| rule_tids.get(p).is_some_and(|tid| *tid < index.len_legalese as u16))
    .count();
```

**seq_match.rs:714:**
```rust
let hispan_count = (ipos..ipos + mlen)
    .filter(|&p| rule_tokens.get(p).is_some_and(|t| *t < len_legalese as u16))
    .count();
```

All matchers compute hispan correctly at creation time by checking each position's token ID against `len_legalese`. The problem is only the count (`hilen`) is stored, not the actual positions.

### Why the Reconstruction Formula is Wrong

```rust
(a.rule_start_token..a.rule_start_token + a.hilen).filter(|&p| a.ispan().contains(&p))
```

This incorrectly assumes:
1. The range `rule_start_token..rule_start_token+hilen` covers all hispan positions
2. Filtering by `ispan()` membership correctly identifies hispan positions

Both assumptions fail when hispan has gaps. The correct filter should check if the token at position `p` is a legalese token (`tid < len_legalese`), not just if `p` is in `ispan`.

---

## Solution: Store hispan_positions Explicitly

Add `hispan_positions: Option<Vec<usize>>` to `LicenseMatch`, mirroring the existing `qspan_positions` and `ispan_positions` fields. This matches Python's approach of storing the actual Span.

### Why This Approach

1. **Matches Python semantics**: Python stores `hispan` as a `Span` (set of positions)
2. **Consistent with existing patterns**: Already using `qspan_positions` and `ispan_positions`
3. **Simple and correct**: No complex reconstruction logic needed
4. **Memory efficient**: Only populated after merge operations, like other position vectors

### Alternative Considered: Reconstruct from Token IDs

Could reconstruct hispan by checking token IDs at each ispan position:
```rust
let hispan: Vec<usize> = ispan.iter()
    .filter(|&p| rule_tokens[*p] < len_legalese)
    .collect();
```

**Rejected because:**
- Requires access to `rule_tokens` (index data) in `combine_matches()`
- Adds complexity and coupling
- Runtime cost for every merge operation

---

## Implementation Steps

### Step 1: Add Field to LicenseMatch

**File:** `src/license_detection/models.rs`

Add new field after `ispan_positions`:
```rust
/// Token positions in the rule that are high-value legalese tokens.
/// None means hispan can be computed from rule_start_token (contiguous case).
/// Some(positions) contains exact positions for non-contiguous hispans (after merge).
#[serde(skip)]
pub hispan_positions: Option<Vec<usize>>,
```

Update `Default` implementation to include `hispan_positions: None`.

### Step 2: Add hispan() Method

**File:** `src/license_detection/models.rs`

Add method to retrieve hispan positions:
```rust
pub fn hispan(&self) -> Vec<usize> {
    if let Some(positions) = &self.hispan_positions {
        positions.clone()
    } else {
        // Contiguous case: compute from token IDs in matched range
        // Note: This is a simplification; callers should populate hispan_positions
        // when the exact positions matter (e.g., after merge)
        (self.rule_start_token..self.rule_start_token + self.hilen).collect()
    }
}
```

### Step 3: Update combine_matches()

**File:** `src/license_detection/match_refine.rs`

Replace lines 125-143 with:
```rust
let a_hispan: HashSet<usize> = a.hispan().into_iter().collect();
let b_hispan: HashSet<usize> = b.hispan().into_iter().collect();
let combined_hispan: HashSet<usize> = a_hispan.union(&b_hispan).copied().collect();
let mut hispan_vec: Vec<usize> = combined_hispan.into_iter().collect();
hispan_vec.sort();
let hilen = hispan_vec.len();

merged.hilen = hilen;
merged.hispan_positions = if hispan_vec.is_empty() {
    None
} else {
    Some(hispan_vec)
};
```

### Step 4: Update Match Creators (Optional Optimization)

**Files:** `src/license_detection/aho_match.rs`, `seq_match.rs`, `hash_match.rs`

When creating matches, populate `hispan_positions` if hispan is non-contiguous:

```rust
// After computing hispan_count, check if positions should be stored
let hispan_positions: Vec<usize> = (0..matched_length)
    .filter(|&p| rule_tids[p] < index.len_legalese as u16)
    .collect();

let hispan_count = hispan_positions.len();
// Check if contiguous (can skip storing positions)
let is_contiguous = hispan_positions.windows(2).all(|w| w[1] == w[0] + 1);

LicenseMatch {
    ...
    hilen: hispan_count,
    hispan_positions: if is_contiguous || hispan_count == 0 { None } else { Some(hispan_positions) },
    ...
}
```

This is an optimization; the core fix works without it because `hispan()` falls back to the computed range.

### Step 5: Update Clone/Sync for Merge Operations

In `merge_overlapping_matches()` and other places where matches are cloned and modified, ensure `hispan_positions` is preserved or recomputed correctly.

---

## Testing Strategy

Following `docs/TESTING_STRATEGY.md` multi-layered approach:

### Unit Tests

**File:** `src/license_detection/match_refine.rs` (add to `#[cfg(test)] mod tests`)

```rust
#[test]
fn test_combine_matches_preserves_non_contiguous_hispan() {
    // Create match A with non-contiguous hispan: {0, 2, 4}
    let mut a = create_test_match_with_tokens("#1", 0, 10, 10);
    a.hilen = 3;
    a.rule_start_token = 0;
    a.hispan_positions = Some(vec![0, 2, 4]);
    a.ispan_positions = Some(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    // Create match B with non-contiguous hispan: {1, 3}
    let mut b = create_test_match_with_tokens("#1", 10, 20, 10);
    b.hilen = 2;
    b.rule_start_token = 0;
    b.hispan_positions = Some(vec![1, 3]);
    b.ispan_positions = Some(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let combined = combine_matches(&a, &b);

    // Combined hispan should be {0, 1, 2, 3, 4}
    assert_eq!(combined.hilen, 5);
    let hispan = combined.hispan_positions.unwrap();
    assert_eq!(hispan, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_combine_matches_hispan_with_none_positions() {
    // Match with no hispan_positions (contiguous case)
    let mut a = create_test_match_with_tokens("#1", 0, 10, 10);
    a.hilen = 5;
    a.rule_start_token = 0;
    a.hispan_positions = None;  // Contiguous hispan at positions 0-4
    a.ispan_positions = None;
    a.matched_length = 10;

    let mut b = create_test_match_with_tokens("#1", 10, 20, 10);
    b.hilen = 5;
    b.rule_start_token = 10;
    b.hispan_positions = None;  // Contiguous hispan at positions 10-14
    b.ispan_positions = None;
    b.matched_length = 10;

    let combined = combine_matches(&a, &b);

    // With both having None positions, should merge the reconstructed ranges
    assert_eq!(combined.hilen, 10);
}
```

### Integration Test

Create a test file that exercises the full license detection pipeline with a document that triggers non-contiguous hispan merging:

**File:** `testdata/license-detection/hispan-test/`

Create test input where a rule with interspersed legalese/non-legalese tokens is matched and merged.

### Golden Test Comparison

Run Python and Rust on the same input and compare `hilen` values in output. Any discrepancy indicates a bug in hispan handling.

---

## Verification Checklist

- [ ] `hispan_positions` field added to `LicenseMatch`
- [ ] `hispan()` method returns correct positions
- [ ] `combine_matches()` correctly unions hispan positions
- [ ] Unit tests pass for non-contiguous hispan cases
- [ ] No regression in existing tests
- [ ] Golden tests show no unexpected `hilen` differences vs Python

---

## Related Files

| File | Change Type |
|------|-------------|
| `src/license_detection/models.rs` | Add `hispan_positions` field, add `hispan()` method |
| `src/license_detection/match_refine.rs` | Update `combine_matches()` to use `hispan()` |
| `src/license_detection/aho_match.rs` | (Optional) Populate `hispan_positions` on creation |
| `src/license_detection/seq_match.rs` | (Optional) Populate `hispan_positions` on creation |
| `src/license_detection/hash_match.rs` | (Optional) Populate `hispan_positions` on creation |

---

## Priority: HIGH

This is a correctness issue that can cause:
- Incorrect `hilen()` values after merge operations
- Wrong filtering decisions in `filter_overlapping_matches()` (uses `hilen` comparisons)
- Mismatches with Python reference output

---

## Related

- PLAN-048: P4 - Original finding
- PLAN-014: Related to hispan/ispan position tracking
- `docs/TESTING_STRATEGY.md` - Testing approach guidelines
