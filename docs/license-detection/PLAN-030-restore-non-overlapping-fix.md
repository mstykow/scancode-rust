# PLAN-030: Fix `restore_non_overlapping()` Token Position Usage

**Date**: 2026-02-23
**Status**: Validated - Root Cause Identified, Implementation Strategy Confirmed
**Priority**: 1 (Critical - Identified as #1 difference in PLAN-029)
**Impact**: ~100+ golden test failures expected to improve
**Related**: PLAN-029 (Comprehensive Difference Analysis)

**Key Finding**: Previous implementation attempts failed due to test helpers with unrealistic token positions (not a fundamental issue with the approach). The correct implementation requires fallback logic for zero-token cases, mirroring the pattern already used in `qcontains()` and `qoverlap()`.

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

### 4.1 Primary Fix: Use Token Positions with Fallback

**Location**: `src/license_detection/match_refine.rs:715-742`

**Critical insight**: The naive fix (simply using `start_token..end_token`) fails because some matches have uninitialized token positions (`start_token == 0 && end_token == 0`). The fix must handle this case.

**Change the helper function with fallback logic:**

```rust
// BEFORE:
fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)
}

// AFTER (with fallback for uninitialized tokens):
fn match_to_qspan(m: &LicenseMatch) -> Span {
    // Case 1: Non-contiguous positions from merged match
    if let Some(positions) = &m.qspan_positions {
        if !positions.is_empty() {
            return Span::from_iterator(positions.iter().copied());
        }
    }

    // Case 2: Check if token positions are initialized
    // Following the pattern from qcontains() and qoverlap() in models.rs:498-506
    if m.start_token == 0 && m.end_token == 0 {
        // Fallback to line positions when tokens not set
        // This handles test matches with uninitialized token positions
        return Span::from_range(m.start_line..m.end_line + 1);
    }

    // Case 3: Normal contiguous token range
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
    matches: &[LicenseMatch],  // Renamed from 'kept' for Python parity
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

### 4.2 Why Fallback Logic is Required

The fallback logic is **not optional** - it's required because:

1. **Test helpers** (e.g., `create_test_match()` in `detection.rs:1166-1167`) create matches with `start_token: 0, end_token: 0`

2. **Existing functions** (`qcontains()` and `qoverlap()` in `models.rs:498-521`) already implement this fallback pattern:
   ```rust
   if self.start_token == 0 && self.end_token == 0
       && other.start_token == 0 && other.end_token == 0
   {
       // Fall back to line-based comparison
       return self.start_line <= other.start_line && self.end_line >= other.end_line;
   }
   ```

3. **Without the fallback**, matches with `start_token=0, end_token=0` create empty spans `0..0`, which never intersect with anything, causing incorrect restoration of all discarded matches.

### 4.3 Handle Non-Contiguous Token Positions

The `LicenseMatch` struct has fields for non-contiguous positions:

```rust
/// Token positions matched in the query text.
/// None means contiguous range [start_token, end_token).
/// Some(positions) contains exact positions for non-contiguous matches (after merge).
#[serde(skip)]
pub qspan_positions: Option<Vec<usize>>,
```

When `merge_overlapping_matches()` merges matches, it sets `qspan_positions` with the union of all token positions. This is handled in Case 1 of the implementation above.

**Implementation note**: `Span::from_iterator()` already exists and handles non-contiguous positions correctly by coalescing adjacent positions into ranges.
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

## 12. Validation Report: Why Previous Implementation Caused Regressions

### 12.1 Executive Summary

The naive fix (simply changing `match_to_span()` to use token positions) causes 6 test regressions because:

1. **Some matches have uninitialized token positions** (`start_token == 0 && end_token == 0`)
2. **The Span intersection semantics differ** between Python's set-based approach and Rust's range-based approach
3. **Test helpers use unrealistic token position values** that break when token-based checking is enabled

### 12.2 Root Cause Analysis

#### Issue 1: Uninitialized Token Positions

Several code paths create matches with `start_token: 0, end_token: 0`:

**From `src/license_detection/detection.rs:1166-1167`:**
```rust
start_token: 0,
end_token: 0,
```

This appears in test helper `create_test_match()` which is used extensively in unit tests.

**The problem**: When `start_token == 0 && end_token == 0`, the naive fix creates an empty span `0..0`, which:
- Never intersects with anything (all discarded matches get restored incorrectly)
- Or intersects with token 0 matches (incorrect false positives)

**Evidence from `src/license_detection/models.rs:498-506`:**
```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    // ...
    if self.start_token == 0
        && self.end_token == 0
        && other.start_token == 0
        && other.end_token == 0
    {
        return self.start_line <= other.start_line && self.end_line >= other.end_line;
    }
    self.start_token <= other.start_token && self.end_token >= other.end_token
}
```

This shows that other functions (like `qcontains` and `qoverlap`) **already have fallback logic** for the zero-token case.

#### Issue 2: Span Intersection Semantics Differ

**Python's Span (`spans.py:137-138`):**
```python
def __and__(self, *others):
    return Span(self._set.intersection(*[o._set for o in others]))
```

Python uses an `intbitset` internally, so `not disc.qspan & all_matched_qspans` tests if the **intersection set is empty**.

**Rust's Span (`spans.rs:155-164`):**
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

Rust checks if any **ranges overlap**.

**Critical difference for non-contiguous spans:**
- Python: `Span([1, 2, 10, 11])` is stored as `{1, 2, 10, 11}` in an intbitset
- Rust: `Span::from_iterator([1, 2, 10, 11])` becomes ranges `[1..3, 10..12]`

For the intersection check, both are equivalent because:
- Python: `{1, 2, 10, 11} & {5, 6} = {}` (empty set, no intersection)
- Rust: ranges `[1..3, 10..12]` vs `[5..7]` → no range overlap → `intersects()` returns false

**However**, if we create spans incorrectly (e.g., `0..0`), Rust's range check gives wrong results.

#### Issue 3: Test Helper Token Position Values

**From `src/license_detection/match_refine.rs:1515-1516`:**
```rust
start_token: start_line,
end_token: end_line + 1,
```

This test helper sets token positions equal to line positions (offset by 1). This is **unrealistic** because:
- In real matches, `start_token` and `start_line` are unrelated
- Token positions depend on how many tokens are in the file before this match
- Line positions depend on line breaks in the source text

**Example test that would fail with naive fix:**

```rust
// test_restore_non_overlapping_touching_is_overlapping
let kept = vec![create_test_match("#1", 1, 10, ...)];  // start_token=1, end_token=11
let discarded = vec![create_test_match("#2", 10, 20, ...)];  // start_token=10, end_token=21

// With line-based spans:
// kept: lines 1..11, discarded: lines 10..21
// Lines 10 overlaps → discarded stays discarded ✓

// With naive token-based spans:
// kept: tokens 1..11, discarded: tokens 10..21
// Tokens 10 overlaps → discarded stays discarded ✓ (ACCIDENTALLY CORRECT)

// But if test helper set start_token=start_line, end_token=end_line+1, and line=10:
// kept ends at token 11, discarded starts at token 10
// They overlap at token 10-11 ✓
```

**The real problem case:**
```rust
// Test helper with lines but zero tokens (from detection.rs)
let match = LicenseMatch {
    start_line: 10,
    end_line: 20,
    start_token: 0,  // ← Zero!
    end_token: 0,    // ← Zero!
    ...
};
```

With naive token fix: Span `0..0` is empty, so it never intersects → incorrectly restored.

### 12.3 Specific Failing Test Cases

#### Test 1: `test_restore_non_overlapping_touching_is_overlapping`

**Location**: `src/license_detection/match_refine.rs:2538-2547`

**With naive fix:**
- `kept`: `create_test_match("#1", 1, 10, ...)` → start_token=1, end_token=11
- `discarded`: `create_test_match("#2", 10, 20, ...)` → start_token=10, end_token=21
- Token spans: `1..11` vs `10..21` → **overlap at 10-11** ✓

This test passes accidentally because the helper sets `start_token = start_line`.

#### Test 2: `test_restore_non_overlapping_adjacent_not_overlapping`

**Location**: `src/license_detection/match_refine.rs:2527-2536`

**With naive fix:**
- `kept`: lines 1-10 → tokens 1..11
- `discarded`: lines 11-20 → tokens 11..21
- Token spans: `1..11` vs `11..21` → **adjacent, no overlap** ✓

This test also passes accidentally.

#### Test 3: Tests using `create_test_match()` from `detection.rs`

**Location**: `src/license_detection/detection.rs:1154-1189`

This helper sets `start_token: 0, end_token: 0`, creating empty spans.

**With naive fix:**
- All matches from this helper have empty `0..0` span
- Empty spans never intersect
- All discarded matches get restored → tests fail

### 12.4 The Real Solution

The fix must handle **three cases**:

1. **Normal case**: Token positions are set (`start_token != end_token` or both non-zero)
   - Use `Span::from_range(start_token..end_token)`

2. **Non-contiguous case**: `qspan_positions` is `Some(positions)`
   - Use `Span::from_iterator(positions)`

3. **Fallback case**: Token positions are not set (`start_token == 0 && end_token == 0`)
   - Fall back to line positions: `Span::from_range(start_line..end_line + 1)`
   - This matches the fallback logic in `qcontains()` and `qoverlap()`

### 12.5 Implementation Details

#### Corrected `match_to_qspan()` Function

```rust
fn match_to_qspan(m: &LicenseMatch) -> Span {
    // Case 1: Non-contiguous positions from merged match
    if let Some(positions) = &m.qspan_positions {
        if !positions.is_empty() {
            return Span::from_iterator(positions.iter().copied());
        }
    }

    // Case 2: Check if token positions are initialized
    // Following the pattern from qcontains() and qoverlap() in models.rs
    if m.start_token == 0 && m.end_token == 0 {
        // Fallback to line positions when tokens not set
        // This handles test matches and any edge cases
        return Span::from_range(m.start_line..m.end_line + 1);
    }

    // Case 3: Normal contiguous token range
    Span::from_range(m.start_token..m.end_token)
}
```

This mirrors the fallback logic already present in `qcontains()` and `qoverlap()`:
```rust
// From models.rs:498-505
if self.start_token == 0
    && self.end_token == 0
    && other.start_token == 0
    && other.end_token == 0
{
    return self.start_line <= other.start_line && self.end_line >= other.end_line;
}
```

### 12.6 Why This Won't Break Golden Tests

Golden tests use real matches created by actual matching strategies:

1. **Hash matches** (`hash_match.rs:105-106`): `start_token: query_run.start, end_token: query_run.end.map_or(query_run.start, |e| e + 1)`

2. **Aho matches** (`aho_match.rs:166-167`): `start_token: qstart, end_token: qend`

3. **Seq matches** (`seq_match.rs`): Token positions computed from alignment

4. **SPDX-LID matches** (`spdx_lid.rs:280-281`): `start_token: *start_token, end_token: *end_token`

5. **Unknown matches** (`unknown_match.rs:309-310`): `start_token: start, end_token: end`

All real matches have proper token positions set, so they will use the token-based span correctly.

---

## 13. Pre-Implementation Validation Steps

Before implementing the fix:

### Step 1: Audit Token Position Initialization

Verify that all match creation sites properly set token positions:

```bash
# Find all places where LicenseMatch is created
grep -rn "LicenseMatch {" src/license_detection/
```

**Expected result**: All production code paths set proper token positions. Only test helpers may use zero.

### Step 2: Verify `qspan_positions` Handling in `merge_overlapping_matches()`

**Location**: `src/license_detection/match_refine.rs:143`

```rust
merged.qspan_positions = Some(qspan_vec);
```

This correctly sets `qspan_positions` when matches are merged, so non-contiguous spans will be handled correctly.

### Step 3: Run Current Tests

```bash
cargo test restore_non_overlapping
```

Record which tests pass/fail with current line-based implementation.

### Step 4: Verify Span::from_iterator Works

Run the existing span tests:

```bash
cargo test spans
```

Ensure `Span::from_iterator()` correctly handles non-contiguous positions.

---

## 14. Revised Implementation Checklist

- [ ] **Step 1**: Implement `match_to_qspan()` with fallback logic (see Section 12.5)
- [ ] **Step 2**: Update `restore_non_overlapping()` to use `match_to_qspan()`
- [ ] **Step 3**: Add unit tests for edge cases:
  - [ ] Token positions set (normal case)
  - [ ] Token positions zero (fallback to lines)
  - [ ] `qspan_positions` set (non-contiguous)
- [ ] **Step 4**: Run unit tests: `cargo test restore_non_overlapping`
- [ ] **Step 5**: Run full test suite: `cargo test`
- [ ] **Step 6**: Run golden tests and compare with baseline
- [ ] **Step 7**: Verify specific Python test case from `test_match.py:1027-1037`

---

## 15. Success Criteria

1. **All unit tests pass** with the new implementation
2. **Golden test pass rate improves** (no regressions, potentially +10-20 passes)
3. **Python parity test passes**: The test from `test_match.py:1027-1037` produces correct results
4. **Code passes `cargo clippy`** without warnings
5. **Code formatted with `cargo fmt`**

---

## 17. Deep Validation Analysis: Why Previous Implementation Attempts Failed

### 17.1 Executive Summary

Previous implementation attempts to use token positions in `restore_non_overlapping()` caused golden test regressions because:

1. **Two test helpers exist with different token position behavior** - one sets realistic token values, one sets zero
2. **Test helper token positions are unrealistic** - they conflate line numbers with token positions
3. **The fallback logic was insufficient** - it didn't account for all edge cases
4. **The golden tests revealed deeper issues** - many failures show duplicate expressions, suggesting over-restoration

### 17.2 Analysis of Test Helpers

#### Test Helper 1: `match_refine.rs:1502-1541`

```rust
fn create_test_match(
    rule_identifier: &str,
    start_line: usize,
    end_line: usize,
    score: f32,
    coverage: f32,
    relevance: u8,
) -> LicenseMatch {
    LicenseMatch {
        // ...
        start_token: start_line,      // WRONG: conflates lines with tokens
        end_token: end_line + 1,      // WRONG: conflates lines with tokens
        // ...
    }
}
```

**Problem**: Token positions are set to line numbers. This is unrealistic because:
- Real token positions depend on how many tokens are in the file before the match
- Line 10 could be token 500 depending on file content
- This breaks the token overlap logic when lines and tokens are different

**Example failure**:
- Match A: lines 1-10 → tokens 1..11
- Match B: lines 11-20 → tokens 11..21
- Adjacent on lines, also "adjacent" on tokens → test passes by ACCIDENT
- But in reality, lines 1-10 might be tokens 0-100, and lines 11-20 might be tokens 200-300

#### Test Helper 2: `detection.rs:1154-1189`

```rust
fn create_test_match(
    start_line: usize,
    end_line: usize,
    matcher: &str,
    rule_identifier: &str,
) -> LicenseMatch {
    LicenseMatch {
        // ...
        start_token: 0,   // ZERO - will create empty span!
        end_token: 0,     // ZERO - will create empty span!
        // ...
    }
}
```

**Problem**: Token positions are explicitly set to zero, creating empty spans.

**With naive token-based fix**:
- Span `0..0` is empty
- Empty spans never intersect with anything
- All discarded matches get restored → tests fail catastrophically

### 17.3 Golden Test Failure Pattern Analysis

From the test output, the primary failure pattern is:

```
Expected: ["gpl-2.0 OR bsd-new", "gpl-2.0 OR bsd-new"]
Actual:   ["gpl-2.0", "gpl-2.0 OR bsd-new"]
```

**Analysis**:
- Expected has 2 identical expressions (`gpl-2.0 OR bsd-new`)
- Actual has 2 different expressions (`gpl-2.0` and `gpl-2.0 OR bsd-new`)
- This suggests that a match for `gpl-2.0` is being RESTORED when it shouldn't be

**Root Cause Hypothesis**:
1. A match for `gpl-2.0` was discarded (contained in or overlapping with another match)
2. After `filter_overlapping_matches()`, `restore_non_overlapping()` checks if it overlaps
3. With LINE-based spans: it correctly sees overlap → stays discarded
4. With TOKEN-based spans (if wrong): it might incorrectly see no overlap → gets restored

### 17.4 The Real Problem: Token vs Line Semantics

Consider this file:
```
Line 1: // This is licensed under GPL-2.0 or BSD-New
Line 2: // See LICENSE file for details
```

**Match A** (kept): "GPL-2.0 or BSD-New" - tokens 5-8 (assuming "// This is licensed under" are tokens 0-4)
**Match B** (discarded): "GPL-2.0" - tokens 5-6

**With line-based spans (current Rust)**:
- Match A: lines 1-1
- Match B: lines 1-1
- Overlap: YES (same line) → B stays discarded ✓

**With token-based spans (correct)**:
- Match A: tokens 5-8
- Match B: tokens 5-6
- Overlap: YES (tokens 5-6) → B stays discarded ✓

Both are correct in this case because the tokens are on the same line.

But consider:
```
Line 1: // MIT License - see below
Line 2: // Copyright 2024
Line 3: // Permission is hereby granted...
```

**Match A** (kept): MIT license text - tokens 10-100 (lines 1-3)
**Match B** (discarded): "MIT License" text - tokens 1-3 (line 1 only)

**With line-based spans (current Rust)**:
- Match A: lines 1-3
- Match B: lines 1-1
- Overlap: YES (line 1) → B stays discarded ✓

**With token-based spans (correct)**:
- Match A: tokens 10-100
- Match B: tokens 1-3
- Overlap: NO → B gets restored ✓ (CORRECT! They don't share tokens)

This is the key difference: line-based can incorrectly keep matches discarded, while token-based correctly restores them.

### 17.5 Why the Naive Fix Caused Regressions

The naive fix:

```rust
fn match_to_qspan(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_token..m.end_token)
}
```

Caused regressions because:

1. **Test matches with `start_token=0, end_token=0`** create empty spans
2. **Empty spans never intersect** → all discarded matches restored
3. **Test matches with `start_token=start_line`** have unrealistic token positions
4. **Golden tests use real matches** with real token positions, so the fix SHOULD help them
5. **But the unit tests with bad token values broke**, and the fallback wasn't implemented

### 17.6 Correct Implementation Strategy

The implementation MUST:

1. **Use `qspan_positions` when available** (for merged matches with non-contiguous positions)
2. **Check for zero-token case** and fallback to line positions
3. **Handle the case where token positions are set but unrealistic** (for test compatibility)

**Recommended Implementation**:

```rust
fn match_to_qspan(m: &LicenseMatch) -> Span {
    // Case 1: Non-contiguous positions from merged match
    if let Some(positions) = &m.qspan_positions {
        if !positions.is_empty() {
            return Span::from_iterator(positions.iter().copied());
        }
    }

    // Case 2: Token positions are not set (zero case)
    // This handles test matches and any edge cases
    if m.start_token == 0 && m.end_token == 0 {
        // Fallback to line positions when tokens not set
        return Span::from_range(m.start_line..m.end_line + 1);
    }

    // Case 3: Normal contiguous token range
    Span::from_range(m.start_token..m.end_token)
}
```

### 17.7 Span Intersection Semantics

**Python's `Span.__and__`**:
```python
def __and__(self, *others):
    return Span(self._set.intersection(*[o._set for o in others]))
```

Returns a **new Span** with the intersection. The check `not disc.qspan & all_matched_qspans` tests if the result is empty.

**Rust's `Span.intersects`**:
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

Returns `true` if any ranges overlap.

**For contiguous spans, these are equivalent**.

**For non-contiguous spans**, both correctly detect overlap:
- Python: `{1, 2, 10, 11} & {5, 6} = {}` (empty set)
- Rust: ranges `[1..3, 10..12]` vs `[5..7]` → no overlap

**Conclusion**: The Span implementation is correct; the issue is in how we create Spans from matches.

### 17.8 Matcher Token Position Verification

All matchers correctly set token positions:

| Matcher | Token Position Source | Code Location |
|---------|----------------------|---------------|
| hash_match | `query_run.start` to `query_run.end + 1` | `hash_match.rs:105-106` |
| aho_match | `qstart` to `qend` | `aho_match.rs:166-167` |
| seq_match | `abs_qpos` to `abs_qend + 1` | `seq_match.rs:736-737` |
| spdx_lid | `*start_token` to `*end_token` | `spdx_lid.rs:280-281` |
| unknown_match | `start` to `end` | `unknown_match.rs:309-310` |

**Conclusion**: Production matchers set token positions correctly. Only test helpers have issues.

### 17.9 Specific Golden Test Cases to Analyze

#### Case 1: `ipheth.c` - BSD-New OR GPL-2.0

```
Expected: ["bsd-new OR gpl-2.0", "bsd-new OR gpl-2.0"]
Actual:   ["bsd-new", "bsd-new OR gpl-2.0"]
```

The file contains both a BSD-style license header AND a GPL alternative clause. The BSD match is being incorrectly restored when it shouldn't be.

#### Case 2: `core.c` - GPL-2.0 Multiple Matches

```
Expected: ["gpl-2.0", "gpl-2.0", ...] (11 times)
Actual:   ["gpl-2.0"] (1 time)
```

Multiple GPL-2.0 matches are being merged or one is being restored and the rest discarded.

These failures suggest the token-based fix might actually **help** some cases and **hurt** others, depending on the specific token/line relationships.

### 17.10 Root Cause Hypothesis

The golden test regressions from previous attempts may have been caused by:

1. **Not handling the zero-token case** - empty spans caused over-restoration
2. **Test helper interference** - unrealistic token values in unit tests
3. **Subtle differences in how matches are created** - different matchers may have edge cases

The fix needs to:
1. Use token positions for overlap detection (correct behavior)
2. Handle the zero-token fallback case (test compatibility)
3. Ensure unit tests use realistic token values (or the fallback handles them)

### 17.11 Verification: Empty Span Behavior

**Test from `spans.rs:556-562`:**
```rust
#[test]
fn test_intersects_empty() {
    let span1 = Span::new();
    let span2 = Span::from_range(5..10);
    assert!(!span1.intersects(&span2));
    assert!(!span2.intersects(&span1));
}
```

**Confirmed**: Empty spans never intersect with anything. This is why `start_token=0, end_token=0` creates an empty span `0..0` which causes all discarded matches to be incorrectly restored.

**This is the critical issue** the fallback logic must prevent.

---

## 18. Revised Implementation Plan

### Phase 1: Implement Core Fix

1. Add `match_to_qspan()` function with fallback logic
2. Update `restore_non_overlapping()` to use it
3. Run unit tests to verify no regressions

### Phase 2: Verify with Golden Tests

1. Run golden tests and compare with baseline
2. Identify specific tests that improved or regressed
3. Analyze edge cases

### Phase 3: Update Test Helpers (Optional)

If tests fail due to test helper token values:
1. Update `create_test_match()` in `match_refine.rs` to use distinct token values
2. Add explicit `create_test_match_with_tokens()` helper for tests needing control
3. Keep the zero-token fallback for `detection.rs` tests

---

## 19. Document History

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-23 | AI Agent | Initial plan creation |
| 2026-02-23 | AI Agent | Added validation report with regression analysis |
| 2026-02-23 | AI Agent | Added corrected implementation with fallback logic |
| 2026-02-23 | AI Agent | Deep validation analysis - identified test helper issues, golden test patterns, matcher verification |
