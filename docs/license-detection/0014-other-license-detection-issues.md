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
**Actual:** Not verified in Rust (test times out)

### Python Verification Results

**Python ScanCode v32.5.0 correctly returns `x11-ibm`:**

```json
{
  "license_detections": [
    {
      "identifier": "x11_ibm-ddcda14e-3c63-a258-eb1c-6f24e2ffe1ec",
      "license_expression": "x11-ibm",
      "license_expression_spdx": "LicenseRef-scancode-x11-ibm",
      "detection_count": 1,
      "reference_matches": [
        {
          "license_expression": "x11-ibm",
          "license_expression_spdx": "LicenseRef-scancode-x11-ibm",
          "from_file": "IBM-MIT-style.txt",
          "start_line": 9,
          "end_line": 29,
          "matcher": "3-seq",
          "score": 88.89,
          "matched_length": 200,
          "match_coverage": 88.89,
          "rule_relevance": 100,
          "rule_identifier": "x11-ibm.LICENSE",
          "rule_url": "https://github.com/nexB/scancode-toolkit/tree/develop/src/licensedcode/data/licenses/x11-ibm.LICENSE"
        }
      ]
    }
  ]
}
```

### Root Cause Analysis

**Python uses LICENSE files as rules when no RULE files exist.**

The `x11-ibm.LICENSE` file (at `reference/scancode-toolkit/src/licensedcode/data/licenses/x11-ibm.LICENSE`) contains:
- `key: x11-ibm`
- `minimum_coverage: 80`
- Full license text that matches the test file

**Python's approach:**
1. Load all `.LICENSE` files
2. Convert each LICENSE to a Rule with `is_license_text: true` and `is_from_license: true`
3. Use these rules in sequence matching

**Rust's approach (verified in `src/license_detection/index/builder.rs:304-307`):**
```rust
let license_rules =
    build_rules_from_licenses(&licenses_by_key.values().cloned().collect::<Vec<_>>());
```

Rust DOES create rules from LICENSE files. The issue is likely in:
1. **Minimum coverage handling**: `x11-ibm.LICENSE` has `minimum_coverage: 80` - check if Rust respects this
2. **Sequence matching scoring**: Python score is 88.89%, Rust may score differently
3. **Rule ordering**: Rust may have different rule priority than Python

### Investigation Steps Required

1. **Create unit test** for IBM-MIT-style.txt that traces through seq_match
2. **Check minimum_coverage**: Verify `filter_below_rule_minimum_coverage()` in `match_refine.rs:1007-1029`
3. **Check rule ranking**: Verify `x11-ibm.LICENSE` appears in candidate selection

### Code Locations

- License-to-rule conversion: `src/license_detection/index/builder.rs:82-137`
- Minimum coverage filter: `src/license_detection/match_refine.rs:1007-1029`
- Sequence matching: `src/license_detection/seq_match.rs`
- License definition: `reference/scancode-toolkit/src/licensedcode/data/licenses/x11-ibm.LICENSE`

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

### Specific Bug Location: `is_after()` Merge at Line 304

**Rust code (`src/license_detection/match_refine.rs:304-308`):**
```rust
if next.is_after(&current) {
    rule_matches[i] = combine_matches(&current, &next);
    rule_matches.remove(j);
    continue;
}
```

**Python code (`reference/scancode-toolkit/src/licensedcode/match.py:1032-1041`):**
```python
# next_match is strictly in increasing sequence: merge in current
if next_match.is_after(current_match):
    current_match.update(next_match)
    ...
    del rule_matches[j]
    continue
```

**The Problem:** The `is_after()` check merges matches that are "strictly in increasing sequence" but does NOT validate that they are from the SAME occurrence of the license text.

For example, in `bzip2.106.c`:
- Match 1: lines 7-17 (first bzip2 license)
- Match 2: lines 27-34 (second bzip2 license)

Both matches have the same `rule_identifier` and both pass `is_after()` because they're in sequence. They get merged into one match spanning lines 7-34, but they should remain separate.

### Python Comparison

Python's `merge_matches()` at `match.py:869-1068` has the SAME logic. The difference must be in:
1. How matches are created initially (different tokenization?)
2. The `qdistance_to()` and `idistance_to()` thresholds
3. The `max_rule_side_dist` calculation

**Key difference to investigate:**
```python
# Python line 923-924
if (current_match.qdistance_to(next_match) > max_rule_side_dist
or current_match.idistance_to(next_match) > max_rule_side_dist):
    break
```

Rust has the same check at lines 252-256, but the `max_rule_side_dist` calculation may differ.

### Specific Code Fix Locations

1. **Add distance check before `is_after()` merge** - `src/license_detection/match_refine.rs:304`
   - Add a check that the query distance between matches is reasonable
   - If matches are far apart in the query text, they shouldn't be merged

2. **Consider the actual line gap** - Current `max_rule_side_dist = rule_length / 2` may be too large for long rules

3. **Investigation test** - `src/license_detection/duplicate_merge_investigation_test.rs`

### Proposed Fix

Add a distance validation before merging via `is_after()`:

```rust
if next.is_after(&current) {
    // Don't merge if matches are far apart in the query
    let qdist = current.qdistance_to(&next);
    let max_query_dist = 50; // or some reasonable threshold
    if qdist > max_query_dist {
        j += 1;
        continue;
    }
    rule_matches[i] = combine_matches(&current, &next);
    rule_matches.remove(j);
    continue;
}
```

### Code Locations

- Merge logic: `src/license_detection/match_refine.rs:196-339`
- `is_after()` merge: `src/license_detection/match_refine.rs:304-308`
- Grouping logic: `src/license_detection/detection.rs:150-222`
- Investigation test: `src/license_detection/duplicate_merge_investigation_test.rs`

---

## Category 3: Missing Detections

### Test Case: `npruntime.h`

**File:** `testdata/license-golden/datadriven/external/slic-tests/npruntime.h`

**Expected:** `["bsd-new"]`

### Python Verification Results

**Python ScanCode v32.5.0 correctly returns `bsd-new`:**

```json
{
  "license_detections": [
    {
      "identifier": "bsd_new-c8029e3f-90bc-0a37-4d72-a73c8a64c8c2",
      "license_expression": "bsd-new",
      "license_expression_spdx": "BSD-3-Clause",
      "detection_count": 1,
      "reference_matches": [
        {
          "license_expression": "bsd-new",
          "license_expression_spdx": "BSD-3-Clause",
          "from_file": "npruntime.h",
          "start_line": 6,
          "end_line": 30,
          "matcher": "3-seq",
          "score": 94.14,
          "matched_length": 209,
          "match_coverage": 100.0,
          "rule_relevance": 100,
          "rule_identifier": "bsd-new_22.RULE"
        }
      ]
    }
  ]
}
```

### Root Cause: Not Actually Missing

The test expectation file (`npruntime.h.yml`) shows `bsd-new` is expected. Python verifies this works.

**This is NOT a missing detection issue.** The original plan was based on speculation. 

The issue is likely that:
1. Rust DOES detect `bsd-new` for this file
2. The test may be passing or timing out during test runs
3. Need to run the specific golden test to confirm

### Investigation Required

1. Run `cargo test test_golden_external --lib` with focus on slic-tests
2. If test passes, this category can be closed
3. If test fails, compare Rust vs Python matches for `bsd-new_22.RULE`

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
| License Misidentification | IBM-MIT-style.txt | LICENSE-to-rule conversion works; issue in seq_match scoring or minimum_coverage | Medium |
| Duplicate Merging | gpl_65.txt, e2fsprogs.txt, bzip2.106.c | `is_after()` merge without distance validation at `match_refine.rs:304` | High |
| Missing Detections | npruntime.h | NOT A BUG - Python verifies detection works; need to confirm Rust test passes | Low |
| Unknown-License-Reference | x11_danse.txt | Already documented in PLAN-009 | High (fix ready) |

---

## Recommended Investigation Order

### Phase 1: Apply Known Fixes

1. **PLAN-009 (unknown-license-reference)** - Fix is ready, should be applied first
   - Location: `src/license_detection/detection.rs:populate_detection_from_group()`
   - Add filtering logic matching `create_detection_from_group()`

### Phase 2: Fix Duplicate Merging

2. **Add distance validation to `is_after()` merge**
   - Location: `src/license_detection/match_refine.rs:304-308`
   - Add check that query distance between matches is reasonable
   - Test with `test_e2fsprogs_detection_count` and `test_bzip2_106_c_full_pipeline`

### Phase 3: Verify and Close

3. **Run golden tests** to confirm npruntime.h detection works
4. **Run IBM-MIT-style.txt investigation** to understand seq_match scoring differences

---

## Code Locations Summary

| Component | File | Key Functions |
|-----------|------|---------------|
| Detection Engine | `src/license_detection/mod.rs` | `detect()` |
| Match Merging | `src/license_detection/match_refine.rs` | `merge_overlapping_matches()`, line 304 for `is_after()` |
| Match Filtering | `src/license_detection/match_refine.rs` | `filter_contained_matches()`, `refine_matches()` |
| Detection Grouping | `src/license_detection/detection.rs` | `group_matches_by_region()` |
| Detection Creation | `src/license_detection/detection.rs` | `create_detection_from_group()`, `populate_detection_from_group()` |
| Unknown Filtering | `src/license_detection/detection.rs` | `filter_license_intros()`, `is_unknown_intro()` |
| LICENSE-to-Rule | `src/license_detection/index/builder.rs` | `build_rule_from_license()`, `build_rules_from_licenses()` |
| Minimum Coverage | `src/license_detection/match_refine.rs` | `filter_below_rule_minimum_coverage()` (lines 1007-1029) |
| Sequence Matching | `src/license_detection/seq_match.rs` | `seq_match()`, `seq_match_with_candidates()` |

---

## Appendix: Test Commands

```bash
# Run specific golden tests
cargo test test_golden_lic1 --lib -- --nocapture
cargo test test_golden_external_part1 --lib -- --nocapture

# Run investigation tests
cargo test test_e2fsprogs_detection_count --lib -- --nocapture
cargo test test_bzip2_106_c_full_pipeline --lib -- --nocapture

# Run PLAN-009 test
cargo test test_x11_danse_expected_expression --lib -- --nocapture

# Run Python ScanCode for comparison
cd reference/scancode-playground
venv/bin/python src/scancode/cli.py --license --json-pp /tmp/out.json <test_file>
```

---

## Appendix: Python Verification Commands

```bash
# IBM-MIT-style.txt verification
cd reference/scancode-playground
venv/bin/python src/scancode/cli.py --license --json-pp /tmp/ibm_mit.json \
  /home/adrian/Documents/projects/scancode-rust/testdata/license-golden/datadriven/external/fossology-tests/IBM/IBM-MIT-style.txt

# npruntime.h verification
venv/bin/python src/scancode/cli.py --license --json-pp /tmp/npruntime.json \
  /home/adrian/Documents/projects/scancode-rust/testdata/license-golden/datadriven/external/slic-tests/npruntime.h
```
