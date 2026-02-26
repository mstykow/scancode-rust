# PLAN-080: Unknown Detection - ucware-eula.txt

## Status: FIXED

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/ucware-eula.txt`

| Expected | Actual (Before Fix) |
|----------|--------|
| `swrule` license detected | `swrule` not detected, extra "unknown" instead |

## Root Cause: Four Issues Found

### Issue 1: ScoresVector Ordering (FIXED)

**Problem**: Rust's `ScoresVector` had wrong ordering - `is_highly_resemblant` was 4th instead of 1st.

**Python ordering** (match_set.py line 452-456):
```python
_scores_vector_fields = [
    'is_highly_resemblant',  # 1st
    'containment',           # 2nd
    'resemblance',           # 3rd
    'matched_length']        # 4th
```

**Fix**: Updated `ScoresVector::cmp()` in `seq_match.rs` to match Python's ordering.

### Issue 2: Candidate Ordering (FIXED)

**Problem**: Rust's `Candidate::cmp()` used `score_vec_full` directly, but Python compares the rounded scores FIRST (via tuple comparison `((svr, svf), ...)`).

**Fix**: Updated `Candidate::cmp()` to compare `score_vec_rounded` first, then `score_vec_full`.

### Issue 3: Step 2 Multiset Ranking (FIXED)

**Problem**: `compute_candidates_with_msets()` step 2 used HIGH multisets for both filtering AND scoring, but Python uses:
- HIGH multisets only for filtering (discard if not enough high matches)
- FULL multisets for containment/resemblance scores

**Key insight** (from Python's `compare_token_sets()`):
```python
intersection = intersector(qset, iset)  # FULL intersection
matched_length = counter(intersection)   # FULL matched length
iset_len = rule.get_length(unique=False) # FULL rule length (not unique)
containment = matched_length / iset_len
```

**Fix**: Changed step 2 to compute intersection and scores using FULL multisets, while keeping HIGH multisets for filtering.

### Issue 4: Phase 3 Candidates Limit (FIXED)

**Problem**: Phase 3 in `detect()` used `seq_match()` which calls `select_candidates(... 50)`, but Python uses `MAX_CANDIDATES = 70`.

**Fix**: Changed Phase 3 to use `compute_candidates_with_msets(... 70)` like Phase 4.

## Files Modified

1. `src/license_detection/seq_match.rs`:
   - `ScoresVector::cmp()` - Fixed ordering to match Python
   - `Candidate::cmp()` - Fixed to compare rounded scores first
   - `compute_candidates_with_msets()` - Fixed step 2 to use FULL multisets for scores

2. `src/license_detection/mod.rs`:
   - Phase 3 changed from `seq_match()` to `compute_candidates_with_msets(... 70)`

## Verification

After fixes:
- swrule is now detected with matcher 3-seq, score 11.0
- Matches Python's detection (matcher 3-seq, score 10.64)

## Remaining Work

The golden test for ucware-eula.txt shows differences in unknown match positioning:
- Expected: `["unknown-license-reference", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "swrule"]`
- Actual: `["unknown-license-reference", "unknown-license-reference", "swrule", "warranty-disclaimer"]`

This is a separate issue related to how unknown matches are created and positioned relative to other matches. The main fix (swrule detection) is working correctly.

## Test Case

Test `test_plan_080_swrule_detection_ucware` in `extra_detection_investigation_test.rs` traces through the entire candidate selection pipeline and verifies swrule detection.
