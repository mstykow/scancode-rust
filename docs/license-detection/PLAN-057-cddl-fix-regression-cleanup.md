# PLAN-057: CDDL Fix Regression Cleanup

## Status: VERIFIED - READY FOR IMPLEMENTATION

## Problem Statement

The CDDL investigation fixes improved lic1 tests (+2 passed) but caused regressions in external (-38 passed) and lic2 (-1 passed) tests.

### Golden Test Changes

| Test Set | Before | After | Change |
|----------|--------|-------|--------|
| lic1 | 240 passed, 51 failed | 242 passed, 49 failed | **+2** |
| lic2 | 802 passed, 51 failed | 801 passed, 52 failed | **-1** |
| external | 2169 passed, 398 failed | 2131 passed, 436 failed | **-38** |

---

## Investigation Results

### Regression Pattern Analysis

Analyzed 50+ failed external tests and identified TWO distinct regression patterns:

#### Pattern 1: Over-Merging (License Expressions Collapsed)

**Expected**: 2 identical expressions → **Actual**: 1 expression

Examples:
- `bsd-2c.txt`: Expected `["bsd-2-clause-views", "bsd-2-clause-views"]` → Actual `["bsd-2-clause-views"]`
- `bsd-4c.txt`: Expected `["bsd-original", "bsd-original"]` → Actual `["bsd-original"]`
- `AAL.txt`: Expected `["attribution", "attribution"]` → Actual `["attribution"]`
- `AFL-2.1.js`: Expected `["afl-2.1 OR bsd-new", "afl-2.1 OR bsd-new"]` → Actual `["afl-2.1 OR bsd-new"]`
- `AMPAS.c`: Expected `["ampas", "ampas"]` → Actual `["ampas"]`
- `apache-2.0_and_apache-2.0.txt`: Expected `["apache-2.0", "apache-2.0"]` → Actual `["apache-2.0"]`

**Impact**: ~70% of regressions (files with duplicate license instances being merged)

#### Pattern 2: Under-Merging (Extra Spurious Detections)

**Expected**: 1 expression → **Actual**: 2+ expressions (often with `unknown-license-reference`)

Examples:
- `isc.txt`: Expected `["isc"]` → Actual `["isc", "isc"]`
- `LGPL-3.0.hxx`: Expected `["lgpl-3.0"]` → Actual `["lgpl-3.0", "lgpl-3.0"]`
- `HPND.c`: Expected `["historical", "historical"]` → Actual `["historical", "historical", "historical"]`
- `Autoconf-exception.m4`: Expected 5 expressions → Actual 4 expressions (one removed)

**Impact**: ~30% of regressions (files getting extra spurious detections)

---

## Root Cause Analysis

### Change 1: `qoverlap()` Set-Based Logic (models.rs:547-584)

**What Changed**: Added set-based overlap computation for matches with different `qspan_positions` modes.

```rust
// NEW: Mixed-mode overlap calculation
if let (Some(self_positions), None) = (&self.qspan_positions, &other.qspan_positions) {
    let self_set: HashSet<usize> = self_positions.iter().copied().collect();
    return (other.start_token..other.end_token)
        .filter(|p| self_set.contains(p))
        .count();
}
```

**Problem**: When match A has `qspan_positions: Some([0, 5, 10])` (sparse) and match B has `qspan_positions: None` with range `[0..20]`, the intersection may compute incorrectly because:
- Sparse positions may not overlap with the contiguous range even though matches physically overlap
- This causes `qoverlap()` to return 0 when it should return a positive value

### Change 2: Surround Merge `qoverlap > 0` Check (match_refine.rs:251-273)

**What Changed**: Added requirement that `qoverlap > 0` before merging surrounded matches.

**Before (HEAD~2)**:
```rust
if current.surround(&next) {
    let combined = combine_matches(&current, &next);
    if combined.qspan().len() == combined.ispan().len() {
        rule_matches[i] = combined;
        rule_matches.remove(j);
        continue;
    }
}
```

**After (HEAD)**:
```rust
if current.surround(&next) {
    let qoverlap = current.qoverlap(&next);  // NEW: Uses set-based overlap
    if qoverlap > 0 {                         // NEW: Blocks merge if 0
        let combined = combine_matches(&current, &next);
        if combined.qspan().len() == combined.ispan().len() {
            rule_matches[i] = combined;
            rule_matches.remove(j);
            continue;
        }
    }
}
```

**Why This Causes Over-Merging**:

When `surround()` returns true but `qoverlap()` returns 0 (due to mixed-mode calculation), the merge is skipped. The match then falls through to later logic that may merge it incorrectly or not at all.

**Critical Insight**: The original Python `merge_matches()` does NOT have a `qoverlap > 0` check for surround merges. It only checks:
1. `surround()` - one match completely surrounds the other
2. `combined.qspan().len() == combined.ispan().len()` - the combined match is valid

---

## Representative Regression Cases

### Case 1: `apache-2.0_and_apache-2.0.txt` (Over-Merge)

**File Content**: Maven pom.xml with Apache-2.0 header at lines 1-17

**Expected**: `["apache-2.0", "apache-2.0"]` - Two separate detections
**Actual**: `["apache-2.0"]` - One detection

**Root Cause**: Two Apache-2.0 matches (one from header, one from license tag in XML) being merged when they shouldn't be.

### Case 2: `bsd-2c.txt` (Over-Merge)

**File Content**: BSD 2-Clause license text

**Expected**: `["bsd-2-clause-views", "bsd-2-clause-views"]`
**Actual**: `["bsd-2-clause-views"]`

**Root Cause**: Two identical BSD license detections (likely from different rules or detection phases) being incorrectly merged.

### Case 3: `isc.txt` (Under-Merge)

**File Content**: ISC license text

**Expected**: `["isc"]`
**Actual**: `["isc", "isc"]`

**Root Cause**: A merge that should have happened is blocked by the `qoverlap > 0` check returning 0.

---

## Proposed Fix

### Option A: Revert Surround Merge Check (Recommended)

Remove the `qoverlap > 0` check from surround merge logic, matching Python behavior:

```rust
if current.surround(&next) {
    // Python doesn't check qoverlap here - only checks combined validity
    let combined = combine_matches(&current, &next);
    if combined.qspan().len() == combined.ispan().len() {
        rule_matches[i] = combined;
        rule_matches.remove(j);
        continue;
    }
}
if next.surround(&current) {
    let combined = combine_matches(&current, &next);
    if combined.qspan().len() == combined.ispan().len() {
        rule_matches[j] = combined;
        rule_matches.remove(i);
        i = i.saturating_sub(1);
        break;
    }
}
```

**Rationale**: 
- Python's `merge_matches()` at `match.py:998-1027` does NOT have this check for surround merges
- The `surround()` function already ensures one match contains the other's bounds
- The `combined.qspan().len() == combined.ispan().len()` check is the real validity gate

**IMPORTANT**: The `qoverlap > 0` check in the **regular overlap merge** (lines 286-300) should be KEPT - Python has this check at `match.py:1050` (`if qoverlap:`). Only the surround merge checks at lines 251-273 should be removed.

### Option B: Fix Mixed-Mode `qoverlap()` Logic

Keep the `qoverlap > 0` check but fix the mixed-mode calculation to use token ranges instead of set intersection:

```rust
// For mixed mode, use simpler range-based overlap
if let (Some(self_positions), None) = (&self.qspan_positions, &other.qspan_positions) {
    // Check if any of self's sparse positions fall within other's range
    return self_positions
        .iter()
        .filter(|&&p| p >= other.start_token && p < other.end_token)
        .count();
}
```

**Risk**: May not fully fix the issue - the root problem is the check itself, not just the calculation.

### Recommended: Option A

The `qoverlap > 0` check for surround merges is unnecessary and causes regressions. Python doesn't have it, and the CDDL fix that prompted this change was for a different issue (rule selection priority, not merge behavior).

---

## Files to Modify

| File | Change | Description |
|------|--------|-------------|
| `src/license_detection/match_refine.rs` | Lines 251-273 | Remove `qoverlap > 0` check from surround merge logic |
| `src/license_detection/models.rs` | No change needed | The mixed-mode `qoverlap()` fix is correct - keep it |

### Code Changes Summary

**Lines 251-260 (REMOVE qoverlap check):**
```rust
// BEFORE
if current.surround(&next) {
    let qoverlap = current.qoverlap(&next);  // REMOVE
    if qoverlap > 0 {                         // REMOVE
        let combined = combine_matches(&current, &next);
        // ...
    }                                         // REMOVE
}

// AFTER
if current.surround(&next) {
    let combined = combine_matches(&current, &next);
    if combined.qspan().len() == combined.ispan().len() {
        // ...
    }
}
```

**Lines 262-273 (REMOVE qoverlap check):**
Same pattern for `next.surround(&current)`.

**Lines 286-300 (KEEP qoverlap check):**
This is the regular overlap merge logic. Python has `if qoverlap:` at `match.py:1050`, so this check is correct and should remain.

---

## Testing Strategy

### Step 1: Verify Fix Doesn't Break CDDL Tests

```bash
cargo test --release -q --lib license_detection::golden_test::test_golden_lic1 2>&1 | grep -E "passed|failed"
```

Expected: lic1 still shows 242 passed (CDDL fix preserved)

### Step 2: Verify lic2 Regression Fixed

```bash
cargo test --release -q --lib license_detection::golden_test::test_golden_lic2 2>&1 | grep -E "passed|failed"
```

Expected: lic2 returns to 802+ passed

### Step 3: Verify external Regression Fixed

```bash
cargo test --release -q --lib license_detection::golden_test 2>&1 | grep "external:"
```

Expected: external returns to 2169+ passed

### Step 4: Full Regression Test

```bash
cargo test --release --lib license_detection::golden_test 2>&1 | tail -20
```

---

## Success Criteria

1. lic1: 242+ passed (maintain CDDL improvement)
2. lic2: 802+ passed (restore baseline)
3. external: 2169+ passed (restore baseline)
4. All clippy warnings resolved (separate cleanup)

---

## Implementation Checklist

- [ ] Revert `qoverlap > 0` check in surround merge logic (match_refine.rs:251-273)
- [ ] Run lic1 golden tests - verify CDDL fix still works
- [ ] Run lic2 golden tests - verify regression fixed
- [ ] Run external golden tests - verify regression fixed
- [ ] Clean up cddl_investigation_test.rs unused imports (separate task)

---

## References

- **Python merge_matches() surround logic**: `reference/scancode-toolkit/src/licensedcode/match.py:998-1027` - No `qoverlap` check
- **Python merge_matches() overlap logic**: `reference/scancode-toolkit/src/licensedcode/match.py:1045-1063` - HAS `qoverlap` check at line 1050
- **Rust merge_overlapping_matches()**: `src/license_detection/match_refine.rs:159-310`
- **Rust qoverlap()**: `src/license_detection/models.rs:547-584`
- **Git diff**: `git diff HEAD~2 HEAD -- src/license_detection/match_refine.rs src/license_detection/models.rs`
