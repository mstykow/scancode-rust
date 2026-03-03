# Plan: Fix Extra/Spurious License Detections

**Status:** Needs Revision  
**Created:** 2026-03-03  
**Updated:** 2026-03-03  
**Related:** `0022-phase6-extra-detections-plan.md`

## Executive Summary

Rust detects licenses that Python doesn't detect, or detects more instances than expected. After verification, the actual root causes are:

1. **`is_candidate_false_positive` uses wrong length field** - Uses `matched_length` instead of `len()` (qspan token count)
2. **Other suspected issues were incorrect** - See verification findings below

---

## Verification Findings (2026-03-03)

### Issue 1: `filter_contained_matches` - INCORRECT ANALYSIS

**Plan Claim:** Missing expression-based containment check.

**Actual Behavior:**
- **Python's `filter_contained_matches` (match.py:1075-1184) does NOT use `licensing_contains()`** - it only uses `qcontains()` for token position containment
- **Rust's implementation matches Python exactly** - both use `qcontains()` only
- Expression containment (`licensing_contains`) is used in `filter_overlapping_matches`, which **Rust correctly implements** in `handle_overlaps.rs:255-321`

**Conclusion:** This is NOT a root cause. The implementation is correct.

### Issue 2: `filter_false_positive_matches` - CORRECTLY IMPLEMENTED

**Plan Claim:** May differ from Python.

**Actual Behavior:**
- Python: `match.rule.is_false_positive` check (match.py:2142)
- Rust: `index.false_positive_rids.contains(&rid)` check (filter_low_quality.rs:396)
- Index builder correctly populates `false_positive_rids` when `rule.is_false_positive` is true (builder/mod.rs:381-382)

**Conclusion:** Correctly implemented. NOT a root cause.

### Issue 3: Detection-Level `is_false_positive` - INCORRECT, ALREADY IMPLEMENTED

**Plan Claim:** "Completely missing in Rust!"

**Actual Implementation:**
- Function exists at `src/license_detection/detection/analysis.rs:71-143`
- Called in `classify_detection()` at line 321
- Constants match Python:
  - `FALSE_POSITIVE_START_LINE_THRESHOLD = 1000` (Python: 1000)
  - `FALSE_POSITIVE_RULE_LENGTH_THRESHOLD = 3` (Python: 3)
- Logic mirrors Python's `is_false_positive()` (detection.py:1162-1239)

**Conclusion:** Already implemented. NOT a root cause.

### Issue 4: Expression Containment in Overlap Filtering - CORRECTLY IMPLEMENTED

**Plan Claim:** Thresholds may not match.

**Actual Behavior:**
- Rust correctly implements `licensing_contains_match()` in:
  - Medium overlap: lines 255-260, 295-310
  - Small overlap with surround: lines 313-321
- Thresholds match Python: `OVERLAP_SMALL = 0.10`, `OVERLAP_MEDIUM = 0.40`

**Conclusion:** Correctly implemented. NOT a root cause.

### Issue 5: `is_candidate_false_positive` Length Check - CONFIRMED BUG

**Location:** `src/license_detection/match_refine/false_positive.rs:21`

**Python (match.py:2674):**
```python
and match.len() <= max_length  # match.len() returns len(self.qspan)
```

**Rust (false_positive.rs:21):**
```rust
let is_short = m.matched_length <= MAX_CANDIDATE_LENGTH;  // WRONG FIELD
```

**The Bug:**
- Python's `match.len()` returns `len(self.qspan)` - the number of matched query tokens
- Rust's `matched_length` is NOT the same as qspan length
- Rust has `m.len()` method that correctly returns qspan length (license_match.rs:238)

**This is a real bug that could cause false positives to not be filtered.**

---

## Actual Root Cause Analysis

### Root Cause 1: `is_candidate_false_positive` Uses Wrong Field

**File:** `src/license_detection/match_refine/false_positive.rs:21`

**Problem:** Uses `matched_length` instead of `len()` (qspan token count).

**Impact:** License list false positive detection may not filter candidates correctly.

**Fix:**
```rust
// Current (WRONG):
let is_short = m.matched_length <= MAX_CANDIDATE_LENGTH;

// Correct:
let is_short = m.len() <= MAX_CANDIDATE_LENGTH;
```

### Root Cause 2: Need to Investigate Specific Failing Cases

The above bug explains some cases, but not all. For cases like `Artistic-2.0.t1` detecting `warranty-disclaimer`, we need:

1. **Trace the specific detection path** - Why is warranty-disclaimer not being filtered?
2. **Check if matches are overlapping or separate** - Containment filtering only applies to overlapping matches
3. **Check expression containment relationships** - Does `artistic-2.0` expression contain `warranty-disclaimer`?

---

## Implementation Steps

### Step 1: Fix `is_candidate_false_positive` Length Check (High Priority)

**File:** `src/license_detection/match_refine/false_positive.rs`

**Line 21:** Change `m.matched_length` to `m.len()`

**Testing:**
```bash
cargo test --lib filter_false_positive_license_lists_matches
cargo test --lib is_candidate_false_positive
```

### Step 2: Debug Specific Failing Cases (High Priority)

For each failing golden test:

1. **Enable debug tracing** to see the match refinement pipeline
2. **Check each filter stage** to see where unwanted matches survive
3. **Compare with Python output** at each stage

**Debug approach:**
```bash
# Run specific test with tracing
RUST_LOG=debug cargo test --release --lib test_artistic_2_0_t1 -- --nocapture
```

### Step 3: Add Missing Test Coverage

Add tests for:
1. `is_candidate_false_positive` with various token lengths
2. Expression containment edge cases
3. Specific golden test cases as unit tests

---

## Test Cases to Verify Fix

### License List False Positive Tests

| Test File | Expected Behavior | Root Cause |
|-----------|-------------------|------------|
| Files with many license refs | Filtered by `filter_false_positive_license_lists_matches` | `is_candidate_false_positive` length bug |

### Investigation Needed

| Test File | Current Behavior | Investigation Required |
|-----------|------------------|------------------------|
| `Artistic-2.0.t1` | Has warranty-disclaimer | Trace match pipeline |
| `OpenSSL.t1` | Separate openssl/ssleay | Check if overlapping |
| `gpl-2.0_9.txt` | Has gpl-1.0-plus | Check containment/expression |

---

## Files to Modify

| File | Changes | Priority |
|------|---------|----------|
| `src/license_detection/match_refine/false_positive.rs` | Fix `is_candidate_false_positive` length check | High |

---

## Removed Items (Not Actually Issues)

1. ~~Add Expression Containment to `filter_contained_matches`~~ - Not needed, Python doesn't do this either
2. ~~Add Detection-Level `is_false_positive` Check~~ - Already implemented
3. ~~Verify `false_positive_rids` Construction~~ - Correctly implemented

---

## Verification Commands

```bash
# Run false positive filter tests
cargo test --lib filter_false_positive_license_lists_matches

# Run golden tests to see overall improvement
cargo test --release -q --lib license_detection::golden_test 2>&1 | grep -c "mismatch"

# Debug specific test case
RUST_LOG=debug cargo test --lib <test_name> -- --nocapture
```

---

## Success Criteria

1. `is_candidate_false_positive` uses correct length field
2. All existing tests continue to pass
3. Golden test failures reduced (measure after fix)
4. Specific failing cases analyzed and root causes identified
