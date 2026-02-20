# LIC1 Golden Test Failure Analysis

**Date**: 2026-02-20  
**Test Suite**: `license_detection::golden_test::golden_tests::test_golden_lic1`  
**Total Tests**: 291  
**Passed**: 224  
**Failed**: 67  
**Pass Rate**: 76.9%

## Executive Summary

The lic1 golden test suite has 67 failures across multiple categories. The failures can be grouped into 5 primary patterns:

| Pattern | Count | Description |
|---------|-------|-------------|
| 1. Extra/False Positive Detections | ~25 | Rust produces additional matches beyond expected |
| 2. Missing Detections | ~15 | Rust fails to detect expected licenses |
| 3. Expression Combination Mismatch | ~12 | License expressions combined differently |
| 4. UTF-8/Encoding Issues | 4 | Test files with non-UTF-8 content |
| 5. Match Deduplication Issues | ~11 | Duplicate or missing duplicate handling |

## Pattern Analysis

### Pattern 1: Extra/False Positive Detections (~25 failures)

**Symptoms**: Rust produces more matches than Python expects.

**Examples**:

- `COPYING.gplv3`: Expected `["gpl-3.0"]`, Got `["gpl-3.0", "gpl-3.0", "gpl-3.0-plus", "warranty-disclaimer", "gpl-1.0-plus", ...]`
- `gpl-2.0-plus_1.txt`: Expected `["gpl-2.0-plus"]`, Got `["gpl-1.0-plus", "gpl-2.0-plus"]`
- `gfdl-1.1_9.RULE`: Expected `["gfdl-1.1", "gpl-1.0-plus"]`, Got `["gfdl-1.1", "gfdl-1.1-plus", ...]`

**Root Cause**:

1. **GPL family rules are hierarchical** - The `gpl-1.0-plus` rule is matching text that Python only matches with `gpl-2.0-plus`. This suggests rule ordering or containment filtering differences.

2. **Embedded license detection** - Files like `COPYING.gplv3` contain embedded license snippets (GPLv3 preamble references older GPL versions). Rust is detecting these as separate matches.

**Code Files to Investigate**:

- `src/license_detection/match_refine.rs` - `filter_contained_matches()`, `merge_overlapping_matches()`
- `src/license_detection/detection.rs` - `is_false_positive()`, `analyze_detection()`
- `src/license_detection/aho_match.rs` - Match collection logic

**Python Reference**:

- `reference/scancode-toolkit/src/licensedcode/match.py` - Lines 800-1000 for match filtering
- `reference/scancode-toolkit/src/licensedcode/detection.py` - Lines 1162-1380 for false positive detection

### Pattern 2: Missing Detections (~15 failures)

**Symptoms**: Rust produces fewer matches than Python expects.

**Examples**:

- `gpl-2.0-plus_33.txt`: Expected 6 matches, Got 1 match
- `do-not-skip-short-gpl-matches.txt`: Expected 6 matches, Got 5 matches
- `gpl-2.0_82.RULE`: Expected 3 matches, Got 1 match

**Root Cause**:

1. **Match merging too aggressive** - Multiple occurrences of the same license are being merged into single matches when they should remain separate.

2. **Short match filtering** - The `filter_too_short_matches()` function in `match_refine.rs` may be incorrectly filtering valid short GPL matches.

**Representative Test - `gpl-2.0-plus_33.txt`**:

```
Expected: ["gpl-2.0-plus", "gpl-2.0-plus", "gpl-1.0-plus", "gpl-1.0-plus", "gpl-2.0-plus", "gpl-1.0-plus"]
Actual:   ["gpl-2.0-plus"]
```

The file contains multiple "License: GPLv2" markers that should each be detected separately.

**Code Files to Investigate**:

- `src/license_detection/match_refine.rs:128-250` - `merge_overlapping_matches()` function
- `src/license_detection/match_refine.rs:62-84` - `filter_too_short_matches()` function

### Pattern 3: Expression Combination Mismatch (~12 failures)

**Symptoms**: License expressions are combined differently between Rust and Python.

**Examples**:

- `eclipse-openj9_html2.html`:
  - Expected: `epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception`
  - Actual: `epl-2.0 OR apache-2.0 OR (gpl-2.0 WITH classpath-exception-2.0 AND gpl-2.0 WITH openjdk-exception)`

- `gpl_and_lgpl_and_gfdl-1.2.txt`:
  - Expected: `gpl-1.0-plus AND lgpl-2.0-plus AND gfdl-1.2`
  - Actual: `["gpl-1.0-plus", "lgpl-2.0-plus", "gfdl-1.2"]` (separate expressions)

**Root Cause**:

1. **Detection grouping logic** - The `group_matches_by_region()` function may be incorrectly splitting or combining matches that should be in the same detection group.

2. **Expression combination for AND/OR** - The `combine_expressions()` function or the logic that calls it may differ from Python.

**Representative Test - `eclipse-openj9_html2.html`**:
The HTML file contains a complex multi-choice license statement:

```
EPL-2.0 OR Apache-2.0 OR (GPL-2.0 WITH Classpath-exception AND GPL-2.0 WITH OpenJDK-exception)
```

Python correctly parses this as a flat OR. Rust is creating nested AND inside OR.

**Code Files to Investigate**:

- `src/license_detection/detection.rs:149-206` - `group_matches_by_region()`
- `src/license_detection/expression.rs:628-666` - `combine_expressions()`
- `src/license_detection/detection.rs:651-663` - `determine_license_expression()`

### Pattern 4: UTF-8/Encoding Issues (4 failures)

**Symptoms**: Test fails with "stream did not contain valid UTF-8".

**Files Affected**:

- `do-not_detect-licenses-in-archive.jar`
- `ecl-1.0.txt`
- `flt9.gif`
- `dm_ddf-v1_2_old.dtd` (different issue - returns empty)

**Root Cause**:
The golden test framework reads files with `fs::read_to_string()` which fails for non-UTF-8 content. The Python implementation handles binary files differently.

**Code Files to Investigate**:

- `src/license_detection/golden_test.rs:110-116` - File reading logic
- Consider using `fs::read()` for binary detection

### Pattern 5: Match Deduplication Issues (~11 failures)

**Symptoms**: Either too few or too many of the same license detected.

**Examples**:

- `fsf-free_and_fsf-free_and_fsf-free.txt`: Expected 3 `fsf-free`, Got 1
- `gpl-2.0_and_lgpl-2.0-plus.txt`: Expected combined expression, Got 2 separate detections
- `godot_COPYRIGHT.txt`: Expected 83 matches, Got 46 matches

**Root Cause**:

1. **Deduplication threshold** - The `remove_duplicate_detections()` function may be using incorrect criteria for determining if two detections are duplicates.

2. **Match coverage calculation** - The `compute_detection_coverage()` may affect how detections are ranked and filtered.

**Representative Test - `fsf-free_and_fsf-free_and_fsf-free.txt`**:
The file contains three separate "fsf-free" license statements. Python detects all three separately. Rust merges them into one.

**Code Files to Investigate**:

- `src/license_detection/detection.rs:891-909` - `remove_duplicate_detections()`
- `src/license_detection/detection.rs:1003-1012` - `compute_detection_identifier()`

## Detailed Failure Categories

### A. GPL Family License Issues (20+ failures)

GPL-related tests show the most failures due to:

1. Hierarchical rule structure (`gpl-1.0-plus` vs `gpl-2.0-plus` vs `gpl-3.0-plus`)
2. Embedded license text detection
3. Short match handling for GPL notices

**Specific Tests**:

- `gpl_19.txt`: Expected `gpl-1.0-plus`, Got `gpl-2.0` (version selection)
- `gpl-2.0_44.txt`: Expected `gpl-2.0`, Got `gpl-2.0-plus` (plus suffix handling)
- `gpl_12.txt`: Expected `["gpl-1.0-plus", "gpl-2.0-plus"]`, Got `["gpl-3.0-plus", "gpl-2.0-plus"]`

### B. GFDL License Issues (5 failures)

GFDL files have embedded text that triggers multiple license rules:

- `gfdl-1.1_1.RULE`: Expected 3 expressions, Got 10 (spurious matches)
- `gfdl-1.3_2.RULE`: Expected 2 expressions, Got 11

### C. Complex Expression Tests (8 failures)

Tests with compound license expressions (AND/OR/WITH):

- `eclipse-omr2.LICENSE`: Complex nested OR/AND expression
- `eclipse-openj9.LICENSE`: Multi-clause license with exceptions

### D. Large File Tests (2 failures)

Large copyright files with many license entries:

- `godot_COPYRIGHT.txt`: 83 expected → 46 actual
- `godot2_COPYRIGHT.txt`: 88 expected → 43 actual

## Recommendations

### Priority 1: Fix Match Containment/Merging Logic

The `merge_overlapping_matches()` function in `match_refine.rs` appears to be too aggressive, merging matches that should remain separate.

**Action**:

1. Review Python's `merge_matches()` at `match.py:800-910`
2. Compare containment detection logic
3. Add debug logging to track merge decisions

### Priority 2: Fix False Positive Detection

The `is_false_positive()` function in `detection.rs` may not be correctly identifying all false positives.

**Action**:

1. Compare with Python's `is_false_positive()` at `detection.py:1162-1380`
2. Verify rule relevance and length thresholds match
3. Test edge cases with short GPL rules

### Priority 3: Fix Expression Combination

For complex expressions, the grouping and combination logic differs from Python.

**Action**:

1. Review `group_matches_by_region()` threshold handling
2. Compare `combine_expressions()` with Python's `combine_expressions()`
3. Add tests for complex nested expressions

### Priority 4: Handle Binary Files

Add proper handling for non-UTF-8 files in the test framework.

**Action**:

1. Update `golden_test.rs` to use binary detection
2. Skip or handle binary files appropriately
3. Match Python's behavior for jar/gif/etc. files

## Test Debugging Commands

```bash
# Run specific failing test with debug output
cargo test --release -q --lib debug_gpl_12 -- --nocapture

# Run with verbose output for a single file
cargo run -- testdata/license-golden/datadriven/lic1/gpl_19.txt

# Compare with Python
cd reference/scancode-toolkit
./scancode -clp testdata/licensedcode/data/datadriven/lic1/gpl_19.txt
```

## Files Requiring Changes

1. `src/license_detection/match_refine.rs`
   - `merge_overlapping_matches()` - Lines 128-250
   - `filter_too_short_matches()` - Lines 62-84
   - `filter_contained_matches()` - Containment logic

2. `src/license_detection/detection.rs`
   - `is_false_positive()` - Lines 310-381
   - `group_matches_by_region()` - Lines 149-206
   - `remove_duplicate_detections()` - Lines 891-909

3. `src/license_detection/expression.rs`
   - `combine_expressions()` - Lines 628-666
   - Expression parsing and combination

4. `src/license_detection/golden_test.rs`
   - UTF-8 handling - Lines 110-116

## Conclusion

The 67 failures stem from 5 root causes, with match containment/merging and false positive detection being the most impactful. The Rust implementation needs to more closely follow Python's behavior for:

1. When to merge overlapping matches
2. How to identify and filter false positives
3. How to group matches into detection regions
4. How to handle duplicate detections

Addressing these will likely resolve the majority of failures.
