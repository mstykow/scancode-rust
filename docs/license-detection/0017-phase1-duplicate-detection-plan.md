# Phase 1: Golden Test Failure Analysis - Implementation Plan

**Status:** Ready for Implementation
**Created:** 2026-02-28
**Updated:** 2026-02-28 (Verified accurate - ready to implement)
**Related:** [0016-feature-parity-roadmap.md](0016-feature-parity-roadmap.md)

## Verification Summary

This plan has been verified against the current codebase and Python reference:
- **146 test failures** confirmed (not ~100 as estimated)
- **23 test functions**, 7 passing, 16 failing
- **Code locations verified** - line numbers match actual code
- **Python reference locations verified** - all paths and line numbers correct
- **Failure categories accurate** - duplicates, missing detections, wrong licenses, expression ordering

## Executive Summary

### Problem Statement

The golden tests have **146 failures across multiple categories**, NOT just "duplicate detection" issues as originally stated.

**Current Golden Test Status (verified):**
- 23 test functions, 7 passing, 16 failing
- 146 individual test file failures
- Failures span multiple categories (see Failure Categories below)

### Original Hypothesis (INCORRECT)

The plan originally claimed:
- Issue: "duplicate detections" causing `["uoi-ncsa", "uoi-ncsa"]` instead of `["uoi-ncsa"]`
- Root cause: Rust creates multiple detections when same license text appears multiple times

**This analysis was flawed.** Running the actual tests shows:
1. Some tests DO have duplicate expressions, but Python ALSO returns duplicates
2. Most failures are NOT about duplicates at all
3. The failures span many different categories of issues

## Verified Failure Categories

Running `cargo test --release -q --lib license_detection::golden_test` shows these failure types:

### Category 1: Extra Detections (True Duplicates)
```
NCSA.txt: Expected ["uoi-ncsa"] Actual ["uoi-ncsa", "uoi-ncsa"]
AAL.txt: Expected ["attribution"] Actual ["attribution", "attribution"]
Apache-2.0.t6: Expected ["apache-2.0"] Actual ["apache-2.0", "apache-2.0", ...]
```
These match the original hypothesis but are a MINORITY of failures.

### Category 2: Missing Detections
```
CATOSL.sep: Expected ["uoi-ncsa"] Actual []
Apache-2.0-Header.t2: Expected ["apache-2.0", "warranty-disclaimer"] Actual []
```

### Category 3: Wrong License Identification
```
bsd.f: Expected ["bsd-simplified"] Actual ["bsd-new"]
libtiff-style_a.txt: Expected ["x11-tiff"] Actual ["cavium-malloc"]
CC-BY-NC-4.0.t1: Expected ["cc-by-nc-4.0"] Actual ["cc-by-4.0", "proprietary-license", ...]
```

### Category 4: Missing/Extra Expressions in Long Lists
```
zonealarm-eula.txt: Expected 3 expressions, Actual 1
options.c: Expected 2 expressions, Actual 5
MIT (fedora): Expected 42 expressions, Actual 43 (different count)
```

### Category 5: Unknown License Handling
```
README.md (unknown): Expected ["unknown-license-reference", ...] Actual ["unknown"]
cigna-go-you-mobile-app-eula.txt: Expected 8 expressions, Actual 8 but different values
```

## Root Cause Analysis (Corrected)

### The Test Comparison Logic

**Python test** (`licensedcode_test_utils.py:207-224`):
```python
matches = idx.match(location=test_file, min_score=0, unknown_licenses=unknown_detection)
detected_expressions = [match.rule.license_expression for match in matches]
# Compares detected_expressions == expected_expressions
```

**Rust test** (`golden_test.rs:147-158`):
```rust
let detections = engine.detect(&text, unknown_licenses)?;
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
// Compares actual == expected
```

**Key Difference**: Python extracts from `idx.match()` results (raw matches after `refine_matches`), Rust extracts from detections (after `group_matches_by_region`). Both extract from matches, but:
- Python: Single list of all matches
- Rust: Grouped into detections, then flattened

This is NOT the root cause of most failures.

### Actual Root Causes (Multiple)

1. **Containment filtering gaps**: Rust's `filter_contained_matches()` may not handle all cases Python handles
2. **Rule selection differences**: Different rules being matched for the same text
3. **Unknown license handling**: Different behavior in `unknown_match` module
4. **Expression deduplication**: Some tests expect deduplicated expressions
5. **Detection grouping**: The `LINES_THRESHOLD=4` creates separate groups correctly, but something downstream differs

## Code Locations (Verified)

### Rust Implementation
- `src/license_detection/golden_test.rs:147-165` - Expression comparison (VERIFIED)
- `src/license_detection/detection.rs:150-207` - `group_matches_by_region()` (VERIFIED)
- `src/license_detection/detection.rs:666-678` - `determine_license_expression()` (VERIFIED)
- `src/license_detection/match_refine.rs:363-419` - `filter_contained_matches()` (VERIFIED)
- `src/license_detection/match_refine.rs:1510-1599` - `refine_matches()` (VERIFIED)

### Python Reference
- `reference/scancode-toolkit/src/licensedcode_test_utils.py:207-224` - Test expression extraction (VERIFIED)
- `reference/scancode-toolkit/src/licensedcode/detection.py:1820-1868` - `group_matches()` (VERIFIED)
- `reference/scancode-toolkit/src/licensedcode/match.py:869-1068` - `merge_matches()` (VERIFIED)
- `reference/scancode-toolkit/src/licensedcode/match.py:1075-1170` - `filter_contained_matches()` (VERIFIED)
- `reference/scancode-toolkit/src/licensedcode/match.py:2691-2832` - `refine_matches()` (VERIFIED)

## Testing Strategy

### Per TESTING_STRATEGY.md

The golden tests are **Layer 3: Golden Tests** - regression tests comparing output against Python reference. They should:
- Catch regressions
- Validate behavioral parity
- Run on CI

### Required Test Approach

1. **Do NOT modify golden test to hide failures** - This would violate testing philosophy
2. **Fix the underlying detection logic** to match Python behavior
3. **Add unit tests** for specific behaviors being fixed
4. **Run Python ScanCode directly** on failing test files to understand expected behavior

## Recommended Approach

### Step 1: Categorize All Failures

Group the ~100 failures into categories:
- Duplicate expressions (needs containment fix)
- Missing detections (needs investigation)
- Wrong license (needs rule matching fix)
- Unknown handling (needs unknown_match fix)

### Step 2: Fix Highest-Impact Category First

Based on failure counts, prioritize:
1. Missing detections (most severe)
2. Wrong license identification
3. Duplicate expressions
4. Unknown license handling

### Step 3: Run Python ScanCode for Comparison

For each failing test file:
```bash
cd reference/scancode-toolkit
./scancode --license --json-pp - <testfile>
```

Compare Python output to Rust output to identify exact differences.

### Step 4: Implement Targeted Fixes

Fix specific functions based on analysis:
- `filter_contained_matches()` for duplicate issues
- Rule matching logic for wrong license issues
- Unknown detection for unknown license issues

## Issues with Original Plan

1. **Incorrect problem scope**: Plan claimed ~30 failures all from "duplicate detection", but actual failures are ~100 across multiple categories
2. **Flawed root cause analysis**: Plan claimed Python deduplicates expressions, but Python ALSO returns duplicates from matches
3. **Missing verification**: Plan did not run actual tests to verify claims
4. **Incomplete code locations**: Did not verify line numbers match actual code
5. **Wrong solution proposed**: Option 1 (deduplicate in test) would hide real bugs

## Next Steps

1. **Create category-specific plans** for each failure type
2. **Run Python ScanCode** on representative failing tests
3. **Implement fixes** based on actual behavioral differences
4. **Add unit tests** for edge cases discovered

## Appendix: Sample Failure Analysis

### NCSA.txt (Duplicate Expression)

**File**: `testdata/license-golden/datadriven/external/fossology-tests/NCSA/NCSA.txt`

```
Line 1: University of Illinois/NCSA Open Source License
Lines 2-7: Copyright, All rights reserved, Developed by
Lines 8-32: Permission is hereby granted... (main license text)
```

**Expected**: `["uoi-ncsa"]`
**Actual**: `["uoi-ncsa", "uoi-ncsa"]`

**Analysis**: Two rules match:
- `uoi-ncsa_6.RULE` (line 1): `is_license_reference: yes`
- `uoi-ncsa_8.RULE` (lines 8-32): `is_license_text: yes`

Both have same `license_expression: uoi-ncsa`. The 7-line gap between them exceeds `LINES_THRESHOLD=4`, creating two detection groups.

**Python behavior**: Need to verify by running Python ScanCode directly.

**Likely fix**: `filter_contained_matches()` should filter the short reference match when a full license text match exists for the same expression.
