# PLAN-058: Lic2 Regression - Duplicate License Detections Merged

## Status: NEEDS INVESTIGATION

## Problem Statement

The CDDL fix caused 3 new regressions in lic2 tests where duplicate license detections are incorrectly merged into a single detection.

### Golden Test Changes

| Test Set | Baseline | Current | Change |
|----------|----------|---------|--------|
| lic1 | 240 passed, 51 failed | 241 passed, 50 failed | **+1** |
| lic2 | 802 passed, 51 failed | 799 passed, 54 failed | **-3** |
| external | 2169 passed, 398 failed | 2176 passed, 391 failed | **+7** |

### New Lic2 Failures

| Test File | Expected | Actual | Issue |
|-----------|----------|--------|-------|
| `1908-bzip2/bzip2.106.c` | `["bzip2-libbzip-2010", "bzip2-libbzip-2010"]` | `["bzip2-libbzip-2010"]` | Under-merge |
| `apache-2.0_and_apache-2.0.txt` | `["apache-2.0", "apache-2.0"]` | `["apache-2.0"]` | Under-merge |
| `aladdin-md5_and_not_rsa-md5.txt` | `["zlib", "zlib"]` | `["zlib"]` | Under-merge |

---

## Root Cause Hypothesis

The `qcontains()` fix for mixed `qspan_positions` modes is causing two separate license detections with the same expression to be incorrectly merged.

**Scenario**:
1. File contains two separate license instances (e.g., two bzip2 licenses in different locations)
2. Each creates a `LicenseMatch` with contiguous positions (`qspan_positions: None`)
3. The `qcontains()` now uses range containment for the `None`/`None` case
4. When one match's range fully contains another, they're merged via `filter_contained_matches()`
5. This incorrectly collapses two separate detections into one

**Key Question**: Why are these matches being marked as "contained" when they represent different locations in the file?

---

## Investigation Required

1. Run `bzip2.106.c` through detection and trace:
   - What matches are created?
   - What are their `start_token`, `end_token`, `qspan_positions`?
   - Which filter is merging them?

2. Compare with Python reference:
   - Does Python keep them separate?
   - What is different about Python's `qcontains()` or merge logic?

3. Check `filter_contained_matches()`:
   - Is `licensing_contains_match()` removal related?
   - Should we restore it for same-expression cases?

---

## Files to Investigate

| File | Purpose |
|------|---------|
| `src/license_detection/models.rs` | `qcontains()` implementation |
| `src/license_detection/match_refine.rs` | `filter_contained_matches()`, merge logic |
| `testdata/license-golden/datadriven/lic2/1908-bzip2/bzip2.106.c` | Test file |

---

## Success Criteria

1. lic2: 802+ passed (restore baseline)
2. lic1: 241+ passed (keep CDDL improvement)
3. external: 2176+ passed (keep improvement)
4. Understand why the merge is happening incorrectly
5. Implement fix without breaking CDDL tests

---

## Related Plans

- PLAN-056: CDDL Rule Selection Investigation (original issue)
- PLAN-057: CDDL Fix Regression Cleanup (surround merge fix)
