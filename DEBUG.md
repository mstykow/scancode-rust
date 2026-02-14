# License Detection Debug Notes

## Current Status

**Golden Tests:** 154 passed, 137 failed (as of commit `a0952ea`)

---

## Issue 1: Pipeline Short-Circuit Causes Missing Detections

### Problem

Files with multiple licenses or license variants (e.g., `sudo` = ISC + DARPA) don't detect all licenses.

**Example:** `double_isc.txt`

- **Expected:** `["isc", "isc", "sudo"]`
- **Actual:** `["isc", "isc AND unknown"]`

The DARPA acknowledgment text is detected as "unknown" instead of "sudo".

### Root Cause

Rust's pipeline in `src/license_detection/mod.rs:111-129` uses `match_coverage` (rule coverage) to skip matchers:

```rust
let has_perfect_match = all_matches.iter().any(|m| m.match_coverage >= 100.0);
if !has_perfect_match {
    // aho_match only runs if no 100% coverage
    let has_high_coverage = all_matches.iter().any(|m| m.match_coverage >= 90.0);
    if !has_high_coverage {
        // seq_match only runs if no 90% coverage
    }
}
```

**The bug:** `match_coverage` measures what percentage of the **RULE** is covered, not the **QUERY**. When ISC matches with 100% coverage, Rust skips `aho_match` and `seq_match` entirely. The DARPA text (lines 36-38) never gets matched against `sudo.LICENSE`.

### Python Behavior

Python's `match_query()` in `index.py:966-1151`:

1. Runs matchers in sequence (spdx_lid → aho → seq)
2. Tracks `already_matched_qspans` - query positions already matched
3. Continues matching unmatched regions until no significant unmatched positions remain
4. The `coverage() == 100` check is for tracking, not skipping matchers

### Recommended Fix

**Option A: Remove the short-circuit entirely** (simplest, most correct)

```rust
// In src/license_detection/mod.rs, replace lines 111-129:
let hash_matches = hash_match(&self.index, &query_run);
all_matches.extend(hash_matches);

let spdx_matches = spdx_lid_match(&self.index, text);
all_matches.extend(spdx_matches);

// Always run aho and seq matchers - they handle overlap internally
let aho_matches = aho_match(&self.index, &query_run);
all_matches.extend(aho_matches);

let seq_matches = seq_match(&self.index, &query_run);
all_matches.extend(seq_matches);

let unknown_matches = unknown_match(&self.index, &query, &all_matches);
all_matches.extend(unknown_matches);
```

**Option B: Track query coverage** (more complex, potential performance gain)

Track which query positions are covered by existing matches. Only skip matchers if the entire query is matched.

### Files Affected

| File | Location |
|------|----------|
| `src/license_detection/mod.rs` | Lines 111-129 (pipeline short-circuit) |
| `reference/scancode-toolkit/src/licensedcode/index.py` | Lines 966-1151 (Python reference) |

---

## Issue 2: Unknown License Intros Appear in Expressions

### Problem

Files produce detections with "unknown" in license expressions when they shouldn't.

**Example:** `COPYING.gplv3`

- **Expected:** `["gpl-3.0"]`
- **Actual:** 8 detections including `"gpl-3.0 AND unknown"`

### Root Cause

Rust builds license expressions from ALL matches, including license intro matches that should be filtered.

### Python Behavior

Python filters intros through two-step process (`detection.py`):

1. **`analyze_detection()`** (line 1760): Returns category `UNKNOWN_INTRO_BEFORE_DETECTION` when an unknown intro is followed by a proper license match.

2. **`get_detected_license_expression()`** (lines 1510-1514): Filters intros before building expression:

   ```python
   elif analysis == DetectionCategory.UNKNOWN_INTRO_BEFORE_DETECTION.value:
       matches_for_expression = filter_license_intros(license_matches)
   ```

3. **`is_license_intro()`** (lines 1349-1365): A match is an intro if:
   - Rule has `is_license_intro` OR `is_license_clue` OR `license_expression == 'free-unknown'`
   - AND matcher is exact (`MATCH_AHO_EXACT`) OR coverage is 100%

### Previous Fix Attempt

**Result:** 5-test regression, reverted.

**Why it failed:**

1. Expression was built BEFORE category analysis (wrong order)
2. `is_unknown_intro()` logic was incomplete - didn't check rule fields
3. Missing the "exact matcher OR 100% coverage" condition

### Recommended Fix

1. **Reorder `create_detection_from_group()`** in `src/license_detection/detection.rs`:
   - Analyze category FIRST
   - Filter matches based on category
   - THEN build expression

2. **Fix `is_unknown_intro()` to match Python**:

   ```rust
   fn is_unknown_intro(m: &LicenseMatch) -> bool {
       let is_intro = m.is_license_intro || m.is_license_clue || 
                      m.license_expression == "free-unknown";
       let is_exact = m.matcher == "2-aho" || m.match_coverage >= 99.99;
       is_intro && is_exact
   }
   ```

3. **Ensure `is_license_intro` is populated** in `LicenseMatch` from rule data.

### Files Affected

| File | Location |
|------|----------|
| `src/license_detection/detection.rs` | `create_detection_from_group()` - reorder logic |
| `src/license_detection/models.rs` | `LicenseMatch` - needs `is_license_intro`, `is_license_clue` fields |
| `reference/scancode-toolkit/src/licensedcode/detection.py` | Lines 1349-1365, 1510-1514 |

---

## Issue 3: Deprecated Rules Handling

### Problem

Test `camellia_bsd.c` expected `bsd-2-clause-first-lines`, got `freebsd-doc` (a deprecated rule).

### Python Behavior

Python **skips deprecated rules by default** (`models.py:1103-1104`):

```python
# always skip deprecated rules
rules = [r for r in rules if not r.is_deprecated]
```

Deprecated rules have `replaced_by` pointing to the new license key.

### Previous Fix Attempt

**Attempted:** Skip deprecated rules during index building.

**Result:** 19-test regression (160 → 141 passed).

**Why it failed:**

- Some tests explicitly expect deprecated license expressions (e.g., `freebsd-doc_*.txt` tests)
- The `freebsd-doc.LICENSE` file is NOT deprecated - only `freebsd-doc_5.RULE` is deprecated
- Tests should match against the non-deprecated LICENSE file

### Recommended Fix

1. **Keep skipping deprecated rules** (matches Python's default behavior)

2. **Update golden tests** that expect deprecated expressions to expect the replacement expressions

3. **Alternative:** Add `--with-deprecated` flag for backwards compatibility

### Files Affected

| File | Location |
|------|----------|
| `src/license_detection/index/builder.rs` | Rule loading - filter deprecated |
| `src/license_detection/rules/loader.rs` | Load `is_deprecated` from rule files |
| `testdata/license-golden/datadriven/lic1/freebsd-doc_*.txt.EXPECTED` | May need updates |

---

## Fixed Issues

### Line Number Calculation (Fixed in `0d72a6e`)

**Problem:** All matches appeared to span the entire document.

**Fix:** Added `QueryRun::line_for_pos()` and used match positions instead of query run boundaries.

**Files:** `src/license_detection/query.rs`, `src/license_detection/aho_match.rs`, `src/license_detection/hash_match.rs`

### GPL False Positive Filter (Fixed in `0d72a6e`)

**Problem:** GPL matches with `matched_length <= 3` caused false positives.

**Fix:** Added `filter_short_gpl_matches()` to remove short GPL matches.

**File:** `src/license_detection/match_refine.rs`

### Rule Sorting (Fixed in `a0952ea`)

**Problem:** Rule IDs differed between Python and Rust due to missing sort.

**Fix:** Added `all_rules.sort()` before assigning rule IDs in `builder.rs`.

**Files:** `src/license_detection/index/builder.rs`, `src/license_detection/models.rs`

---

## Priority Order

1. **High:** Issue 1 (Pipeline Short-Circuit) - Affects many multi-license files
2. **Medium:** Issue 2 (Unknown in Expressions) - Affects expression accuracy
3. **Low:** Issue 3 (Deprecated Rules) - Only affects specific test cases
