# PLAN-017: Remaining License Detection Fixes

## Status: In Progress

### Current Test Results

| Test Suite | Baseline | PLAN-016 | PLAN-017 | Change |
|------------|----------|----------|----------|--------|
| lic1 | 213/78 | 219/72 | 229/62 | +10 |
| lic2 | 759/94 | 773/80 | 777/76 | +4 |
| lic3 | 242/50 | 251/41 | 252/40 | +1 |
| lic4 | 265/85 | 282/68 | 282/68 | 0 |
| external | 1935/632 | 1880/687 | 1897/670 | +17 |
| unknown | 2/8 | 2/8 | 2/8 | 0 |

---

## Completed Fixes

### Issue 2: Golden Test Comparison ✅ DONE

**Fix**: Changed test to flatten `detection.matches` instead of comparing detection expressions.

```rust
// BEFORE:
let actual: Vec<&str> = detections
    .iter()
    .map(|d| d.license_expression.as_deref().unwrap_or(""))
    .collect();

// AFTER:
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

**Impact**: +49 tests passed on lic1-4, but -56 regression on external.

### Detection.matches Storage Fix ✅ DONE

**Fix**: Store raw matches in `detection.matches`, use filtered matches only for expression computation.

**Impact**: Minimal change (within noise margin).

### Issue 3: Remove filter_short_gpl_matches ✅ DONE

**File**: `src/license_detection/match_refine.rs`

**Fix**: Deleted `filter_short_gpl_matches()` function and its call. Python does NOT have this filter - it was incorrectly added.

**Impact**: +8 tests passed on lic1, +2 on lic2, +4 on external.

### Issue 5: Add Unknown License Filter ✅ DONE

**File**: `src/license_detection/match_refine.rs`, `src/license_detection/mod.rs`

**Fix**: Added `filter_invalid_contained_unknown_matches()` function that filters unknown matches contained within good matches' qregion (token span).

**Impact**: +2 tests passed on lic1, +2 on lic2, +13 on external.

---

## Remaining Issues

### Issue 4: Query Run Lazy Evaluation ✅ READY

**File**: `src/license_detection/query.rs`

Change `QueryRun` to store `&Query` reference and compute `high_matchables()`/`low_matchables()` on-demand.

### Issue 6: Add filter_matches_missing_required_phrases ⚠️ COMPLEX

**File**: `src/license_detection/match_refine.rs`

Add filter that removes matches where required phrases weren't matched. This requires:

1. Parse `{{...}}` markers from rule text
2. Track required phrase spans during matching
3. Filter matches missing required phrases
4. Handle `is_continuous` rules (3317 rules)
5. Handle `is_required_phrase` rules (1927 rules)

**Python implementation**: `reference/scancode-toolkit/src/licensedcode/match.py:2154-2328`

**This filter is called FIRST in Python's refine pipeline.**

---

## Implementation Order

1. ~~**Issue 3** (Remove filter_short_gpl_matches)~~ ✅ DONE
2. ~~**Issue 5** (Add unknown filter)~~ ✅ DONE
3. **Issue 6** (Add required phrases filter) - Complex, requires rule parsing changes
4. **Issue 4** (Query Run lazy eval) - Architectural change

---

## Verification Commands

```bash
cargo test --release -q --lib license_detection::golden_test
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```
