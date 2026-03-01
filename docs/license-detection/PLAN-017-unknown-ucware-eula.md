# PLAN-017: unknown/ucware-eula.txt

## Status: COMPLETE

## Test File
`testdata/license-golden/datadriven/unknown/ucware-eula.txt`

## Issue
**Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "swrule"]`
**Actual (before fix):** `["unknown", "warranty-disclaimer", "unknown"]`
**Actual (after fix):** PASSES (golden test shows 5 passed, ucware no longer in failures)

## Root Cause

Rust's `filter_contained_matches()` had **expression-based containment logic** that Python doesn't have.

Python's `filter_contained_matches()` (match.py:1075-1200) uses ONLY `qcontains()` for token position containment.

Rust added extra logic that filtered matches based on `licensing_contains()` expression containment.

## Fix Applied

Removed expression-based containment from two places in `src/license_detection/match_refine.rs`:

1. **filter_contained_matches()** (lines 420-438) - Removed the expression-based containment block

2. **filter_license_references_with_text_match()** (lines 483-512) - Removed Case 2 (expression containment case)

Python uses expression-based containment ONLY in `filter_overlapping_matches()` for MEDIUM/SMALL overlap cases, which Rust still has.

## Results

- **Before fix:** 4 passed, 6 failed (including ucware-eula.txt)
- **After fix:** 5 passed, 5 failed (ucware-eula.txt now passes!)

## Success Criteria
- [x] Python implementation analyzed
- [x] Rust implementation analyzed
- [x] Identified expression-based containment as the divergence
- [x] Removed expression-based containment from filter_contained_matches()
- [x] Removed Case 2 from filter_license_references_with_text_match()
- [x] ucware-eula.txt now passes

## Remaining Work

The other 5 failures in the unknown test suite are separate issues (different from PLAN-017):
- README.md, cigna-go-you-mobile-app-eula.txt, citrix.txt, qt.commercial.txt, scea.txt

These appear to be related to unknown license detection pipeline differences and should be investigated separately.
