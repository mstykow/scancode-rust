# PLAN-025: Fix False Positive/Containment Filtering Logic

**Date**: 2026-02-20  
**Status**: Partially Implemented  
**Priority**: 2 (Second highest impact)  
**Estimated Impact**: ~30-40 golden test failures  

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

4. **Expression containment is correctly implemented** - Rust's `filter_overlapping_matches()` already uses `licensing_contains_match()` for medium/small overlaps at lines 551-627, matching Python's logic.

---

## Current Implementation Status

### ✅ ALREADY CORRECT

| Component | File | Lines | Status |
|-----------|------|-------|--------|
| `licensing_contains()` empty handling | expression.rs | 444-449 | Returns `false` for empty expressions |
| `filter_overlapping_matches()` early break | match_refine.rs | 486-488 | Breaks when `next_start >= current_end` |
| `filter_overlapping_matches()` licensing containment | match_refine.rs | 551-627 | Correctly uses `licensing_contains_match()` for medium/small |
| `filter_overlapping_matches()` false positive skip | match_refine.rs | 490-495 | Correctly skips when both are FP |
| `qcontains()` method | models.rs | 457-472 | Correctly checks token containment |

### ❌ STILL NEEDS FIXES

| Component | File | Lines | Issue |
|-----------|------|-------|-------|
| `licensing_contains_match()` | match_refine.rs | 452-457 | Wrong fallback for empty expressions |
| `filter_contained_matches()` | match_refine.rs | 319-353 | Missing early break + equals case |

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

## Rust Current Implementation

### `filter_contained_matches()` (match_refine.rs:319-353)

**Current state**:

- ✓ Correct sorting: `(start_token, hilen desc, matched_length desc, rule_identifier)`
- ✓ Correct `qcontains()` logic
- ✗ Missing early break when `next.end_token > current.end_token`
- ✗ Missing equals case for identical qspans
- ✗ Uses different algorithm (single-pass `kept` vector vs Python's in-place removal)

**Current code**:

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // ... sorting ...
    
    let mut kept = Vec::new();
    let mut discarded = Vec::new();

    for current in sorted {
        let is_contained = kept
            .iter()
            .any(|kept_match: &&LicenseMatch| kept_match.qcontains(current));

        if !is_contained {
            kept.push(current);
        } else {
            discarded.push(current.clone());
        }
    }

    (kept.into_iter().cloned().collect(), discarded)
}
```

**Issue**: The single-pass approach cannot implement Python's behavior because:

1. Python's early break requires nested while loops with sorted matches
2. Python's equals case can remove the current match (requires index manipulation)
3. The current Rust approach only checks if `kept` contains `current`, not bidirectional containment

### `licensing_contains_match()` (match_refine.rs:452-457)

**Current code**:

```rust
fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return current.matched_length >= other.matched_length * 2;  // WRONG
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}
```

**Issue**: The 2x length threshold doesn't match Python's behavior. Python returns `false` (via `None`) when either expression is empty.

### `licensing_contains()` (expression.rs:444-506) - CORRECT

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

---

## Implementation Plan

### Step 1: Fix `licensing_contains_match()` Fallback (HIGH PRIORITY)

**File**: `src/license_detection/match_refine.rs:452-457`

**Change**:

```rust
fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return false;  // Match Python: no containment for empty expressions
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}
```

**Impact**: This prevents incorrect expression containment matches when one or both expressions are empty. This is called from `filter_overlapping_matches()` for medium/small overlaps.

### Step 2: Refactor `filter_contained_matches()` to Match Python Structure (HIGH PRIORITY)

**File**: `src/license_detection/match_refine.rs:319-353`

Rewrite to match Python's in-place removal pattern:

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.len() < 2 {
        return (matches.to_vec(), Vec::new());
    }

    let mut matches: Vec<LicenseMatch> = matches.to_vec();
    let mut discarded = Vec::new();

    // Sort: start, hilen desc, len desc, matcher_order (use rule_identifier as proxy)
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

            // Early break: next ends AFTER current (sorted by start, so no more overlaps possible)
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

**Why this matters**: Without the early break and equals case, Rust incorrectly compares matches that shouldn't be compared, potentially filtering matches that Python keeps.

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

---

## Implementation Order

### Phase 1: Core Fixes (1-2 hours)

1. **Fix `licensing_contains_match()` fallback** (Step 1)
   - Simple one-line change
   - High impact on expression containment
   - Test: Unit tests + golden tests

2. **Refactor `filter_contained_matches()`** (Step 2)
   - Add early break + equals case
   - Match Python's in-place removal pattern
   - Test: Unit tests + golden tests

### Phase 2: Verification (1 hour)

1. **Run full golden test suite**
   - Verify improvements
   - Document remaining failures
   - Compare before/after counts

---

## Expected Outcomes

### Tests Fixed (Estimated)

| Suite | Current Failures | Expected After Fix |
|-------|------------------|-------------------|
| lic1 | ~67 | ~58 (9 fewer) |
| lic2 | ~78 | ~70 (8 fewer) |
| lic3 | ~42 | ~36 (6 fewer) |
| lic4 | ~65 | ~58 (7 fewer) |
| **Total** | **~252** | **~222 (30 fewer)** |

### Why Lower Estimates Than Original Plan

The original plan proposed adding a new `filter_contained_license_expressions()` function. This is **NOT needed** because:

1. Rust already has `licensing_contains_match()` in `filter_overlapping_matches()` for medium/small overlaps
2. Python handles expression containment entirely within `filter_overlapping_matches()`, not as a separate pass
3. The actual fixes needed are the early break, equals case, and empty expression fallback

---

## Code Changes Summary

| File | Lines | Function | Change | Status |
|------|-------|----------|--------|--------|
| `match_refine.rs` | 452-457 | `licensing_contains_match()` | Return `false` for empty expressions | TODO |
| `match_refine.rs` | 319-353 | `filter_contained_matches()` | Refactor with early break + equals case | TODO |
| `match_refine.rs` | tests | New tests | Add early break, equals, empty tests | TODO |

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

### What Was Already Implemented (Since Plan Creation)

1. ✓ `filter_overlapping_matches()` early break at lines 486-488
2. ✓ `licensing_contains()` returns `false` for empty expressions at lines 444-449
3. ✓ `qcontains()` correctly checks token positions at models.rs:457-472

### What Still Needs Implementation

1. **`licensing_contains_match()` fallback fix** - Return `false` instead of 2x length check
2. **`filter_contained_matches()` refactor** - Add early break and equals case

### Key Insight

The main issues are:

- Early break prevents incorrect comparisons (in `filter_contained_matches`)
- Equals case ensures correct match selection (in `filter_contained_matches`)
- Empty expression fallback was causing false positives (in `licensing_contains_match`)

Expression containment is already correctly implemented in `filter_overlapping_matches()`. The fixes above should resolve most of the false positive issues.
