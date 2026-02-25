# PLAN-044: filter_contained_matches Parity

## Status: READY FOR IMPLEMENTATION

Root cause identified. Previous regression was due to incomplete implementation combined with incorrect span equality logic.

---

## Executive Summary

Two parity issues exist in `filter_contained_matches()`:

| Issue | Python Behavior | Current Rust Behavior | Impact |
|-------|-----------------|----------------------|--------|
| Expression-based containment | NOT used | Used via `licensing_contains_match()` | Rust discards MORE matches |
| Span equality | Set-based (`qspan == qspan`) | Bounds-based (`start == start && end == end`) | Rust incorrectly identifies non-equal spans as equal |

**Root Cause of Previous Regression**: Removing `licensing_contains_match()` alone is correct, but the previous attempt may have also changed span equality logic incorrectly, OR the tests were passing for wrong reasons.

---

## Detailed Analysis

### Issue 1: Expression-Based Containment

**Python** (`match.py:1157-1176`):
```python
# remove contained matched spans
if current_match.qcontains(next_match):  # Position-based ONLY
    discarded_append(matches_pop(j))
    continue

# remove contained matches the other way  
if next_match.qcontains(current_match):  # Position-based ONLY
    discarded_append(matches_pop(i))
    i -= 1
    break
```

**Rust** (`match_refine.rs:363-371`):
```rust
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    //                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ EXTRA!
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) || licensing_contains_match(&next, &current) {
    //                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ EXTRA!
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

**Key Observation**: Expression-based containment (`licensing_contains`) is ONLY used in `filter_overlapping_matches` in Python, NOT in `filter_contained_matches`.

**Why This Matters**:
- `filter_contained_matches` handles **position containment** (one match's span is inside another's)
- `filter_overlapping_matches` handles **semantic containment** (one license expression contains another)
- These are DIFFERENT concepts and should be kept separate

---

### Issue 2: Span Equality

**Python** (`match.py:1137`):
```python
if current_match.qspan == next_match.qspan:
```

Python's `Span.__eq__` (`spans.py:134-135`):
```python
def __eq__(self, other):
    return isinstance(other, Span) and self._set == other._set
```

This is **set equality** - two spans are equal only if they contain exactly the same positions.

**Rust** (`match_refine.rs:352`):
```rust
if current.qstart() == next.qstart() && current.end_token == next.end_token {
```

This is **bounds equality** - two spans are equal if they have the same start and end bounds.

**Problem with Bounds Equality**:

```
Match A: positions {1, 5, 10}     → start=1, end=10
Match B: positions {1,2,3,4,5,6,7,8,9,10} → start=1, end=10

Python: A.qspan == B.qspan → False (different position sets)
Rust:   start == start && end == end → True (WRONG!)
```

**When This Matters**:
- Non-contiguous matches (sparse `qspan_positions`)
- Created by `merge_overlapping_matches()` when combining overlapping matches
- Common in license detection with gaps

---

## Root Cause of Previous Regression

The previous implementation attempt:
1. ✅ Removed `licensing_contains_match()` (correct)
2. ❌ May have incorrectly implemented `spans_equal()` 
3. ❌ Did not properly handle `qspan_positions` fallback

**Actual Regression Mechanism**:
- Removing `licensing_contains_match()` correctly keeps more matches
- But incorrect span equality caused wrong matches to be identified as duplicates
- Net result: -6 tests (some should pass, some should fail differently)

---

## Implementation Plan

### Step 1: Fix Span Equality Helper

Create a proper `spans_equal()` function that matches Python's set-based equality.

**Key insight**: Python's `Span.__eq__` (spans.py:134-135) compares `_set == _set` (set equality).
Rust's `qspan()` method already handles both cases correctly:
- Returns `positions.clone()` if `qspan_positions.is_some()`
- Returns `(start_token..end_token).collect()` otherwise

Therefore, the correct implementation is simply:

```rust
fn spans_equal(a: &LicenseMatch, b: &LicenseMatch) -> bool {
    a.qspan() == b.qspan()
}
```

This matches Python's `qspan == qspan` exactly because:
- Both return `Vec<usize>` representing the set of token positions
- `Vec` equality checks same length and same elements in same order
- Since both are sorted, this is equivalent to set equality

### Step 2: Remove Expression-Based Containment

Replace lines 363-371:

```rust
// BEFORE:
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) || licensing_contains_match(&next, &current) {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}

// AFTER (matches Python):
if current.qcontains(&next) {
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

### Step 3: Use spans_equal() for Equality Check

Replace line 352:

```rust
// BEFORE:
if current.qstart() == next.qstart() && current.end_token == next.end_token {

// AFTER:
if spans_equal(&current, &next) {
```

---

## Testing Strategy

Following `docs/TESTING_STRATEGY.md`:

### Unit Tests (Layer 1)

Add tests to `src/license_detection/match_refine.rs` in the `tests` module:

#### Test 1: spans_equal with identical contiguous spans
```rust
#[test]
fn test_spans_equal_contiguous_identical() {
    let mut a = create_test_match_with_tokens("#1", 0, 10, 10);
    a.qspan_positions = None;
    let mut b = create_test_match_with_tokens("#2", 0, 10, 10);
    b.qspan_positions = None;
    
    assert!(spans_equal(&a, &b));
}
```

#### Test 2: spans_equal with different contiguous spans
```rust
#[test]
fn test_spans_equal_contiguous_different() {
    let mut a = create_test_match_with_tokens("#1", 0, 10, 10);
    a.qspan_positions = None;
    let mut b = create_test_match_with_tokens("#2", 0, 15, 15);
    b.qspan_positions = None;
    
    assert!(!spans_equal(&a, &b));
}
```

#### Test 3: spans_equal with identical sparse spans
```rust
#[test]
fn test_spans_equal_sparse_identical() {
    let mut a = create_test_match_with_tokens("#1", 0, 10, 10);
    a.qspan_positions = Some(vec![1, 5, 10]);
    let mut b = create_test_match_with_tokens("#2", 0, 10, 10);
    b.qspan_positions = Some(vec![1, 5, 10]);
    
    assert!(spans_equal(&a, &b));
}
```

#### Test 4: spans_equal with different sparse spans (same bounds!)
```rust
#[test]
fn test_spans_equal_sparse_different_same_bounds() {
    // CRITICAL TEST: Same bounds, different positions
    let mut a = create_test_match_with_tokens("#1", 1, 10, 10);
    a.qspan_positions = Some(vec![1, 5, 10]);  // sparse: 3 positions
    
    let mut b = create_test_match_with_tokens("#2", 1, 10, 10);
    b.qspan_positions = Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);  // dense: 10 positions
    
    // Python: qspan == qspan → False (different sets)
    // This MUST be False in Rust too
    assert!(!spans_equal(&a, &b));
}
```

#### Test 5: spans_equal mixed contiguous/sparse with same positions
```rust
#[test]
fn test_spans_equal_mixed_contiguous_sparse_same_positions() {
    let mut a = create_test_match_with_tokens("#1", 0, 5, 5);
    a.qspan_positions = None;  // contiguous [0,1,2,3,4]
    
    let mut b = create_test_match_with_tokens("#2", 0, 5, 5);
    b.qspan_positions = Some(vec![0, 1, 2, 3, 4]);  // same as contiguous
    
    // qspan() for a: [0,1,2,3,4] (from range)
    // qspan() for b: [0,1,2,3,4] (explicit)
    // Should be equal - same actual positions
    assert!(spans_equal(&a, &b));
}

#### Test 6: spans_equal mixed contiguous/sparse with different positions
```rust
#[test]
fn test_spans_equal_mixed_contiguous_sparse_different_positions() {
    let mut a = create_test_match_with_tokens("#1", 0, 5, 5);
    a.qspan_positions = None;  // contiguous [0,1,2,3,4]
    
    let mut b = create_test_match_with_tokens("#2", 0, 5, 5);
    b.qspan_positions = Some(vec![0, 1, 4]);  // sparse subset
    
    // qspan() for a: [0,1,2,3,4]
    // qspan() for b: [0,1,4]
    // NOT equal - different position sets
    assert!(!spans_equal(&a, &b));
}
```

#### Test 7: filter_contained_matches WITHOUT licensing_contains_match
```rust
#[test]
fn test_filter_contained_matches_no_expression_containment() {
    // Two matches with DIFFERENT license expressions but same position
    // One is NOT contained in the other positionally
    // Neither should be discarded for expression reasons in filter_contained_matches
    
    let mut gpl = create_test_match_with_tokens("gpl-2.0", 0, 10, 10);
    gpl.license_expression = "gpl-2.0".to_string();
    
    let mut mit = create_test_match_with_tokens("mit", 15, 25, 10);
    mit.license_expression = "mit".to_string();
    
    let (filtered, discarded) = filter_contained_matches(&[gpl.clone(), mit.clone()]);
    
    // Neither positionally contains the other, both should be kept
    assert_eq!(filtered.len(), 2);
    assert_eq!(discarded.len(), 0);
}
```

#### Test 8: Expression containment is handled in filter_overlapping_matches
```rust
#[test]
fn test_expression_containment_in_overlapping_not_contained() {
    // This test verifies that expression-based containment is correctly
    // handled in filter_overlapping_matches, not filter_contained_matches
    
    // "gpl-2.0 WITH exception" contains "gpl-2.0" expression-wise
    // But if they don't overlap positionally, filter_contained_matches should NOT discard
    
    let mut gpl_with_exception = create_test_match_with_tokens("gpl-2.0-with-exception", 0, 20, 20);
    gpl_with_exception.license_expression = "gpl-2.0 WITH autoconf-exception-3.0".to_string();
    
    let mut gpl = create_test_match_with_tokens("gpl-2.0", 50, 60, 10);  // No positional overlap!
    gpl.license_expression = "gpl-2.0".to_string();
    
    let (filtered, _) = filter_contained_matches(&[gpl_with_exception.clone(), gpl.clone()]);
    
    // No positional containment, both should be kept
    assert_eq!(filtered.len(), 2);
}
```

### Golden Tests (Layer 2)

Run existing golden tests after implementation:
```bash
cargo test --test license_detection_golden_test
```

**Expected Results**:
- Baseline: 3780 passed, 583 failed
- After fix: Should match or improve (parity is the goal)
- Any regression indicates incomplete fix or other issues

### Integration Tests (Layer 3)

Run full test suite:
```bash
cargo test --all
```

---

## Files to Modify

| File | Lines | Change |
|------|-------|--------|
| `src/license_detection/match_refine.rs` | 352-371 | Remove `licensing_contains_match`, add `spans_equal()` |

---

## Verification Checklist

Before marking complete:

- [ ] `spans_equal()` function implemented with all edge cases
- [ ] Expression-based containment removed from `filter_contained_matches`
- [ ] All new unit tests pass
- [ ] Existing `test_filter_contained_*` tests still pass
- [ ] Golden test count matches or exceeds baseline
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)

---

## Reference

- **Python**: `reference/scancode-toolkit/src/licensedcode/match.py:1075-1184`
- **Python Span**: `reference/scancode-toolkit/src/licensedcode/spans.py:134-135` (set equality)
- **Rust**: `src/license_detection/match_refine.rs:326-380`
- **Related**: `filter_overlapping_matches` correctly uses `licensing_contains_match` (keep this)
