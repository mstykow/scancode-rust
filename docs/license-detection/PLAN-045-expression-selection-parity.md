# PLAN-045: Expression Selection Parity for Overlapping Matches

**Status: ⚠️ IMPLEMENTATION ATTEMPTED - CAUSED REGRESSION**

## Implementation Attempt (2026-02-24)

**Result:** Implementation caused regression - CDDL tests showed BOTH expressions instead of one.

**What was tried:**

1. Added `qspan_equal()` helper function using HashSet comparison
2. Added `compare_match_quality()` function with coverage → score → hilen → len → matcher_order → rule_identifier
3. Updated `filter_contained_matches()` to use qspan equality instead of bounds check

**Regression observed:**

- `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt`: Now outputs BOTH expressions
- Tests regressed from baseline

**Root cause analysis:**

- The qspan equality check was finding matches equal when they shouldn't be
- CDDL 1.0 and CDDL 1.1 rules have DIFFERENT qspans (different token positions)
- The issue is NOT in `filter_contained_matches` - both matches survive because they have different qspans and different expressions
- The issue may be in detection-level deduplication

**Recommendation:** Investigate detection-level deduplication in `detection.rs` instead of match refinement.

---

## Summary

When multiple license rules match overlapping text with different license expressions (e.g., CDDL 1.0 vs CDDL 1.1), Python ScanCode selects a single "best" expression while the Rust implementation currently outputs both. This causes the CDDL glassfish golden test to fail with duplicate expressions.

**Test Case**: `datadriven/lic1/cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt`

| Implementation | Output |
|----------------|--------|
| Python (Expected) | `["(cddl-1.0 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0"]` |
| Rust (Actual) | `["(cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0", "(cddl-1.0 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0"]` |

The root cause: Rust creates two separate `LicenseDetection` objects from two different rules matching the same text region, while Python's match refinement pipeline filters out the inferior match during `filter_contained_matches()` or `filter_overlapping_matches()`.

---

## Python's Approach (Verified)

### 1. filter_contained_matches() - Lines 1075-1184

**Exact Python code at lines 1137-1155:**

```python
# equals matched spans
if current_match.qspan == next_match.qspan:
    if current_match.coverage() >= next_match.coverage():
        if trace:
            logger_debug(
                '    ---> ###filter_contained_matches: '
                'next EQUALS current, '
                'removed next with lower or equal coverage', matches[j])

        discarded_append(matches_pop(j))
        continue
    else:
        if trace:
            logger_debug(
                '    ---> ###filter_contained_matches: '
                'next EQUALS current, '
                'removed current with lower coverage', matches[i])
        discarded_append(matches_pop(i))
        i -= 1
        break
```

**Sorting at line 1099:**

```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
```

### 2. filter_overlapping_matches() - Lines 1187-1523

**Overlap thresholds at lines 1213-1216:**

```python
OVERLAP_SMALL = 0.10
OVERLAP_MEDIUM = 0.40
OVERLAP_LARGE = 0.70
OVERLAP_EXTRA_LARGE = 0.90
```

**Key filtering logic with licensing_contains() at lines 1374-1385:**

```python
if (current_match.licensing_contains(next_match)
    and current_match.len() >= next_match.len()
    and current_match.hilen() >= next_match.hilen()
):
    if trace:
        logger_debug(
            '      ---> ###filter_overlapping_matches: '
            'MEDIUM next included with next licensing contained, '
            'removed next', matches[j],)

    discarded_append(matches_pop(j))
    continue
```

**Similar patterns at lines:**

- 1404-1416: MEDIUM next includes current
- 1424-1435: MEDIUM current with licensing_contains
- 1437-1449: MEDIUM current with reverse licensing_contains
- 1451-1464: SMALL next surrounded with licensing_contains
- 1466-1480: SMALL current surrounded with licensing_contains

### 3. licensing_contains() Method - models.py:2065-2073

**Exact Python code:**

```python
def licensing_contains(self, other):
    """
    Return True if this rule licensing contains the other rule licensing.
    """
    if self.license_expression and other.license_expression:
        return self.licensing.contains(
            expression1=self.license_expression_object,
            expression2=other.license_expression_object,
        )
```

**Called from LicenseMatch at match.py:388-392:**

```python
def licensing_contains(self, other):
    """
    Return True if this match licensing contains the other match licensing.
    """
    return self.rule.licensing_contains(other.rule)
```

### 4. Refinement Pipeline - match.py:2691-2833

**Exact order verified:**

```python
def refine_matches(matches, query=None, min_score=0, ...):
    # Line 2719: First merge
    matches = merge_matches(matches)
    
    # Lines 2744-2769: Various filters
    matches, discarded = filter_matches_missing_required_phrases(matches)
    matches, discarded = filter_spurious_matches(matches)
    matches, discarded = filter_below_rule_minimum_coverage(matches)
    matches, discarded = filter_matches_to_spurious_single_token(matches, query)
    matches, discarded = filter_too_short_matches(matches)
    matches, discarded = filter_short_matches_scattered_on_too_many_lines(matches)
    matches, discarded = filter_invalid_matches_to_single_word_gibberish(matches)
    
    # Line 2773: Second merge
    matches = merge_matches(matches)
    
    # Line 2781: KEY - filter_contained_matches
    matches, discarded_contained = filter_contained_matches(matches)
    
    # Line 2790: KEY - filter_overlapping_matches
    matches, discarded_overlapping = filter_overlapping_matches(matches)
    
    # Lines 2793-2803: restore_non_overlapping for both
    if discarded_contained:
        to_keep, discarded_contained = restore_non_overlapping(matches, discarded_contained)
        matches.extend(to_keep)
    
    if discarded_overlapping:
        to_keep, discarded_overlapping = restore_non_overlapping(matches, discarded_overlapping)
        matches.extend(to_keep)
    
    # Line 2805: Second pass of filter_contained_matches
    matches, discarded_contained = filter_contained_matches(matches)
    
    # Lines 2809-2817: False positive filters
    if filter_false_positive:
        matches, discarded = filter_false_positive_matches(matches)
        matches, discarded = filter_false_positive_license_lists_matches(matches)
    
    # Line 2825: Final merge
    matches = merge_matches(matches)
    
    return matches, all_discarded
```

---

## Rust's Current Approach

### 1. filter_contained_matches() - match_refine.rs:326-380

**Current sorting:**

```rust
matches.sort_by(|a, b| {
    a.qstart()
        .cmp(&b.qstart())
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

**Equal span check (lines 353-362):**

```rust
if current.qstart() == next.qstart() && current.end_token == next.end_token {
    if current.match_coverage >= next.match_coverage {
        discarded.push(matches.remove(j));
        continue;
    } else {
        discarded.push(matches.remove(i));
        i = i.saturating_sub(1);
        break;
    }
}
```

### 2. filter_overlapping_matches() - match_refine.rs:513-716

**Current sorting with rule_identifier tiebreaker (line 524-531):**

```rust
matches.sort_by(|a, b| {
    a.qstart()
        .cmp(&b.qstart())
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        .then_with(|| a.rule_identifier.cmp(&b.rule_identifier))
});
```

**licensing_contains_match() helper at lines 506-511:**

```rust
fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return false;
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}
```

---

## Gap Analysis

### Issue 1: CDDL 1.0 vs CDDL 1.1 Are NOT Expression Subsumption

The expressions:

- `(cddl-1.0 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0`
- `(cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0`

**These do NOT subsume each other** via `licensing_contains()`:

- `cddl-1.0` != `cddl-1.1`
- Neither expression contains all keys of the other

This means `licensing_contains()` cannot help select between these two expressions.

### Issue 2: Different Rules Match Same Text

CDDL 1.0 and CDDL 1.1 rules match the **same or nearly identical text** with different expressions. This creates two matches with:

- Nearly identical `qspan` (query token positions)
- Different `license_expression`
- Potentially different `coverage` values

### Issue 3: Equal Span Check Uses Wrong Comparison

**Python compares:** `current_match.qspan == next_match.qspan`

**Rust compares:** `current.qstart() == next.qstart() && current.end_token == next.end_token`

**Problem:** Rust uses bounds only, while Python compares the full qspan (which is a Span object). If matches have qspan_positions set, Python compares the actual positions, not just start/end bounds.

### Issue 4: Matches May Have Slightly Different Spans

If the CDDL 1.0 and CDDL 1.1 rules match slightly different token ranges, the equal span check fails. Then the filtering relies on:

1. `qcontains()` - containment check
2. `licensing_contains_match()` - expression subsumption

But if spans are nearly equal but not identical, and expressions don't subsume, **both matches survive**.

### Issue 5: Detection-Level Deduplication Uses Expression Key

In `apply_detection_preferences()` (detection.rs:1089-1127):

```rust
let expr = detection.license_expression.clone().unwrap_or_else(String::new);
// ...
processed.insert(expr, (score, best_matcher_priority, detection));
```

Detections are keyed by `expr`. If CDDL 1.0 and CDDL 1.1 have **different expressions**, they are kept as separate detections!

---

## Root Cause

When two matches have **nearly identical qspans** (not exactly equal) with **different license expressions** that don't subsume each other, neither `filter_contained_matches` nor `filter_overlapping_matches` removes either match. Both survive to detection creation, where they create separate `LicenseDetection` objects with different expressions.

Python handles this case by:

1. Using exact `qspan` comparison (full position set, not just bounds)
2. When qspans are equal, keeping the one with higher coverage
3. If coverage is also equal, keeping the first one (stable sort)

---

## Step-by-Step Implementation Plan

### Step 1: Fix qspan Equality Check in filter_contained_matches

**File:** `src/license_detection/match_refine.rs`

**Current code (line 353):**

```rust
if current.qstart() == next.qstart() && current.end_token == next.end_token {
```

**Update to use full qspan comparison:**

```rust
if current.qspan() == next.qspan() {
```

This requires implementing `PartialEq` for the `qspan()` return type, or using a helper function:

```rust
fn qspan_equal(a: &LicenseMatch, b: &LicenseMatch) -> bool {
    let a_qspan = a.qspan();
    let b_qspan = b.qspan();
    a_qspan.len() == b_qspan.len() && a_qspan == b_qspan
}
```

### Step 2: Add Coverage + Score Tiebreaker for Equal Qspans

When qspans are equal but coverage differs, keep higher coverage. When coverage is also equal, use score as tiebreaker:

```rust
if qspan_equal(&current, &next) {
    // Compare coverage first
    match current.match_coverage.partial_cmp(&next.match_coverage) {
        Some(Ordering::Greater) | Some(Ordering::Equal) => {
            discarded.push(matches.remove(j));
            continue;
        }
        Some(Ordering::Less) => {
            discarded.push(matches.remove(i));
            i = i.saturating_sub(1);
            break;
        }
        None => {
            // NaN case - use score as fallback
            if current.score >= next.score {
                discarded.push(matches.remove(j));
                continue;
            } else {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }
        }
    }
}
```

### Step 3: Add Near-Equal Qspan Handling for filter_overlapping_matches

For matches with nearly identical spans (high overlap ratio), add special handling:

```rust
// After existing overlap checks, add near-equal handling
let overlap_ratio = overlap as f64 / current_len.min(next_len) as f64;
if overlap_ratio >= 0.95 {
    // Nearly identical spans - pick the better one
    let current_better = current.match_coverage > next.match_coverage
        || (current.match_coverage == next.match_coverage && current.score >= next.score);
    
    if current_better {
        discarded.push(matches.remove(j));
        continue;
    } else {
        discarded.push(matches.remove(i));
        i = i.saturating_sub(1);
        break;
    }
}
```

### Step 4: Add Rule Quality Comparison Function

**File:** `src/license_detection/match_refine.rs`

```rust
/// Compare two matches by quality metrics.
/// Returns Ordering::Greater if a is better than b.
fn compare_match_quality(a: &LicenseMatch, b: &LicenseMatch) -> Ordering {
    // 1. Higher coverage wins
    let cov_cmp = a.match_coverage.partial_cmp(&b.match_coverage)
        .unwrap_or(Ordering::Equal);
    if cov_cmp != Ordering::Equal {
        return cov_cmp;
    }
    
    // 2. Higher score wins
    let score_cmp = a.score.partial_cmp(&b.score)
        .unwrap_or(Ordering::Equal);
    if score_cmp != Ordering::Equal {
        return score_cmp;
    }
    
    // 3. Higher hilen (high-value tokens) wins
    let hilen_cmp = a.hilen.cmp(&b.hilen);
    if hilen_cmp != Ordering::Equal {
        return hilen_cmp;
    }
    
    // 4. Longer match wins
    let len_cmp = a.matched_length.cmp(&b.matched_length);
    if len_cmp != Ordering::Equal {
        return len_cmp;
    }
    
    // 5. Better matcher wins (lower order = better)
    a.matcher_order().cmp(&b.matcher_order())
}
```

### Step 5: Update filter_contained_matches to Use Quality Comparison

```rust
if qspan_equal(&current, &next) {
    match compare_match_quality(&current, &next) {
        Ordering::Greater | Ordering::Equal => {
            discarded.push(matches.remove(j));
            continue;
        }
        Ordering::Less => {
            discarded.push(matches.remove(i));
            i = i.saturating_sub(1);
            break;
        }
    }
}
```

### Step 6: Update filter_overlapping_matches for Near-Equal Cases

Add handling for extra-large overlap cases where neither expression subsumes:

```rust
// After existing extra_large_next/current checks, add:
if extra_large_next && extra_large_current {
    // Both have extreme overlap - use quality comparison
    match compare_match_quality(&matches[i], &matches[j]) {
        Ordering::Greater | Ordering::Equal => {
            discarded.push(matches.remove(j));
            continue;
        }
        Ordering::Less => {
            discarded.push(matches.remove(i));
            i = i.saturating_sub(1);
            break;
        }
    }
}
```

### Step 7: Add Deterministic Tiebreaker

For complete determinism when all metrics are equal, add rule_identifier as final tiebreaker in `compare_match_quality`:

```rust
// 6. Deterministic tiebreaker: rule_identifier
a.rule_identifier.cmp(&b.rule_identifier)
```

### Step 8: Test with CDDL Glassfish File

```bash
cargo test test_license_golden_cddl -- --nocapture
```

Verify only one expression is returned.

---

## Expected Impact on Golden Tests

### Tests That Should Pass After Fix

1. `datadriven/lic1/cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_*.txt`
   - Should return single expression (CDDL 1.0 or CDDL 1.1 based on quality)

### Tests That May Need Investigation

If any tests currently expect multiple similar expressions:

1. Verify Python behavior for those cases
2. Update golden files to match Python output
3. Document any edge cases found

### Tests That Should Not Change

- Tests with non-overlapping matches
- Tests with clearly different expressions (e.g., MIT vs Apache)
- Tests where expressions properly subsume each other

---

## Key Files to Modify

| File | Purpose |
|------|---------|
| `src/license_detection/match_refine.rs` | Fix qspan equality, add quality comparison |
| `src/license_detection/models.rs` | Ensure `qspan()` returns comparable type |

---

## Verification Checklist

- [ ] `qspan()` returns `Vec<usize>` that can be compared for equality
- [ ] `filter_contained_matches` uses full qspan comparison
- [ ] Quality comparison uses coverage → score → hilen → len → matcher_order → rule_identifier
- [ ] Near-equal span handling in `filter_overlapping_matches`
- [ ] CDDL glassfish golden tests pass
- [ ] No regressions in other golden tests

---

## Appendix: Python Code References

| Function | File | Lines | Purpose |
|----------|------|-------|---------|
| `merge_matches()` | `match.py` | 869-1068 | Merge matches for same rule |
| `filter_contained_matches()` | `match.py` | 1075-1184 | Remove contained matches |
| `filter_overlapping_matches()` | `match.py` | 1187-1523 | Remove overlapping matches |
| `restore_non_overlapping()` | `match.py` | 1526-1548 | Restore non-overlapping discarded |
| `refine_matches()` | `match.py` | 2691-2833 | Main refinement pipeline |
| `licensing_contains()` | `models.py` | 2065-2073 | Expression subsumption check |
| `LicenseMatch.licensing_contains()` | `match.py` | 388-392 | Match-level subsumption |

---

## Appendix: Rust Code References

| Function | File | Lines | Purpose |
|----------|------|-------|---------|
| `merge_overlapping_matches()` | `match_refine.rs` | 159-302 | Merge matches for same rule |
| `filter_contained_matches()` | `match_refine.rs` | 326-380 | Remove contained matches |
| `filter_overlapping_matches()` | `match_refine.rs` | 513-716 | Remove overlapping matches |
| `restore_non_overlapping()` | `match_refine.rs` | 722-745 | Restore non-overlapping discarded |
| `refine_matches()` | `match_refine.rs` | 1434-1488 | Main refinement pipeline |
| `licensing_contains()` | `expression.rs` | 444-506 | Expression subsumption check |
| `licensing_contains_match()` | `match_refine.rs` | 506-511 | Match-level subsumption |
| `qcontains()` | `models.rs` | 499-528 | Span containment check |
| `qspan()` | `models.rs` | 557-565 | Get qspan positions |
