# PLAN-030: Fix `restore_non_overlapping()` Token Position Usage

**Date**: 2026-02-23
**Status**: Analysis Complete - Implementation Pending
**Priority**: 1 (Critical - Identified as #1 difference in PLAN-029)
**Impact**: ~100+ golden test failures
**Related**: PLAN-029 (Comprehensive Difference Analysis)

---

## Executive Summary

The Rust implementation of `restore_non_overlapping()` uses **line-based spans** while the Python reference uses **token-based spans**. This fundamental mismatch causes incorrect match restoration - matches may be incorrectly restored when they overlap on line boundaries but not token boundaries, or incorrectly discarded when they overlap on token boundaries but not line boundaries.

---

## 1. Problem Description

### 1.1 Core Issue

The function `restore_non_overlapping()` is designed to restore previously discarded matches that don't actually overlap with any kept matches. The determination of "overlap" must be done at the **token position level**, not the **line number level**.

**Why this matters:**

- A single line can contain multiple tokens
- Two matches can share a line but use different tokens on that line
- Conversely, two matches can use the same tokens but span different lines

### 1.2 Concrete Example

Consider this scenario:

```
Line 1: "This software is licensed under MIT"
Line 2: "See the LICENSE file for details"
```

Match A (kept): Tokens 0-5 (Line 1, "This software is licensed under")
Match B (discarded): Tokens 4-8 (Line 1-2, "under MIT See the")

**With line-based spans (current Rust):**

- Match A spans Line 1..Line 1
- Match B spans Line 1..Line 2
- Lines overlap → Match B stays discarded (WRONG if tokens don't overlap)

**With token-based spans (Python):**

- Match A spans tokens 0..5
- Match B spans tokens 4..8
- Tokens 4 overlaps → Match B stays discarded (CORRECT)

But if Match B was tokens 6-10 instead:

- **Line-based:** Still overlaps on Line 1..Line 2 vs Line 1
- **Token-based:** No overlap (6-10 vs 0-5) → Match B is restored (CORRECT)

---

## 2. Current State Analysis

### 2.1 Rust Implementation

**File**: `src/license_detection/match_refine.rs`
**Lines**: 688-715

```rust
fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)  // <-- USES LINE POSITIONS
}

pub fn restore_non_overlapping(
    kept: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    let all_matched_qspans = kept
        .iter()
        .fold(Span::new(), |acc, m| acc.union_span(&match_to_span(m)));  // <-- LINE-BASED

    let mut to_keep = Vec::new();
    let mut to_discard = Vec::new();

    let merged_discarded = merge_overlapping_matches(&discarded);

    for disc in merged_discarded {
        let disc_span = match_to_span(&disc);  // <-- LINE-BASED
        if !disc_span.intersects(&all_matched_qspans) {
            to_keep.push(disc);
        } else {
            to_discard.push(disc);
        }
    }

    (to_keep, to_discard)
}
```

### 2.2 Python Reference Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/match.py`
**Lines**: 1526-1548

```python
def restore_non_overlapping(matches, discarded):
    """
    Return a tuple of (matches, discarded) sequences of LicenseMatch given
    `matches` and `discarded` sequences of LicenseMatch. Reintegrate as matches
    these that may have been filtered too agressively.
    """
    all_matched_qspans = Span().union(*(m.qspan for m in matches))  # <-- TOKEN-BASED

    to_keep = []
    to_keep_append = to_keep.append

    to_discard = []
    to_discard_append = to_discard.append

    for disc in merge_matches(discarded):
        if not disc.qspan & all_matched_qspans:  # <-- TOKEN INTERSECTION CHECK
            # keep previously discarded matches that do not intersect at all
            to_keep_append(disc)
            disc.discard_reason = DiscardReason.NOT_DISCARDED
        else:
            to_discard_append(disc)

    return to_keep, to_discard
```

---

## 3. Detailed Difference Analysis

### 3.1 ALL Differences Between Rust and Python

| # | Aspect | Python | Rust | Impact |
|---|--------|--------|------|--------|
| 1 | **Span type** | Token positions (`m.qspan`) | Line positions (`m.start_line..m.end_line`) | **CRITICAL** |
| 2 | **Intersection check** | Set intersection (`&`) returns empty set | Range intersection (`intersects()`) returns boolean | **HIGH** for non-contiguous spans |
| 3 | **Union building** | `Span().union(*(m.qspan for m in matches))` | `fold(Span::new(), \|acc, m\| acc.union_span(&match_to_span(m)))` | MEDIUM - semantically same but different span type |
| 4 | **Discard reason tracking** | Sets `disc.discard_reason = DiscardReason.NOT_DISCARDED` | Not implemented | LOW - diagnostic only |
| 5 | **Variable naming** | `matches` (not `kept`) | `kept` | COSMETIC |
| 6 | **Function name (merge)** | `merge_matches()` | `merge_overlapping_matches()` | Already implemented correctly |

### 3.2 Difference #1: Span Type (CRITICAL)

**Python**: Uses `m.qspan` which is a `Span` object containing token positions.

From `match.py:179-184`:

```python
qspan = attr.ib(
    metadata=dict(
        help='query text matched Span, start at zero which is the absolute '
             'query start (not the query_run start)'
    )
)
```

The `qspan` is created during matching (see `match_seq.py:113-121`):

```python
qspan_end = qpos + mlen
qspan = Span(range(qpos, qspan_end))
```

**Rust**: Uses `m.start_line..m.end_line + 1` which are line numbers.

The `LicenseMatch` struct already has token position fields (`start_token`, `end_token`) but they are not being used.

### 3.3 Difference #2: Intersection Semantics (HIGH)

**Python's `Span.__and__`** (`spans.py:137-138`):

```python
def __and__(self, *others):
    return Span(self._set.intersection(*[o._set for o in others]))
```

This returns a **new Span** containing only the intersecting elements. The check `not disc.qspan & all_matched_qspans` tests if this new span is empty.

**Rust's `Span.intersects()`** (`spans.rs:155-164`):

```rust
pub fn intersects(&self, other: &Span) -> bool {
    for self_range in &self.ranges {
        for other_range in &other.ranges {
            if self_range.start < other_range.end && other_range.start < self_range.end {
                return true;
            }
        }
    }
    false
}
```

This returns `true` if any ranges overlap.

**Semantic difference for non-contiguous spans:**

Consider:

- Span A: tokens [1, 2, 10, 11] (non-contiguous, two ranges: 1-2 and 10-11)
- Span B: tokens [5, 6] (contiguous range 5-6)

**Python:**

```python
A & B  # Returns empty Span() - no common elements
not (A & B)  # True - correctly identifies no overlap
```

**Rust:**

```rust
A.intersects(&B)  // Checks if ranges overlap: (1-2) vs (5-6) = false, (10-11) vs (5-6) = false
// Returns false - also correct
```

For this case, both are correct. But if we represent Span A as ranges:

- Range 1: 1..3 (tokens 1, 2)
- Range 2: 10..12 (tokens 10, 11)

And Span B as range 5..7 (tokens 5, 6), then:

- No range overlap → `intersects()` returns `false` → Correct

However, if we use LINE positions where:

- Match A spans lines 1-5 (because tokens 1-2 are on line 1, tokens 10-11 are on line 3)
- Match B spans lines 2-3 (because tokens 5-6 are on line 2)

Then:

- Line 3 is in both spans → `intersects()` returns `true` → **WRONG**

---

## 4. Proposed Changes

### 4.1 Primary Fix: Use Token Positions

**Location**: `src/license_detection/match_refine.rs:688-715`

**Change the helper function:**

```rust
// BEFORE:
fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)
}

// AFTER:
fn match_to_qspan(m: &LicenseMatch) -> Span {
    // Use token positions, which are the correct semantic for overlap detection
    Span::from_range(m.start_token..m.end_token)
}
```

**Update the main function:**

```rust
// BEFORE:
pub fn restore_non_overlapping(
    kept: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    let all_matched_qspans = kept
        .iter()
        .fold(Span::new(), |acc, m| acc.union_span(&match_to_span(m)));

    let mut to_keep = Vec::new();
    let mut to_discard = Vec::new();

    let merged_discarded = merge_overlapping_matches(&discarded);

    for disc in merged_discarded {
        let disc_span = match_to_span(&disc);
        if !disc_span.intersects(&all_matched_qspans) {
            to_keep.push(disc);
        } else {
            to_discard.push(disc);
        }
    }

    (to_keep, to_discard)
}

// AFTER:
pub fn restore_non_overlapping(
    matches: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // Build union of all matched token positions
    // Python: all_matched_qspans = Span().union(*(m.qspan for m in matches))
    let all_matched_qspans = matches
        .iter()
        .fold(Span::new(), |acc, m| acc.union_span(&match_to_qspan(m)));

    let mut to_keep = Vec::new();
    let mut to_discard = Vec::new();

    let merged_discarded = merge_overlapping_matches(&discarded);

    for disc in merged_discarded {
        let disc_qspan = match_to_qspan(&disc);
        // Check if token positions intersect
        // Python: if not disc.qspan & all_matched_qspans:
        if !disc_qspan.intersects(&all_matched_qspans) {
            // Keep previously discarded matches that do not intersect at all
            to_keep.push(disc);
        } else {
            to_discard.push(disc);
        }
    }

    (to_keep, to_discard)
}
```

### 4.2 Handle Non-Contiguous Token Positions (Optional Enhancement)

The `LicenseMatch` struct has fields for non-contiguous positions:

```rust
/// Token positions matched in the query text.
/// None means contiguous range [start_token, end_token).
/// Some(positions) contains exact positions for non-contiguous matches (after merge).
#[serde(skip)]
pub qspan_positions: Option<Vec<usize>>,
```

For maximum accuracy, we should use these when available:

```rust
fn match_to_qspan(m: &LicenseMatch) -> Span {
    if let Some(positions) = &m.qspan_positions {
        // Non-contiguous match: use exact positions
        Span::from_iterator(positions.iter().copied())
    } else {
        // Contiguous match: use range
        Span::from_range(m.start_token..m.end_token)
    }
}
```

**Note**: The current `Span::from_iterator()` implementation exists but may need verification for performance with large position sets.

### 4.3 Variable Naming Alignment (Minor)

Consider renaming `kept` to `matches` to match Python parameter name:

```rust
pub fn restore_non_overlapping(
    matches: &[LicenseMatch],  // Was: kept
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>)
```

Update call sites (lines 1443, 1451):

```rust
// Line 1443:
let (restored_contained, _) = restore_non_overlapping(&kept, discarded_contained);
// Becomes:
let (restored_contained, _) = restore_non_overlapping(&kept, discarded_contained);
// (No change needed - local variable name is fine)

// Line 1451:
restore_non_overlapping(&matches_after_first_restore, discarded_overlapping);
// (No change needed)
```

---

## 5. Edge Cases and Error Handling

### 5.1 Empty Spans

**Scenario**: `kept` is empty.

**Current behavior** (both Python and Rust):

- `all_matched_qspans` is empty
- All discarded matches should be restored (they don't intersect with nothing)

**Verification**: Test `test_restore_non_overlapping_empty_kept` already covers this.

### 5.2 Zero Token Positions

**Scenario**: A match has `start_token == 0 && end_token == 0`.

**Analysis**: This indicates uninitialized token positions. The current tests use `create_test_match()` which sets:

```rust
start_token: start_line,
end_token: end_line + 1,
```

**Mitigation**: Add assertion or default behavior:

```rust
fn match_to_qspan(m: &LicenseMatch) -> Span {
    if m.start_token == 0 && m.end_token == 0 {
        // Fallback to line positions if tokens not set
        // This shouldn't happen in production but provides safety
        Span::from_range(m.start_line..m.end_line + 1)
    } else {
        Span::from_range(m.start_token..m.end_token)
    }
}
```

### 5.3 Non-Contiguous Spans After Merge

**Scenario**: After `merge_overlapping_matches()`, a match may have non-contiguous `qspan_positions`.

**Python behavior**: `merge_matches()` creates `qspan = Span(self.qspan | other.qspan)` which correctly handles non-contiguous spans.

**Rust behavior**: Need to verify `merge_overlapping_matches()` properly sets `qspan_positions`.

**Mitigation**: Use the enhancement from Section 4.2 to handle `qspan_positions`.

### 5.4 Token vs Line Number Off-by-One

**Observation**: Line numbers are 1-indexed, token positions are 0-indexed.

**Current code**:

```rust
Span::from_range(m.start_line..m.end_line + 1)  // +1 because end_line is inclusive
Span::from_range(m.start_token..m.end_token)    // end_token is exclusive, no +1 needed
```

This is correct - the `+1` adjustment is only needed for line numbers.

---

## 6. Test Requirements

Per `docs/TESTING_STRATEGY.md`, this change requires:

### 6.1 Unit Tests (Layer 1)

**Location**: `src/license_detection/match_refine.rs` in `#[cfg(test)] mod tests`

**Required new tests**:

1. **Token overlap detection** - matches with same lines but different tokens:

```rust
#[test]
fn test_restore_non_overlapping_same_line_different_tokens() {
    // Match A: tokens 0-5, lines 1-1
    // Match B: tokens 10-15, lines 1-1
    // Same line but no token overlap → B should be restored
}
```

1. **Token overlap across lines** - matches with different lines but overlapping tokens:

```rust
#[test]
fn test_restore_non_overlapping_different_lines_overlapping_tokens() {
    // Match A: tokens 0-10, lines 1-2
    // Match B: tokens 5-15, lines 2-3
    // Token overlap at 5-10 → B should NOT be restored
}
```

1. **Non-contiguous token positions**:

```rust
#[test]
fn test_restore_non_overlapping_non_contiguous_qspan() {
    // Match A: qspan_positions = Some(vec![1, 2, 10, 11])
    // Match B: qspan_positions = Some(vec![5, 6])
    // No overlap → B should be restored
}
```

1. **Token positions not initialized**:

```rust
#[test]
fn test_restore_non_overlapping_zero_tokens_fallback() {
    // Match with start_token=0, end_token=0
    // Should fallback to line-based or handle gracefully
}
```

### 6.2 Update Existing Tests

**Location**: `src/license_detection/match_refine.rs:2318-2464`

**Issue**: Existing tests use `create_test_match()` which sets:

```rust
start_token: start_line,
end_token: end_line + 1,
```

This means `start_token != start_line`, so tests will behave differently after the fix.

**Action**: Update test helper to be explicit about token positions:

```rust
fn create_test_match(
    rule_identifier: &str,
    start_line: usize,
    end_line: usize,
    score: f32,
    coverage: f32,
    relevance: u8,
) -> LicenseMatch {
    // Add explicit token positions that differ from lines
    LicenseMatch {
        // ...
        start_line,
        end_line,
        start_token: start_line * 10,  // Tokens don't equal lines
        end_token: (end_line + 1) * 10,
        // ...
    }
}
```

Or create a new helper:

```rust
fn create_test_match_with_explicit_tokens(
    rule_identifier: &str,
    start_line: usize,
    end_line: usize,
    start_token: usize,
    end_token: usize,
    score: f32,
    coverage: f32,
    relevance: u8,
) -> LicenseMatch {
    // ...
}
```

### 6.3 Golden Tests (Layer 2)

**Existing golden tests should pass more reliably** after this fix, as token-based overlap is what Python uses.

**Specific tests to verify**:

- Any test involving multiple matches on the same line
- Any test involving matches that span multiple lines

### 6.4 Python Test Reference

**File**: `reference/scancode-toolkit/tests/licensedcode/test_match.py:1020-1037`

```python
def test_restore_non_overlapping_restores_non_overlapping(self):
    # m1 = Span(0, 5)
    # m2 = Span(0, 40)  # Contains m1
    # m3 = Span(6, 120) # Does not overlap with m1's tokens
    
    result, discarded = filter_overlapping_matches([m2, m1, m3])
    assert result == [m3]
    assert discarded == [m1, m2]
    
    result, discarded = restore_non_overlapping(result, discarded)
    assert result == [m1]  # m1 restored because its tokens (0-5) don't overlap with m3 (6-120)
    assert discarded == [m2]
```

This test demonstrates that `m1` with tokens 0-5 is restored because it doesn't overlap with `m3`'s tokens 6-120, even though `m1` was contained within `m2` (which overlaps with `m3`).

---

## 7. Risk Assessment

### 7.1 Impact Analysis

| Component | Risk Level | Description |
|-----------|------------|-------------|
| `restore_non_overlapping()` | **HIGH** | Core function being modified |
| `merge_overlapping_matches()` | LOW | Already uses token positions internally |
| `filter_overlapping_matches()` | LOW | Not affected by this change |
| Call sites (lines 1443, 1451) | MEDIUM | Must verify behavior still correct |
| Unit tests | MEDIUM | Tests may need updates for new semantics |

### 7.2 Regression Risk

**Before fix**: Incorrect behavior (line-based) that some tests may have adapted to.

**After fix**: Correct behavior (token-based) matching Python.

**Mitigation**: Run full golden test suite before and after. Expect improved pass rate.

### 7.3 Dependencies

**No dependencies on other fixes** - this is a standalone change.

**However**, other fixes may expose issues that were hidden by the line-based behavior. For example:

- If token positions aren't being set correctly during matching, this fix will expose that
- If `merge_overlapping_matches()` doesn't preserve token positions, this fix will expose that

---

## 8. Implementation Checklist

- [ ] **Step 1**: Rename `match_to_span` to `match_to_qspan` (or create new function)
- [ ] **Step 2**: Change implementation to use token positions
- [ ] **Step 3**: Add optional handling for `qspan_positions` (non-contiguous)
- [ ] **Step 4**: Update function parameter name from `kept` to `matches` (optional)
- [ ] **Step 5**: Add new unit tests for token-based overlap
- [ ] **Step 6**: Update existing unit tests if needed
- [ ] **Step 7**: Run unit tests: `cargo test restore_non_overlapping`
- [ ] **Step 8**: Run full test suite: `cargo test`
- [ ] **Step 9**: Run golden tests and verify improved pass rate
- [ ] **Step 10**: Update documentation comments

---

## 9. Code Changes Summary

### 9.1 File: `src/license_detection/match_refine.rs`

**Lines 688-715** - Replace `match_to_span()` and update `restore_non_overlapping()`:

```rust
/// Convert a LicenseMatch's token positions to a Span for overlap detection.
/// 
/// Uses token positions (not line positions) to correctly identify overlap
/// at the token level, matching Python's `m.qspan` behavior.
fn match_to_qspan(m: &LicenseMatch) -> Span {
    // Use qspan_positions if available (for non-contiguous matches after merge)
    if let Some(positions) = &m.qspan_positions {
        Span::from_iterator(positions.iter().copied())
    } else {
        // For contiguous matches, use the token range
        Span::from_range(m.start_token..m.end_token)
    }
}

/// Restore previously discarded matches that don't overlap with kept matches.
///
/// This function checks overlap at the **token position level**, not line level.
/// A match that shares a line but not tokens with kept matches will be restored.
///
/// Based on Python: `restore_non_overlapping()` (match.py:1526-1548)
pub fn restore_non_overlapping(
    matches: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // Build union of all matched token positions
    // Python: all_matched_qspans = Span().union(*(m.qspan for m in matches))
    let all_matched_qspans = matches
        .iter()
        .fold(Span::new(), |acc, m| acc.union_span(&match_to_qspan(m)));

    let mut to_keep = Vec::new();
    let mut to_discard = Vec::new();

    // Python calls merge_matches() on discarded before processing
    let merged_discarded = merge_overlapping_matches(&discarded);

    for disc in merged_discarded {
        let disc_qspan = match_to_qspan(&disc);
        // Python: if not disc.qspan & all_matched_qspans:
        // Check if token positions intersect
        if !disc_qspan.intersects(&all_matched_qspans) {
            // Keep previously discarded matches that do not intersect at all
            to_keep.push(disc);
        } else {
            to_discard.push(disc);
        }
    }

    (to_keep, to_discard)
}
```

### 9.2 Update Call Sites

**Line 1443** - No change needed (variable names stay same):

```rust
let (restored_contained, _) = restore_non_overlapping(&kept, discarded_contained);
```

**Line 1451** - No change needed:

```rust
let (restored_overlapping, _) =
    restore_non_overlapping(&matches_after_first_restore, discarded_overlapping);
```

---

## 10. Verification Plan

### 10.1 Pre-Implementation

1. Run current golden tests and record baseline pass rate
2. Identify specific tests that should improve after fix

### 10.2 Post-Implementation

1. Run `cargo test restore_non_overlapping` - all tests must pass
2. Run `cargo test --lib` - all library tests must pass
3. Run `cargo test` (full suite) - verify no regressions
4. Run golden tests - expect improved pass rate
5. Compare specific test outputs with Python reference

### 10.3 Success Criteria

- All existing unit tests pass (with expected updates)
- Golden test pass rate improves by ~10-20 tests
- No new test failures introduced
- Code passes `cargo clippy` without warnings
- Code formatted with `cargo fmt`

---

## 11. References

- **PLAN-029**: Comprehensive Difference Analysis (identifies this as #1 issue)
- **Python source**: `reference/scancode-toolkit/src/licensedcode/match.py:1526-1548`
- **Python Span**: `reference/scancode-toolkit/src/licensedcode/spans.py`
- **Rust source**: `src/license_detection/match_refine.rs:688-715`
- **Rust models**: `src/license_detection/models.rs` (LicenseMatch struct)
- **Testing strategy**: `docs/TESTING_STRATEGY.md`

---

## 12. Document History

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-23 | AI Agent | Initial plan creation |
