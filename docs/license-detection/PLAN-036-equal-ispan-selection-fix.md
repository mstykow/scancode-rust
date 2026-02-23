# PLAN-036: Fix Equal ISpan Match Selection to Use Magnitude

**Status**: Draft  
**Priority**: High  
**Component**: License Detection / Match Refinement  
**Created**: 2026-02-23

---

## Summary

When two matches have equal `ispan` (rule-side token positions) and overlap in `qspan` (query-side token positions), Python ScanCode prefers the match with smaller `qspan.magnitude()` (span extent), while Rust currently prefers the match with larger `matched_length` (position count). These are different metrics that can produce different results for non-contiguous matches.

---

## Problem Description

### The Behavioral Difference

In the `merge_overlapping_matches()` function, when two matches have:

1. Equal `ispan()` (same rule-side positions)
2. Overlapping `qspan()` (query-side positions overlap)

The implementation must decide which match to keep. Python and Rust use different selection criteria:

| Implementation | Selection Criterion | Meaning |
|----------------|---------------------|---------|
| **Python** | `qspan.magnitude()` (smaller is better) | Span extent from first to last position |
| **Rust** | `matched_length` (larger is better) | Count of matched positions |

### Why This Matters

For **contiguous matches** (all positions sequential), both metrics are equivalent:

- `magnitude()` = `len()` = `matched_length`

For **non-contiguous matches** (positions have gaps), they differ:

- `magnitude()` = extent including gaps
- `len()` = count of actual positions only

**Example**:

- Match A has positions [0, 1, 2, 3, 4] (contiguous)
  - `len()` = 5, `magnitude()` = 5
- Match B has positions [0, 50, 100] (sparse)
  - `len()` = 3, `magnitude()` = 101

If both have equal `ispan` and overlap:

- Python would keep Match A (magnitude 5 < 101)
- Rust would keep Match B (matched_length 3 vs 5... wait, that's backwards)

Actually, looking at the current Rust code:

```rust
if current.matched_length >= next.matched_length {
    rule_matches.remove(j);  // keep current
```

Rust keeps the one with **larger** `matched_length`. So in the example above, Rust would keep Match A (5 >= 3).

But the logic is inverted from Python:

- Python: smaller magnitude is better (denser/shorter span)
- Rust: larger matched_length is better (more matched positions)

For contiguous matches, both agree (larger len = larger magnitude).
For non-contiguous matches, they can disagree:

Consider:

- Match A: positions [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] (len=10, magnitude=10)
- Match B: positions [0, 100] (len=2, magnitude=101)

Python: keeps A (magnitude 10 < 101) ✓
Rust: keeps A (matched_length 10 >= 2) ✓
Result: Same!

But consider:

- Match A: positions [0, 1, 2, 3, 4] (len=5, magnitude=5)
- Match B: positions [0, 1, 2, 3, 4, 100, 200] (len=7, magnitude=201)

Python: keeps A (magnitude 5 < 201) ✓
Rust: keeps B (matched_length 7 >= 5) ✗
Result: **Different!**

---

## Current State Analysis

### Rust Implementation

**File**: `src/license_detection/match_refine.rs`  
**Lines**: 225-234

```rust
if current.ispan() == next.ispan() && current.qoverlap(&next) > 0 {
    if current.matched_length >= next.matched_length {
        rule_matches.remove(j);
        continue;
    } else {
        rule_matches.remove(i);
        i = i.saturating_sub(1);
        break;
    }
}
```

**Issues**:

1. Uses `matched_length` instead of `qspan.magnitude()`
2. Comparison direction: `>=` keeps larger, Python uses `<=` to keep smaller magnitude

### Python Reference Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/match.py`  
**Lines**: 946-970

```python
# if we have two equal ispans and some overlap
# keep the shortest/densest match in qspan e.g. the smallest magnitude of the two
if current_match.ispan == next_match.ispan and current_match.overlap(next_match):
    cqmag = current_match.qspan.magnitude()
    nqmag = next_match.qspan.magnitude()
    if cqmag <= nqmag:
        del rule_matches[j]
        continue
    else:
        del rule_matches[i]
        i -= 1
        break
```

### Python's Span.magnitude() Method

**File**: `reference/scancode-toolkit/src/licensedcode/spans.py`  
**Lines**: 262-289

```python
def magnitude(self):
    """
    Return the maximal length represented by this span start and end. The
    magnitude is the same as the length for a contiguous span. It will be
    greater than the length for a span with non-contiguous int items.
    An empty span has a zero magnitude.
    """
    if not self._set:
        return 0
    return self.end - self.start + 1
```

**Key insight**: `magnitude()` is simply `end - start + 1` (extent from first to last position).

---

## Rust Span Analysis

### Current Span Implementation

**File**: `src/license_detection/spans.rs`

Rust has a `Span` struct but it's not directly used by `LicenseMatch` for qspan/ispan. Instead, `LicenseMatch` uses:

- `qspan_positions: Option<Vec<usize>>` for query-side positions
- `ispan_positions: Option<Vec<usize>>` for rule-side positions

### Current LicenseMatch Methods

**File**: `src/license_detection/models.rs`

```rust
// Line 509-515: qspan() returns Vec<usize> of positions
pub fn qspan(&self) -> Vec<usize> {
    if let Some(positions) = &self.qspan_positions {
        positions.clone()
    } else {
        (self.start_token..self.end_token).collect()
    }
}

// Line 541-553: qspan_bounds() returns (min, max+1) - exclusive end
pub fn qspan_bounds(&self) -> (usize, usize) {
    if let Some(positions) = &self.qspan_positions {
        if positions.is_empty() {
            return (0, 0);
        }
        (
            *positions.iter().min().unwrap(),
            *positions.iter().max().unwrap() + 1,
        )
    } else {
        (self.start_token, self.end_token)
    }
}
```

### Computing Magnitude from qspan_bounds()

Since Python's magnitude is `end - start + 1` (inclusive end) and Rust's `qspan_bounds()` returns `(start, end)` with exclusive end, the equivalent is:

```rust
// Python: magnitude = end - start + 1  (inclusive end)
// Rust: qspan_bounds() returns (start, end) with exclusive end
// Rust equivalent: magnitude = end - start
let (qstart, qend) = match.qspan_bounds();
let magnitude = qend.saturating_sub(qstart);
```

**Note**: The Python magnitude is `end - start + 1` because Python's Span uses inclusive end. Rust's `qspan_bounds()` returns exclusive end (Rust convention), so `end - start` gives the same result.

**Verification**:

- Python: Span([4, 8]) has start=4, end=8, magnitude = 8 - 4 + 1 = 5
- Rust: positions [4, 8], qspan_bounds() = (4, 9), magnitude = 9 - 4 = 5 ✓

---

## Proposed Changes

### Change 1: Add qspan_magnitude() Method to LicenseMatch

**File**: `src/license_detection/models.rs`  
**Location**: After `qspan_bounds()` method (around line 553)

```rust
/// Return the magnitude of the qspan (span extent from first to last position).
///
/// This is equivalent to Python's `qspan.magnitude()` - the total extent
/// of the span including gaps, not just the count of positions.
/// For contiguous spans, magnitude equals length.
/// For non-contiguous spans, magnitude > length.
///
/// # Example
/// ```
/// // Positions [4, 8] have magnitude 5 (extent from 4 to 8 inclusive)
/// // but length 2 (only 2 positions)
/// ```
pub fn qspan_magnitude(&self) -> usize {
    let (start, end) = self.qspan_bounds();
    end.saturating_sub(start)
}
```

### Change 2: Update merge_overlapping_matches() to Use Magnitude

**File**: `src/license_detection/match_refine.rs`  
**Lines**: 225-234

**Before**:

```rust
if current.ispan() == next.ispan() && current.qoverlap(&next) > 0 {
    if current.matched_length >= next.matched_length {
        rule_matches.remove(j);
        continue;
    } else {
        rule_matches.remove(i);
        i = i.saturating_sub(1);
        break;
    }
}
```

**After**:

```rust
// if we have two equal ispans and some overlap
// keep the shortest/densest match in qspan e.g. the smallest magnitude of the two
if current.ispan() == next.ispan() && current.qoverlap(&next) > 0 {
    let current_mag = current.qspan_magnitude();
    let next_mag = next.qspan_magnitude();
    if current_mag <= next_mag {
        rule_matches.remove(j);
        continue;
    } else {
        rule_matches.remove(i);
        i = i.saturating_sub(1);
        break;
    }
}
```

---

## Test Requirements

Per `docs/TESTING_STRATEGY.md`, this fix requires:

### 1. Unit Tests (Layer 1)

**File**: `src/license_detection/match_refine.rs` (in the `#[cfg(test)] mod tests` block)

**Test Cases Required**:

#### Test 1: Equal ISpan with Same Magnitude (Contiguous Matches)

```rust
#[test]
fn test_merge_equal_ispan_contiguous_same_magnitude() {
    // Both matches contiguous, same magnitude, equal ispan
    // Should keep either one (deterministic behavior)
    let mut m1 = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
    m1.matched_length = 10;
    m1.start_token = 0;
    m1.end_token = 10;
    
    let mut m2 = create_test_match("#1", 1, 10, 0.85, 85.0, 100);
    m2.matched_length = 10;
    m2.start_token = 0;
    m2.end_token = 10;
    
    let merged = merge_overlapping_matches(&[m1, m2]);
    assert_eq!(merged.len(), 1);  // One match kept
}
```

#### Test 2: Equal ISpan with Different Magnitude (Non-Contiguous)

```rust
#[test]
fn test_merge_equal_ispan_sparse_vs_dense() {
    // Match A: dense (magnitude = length)
    // Match B: sparse (magnitude > length)
    // Should keep the denser match (smaller magnitude)
    
    let mut dense = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
    dense.matched_length = 5;
    dense.start_token = 0;
    dense.end_token = 5;
    // magnitude = 5, length = 5
    
    let mut sparse = create_test_match("#1", 1, 10, 0.85, 85.0, 100);
    sparse.matched_length = 3;
    sparse.qspan_positions = Some(vec![0, 50, 100]);
    // magnitude = 101, length = 3
    
    // Both have same ispan (rule positions 1-10)
    // Dense has smaller magnitude, should be kept
    let merged = merge_overlapping_matches(&[dense.clone(), sparse.clone()]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].qspan_magnitude(), 5);  // Dense match kept
}
```

#### Test 3: Equal ISpan with Reversed Order

```rust
#[test]
fn test_merge_equal_ispan_sparse_vs_dense_reversed() {
    // Same as above but sparse match comes first
    // Should still keep the denser match
    
    let mut dense = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
    dense.matched_length = 5;
    dense.start_token = 0;
    dense.end_token = 5;
    
    let mut sparse = create_test_match("#1", 1, 10, 0.85, 85.0, 100);
    sparse.matched_length = 3;
    sparse.qspan_positions = Some(vec![0, 50, 100]);
    
    // Sparse first, dense second
    let merged = merge_overlapping_matches(&[sparse.clone(), dense.clone()]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].qspan_magnitude(), 5);  // Dense match kept
}
```

#### Test 4: qspan_magnitude() Method

```rust
#[test]
fn test_qspan_magnitude_contiguous() {
    let mut m = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
    m.start_token = 5;
    m.end_token = 15;
    assert_eq!(m.qspan_magnitude(), 10);
}

#[test]
fn test_qspan_magnitude_non_contiguous() {
    let mut m = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
    m.qspan_positions = Some(vec![4, 8]);
    assert_eq!(m.qspan_magnitude(), 5);  // 8 - 4 + 1 = 5
}

#[test]
fn test_qspan_magnitude_empty() {
    let mut m = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
    m.qspan_positions = Some(vec![]);
    assert_eq!(m.qspan_magnitude(), 0);
}
```

### 2. Golden Tests (Layer 2)

Run existing golden tests to verify no regressions:

```bash
cargo test --test license_detection_golden_test
```

If any golden tests fail, analyze whether the difference is:

1. **Expected**: The fix changes behavior to match Python (update golden files)
2. **Unexpected**: A bug in the implementation (fix code)

---

## Risk Assessment

### Low Risk Areas

1. **Contiguous matches**: The new logic produces identical results since `magnitude == matched_length` for contiguous spans.

2. **Most common case**: Most license matches are contiguous, so the majority of scans will see no change.

3. **Backward compatible**: The fix moves Rust behavior toward Python parity, reducing differences.

### Medium Risk Areas

1. **Non-contiguous matches**: Matches with gaps (from merging or partial alignments) may now select differently. This is the intended fix, but could expose edge cases.

2. **Order-dependent tests**: If any tests relied on the old selection behavior, they will fail and need updating.

### Mitigation Strategies

1. **Comprehensive test coverage**: Add specific unit tests for both contiguous and non-contiguous cases before implementing the fix.

2. **Run golden test suite**: Compare before/after results on real-world license detection samples.

3. **Document behavior change**: Update code comments to explain the magnitude-based selection criterion.

---

## Implementation Checklist

- [ ] Add `qspan_magnitude()` method to `LicenseMatch` in `src/license_detection/models.rs`
- [ ] Update comparison in `merge_overlapping_matches()` at `src/license_detection/match_refine.rs:225-234`
- [ ] Add unit tests for `qspan_magnitude()` method
- [ ] Add unit tests for equal ispan selection with various magnitude scenarios
- [ ] Run existing test suite: `cargo test`
- [ ] Run golden tests: `cargo test --test license_detection_golden_test`
- [ ] Update code comments to document the behavior
- [ ] Verify behavior matches Python on sample files

---

## References

- Python implementation: `reference/scancode-toolkit/src/licensedcode/match.py:946-970`
- Python Span.magnitude(): `reference/scancode-toolkit/src/licensedcode/spans.py:262-289`
- Rust implementation: `src/license_detection/match_refine.rs:225-234`
- Rust qspan_bounds(): `src/license_detection/models.rs:541-553`
- Testing strategy: `docs/TESTING_STRATEGY.md`
