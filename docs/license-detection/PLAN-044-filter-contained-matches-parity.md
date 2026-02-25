# PLAN-044: filter_contained_matches Parity

## Status: NEEDS INVESTIGATION

Previous implementation attempts caused -6 regression. Need to investigate root cause before re-attempting.

---

## Summary

Two parity issues in `filter_contained_matches()`:

1. **licensing_contains_match extension**: Rust adds expression-based containment that Python does NOT have
2. **spans_equal check**: Rust uses bounds comparison instead of set equality for non-contiguous spans

---

## Previous Implementation Results

| Change | Result | Impact |
|--------|--------|--------|
| Remove `licensing_contains_match()` | -6 regression | Matches that should be deduplicated were kept |
| Add `spans_equal()` for non-contiguous | Compounded regression | More matches kept when they should be deduplicated |

**Baseline**: 3780 passed, 583 failed
**After changes**: 3774 passed, 589 failed

---

## Issue 1: Expression-Based Containment

**Python** (match.py:1157-1176):
```python
if current_match.qcontains(next_match):  # Position-based only
    discarded_append(matches_pop(j))
```

**Rust** (match_refine.rs:364-372):
```rust
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    discarded.push(matches.remove(j));
```

**Key difference**: Rust adds `|| licensing_contains_match(...)` for expression subsumption (e.g., `gpl-2.0 WITH exception` contains `gpl-2.0`).

**Python behavior**: Expression subsumption is ONLY in `filter_overlapping_matches`, NOT in `filter_contained_matches`.

**Options**:
1. **Keep extension**: Accept divergence, may be beneficial
2. **Remove for parity**: Match Python exactly, but caused -6 regression

---

## Issue 2: Non-Contiguous Span Equality

**Python** uses `Span.__eq__` which is set-based:
```python
if current_match.qspan == next_match.qspan:  # Set equality
```

**Rust** uses bounds comparison:
```rust
if current.start_token == next.start_token && current.end_token == next.end_token {
```

**Problem**: Two matches with same bounds but different actual positions are incorrectly considered equal.

**Example**:
- Match A: tokens {1, 2, 10, 11} (start=1, end=11)
- Match B: tokens {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11} (start=1, end=11)
- Python: `A.qspan == B.qspan` → False (different sets)
- Rust: Same bounds → True (WRONG)

**Fix**: Add `spans_equal()` helper using `qspan_positions` when available.

---

## Investigation Needed

1. Why did removing `licensing_contains_match` cause regression?
   - Are there tests that depend on this filtering?
   - Is the extension actually correct behavior?

2. Is the regression acceptable for parity?
   - -6 tests may be expected if Rust was "wrong" before
   - Need to compare specific test outputs with Python

---

## Implementation (After Investigation)

If parity is confirmed as goal:

```rust
// Remove licensing_contains_match from filter_contained_matches:
if current.qcontains(&next) {  // Position-based only
    discarded.push(matches.remove(j));
    continue;
}

// Add spans_equal helper for non-contiguous spans:
fn spans_equal(a: &LicenseMatch, b: &LicenseMatch) -> bool {
    match (&a.qspan_positions, &b.qspan_positions) {
        (Some(a_pos), Some(b_pos)) => a_pos == b_pos,
        _ => a.start_token == b.start_token && a.end_token == b.end_token,
    }
}
```

---

## Files to Modify

- `src/license_detection/match_refine.rs:353-372`

---

## Reference

- Python: `licensedcode/match.py:1075-1184`
- Python Span: `licensedcode/spans.py`
- PLAN-027: Original documentation of deviation
- PLAN-053: Simplified version of this plan (consolidated)
