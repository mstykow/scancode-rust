# BUG-010: filter_overlapping_matches Regression

## Summary

After adding `filter_overlapping_matches` after `filter_contained_matches` in the aho matching phase (mod.rs:195-196), the file `gpl-2.0-plus_and_mit_1.txt` is missing one MIT match.

**Expected:** `["gpl-2.0-plus", "mit", "mit", "gpl-1.0-plus"]`
**Actual:** `["gpl-2.0-plus", "mit", "gpl-1.0-plus"]` (missing one "mit")

## Investigation Results

### Root Cause

The issue is NOT in `filter_overlapping_matches` itself. The root cause is in how Rust's aho matching produces different matches than Python's.

#### Python behavior:
- Produces `mit_30.RULE` at qspan(202, 203), len=2, lines 40-40
- Does NOT produce `mit_31.RULE`
- Final: 2 MIT matches (mit_30.RULE at line 40, mit.LICENSE at lines 42-61)

#### Rust behavior:
- Produces BOTH `mit_30.RULE` (tokens 202-204, len=2) AND `mit_31.RULE` (tokens 202-205, len=3) at the same location
- `filter_contained_matches` discards `mit_30.RULE` as contained in `mit_31.RULE`
- Then `filter_overlapping_matches` discards `mit_1340.RULE` (which overlaps heavily with mit.LICENSE)
- Final: 1 MIT match (mit.LICENSE at lines 42-61)

### Pipeline Trace

```
=== STEP 0: RAW AHO MATCHES ===
Count: 68
  mit_30.RULE at tokens 202-204 (len=2, hilen=1)
  mit_31.RULE at tokens 202-205 (len=3, hilen=2)
  mit_1340.RULE at tokens 203-354 (len=151, hilen=37)
  mit.LICENSE at tokens 205-366 (len=161, hilen=37)
  ... (many more MIT rules)

=== STEP 1: AFTER merge_overlapping_matches ===
  mit.LICENSE at tokens 205-366
  mit_30.RULE at tokens 202-204  <- Kept
  mit_31.RULE at tokens 202-205  <- Kept (both kept!)

=== STEP 2: AFTER filter_contained_matches ===
MIT kept: 3
  KEPT: mit_31.RULE at tokens 202-205  <- mit_30.RULE discarded as contained
  KEPT: mit_1340.RULE at tokens 203-354
  KEPT: mit.LICENSE at tokens 205-366

MIT discarded: 7
  DISCARDED: mit_30.RULE at tokens 202-204  <- PROBLEM: this should be kept!

=== STEP 3: AFTER filter_overlapping_matches ===
MIT kept: 1
  KEPT: mit.LICENSE at tokens 205-366

MIT discarded: 2
  DISCARDED: mit_31.RULE at tokens 202-205
  DISCARDED: mit_1340.RULE at tokens 203-354
```

### The Two Problems

1. **Aho matching difference**: Rust produces both `mit_30.RULE` and `mit_31.RULE` for the same text, but Python only produces `mit_30.RULE`. The rules are:
   - `mit_30.RULE`: "License: MIT" (is_required_phrase: yes)
   - `mit_31.RULE`: "License: {{MIT license}}"

2. **filter_contained_matches filtering**: When both rules are present, `mit_30.RULE` (shorter) is discarded as contained in `mit_31.RULE` (longer, same start position). Python doesn't have this issue because it doesn't produce `mit_31.RULE`.

3. **filter_overlapping_matches then filters remaining MIT matches**: With `mit_30.RULE` gone, `mit_31.RULE` and `mit_1340.RULE` are discarded due to heavy overlap with `mit.LICENSE`.

### Why Python Doesn't Have This Problem

Python's aho matching likely has one of these behaviors:
1. Only produces one match per location (longest or highest priority)
2. Has different rule ordering that prefers `mit_30.RULE`
3. Has different handling of `is_required_phrase` rules
4. Filters overlapping aho matches differently

## Files Involved

- Test file: `testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_mit_1.txt`
- Investigation test: `src/license_detection/investigation/ietf_regression_test.rs`
- Aho matching: `src/license_detection/aho_match.rs`
- Filtering: `src/license_detection/match_refine.rs`

## Resolution Options

### Option 1: Fix aho matching to match Python behavior
Investigate why Python doesn't produce `mit_31.RULE` and replicate that in Rust.

### Option 2: Preserve `is_required_phrase` matches
The `mit_30.RULE` has `is_required_phrase: yes` in its rule. We could modify `filter_contained_matches` to NOT discard matches that have `is_required_phrase: yes` when they're contained in a match without that flag.

### Option 3: Revert filter_overlapping_matches for aho phase
The original issue (PLAN-002) that led to adding `filter_overlapping_matches` was about GPL/MPL detection. We could:
- Keep `filter_overlapping_matches` in the final refine phase
- But NOT add it after the aho phase in mod.rs

### Recommended: Option 1
Investigate Python's aho matching to understand why it doesn't produce `mit_31.RULE`. This is likely the correct fix.

## Status

**INVESTIGATION COMPLETE** - Root cause identified, awaiting fix decision.
