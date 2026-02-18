# PLAN-017: Remaining License Detection Fixes

## Status: In Progress

### Current Test Results

| Test Suite | Baseline | After Fixes | Status |
|------------|----------|-------------|--------|
| lic1 | 213/78 | 219/72 | +6 passed |
| lic2 | 759/94 | 773/80 | +14 passed |
| lic3 | 242/50 | 251/41 | +9 passed |
| lic4 | 265/85 | 282/68 | +17 passed |
| external | 1935/632 | 1880/687 | -55 (regression) |
| unknown | 2/8 | 2/8 | no change |

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

---

## External Regression Analysis

### Root Cause: Missing Filter

**Missing filter**: `filter_matches_missing_required_phrases`

Python calls this filter FIRST in the pipeline. It removes matches where:

- `is_continuous: true` but match has gaps
- `{{...}}` required phrase markers weren't matched

**Stats**:

- 3317 rules have `is_continuous: yes`
- 1927 rules have `is_required_phrase: yes`
- 4 licenses have `{{...}}` required phrase markers

### Regression Breakdown

| Category | Count | % of Failures |
|----------|-------|---------------|
| MORE matches than expected | 376 | 56% |
| FEWER matches than expected | 228 | 34% |
| Same count, different expressions | 61 | 9% |

---

## Remaining Issues

### Issue 3: Remove filter_short_gpl_matches ✅ READY

**File**: `src/license_detection/match_refine.rs:31-43`

Delete `filter_short_gpl_matches()` function and its call. Python does NOT have this filter.

### Issue 4: Query Run Lazy Evaluation ✅ READY

**File**: `src/license_detection/query.rs`

Change `QueryRun` to store `&Query` reference and compute `high_matchables()`/`low_matchables()` on-demand.

### Issue 5: Add Unknown License Filter ✅ READY

**File**: `src/license_detection/match_refine.rs`

Add `filter_invalid_contained_unknown_matches()` function and call it after `unknown_match()`.

### Issue 6: Add filter_matches_missing_required_phrases ⚠️ NEW

**File**: `src/license_detection/match_refine.rs`

Add filter that removes matches where required phrases weren't matched. This requires:

1. Parse `{{...}}` markers from rule text
2. Track required phrase spans during matching
3. Filter matches missing required phrases

**This is the likely cause of external regression.**

---

## Implementation Order

1. **Issue 3** (Remove filter_short_gpl_matches) - Simple deletion
2. **Issue 5** (Add unknown filter) - Simple addition
3. **Issue 6** (Add required phrases filter) - More complex, may fix external regression
4. **Issue 4** (Query Run lazy eval) - Architectural change

---

## Verification Commands

```bash
cargo test --release -q --lib license_detection::golden_test
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```
