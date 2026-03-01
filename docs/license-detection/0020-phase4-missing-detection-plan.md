# Phase 4: Missing Detection Implementation Plan

**Status:** Planning  
**Created:** 2026-03-01  
**Related:** `docs/license-detection/0016-feature-parity-roadmap.md` (Phase 4)

## Executive Summary

**Problem:** Some files have expected license detections that Rust completely misses or partially misses.

**Impact:** ~25+ test failures across the golden test suite.

**Complexity:** High - Each failure may require individual investigation.

**Approach:** Categorize failures by root cause, implement targeted fixes for each category.

---

## Failure Categories

### Category A: Complete Missing Detection

Files where Rust returns `[]` but Python finds licenses.

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `Apache-2.0-Header.t2` | `["apache-2.0", "warranty-disclaimer"]` | `[]` | Short header match not triggering |
| `gpl-3+-with-rem-comment.xml` | `["gpl-3.0-plus"]` | `[]` | XML comment handling |
| `CATOSL.sep` | `["uoi-ncsa"]` | `[]` | Non-standard file extension |

### Category B: Partial Missing Detection

Files where Rust finds fewer detections than Python.

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `NASA-1.3.t1` | `["nasa-1.3", "nasa-1.3", "nasa-1.3"]` | `["nasa-1.3", "nasa-1.3"]` | Detection count mismatch |
| `APSL-1.2.t1` | `["apsl-1.2", "apsl-1.2"]` | `["apsl-1.2"]` | Detection count mismatch |

**Note:** The NASA-1.3.t1 file is a single 291-line NASA-1.3 license text. The expected 3 detections likely represent different matching rules or regions within the same license (not 3 separate licenses). Investigation needed to understand Python's detection breakdown.

### Category C: Expression Combination Issues

Files where matches should be combined into a single expression with OR.

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `Ruby.t2` | `["gpl-2.0 OR other-copyleft"]` | `["gpl-2.0", "other-copyleft"]` | Matches split into separate detections |
| `NPL-1.1.t1` | `["npl-1.1"]` | `["npl-1.1", "mpl-1.1"]` | Extra detection (inverse) |

**Note:** For Ruby.t2, the expected output is a **single** expression `gpl-2.0 OR other-copyleft` (not separate detections). The file contains a dual-licensing clause: "under either the terms of the GPLv2 or the conditions below". Python combines these into one detection with an OR expression. Rust is creating separate detections for each match.

---

## Root Cause Analysis

### Root Cause 1: Short Header Match Thresholds

**Symptoms:** Short license headers (like Apache-2.0 headers) are not detected.

**Evidence:** `Apache-2.0-Header.t2`:
```
Copyright [yyyy] [name of owner]

Licensed under the Apache License, Version 2.0 (the "License"); you
may not use this file except in compliance with the License.

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
implied. See the License for the specific language governing
permissions and limitations under the License.
```

**Analysis:**
1. This is a short Apache-2.0 header (missing the "You may obtain" clause)
2. Python has specific rules for short headers with lower thresholds
3. Rust's candidate selection may filter these out due to:
   - `min_matched_length` threshold too high
   - `min_high_matched_length` threshold too high
   - Coverage threshold filtering

**Code Locations:**
- `src/license_detection/seq_match.rs:346-354` - min_matched_length checks
- `src/license_detection/rules/thresholds.rs` - threshold computation

**Fix Strategy:**
1. Check if Apache-2.0 header rules have correct `min_matched_length` values
2. Verify `min_high_matched_length_unique` is appropriate for short rules
3. Ensure short rules are not filtered by `filter_too_short_matches()`

### Root Cause 2: File Extension Filtering

**Symptoms:** Files with non-standard extensions are not scanned.

**Evidence:** `CATOSL.sep` - A `.sep` file containing:
```
/*
  Copyright (c) 2009 Actian Corporation

This file is distributed under the CA Trusted Open Source License(CATOSL).
For the exact terms of the license go to http://ca.com/opensource/catosl 
...
*/
```

**Analysis:**
1. The file contains a clear license reference
2. Python detects `uoi-ncsa` (CATOSL is based on NCSA/UOI license)
3. Rust returns empty - the file may not be processed correctly

**Investigation Needed:**
1. Is the file being read correctly?
2. Is the text being tokenized?
3. Is there a rule for CATOSL/uoi-ncsa reference?
4. Is the Aho-Corasick matcher finding the rule?

**Fix Strategy:**
1. Add debug test to trace the detection pipeline for this file
2. Verify file is read by `extract_text_from_file()`
3. Check if `uoi-ncsa` rule exists in the index
4. Trace matchers to see which one should catch this

### Root Cause 3: BASIC REM Comment Handling (Not XML Comments)

**Symptoms:** License text in BASIC-style REM comments is not detected.

**Evidence:** `gpl-3+-with-rem-comment.xml` expects `["gpl-3.0-plus"]` but gets `[]`

**Actual File Content:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<script:module xmlns:script="http://openoffice.org/2000/script" ...>
REM                        *****  BASIC  *****
REM                           ***** Canzeley *****
REM              Copyright (C) 2008, 2009 Dr. Michael Stehmann
REM This program is free software: you can redistribute it and/or modify it
REM under the terms of the GNU General Public License as published by the
REM Free Software Foundation, either version 3 of the License, or any later version.
...
```

**Analysis:**
1. The file is an OpenOffice BASIC script embedded in XML
2. Comments are BASIC-style `REM` lines, NOT XML comments (`<!-- ... -->`)
3. The license text is within the REM comment block
4. The tokenizer may not properly handle REM-style comments
5. Python's tokenizer has special handling for BASIC REM comments

**Code Locations:**
- `src/license_detection/tokenize.rs` - Tokenization logic
- `reference/scancode-toolkit/src/licensedcode/tokenize.py` - Python tokenizer for comparison

**Fix Strategy:**
1. Add REM comment stripping to tokenizer (similar to how `//` and `#` comments are handled)
2. Check Python's `index_tokenizer()` for REM handling
3. Verify tokens are correctly extracted from REM-prefixed lines

### Root Cause 4: Detection Merging Issues

**Symptoms:** Multiple occurrences of the same license are merged into fewer detections.

**Evidence:** `NASA-1.3.t1` expects 3 detections, gets 2.

**Analysis:**
1. The file is 291 lines - a full NASA-1.3 license
2. Python detects `nasa-1.3` three times (likely at different positions)
3. Rust merges them into 2 detections

**Possible Causes:**
1. Match merging in `merge_overlapping_matches()` is too aggressive
2. Detection grouping in `group_matches_by_region()` combines matches
3. Expression deduplication within a detection removes duplicates

**Code Locations:**
- `src/license_detection/match_refine.rs:196-339` - `merge_overlapping_matches()`
- `src/license_detection/detection.rs:150-207` - `group_matches_by_region()`

**Fix Strategy:**
1. Determine if Python's 3 detections are at different locations
2. If they're at different locations, ensure Rust doesn't merge them
3. If they're overlapping, ensure merging matches Python's behavior

### Root Cause 5: OR Expression Not Preserved from Rules

**Symptoms:** Matches from rules with OR in their `license_expression` are split into separate detections instead of combined.

**Evidence:** `Ruby.t2`:
```
MyProg is copyrighted free software by Go Gopher and contributors.
You can redistribute it
and/or modify it under either the terms of the GPLv2 or the conditions
below:
...
```

Expected: Single detection with expression `gpl-2.0 OR other-copyleft`
Actual: Two separate detections: `gpl-2.0` and `other-copyleft`

**Analysis:**
1. The file contains dual-licensing text: "under either the terms of the GPLv2 or the conditions below"
2. Python finds a single rule with `license_expression: "gpl-2.0 OR other-copyleft"`
3. Rust appears to either:
   - Find multiple rules (one for gpl-2.0, one for other-copyleft)
   - Or find one rule but split the OR expression during detection
4. `determine_license_expression()` combines matches with AND by default, not OR
5. The rule's `license_expression` should be preserved when a match is found

**Code Locations:**
- `src/license_detection/detection.rs:666-678` - `determine_license_expression()` (uses AND)
- `src/license_detection/expression.rs` - Expression combination logic
- `src/license_detection/models.rs` - LicenseMatch stores `license_expression`

**Fix Strategy:**
1. Investigate which rules match Ruby.t2 (single OR rule vs multiple rules)
2. If single rule with OR expression: preserve OR in the detection
3. If multiple rules: need to detect they should be combined with OR
4. Check Python's behavior for dual-license detection grouping

---

## Implementation Plan

### Step 1: Create Investigation Tests

**Note:** Investigation test file already exists at `src/license_detection/missing_detection_investigation_test.rs`. It currently has tests for e2fsprogs.txt (lgpl-2.1-plus detection). Add tests for the new failure cases:

```rust
// Add to src/license_detection/missing_detection_investigation_test.rs

#[test]
fn debug_apache_header_t2() {
    // Trace detection pipeline for Apache-2.0-Header.t2
    // Check: tokenization, candidate selection, match refinement
    // File: testdata/license-golden/datadriven/external/glc/Apache-2.0-Header.t2
}

#[test]
fn debug_catosl_sep() {
    // Trace detection pipeline for CATOSL.sep
    // Check: file reading, rule existence, matcher output
    // File: testdata/license-golden/datadriven/external/atarashi/CATOSL.sep
}

#[test]
fn debug_gpl_rem_comment_xml() {
    // Trace detection pipeline for gpl-3+-with-rem-comment.xml
    // Check: REM comment tokenization, GPL rule matching
    // File: testdata/license-golden/datadriven/external/licensecheck/devscripts/gpl-3+-with-rem-comment.xml
}

#[test]
fn debug_nasa_1_3_t1() {
    // Trace why NASA-1.3 is detected 2 times instead of 3
    // Check: match merging, detection grouping
    // File: testdata/license-golden/datadriven/external/glc/NASA-1.3.t1
}

#[test]
fn debug_ruby_t2() {
    // Trace why Ruby.t2 produces separate detections instead of OR expression
    // Check: which rules match, how expression is determined
    // File: testdata/license-golden/datadriven/external/glc/Ruby.t2
}
```

### Step 2: Fix Short Header Detection

**Files:** `src/license_detection/seq_match.rs`, `src/license_detection/rules/thresholds.rs`

1. Add debug logging to `compute_candidates_with_msets()` for short rules
2. Check Apache-2.0 header rules' `min_matched_length` values
3. Verify `filter_too_short_matches()` logic

**Validation:**
```bash
cargo test debug_apache_header_t2 --lib -- --nocapture
cargo test test_golden_external_part6 --lib  # Should pass Apache-2.0-Header.t2
```

### Step 3: Fix File Extension Handling

**Files:** `src/scanner/mod.rs`, `src/utils/file_text.rs`

1. Verify `.sep` files are being scanned
2. Check if file extension filtering excludes any extensions
3. Ensure text extraction works for all text-based files

**Validation:**
```bash
cargo test debug_catosl_sep --lib -- --nocapture
```

### Step 4: Fix Detection Merging

**Files:** `src/license_detection/detection.rs`

1. Analyze Python's detection count for NASA-1.3.t1
2. Compare Rust's match positions vs Python's
3. Adjust merging logic if needed

**Validation:**
```bash
cargo test debug_nasa_1_3_t1 --lib -- --nocapture
cargo test test_golden_external_part7 --lib  # Should pass NASA-1.3.t1
```

### Step 5: Fix OR Expression Combination

**Files:** `src/license_detection/detection.rs`, `src/license_detection/expression.rs`

1. Identify when matches should be combined with OR
2. Implement OR-aware expression combination
3. Handle dual-license rules correctly

**Validation:**
```bash
cargo test test_golden_external_part7 --lib  # Should pass Ruby.t2
```

---

## Test Cases to Verify

### Primary Test Cases (Complete Missing)

| Test | Expected Result |
|------|-----------------|
| `Apache-2.0-Header.t2` | Detect `apache-2.0` and `warranty-disclaimer` |
| `gpl-3+-with-rem-comment.xml` | Detect `gpl-3.0-plus` |
| `CATOSL.sep` | Detect `uoi-ncsa` |

### Secondary Test Cases (Partial Missing)

| Test | Expected Result |
|------|-----------------|
| `NASA-1.3.t1` | Detect 3 occurrences of `nasa-1.3` |
| `APSL-1.2.t1` | Detect 2 occurrences of `apsl-1.2` |

**Note:** Files located at:
- `testdata/license-golden/datadriven/external/glc/NASA-1.3.t1`
- `testdata/license-golden/datadriven/external/glc/APSL-1.2.t1`

### Tertiary Test Cases (Expression Combination)

| Test | Expected Result |
|------|-----------------|
| `Ruby.t2` | Combine as `gpl-2.0 OR other-copyleft` |
| `BSL-1.0_or_MIT.txt` | Combine as `mit OR boost-1.0` |

---

## Testing Strategy

### Per-Fix Validation

1. Create debug test for specific failure
2. Implement fix
3. Run debug test to verify behavior change
4. Run related golden test subset
5. Run full golden test suite for regressions

### Full Suite Validation

**Note:** Full golden test suite runs are slow (~10+ minutes). Use targeted tests during development:

```bash
# Run a specific test suite part (faster than full suite)
cargo test test_golden_external_part6 --lib

# Run all golden tests (slow, use for final validation)
cargo test --lib license_detection::golden_test

# Count remaining failures
cargo test --lib license_detection::golden_test 2>&1 | grep -c "mismatch"
```

### Regression Prevention

Add specific regression tests:

```rust
#[test]
fn regression_apache_header_detection() {
    let text = "Copyright [yyyy] [name of owner]\n\nLicensed under the Apache License, Version 2.0...";
    let detections = engine.detect(text, false).unwrap();
    assert!(detections.iter().any(|d| d.matches.iter().any(|m| m.license_expression == "apache-2.0")));
}
```

---

## Code Locations Summary

| Component | File | Function |
|-----------|------|----------|
| Candidate selection | `src/license_detection/seq_match.rs` | `compute_candidates_with_msets()` |
| Match thresholds | `src/license_detection/rules/thresholds.rs` | `compute_thresholds_occurrences()`, `compute_thresholds_unique()` |
| Match filtering | `src/license_detection/match_refine.rs:98` | `filter_too_short_matches()` |
| Match merging | `src/license_detection/match_refine.rs:196-339` | `merge_overlapping_matches()` |
| Detection grouping | `src/license_detection/detection.rs:150-207` | `group_matches_by_region()` |
| Expression determination | `src/license_detection/detection.rs:666-678` | `determine_license_expression()` |
| Expression combination | `src/license_detection/expression.rs:628-666` | `combine_expressions()` |
| File scanning | `src/scanner/mod.rs` | File filtering |
| Text extraction | `src/utils/file_text.rs` | `extract_text_from_file()` |
| Tokenization | `src/license_detection/tokenize.rs` | Token extraction |
| Investigation tests | `src/license_detection/missing_detection_investigation_test.rs` | Debug tests |

---

## Quick Validation Commands

**Run specific investigation tests (fast):**
```bash
# Run individual debug tests
cargo test debug_apache_header_t2 --lib -- --nocapture
cargo test debug_catosl_sep --lib -- --nocapture
cargo test debug_gpl_rem --lib -- --nocapture
cargo test debug_nasa_1_3 --lib -- --nocapture
cargo test debug_ruby --lib -- --nocapture
```

**Run golden test for specific test file (slow):**
```bash
# Run specific golden test suite
cargo test test_golden_external_glc --lib
```

**Note:** The full golden test suite is split into multiple parts (`test_golden_external_part1` through `test_golden_external_part10`). Each part tests a subset of the external test files.

---

## Risk Assessment

### High Risk

1. **Threshold changes** - May cause false positives if lowered too much
2. **Merging logic** - Changes may affect many tests unpredictably

### Medium Risk

1. **File extension handling** - Could inadvertently scan binary files
2. **Expression combination** - May create invalid expressions

### Mitigation

1. Run full golden test suite after each fix
2. Add regression tests for each fixed case
3. Compare outputs with Python reference for edge cases

---

## Estimated Effort

| Task | Complexity | Time Estimate |
|------|------------|---------------|
| Investigation tests | Medium | 2-3 hours |
| Short header fix | Medium | 3-4 hours |
| File extension fix | Simple | 1-2 hours |
| Detection merging fix | Complex | 4-6 hours |
| OR expression fix | Medium | 3-4 hours |
| Testing & validation | Medium | 2-3 hours |

**Total:** 15-22 hours

---

## Success Criteria

1. All "complete missing detection" tests pass
2. All "partial missing detection" tests pass
3. No regressions in other golden tests
4. Regression tests added for each fixed case

---

## Verification Status

**Verified by:** AI Agent  
**Date:** 2026-03-01

### Code Locations Verified ✓

| Location | Status | Notes |
|----------|--------|-------|
| `src/license_detection/seq_match.rs:346-354` | ✓ Exists | Threshold checks confirmed |
| `src/license_detection/rules/thresholds.rs` | ✓ Exists | Full threshold computation |
| `src/license_detection/match_refine.rs:98` | ✓ Exists | `filter_too_short_matches()` |
| `src/license_detection/match_refine.rs:196-339` | ✓ Exists | `merge_overlapping_matches()` |
| `src/license_detection/detection.rs:150-207` | ✓ Exists | `group_matches_by_region()` |
| `src/license_detection/detection.rs:666-678` | ✓ Exists | `determine_license_expression()` |
| `src/license_detection/expression.rs` | ✓ Exists | Full expression handling |
| `src/utils/file_text.rs` | ✓ Exists | Text extraction |
| `src/scanner/mod.rs` | ✓ Exists | File processing |

### Test Files Verified ✓

| File | Status | Location |
|------|--------|----------|
| `Apache-2.0-Header.t2` | ✓ Exists | `testdata/license-golden/datadriven/external/glc/` |
| `Apache-2.0-Header.t2.yml` | ✓ Exists | Expects `["apache-2.0", "warranty-disclaimer"]` |
| `gpl-3+-with-rem-comment.xml` | ✓ Exists | `testdata/license-golden/datadriven/external/licensecheck/devscripts/` |
| `gpl-3+-with-rem-comment.xml.yml` | ✓ Exists | Expects `["gpl-3.0-plus"]` |
| `CATOSL.sep` | ✓ Exists | `testdata/license-golden/datadriven/external/atarashi/` |
| `CATOSL.sep.yml` | ✓ Exists | Expects `["uoi-ncsa"]` |
| `NASA-1.3.t1` | ✓ Exists | `testdata/license-golden/datadriven/external/glc/` |
| `NASA-1.3.t1.yml` | ✓ Exists | Expects 3 `nasa-1.3` detections |
| `APSL-1.2.t1` | ✓ Exists | `testdata/license-golden/datadriven/external/glc/` |
| `APSL-1.2.t1.yml` | ✓ Exists | Expects 2 `apsl-1.2` detections |
| `Ruby.t2` | ✓ Exists | `testdata/license-golden/datadriven/external/glc/` |
| `Ruby.t2.yml` | ✓ Exists | Expects `["gpl-2.0 OR other-copyleft"]` |

### Corrections Made to Plan

1. **Root Cause 3 (XML Comment):** CORRECTED - The file contains BASIC REM comments, not XML comments. The `.xml` extension is misleading; it's an OpenOffice BASIC script embedded in XML.

2. **Root Cause 5 (OR Expression):** ENHANCED - Added more detailed analysis including that `determine_license_expression()` uses AND combination by default.

3. **Test File Names:** CORRECTED - `APS L-1.2.t1` → `APSL-1.2.t1` (correct filename is `APSL-1.2.t1`).

4. **Investigation Test File:** NOTED - The file `missing_detection_investigation_test.rs` already exists with e2fsprogs tests.

### Python Reference Comparison

Threshold computation in `src/license_detection/rules/thresholds.rs` matches Python's `compute_thresholds_occurences()` in `reference/scancode-toolkit/src/licensedcode/models.py:2628-2668`:
- Both use same boundary values (3, 10, 30, 200)
- Both apply same coverage percentages (100, 80, 50)
- Both cap `min_high_matched_length` at `MIN_MATCH_HIGH_LENGTH` (3)
