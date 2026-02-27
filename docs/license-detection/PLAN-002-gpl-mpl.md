# PLAN-002: gpl-2.0-plus_and_mpl-1.0.txt

## Status: OPEN - NEEDS FIX

## Test File
`testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_mpl-1.0.txt`

## Issue
Should detect `mpl-1.0 OR gpl-2.0-plus` as single expression, but Rust detects them separately.

**Expected:** `["mpl-1.0 OR gpl-2.0-plus"]`
**Actual:** `["mpl-1.0 AND gpl-1.0-plus AND gpl-2.0-plus"]`

## Root Cause Analysis

### Python Behavior (Correct)
Python matches a **combined rule** `mpl-1.0_or_gpl-2.0-plus_2.RULE` via the **seq matcher** (near-duplicate detection):
- Single match: `mpl-1.0 OR gpl-2.0-plus` at lines 1-21
- Matcher: `3-seq`
- Rule: `mpl-1.0_or_gpl-2.0-plus_2.RULE`
- Score: 89.55, coverage: 100%

**Python's `is_matchable` check returns `True`** because position 0 remains uncovered, so seq matching runs.

### Rust Behavior (Incorrect)
Rust finds **three separate matches** via the **aho matcher**:
1. `mpl-1.0` at lines 3-11 (rule: `mpl-1.0_22.RULE`, 100% coverage)
2. `gpl-1.0-plus` at line 13 (rule: `gpl_bare_word_only.RULE`, 100% coverage) **← EXTRA MATCH**
3. `gpl-2.0-plus` at lines 17-21 (rule: `gpl-2.0-plus_85.RULE`, 100% coverage)

**Rust's `is_matchable` check returns `False`** because ALL high_matchable positions are covered.

### The Exact Bug

In `src/license_detection/mod.rs:193-198`:

```rust
let merged_aho = merge_overlapping_matches(&aho_matches);
let (filtered_aho, _discarded_aho) =
    match_refine::filter_contained_matches(&merged_aho);  // ← MISSING filter_overlapping_matches
for m in &filtered_aho {
    if m.match_coverage >= 99.99 && m.end_token > m.start_token {
        matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
    }
    // ...
}
```

Rust does NOT call `filter_overlapping_matches` after `filter_contained_matches`. This leaves 4 matches (including overlapping mpl-1.0 rules at tokens 0-81 and 1-100), which together cover ALL high_matchable positions.

Python's `refine_matches` (with `merge=False`) calls BOTH:
1. `filter_contained_matches` - reduces 50 → 4 matches
2. `filter_overlapping_matches` - reduces 4 → 3 matches

With only 3 matches, position 0 (a high_matchable token) remains uncovered, so `is_matchable` returns `True` and seq matching runs.

### Evidence from Investigation

**Rust matched_qspans (4 matches):**
- mpl-1.0 qspan=0-81 (covers position 0)
- mpl-1.0 qspan=1-100 
- gpl-1.0-plus qspan=101-101
- gpl-2.0-plus qspan=107-219
- **Result:** All 41 high_matchables covered → `is_matchable = False` → seq skipped

**Python matched_qspans (3 matches):**
- mpl-1.0 qspan=1-100 (does NOT cover position 0)
- gpl-1.0-plus qspan=101-101
- gpl-2.0-plus qspan=107-219
- **Result:** Position 0 uncovered → `is_matchable = True` → seq runs

## Fix Required

**Location:** `src/license_detection/mod.rs:193-208`

Add `filter_overlapping_matches` AND `restore_non_overlapping` after `filter_contained_matches` to match Python's behavior:

```rust
let merged_aho = merge_overlapping_matches(&aho_matches);
let (non_contained_aho, _) = match_refine::filter_contained_matches(&merged_aho);
let (non_overlapping_aho, discarded_overlapping) = match_refine::filter_overlapping_matches(non_contained_aho, &self.index);
let filtered_aho = match_refine::restore_non_overlapping(non_overlapping_aho, discarded_overlapping);
for m in &filtered_aho {
    // ... rest unchanged
}
```

This reduces the matched_qspans to 3 (like Python), leaving position 0 uncovered, so seq matching runs and finds the combined rule.

## Regression Found: gpl-2.0-plus_and_mit_1.txt

### Status: ANALYZED - FIX IDENTIFIED

### Test File
`testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_mit_1.txt`

### Issue
Initial fix (adding only `filter_overlapping_matches`) caused regression.

**Expected:** `["gpl-2.0-plus", "mit", "mit", "gpl-1.0-plus"]`
**Actual with fix:** `["gpl-2.0-plus", "mit", "gpl-1.0-plus"]` (missing one "mit")

### Root Cause

Python's `filter_overlapping_matches` discards 2 MIT matches, but then `restore_non_overlapping` restores one of them because it doesn't actually overlap with the kept match.

**Python pipeline:**
- After `filter_contained`: 3 MIT kept (`mit_31`, `mit_1340`, `mit.LICENSE`)
- After `filter_overlapping`: 1 MIT kept (`mit.LICENSE`), 2 MIT discarded
- After `restore_non_overlapping`: 2 MIT (`mit.LICENSE` + `mit_31` restored)

**Rust with initial fix (Phase 1c):**
- After `filter_contained`: 3 MIT kept
- After `filter_overlapping`: 1 MIT kept
- **MISSING `restore_non_overlapping`** - so `mit_31` is NOT restored

### MIT Match Details

| Rule | Tokens | Overlaps with mit.LICENSE? | Status |
|------|--------|---------------------------|--------|
| mit_31.RULE | 202-205 | No (adjacent, not overlapping) | Should be restored |
| mit_1340.RULE | 203-354 | Yes | Correctly discarded |
| mit.LICENSE | 205-366 | N/A | Always kept |

The `mit_31.RULE` match (tokens 202-205) does NOT overlap with `mit.LICENSE` (tokens 205-366) - they are adjacent. So `restore_non_overlapping` correctly restores it.

### Updated Fix

Must call `restore_non_overlapping` after `filter_overlapping_matches` (see Fix Required section above).

### Investigation Tests

`src/license_detection/investigation/gpl_mit_regression_test.rs`

## Investigation Tests

`src/license_detection/investigation/gpl_mpl_test.rs`

## Key Files

- Detection pipeline: `src/license_detection/mod.rs:189-207`
- Python reference: `reference/scancode-playground/src/licensedcode/index.py:1000-1070`
- Python refine_matches: `reference/scancode-playground/src/licensedcode/match.py:2691-2820`
- Combined rule: `reference/scancode-toolkit/src/licensedcode/data/rules/mpl-1.0_or_gpl-2.0-plus_2.RULE`
