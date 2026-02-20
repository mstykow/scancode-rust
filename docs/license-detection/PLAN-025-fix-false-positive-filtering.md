# PLAN-025: Fix False Positive/Containment Filtering Logic

**Date**: 2026-02-20  
**Status**: Implementation Plan (Validated)  
**Priority**: 2 (Second highest impact)  
**Estimated Impact**: ~40 golden test failures  

---

## Executive Summary

Rust detects additional licenses not in Python output due to differences in false positive and containment filtering logic. This plan addresses Pattern B from PLAN-023-failure-analysis-summary.md.

### Symptoms

- `warranty-disclaimer` appearing unexpectedly
- `unknown-license-reference` over-detection
- `proprietary-license` extra matches
- Exception components alongside combined expressions (e.g., `gpl-2.0 WITH exception` AND `gpl-2.0` separately)

### Root Cause Analysis (Verified)

After detailed comparison with Python reference implementation:

1. **`filter_contained_matches()` missing early break** - Rust doesn't exit the inner loop when no more overlaps are possible, causing incorrect filtering of matches that shouldn't be compared.

2. **`filter_contained_matches()` missing equals case** - When two matches have identical qspans, Python keeps the higher coverage one; Rust relies only on sort order.

3. **`licensing_contains_match()` incorrect fallback** - Returns `true` based on a 2x length threshold for empty expressions; Python always returns `false`.

4. **Expression containment is correctly implemented** - Rust's `filter_overlapping_matches()` already uses `licensing_contains_match()` for medium/small overlaps at lines 481-557, matching Python's logic.

---

## Python Behavior Analysis (Verified)

### `filter_contained_matches()` (Python match.py:1075-1184)

**Sorting**: `(m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)`

**Loop structure**:
```python
i = 0
while i < len(matches) - 1:
    j = i + 1
    while j < len(matches):
        current_match = matches[i]
        next_match = matches[j]
        
        # 1. EARLY BREAK: No overlap possible
        if next_match.qend > current_match.qend:
            j += 1
            break  # Exit inner loop
        
        # 2. EQUALS CASE: Same qspan
        if current_match.qspan == next_match.qspan:
            if current_match.coverage() >= next_match.coverage():
                discarded_append(matches_pop(j))
                continue
            else:
                discarded_append(matches_pop(i))
                i -= 1
                break
        
        # 3. qcontains() checks (both directions)
        if current_match.qcontains(next_match):
            discarded_append(matches_pop(j))
            continue
        if next_match.qcontains(current_match):
            discarded_append(matches_pop(i))
            i -= 1
            break
        
        j += 1
    i += 1
```

**Key insight**: Python does NOT use `licensing_contains()` here - only position-based containment.

### `filter_overlapping_matches()` (Python match.py:1187-1523)

**False positive skip** (lines 1276-1286) - Rust ✓ has this at match_refine.rs:420-425.

**Licensing containment** - Used in medium/small overlap cases:
- Lines 1374-1385: `medium_next` with `current_match.licensing_contains(next_match)`
- Lines 1404-1416: `medium_next` with `next_match.licensing_contains(current_match)`
- Lines 1424-1435: `medium_current` with `current_match.licensing_contains(next_match)`
- Lines 1437-1449: `medium_current` with `next_match.licensing_contains(current_match)`
- Lines 1451-1464: `small_next` with surround + licensing_contains
- Lines 1466-1480: `small_current` with surround + licensing_contains

**Extra large/large cases** do NOT use `licensing_contains()` - they filter based purely on length/hilen.

### `licensing_contains()` (Python models.py:2065-2073)

```python
def licensing_contains(self, other):
    if self.license_expression and other.license_expression:
        return self.licensing.contains(
            expression1=self.license_expression_object,
            expression2=other.license_expression_object,
        )
```

**Key**: Returns `None` (falsy) if either expression is empty - no fallback logic.

---

## Rust Current Implementation (Verified)

### `filter_contained_matches()` (match_refine.rs:249-283)

**Current state**:
- ✓ Correct sorting: `(start_token, hilen desc, matched_length desc, rule_identifier)`
- ✓ Correct `qcontains()` logic
- ✗ Missing early break when `next.end_token > current.end_token`
- ✗ Missing equals case for identical qspans

### `filter_overlapping_matches()` (match_refine.rs:389-592)

**Current state**:
- ✓ False positive skip at lines 420-425
- ✓ Licensing containment for medium overlaps at lines 481-497
- ✓ Licensing containment for medium_current at lines 519-536
- ✓ Licensing containment for small + surround at lines 538-557
- ✓ Sandwich detection at lines 559-584

### `licensing_contains_match()` (match_refine.rs:382-387)

```rust
fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return current.matched_length >= other.matched_length * 2;  // WRONG
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}
```

**Issue**: 2x length threshold fallback doesn't match Python's behavior.

### `licensing_contains()` (expression.rs:444-506)

```rust
pub fn licensing_contains(container: &str, contained: &str) -> bool {
    let container = container.trim();
    let contained = contained.trim();
    if container.is_empty() || contained.is_empty() {
        return false;  // Correct!
    }
    // ... WITH decomposition handled at lines 488-494
}
```

**Verified**: WITH expression handling is correct - see test `test_with_decomposition()` at lines 1697-1711.

---

## Implementation Plan

### Step 1: Fix `filter_contained_matches()` Early Break

**File**: `src/license_detection/match_refine.rs:249-283`

**Issue**: The plan previously suggested `if current.end_token < next.start_token` but Python's actual condition is `if next_match.qend > current_match.qend`.

**Correct fix** - Add at start of inner loop (after retrieving current and next):

```rust
// Python: if next_match.qend > current_match.qend: break
// This means next ends AFTER current, so no containment possible
if next.end_token > current.end_token {
    // No more overlaps possible with current match
    // (sorted by start, so all remaining j's will also end after current)
    break;
}
```

**Why this matters**: Without this break, Rust incorrectly compares matches that shouldn't be compared, potentially filtering matches that Python keeps.

### Step 2: Fix `filter_contained_matches()` Equals Case

**File**: `src/license_detection/match_refine.rs:249-283`

**Add after early break check**:

```rust
// Python: if current_match.qspan == next_match.qspan
if current.start_token == next.start_token && current.end_token == next.end_token {
    if current.match_coverage >= next.match_coverage {
        discarded.push(next.clone());
        // Continue checking other j values
        continue;
    } else {
        discarded.push(current.clone());
        // Need to adjust i and break (current was removed)
        // This is tricky with the current implementation...
    }
}
```

**Note**: The current implementation uses a single-pass approach with `kept` vector. The equals case requires modifying the loop structure to match Python's in-place removal pattern.

### Step 3: Fix `licensing_contains_match()` Fallback

**File**: `src/license_detection/match_refine.rs:382-387`

**Change**:

```rust
fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return false;  // Match Python: no containment for empty expressions
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}
```

**Impact**: This prevents incorrect expression containment matches when one or both expressions are empty.

### Step 4: Refactor `filter_contained_matches()` to Match Python Structure

**File**: `src/license_detection/match_refine.rs:249-283`

The current Rust implementation uses a different algorithm than Python. Consider rewriting to match Python's in-place removal pattern:

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.len() < 2 {
        return (matches.to_vec(), Vec::new());
    }

    let mut matches: Vec<LicenseMatch> = matches.to_vec();
    let mut discarded = Vec::new();

    // Sort: start, hilen desc, len desc, matcher_order
    matches.sort_by(|a, b| {
        a.start_token
            .cmp(&b.start_token)
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    let mut i = 0;
    while i < matches.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < matches.len() {
            let current = matches[i].clone();
            let next = matches[j].clone();

            // Early break: no overlap possible
            if next.end_token > current.end_token {
                break;
            }

            // Equals case: same qspan
            if current.start_token == next.start_token && current.end_token == next.end_token {
                if current.match_coverage >= next.match_coverage {
                    discarded.push(matches.remove(j));
                    continue;
                } else {
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
            }

            // qcontains checks
            if current.qcontains(&next) {
                discarded.push(matches.remove(j));
                continue;
            }
            if next.qcontains(&current) {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            j += 1;
        }
        i += 1;
    }

    (matches, discarded)
}
```

---

## Testing Strategy

### Unit Tests

**Add to `match_refine.rs` tests**:

```rust
#[test]
fn test_filter_contained_matches_early_break() {
    // Match A: tokens 0-10, match B: tokens 5-15, match C: tokens 20-30
    // When comparing A with B (overlap), should break before comparing A with C
    // because C ends after A (C.end > A.end)
}

#[test]
fn test_filter_contained_matches_equals_case_higher_coverage() {
    // Two matches with same qspan, different coverage
    // Should keep higher coverage match
}

#[test]
fn test_licensing_contains_match_empty_expressions() {
    let m1 = create_test_match_with_expression("", 0, 10);
    let m2 = create_test_match_with_expression("mit", 0, 10);
    assert!(!licensing_contains_match(&m1, &m2));
    assert!(!licensing_contains_match(&m2, &m1));
}
```

**Existing tests that verify WITH handling** (expression.rs:1697-1711):
- `test_with_decomposition()` - Already verifies `licensing_contains("gpl-2.0 WITH exception", "gpl-2.0")` returns true

### Golden Tests

Run specific golden tests to verify fixes:

```bash
# Run specific failing tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic3 -- --test-threads=1

# Debug specific expression containment
cargo test --release -q --lib debug_lgpl_exception -- --nocapture

# Run all golden tests
cargo test --release -q --lib license_detection::golden_test
```

### Test Cases to Add

| Test | File | Purpose |
|------|------|---------|
| `test_filter_contained_early_break` | match_refine.rs | Verify early break prevents incorrect comparisons |
| `test_filter_contained_equals` | match_refine.rs | Verify equals case keeps higher coverage |
| `test_licensing_contains_empty` | match_refine.rs | Verify empty expressions return false |

---

## Implementation Order

### Phase 1: Core Fixes (1-2 hours)

1. **Fix `licensing_contains_match()` fallback** (Step 3)
   - Simple one-line change
   - High impact on expression containment
   - Test: Unit tests + golden tests

2. **Add early break to `filter_contained_matches()`** (Step 1)
   - Match Python's loop optimization
   - Test: Unit test for early break behavior

3. **Add equals case to `filter_contained_matches()`** (Step 2)
   - Handle identical qspans correctly
   - Test: Unit test for equals case

### Phase 2: Verification (1 hour)

4. **Run full golden test suite**
   - Verify improvements
   - Document remaining failures
   - Compare before/after counts

---

## Expected Outcomes

### Tests Fixed (Estimated)

| Suite | Current Failures | Expected After Fix |
|-------|------------------|-------------------|
| lic1 | 67 | ~58 (9 fewer) |
| lic2 | 78 | ~70 (8 fewer) |
| lic3 | 42 | ~36 (6 fewer) |
| lic4 | 65 | ~58 (7 fewer) |
| **Total** | **252** | **~222 (30 fewer)** |

### Why Lower Estimates Than Original Plan

The original plan proposed adding a new `filter_contained_license_expressions()` function. This is **NOT needed** because:

1. Rust already has `licensing_contains_match()` in `filter_overlapping_matches()` for medium/small overlaps
2. Python handles expression containment entirely within `filter_overlapping_matches()`, not as a separate pass
3. The actual fixes needed are the early break, equals case, and empty expression fallback

---

## Code Changes Summary

| File | Lines | Function | Change |
|------|-------|----------|--------|
| `match_refine.rs` | 382-387 | `licensing_contains_match()` | Return `false` for empty expressions |
| `match_refine.rs` | 249-283 | `filter_contained_matches()` | Add early break + equals case |
| `match_refine.rs` | tests | New tests | Add early break, equals, empty tests |

---

## Python Reference Files

| File | Lines | Purpose |
|------|-------|---------|
| `match.py` | 1075-1184 | `filter_contained_matches()` |
| `match.py` | 1187-1523 | `filter_overlapping_matches()` |
| `match.py` | 388-392 | `licensing_contains()` on match |
| `models.py` | 2065-2073 | `licensing_contains()` on rule |

---

## Validation Summary

### What Was Correct in Original Plan

1. ✓ Root cause identification (filtering differences)
2. ✓ Python behavior analysis for `filter_contained_matches()`
3. ✓ False positive skip already implemented in Rust
4. ✓ `licensing_contains()` already handles WITH expressions
5. ✓ Sandwich detection already implemented in Rust

### What Was Missing or Inaccurate

1. **Early break condition was wrong**: Plan said `current.end_token < next.start_token` but Python uses `next.qend > current.qend`. Fixed.

2. **Unnecessary Step 2 removed**: The proposed `filter_contained_license_expressions()` function is NOT what Python does. Python handles expression containment within `filter_overlapping_matches()` only for medium/small overlaps. Rust already has this.

3. **Redundant Step 5 removed**: Tests for WITH expression handling already exist at `expression.rs:1697-1711`.

4. **Added Step 4**: Recommended refactoring `filter_contained_matches()` to match Python's in-place removal pattern for correctness.

5. **Corrected estimates**: Lower estimates because fewer changes needed than originally thought.

### Key Insight

The main issues are:
- Early break prevents incorrect comparisons
- Equals case ensures correct match selection
- Empty expression fallback was causing false positives

Expression containment is already correctly implemented in `filter_overlapping_matches()`. The fixes above should resolve most of the false positive issues.
