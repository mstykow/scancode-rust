# PLAN-053: Remove licensing_contains from filter_contained_matches

## Status: NOT IMPLEMENTED

## Summary

Rust's `filter_contained_matches()` uses `licensing_contains_match()` for expression-based subsumption, but Python does NOT. Python only uses expression subsumption in `filter_overlapping_matches()`. This deviation causes Rust to filter more matches than Python in some cases.

---

## Problem Statement

**Rust** (match_refine.rs:323-377):

```rust
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) || licensing_contains_match(&next, &current) {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

**Python** (match.py:1157-1176):

```python
# Python ONLY uses qcontains for containment, NOT licensing_contains:
if current_match.qcontains(next_match):
    discarded_append(matches_pop(j))
    continue

if next_match.qcontains(current_match):
    discarded_append(matches_pop(i))
    i -= 1
    break
```

**Key difference**: Rust adds `|| licensing_contains_match(...)` but Python does NOT.

---

## Impact

Rust is **MORE AGGRESSIVE** in filtering matches:
- May over-filter cases where Python keeps matches
- Causes differences in golden test output
- Violates the goal of Python parity

---

## Implementation

**Location**: `src/license_detection/match_refine.rs:364-372`

Remove `licensing_contains_match()` from `filter_contained_matches()`:

```rust
// Before:
if current.qcontains(&next) || licensing_contains_match(&current, &next) {

// After (match Python):
if current.qcontains(&next) {
```

Apply to both containment checks in the function.

---

## Note

Expression subsumption via `licensing_contains_match()` should remain in `filter_overlapping_matches()` where Python also uses it.

---

## Priority: HIGH

This is a direct parity violation that may cause golden test failures.

---

## Verification

1. Run golden tests before and after change
2. Verify tests that were over-filtered now pass
3. Check for any regressions

---

## Reference

- PLAN-027: Documents this deviation as "intentional extension"
- PLAN-048: P5 notes this as "intentional extension" but now reconsidered
- Python reference: `licensedcode/match.py:1157-1176`
