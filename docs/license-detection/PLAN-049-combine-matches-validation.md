# PLAN-049: Add Validation to combine_matches()

## Status: NOT IMPLEMENTED

## Summary

Rust's `combine_matches()` does NOT validate that matches have the same `rule_identifier` before combining. Python throws `TypeError` if rules differ. This can cause undefined behavior if matches from different rules end up in the same group.

---

## Problem Statement

**Python** (match.py:642-646):

```python
def combine(self, other):
    if self.rule != other.rule:
        raise TypeError('Cannot combine matches with different rules')
```

**Rust** (match_refine.rs:106-146):

```rust
fn combine_matches(a: &LicenseMatch, b: &LicenseMatch) -> LicenseMatch {
    let mut merged = a.clone();
    // No validation that a and b have the same rule_identifier!
```

---

## Impact

If matches from different rules ever end up in the same group, Rust silently merges them with undefined behavior. This could lead to:
- Incorrect match data
- Wrong license expressions
- Subtle bugs in downstream processing

---

## Implementation

**Location**: `src/license_detection/match_refine.rs:106-146`

Add validation at the start of `combine_matches()`:

```rust
fn combine_matches(a: &LicenseMatch, b: &LicenseMatch) -> LicenseMatch {
    assert_eq!(
        a.rule_identifier, b.rule_identifier,
        "Cannot combine matches with different rules: {} vs {}",
        a.rule_identifier, b.rule_identifier
    );
    
    let mut merged = a.clone();
    // ... rest of implementation
}
```

Or return an error if assertions are not desired in production code.

---

## Priority: HIGH

This is a correctness issue that could cause silent data corruption.

---

## Verification

1. Run existing unit tests
2. Run golden tests to verify no regressions
3. Add unit test for the validation itself

## Reference

- PLAN-048: P2 - Original finding
