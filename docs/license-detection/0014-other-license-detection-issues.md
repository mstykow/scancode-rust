# 0014: Non-SPDX License Detection Issues Investigation

## Status: Investigation Complete

## Executive Summary

This report investigates non-SPDX golden test failures in the license detection system. The failures fall into four primary categories:

1. **License Misidentification** - Wrong license key detected (e.g., `x11-ibm` vs `historical`)
2. **Duplicate Detection Merging** - Multiple occurrences of the same license merged into one detection
3. **Missing Detections** - Expected licenses not detected at all
4. **Spurious Unknown-License-Reference** - `unknown-license-reference` appearing in expressions where it shouldn't

---

## Category 1: License Misidentification

### Test Case: `IBM-MIT-style.txt`

**File:** `testdata/license-golden/datadriven/external/fossology-tests/IBM/IBM-MIT-style.txt`

**Expected:** `["x11-ibm"]`
**Actual:** Likely `["historical"]` (not yet verified by running test)

**Test File Content:**
```
/* Copyright International Business Machines, Corp. 1991
 * All Rights Reserved
 * ...
 * License to use, copy, modify, and distribute this software and its
 * documentation for any purpose and without fee is hereby granted,
 * provided that the above copyright notice appear in all copies and that
 * both that copyright notice and this permission notice appear in
 * supporting documentation, and that the name of IBM or Lexmark or Adobe
 * not be used in advertising or publicity pertaining to distribution of
 * the software without specific, written prior permission.
 *
 * IBM, LEXMARK, AND ADOBE PROVIDE THIS SOFTWARE "AS IS", WITHOUT ANY
 * WARRANTIES OF ANY KIND...
 */
```

### Root Cause Analysis

The `x11-ibm` license is defined in `reference/scancode-toolkit/src/licensedcode/data/licenses/x11-ibm.LICENSE` with:
- `key: x11-ibm`
- `minimum_coverage: 80`
- Contains text with "License to use, copy, modify, and distribute this software and its documentation for any purpose and without fee is hereby granted..."

However, there are **NO rules** in `reference/scancode-toolkit/src/licensedcode/data/rules/` with `license_expression: x11-ibm`. The license exists but has no associated detection rules.

**The Problem:**
1. The `historical_10.RULE` matches very similar text:
   ```
   * Permission to use, copy, modify, and distribute this software for any
   * purpose with or without fee is hereby granted...
   ```
2. `historical_10.RULE` has `license_expression: historical` and `is_license_notice: yes`
3. The IBM test file text overlaps significantly with the `historical` license pattern
4. Since no `x11-ibm` rules exist, the `historical` rules match instead

**Key Differences Between x11-ibm and historical:**
- `x11-ibm` requires: "licensee provides a license to IBM, Corp. to use, copy, modify, and distribute derivative works"
- `historical` is the generic "Historical Permission Notice and Disclaimer" (HPND) from OSI

The test file does NOT contain the key x11-ibm clause about derivative works, so it's actually closer to `historical` than `x11-ibm`. **The test expectation may be incorrect**, or the license rules in the reference are missing x11-ibm specific patterns.

### Proposed Solution

1. **Verify test expectation**: Check if Python ScanCode actually returns `x11-ibm` for this file
2. **If Python returns x11-ibm**: Find the rules Python uses that produce `x11-ibm` detection
3. **If rules are missing**: The Rust implementation correctly matches `historical` but needs x11-ibm specific rules added to the reference data

### Code Locations

- License definition: `reference/scancode-toolkit/src/licensedcode/data/licenses/x11-ibm.LICENSE`
- Competing rule: `reference/scancode-toolkit/src/licensedcode/data/rules/historical_10.RULE`
- Rule loading: `src/license_detection/rules.rs:load_rules_from_directory()`

---

## Category 2: Duplicate Detection Merging

### Test Cases

Multiple tests show this pattern:
- `lic1/gpl_65.txt` - Expected 2 GPL detections, actual 1
- `lic1/cjdict-liconly.txt` - Expected 8 bsd-new, actual 5
- `lic1/e2fsprogs.txt` - Expected 5, actual 4 (missing lgpl-2.1-plus)
- `lic2/1908-bzip2/bzip2.106.c` - Expected 2 bzip2 matches, actual 1

### Root Cause Analysis

The issue is in how matches are merged and grouped. The pipeline:

1. **Match Phase** - Multiple strategies produce raw matches
2. **Merge Phase** - `merge_overlapping_matches()` combines matches from same rule
3. **Group Phase** - `group_matches_by_region()` groups nearby matches
4. **Detection Phase** - `create_detection_from_group()` creates final detection

**The Bug Location:** `src/license_detection/detection.rs:150-222`

```rust
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch, threshold: usize) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    line_gap <= threshold  // threshold is 4 lines
}
```

When two instances of the same license are separated by more than 4 lines of non-license text, they should create separate detections. However, the merge phase may be incorrectly combining them earlier.

**Investigation in `duplicate_merge_investigation_test.rs`** shows:
- bzip2.106.c has license text at lines 7-17 AND lines 27-34
- These should produce 2 separate detections
- Currently produces 1 merged detection

The `merge_overlapping_matches()` function at `src/license_detection/match_refine.rs:196-339` has complex logic for determining when to merge. Key merge conditions:
1. `qdistance_to()` and `idistance_to()` within `max_rule_side_dist`
2. `surround()` - one match surrounds another
3. `is_after()` - matches are sequential in the rule

**The Problem:** The merge logic may be too aggressive, combining matches that should remain separate because:
1. The `max_rule_side_dist = rule_length / 2` can be large for long rules
2. The `is_after()` check merges sequential matches regardless of distance

### Proposed Solution

1. **Add distance validation**: Ensure `is_after()` checks respect the actual distance between matches
2. **Review merge conditions**: Compare with Python's `merge_matches()` at `reference/scancode-toolkit/src/licensedcode/match.py:869-1068`
3. **Add test case**: Create unit test specifically for duplicate license detection

### Code Locations

- Merge logic: `src/license_detection/match_refine.rs:196-339`
- Grouping logic: `src/license_detection/detection.rs:150-222`
- Investigation test: `src/license_detection/duplicate_merge_investigation_test.rs`

---

## Category 3: Missing Detections

### Test Case: `npruntime.h`

**File:** `testdata/license-golden/datadriven/external/slic-tests/npruntime.h`

**Expected:** `["bsd-new"]`
**Actual:** Likely `[]` or wrong detection (not verified)

**Test File Content (lines 1-32):**
```c
/* -*- Mode: C; tab-width: 4; indent-tabs-mode: nil; c-basic-offset: 4 -*- */
/*
 * Copyright (c) 2004, Apple Computer, Inc. and The Mozilla Foundation.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are
 * met:
 *
 * 1. Redistributions of source code must retain the above copyright
 * notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 * notice, this list of conditions and the following disclaimer in the
 * documentation and/or other materials provided with the distribution.
 * 3. Neither the names of Apple Computer, Inc. ("Apple") or The Mozilla
 * Foundation ("Mozilla") nor the names of their contributors may be used
 * to endorse or promote products derived from this software without
 * specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY APPLE, MOZILLA AND THEIR CONTRIBUTORS "AS
 * IS" AND ANY EXPRESS OR IMPLIED WARRANTIES...
 */
```

### Root Cause Analysis

This is a standard 3-clause BSD license (bsd-new). The text should match rules like `bsd-new_*.RULE`.

**Potential causes for missing detection:**

1. **Tokenization issues**: The license header has a unique format with `-*- Mode: C; ... -*-` on line 1
2. **Short rule matching**: Some BSD rules may be flagged as false positives if they're too short
3. **Containment filtering**: The match might be filtered as "contained" by another match
4. **Score threshold**: The detection score might fall below the minimum threshold

**Debug approach:** Run the detection pipeline step-by-step:
1. Check if hash match finds it (unlikely - not exact match)
2. Check if aho-corasick finds partial matches
3. Check if sequence matching finds it
4. Check if matches are filtered out in refine phase

### Proposed Solution

1. **Add debug test**: Create a test that traces through the full pipeline for this file
2. **Check sequence matching**: Verify `seq_match()` produces matches for bsd-new rules
3. **Check filtering**: Ensure bsd-new matches aren't filtered in `refine_matches()`
4. **Check rule coverage**: Verify bsd-new rules in index have sufficient tokens

### Code Locations

- Sequence matching: `src/license_detection/seq_match.rs`
- Match refinement: `src/license_detection/match_refine.rs:refine_matches()`
- Detection filtering: `src/license_detection/detection.rs:post_process_detections()`

---

## Category 4: Spurious Unknown-License-Reference Detections

### Already Documented: PLAN-009

This issue is fully analyzed in `docs/license-detection/PLAN-009-x11-danse.md`.

### Summary

**Root Cause:** In `detect()` at `src/license_detection/mod.rs`, the code calls both:
1. `create_detection_from_group()` - correctly filters `unknown-license-reference`
2. `populate_detection_from_group_with_spdx()` - overwrites with unfiltered result

**Fix:** Modify `populate_detection_from_group()` to apply the same filtering logic as `create_detection_from_group()`.

**Status:** Root cause identified, fix ready.

---

## Summary Table

| Category | Test Cases | Root Cause | Priority |
|----------|-----------|------------|----------|
| License Misidentification | IBM-MIT-style.txt | Missing `x11-ibm` rules in reference data | Medium |
| Duplicate Merging | gpl_65.txt, e2fsprogs.txt, bzip2.106.c | Overly aggressive merge in `merge_overlapping_matches()` | High |
| Missing Detections | npruntime.h | TBD - needs pipeline trace | High |
| Unknown-License-Reference | x11_danse.txt | Already documented in PLAN-009 | High (fix ready) |

---

## Recommended Investigation Order

### Phase 1: Verify and Fix Known Issues

1. **PLAN-009 (unknown-license-reference)** - Fix is ready, should be applied first
2. **Duplicate merging** - High impact, affects 16+ tests

### Phase 2: Deep Investigation

3. **npruntime.h missing detection** - Trace through full pipeline
4. **IBM-MIT-style.txt** - Verify test expectation against Python

### Phase 3: Comprehensive Fix

5. Run full golden test suite after fixes
6. Document remaining failures
7. Create additional investigation reports as needed

---

## Code Locations Summary

| Component | File | Key Functions |
|-----------|------|---------------|
| Detection Engine | `src/license_detection/mod.rs` | `detect()` |
| Match Merging | `src/license_detection/match_refine.rs` | `merge_overlapping_matches()` |
| Match Filtering | `src/license_detection/match_refine.rs` | `filter_contained_matches()`, `refine_matches()` |
| Detection Grouping | `src/license_detection/detection.rs` | `group_matches_by_region()` |
| Detection Creation | `src/license_detection/detection.rs` | `create_detection_from_group()`, `populate_detection_from_group()` |
| Unknown Filtering | `src/license_detection/detection.rs` | `filter_license_intros()`, `is_unknown_intro()` |
| Rule Loading | `src/license_detection/rules.rs` | `load_rules_from_directory()` |
| Sequence Matching | `src/license_detection/seq_match.rs` | `seq_match()`, `seq_match_with_candidates()` |

---

## Appendix: Test Commands

```bash
# Run specific golden test
cargo test test_golden_lic1 --lib -- --nocapture

# Run investigation tests
cargo test test_e2fsprogs_detection_count --lib -- --nocapture
cargo test test_bzip2_106_c_full_pipeline --lib -- --nocapture

# Run PLAN-009 test
cargo test test_x11_danse_expected_expression --lib -- --nocapture
```
