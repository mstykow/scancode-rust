# PLAN-023: Golden Test Failure Analysis - Summary

**Date**: 2026-02-20  
**Status**: Analysis Complete  
**Scope**: All license detection golden test suites

## Executive Summary

| Suite | Tests | Passed | Failed | Pass Rate |
|-------|-------|--------|--------|-----------|
| lic1 | 291 | 224 | 67 | 76.9% |
| lic2 | 853 | 775 | 78 | 90.9% |
| lic3 | 292 | 250 | 42 | 85.6% |
| lic4 | 350 | 285 | 65 | 81.4% |
| **Total** | **1786** | **1534** | **252** | **85.9%** |

Detailed analysis in:

- [PLAN-023-failure-analysis-lic1.md](PLAN-023-failure-analysis-lic1.md)
- [PLAN-023-failure-analysis-lic2.md](PLAN-023-failure-analysis-lic2.md)
- [PLAN-023-failure-analysis-lic3.md](PLAN-023-failure-analysis-lic3.md)
- [PLAN-023-failure-analysis-lic4.md](PLAN-023-failure-analysis-lic4.md)

---

## Cross-Suite Failure Patterns

### Pattern A: Match Merging/Deduplication (Estimated 80+ failures)

**Symptoms**:

- Rust merges matches that Python keeps separate (fewer detections)
- Rust keeps separate matches that Python merges (duplicate expressions)

**Root Cause**: The `merge_overlapping_matches()` and `remove_duplicate_detections()` functions differ from Python's logic.

**Python Behavior** (from `match.py:869-1068`):

- Uses `qdistance_to()` and `idistance_to()` for merge distance
- `max_rule_side_dist = min((rule_length // 2) or 1, max_dist)` threshold
- Merges based on token proximity AND expression equality

**Current Rust Behavior**:

- Merges based primarily on token overlap
- May not have equivalent `idistance_to()` distance calculation
- Deduplication may use expression instead of location

**Files to Fix**:

- `src/license_detection/match_refine.rs:128-229` - `merge_overlapping_matches()`
- `src/license_detection/detection.rs:891-909` - `remove_duplicate_detections()`

**Evidence from Suites**:

- lic1 Pattern 1, 2, 5 (~40 failures)
- lic2 Pattern 1, 6 (~45 failures)
- lic3 Pattern 1, 2, 7 (~20 failures)
- lic4 Pattern 1 (~22 failures)

---

### Pattern B: Extra/False Positive Detections (Estimated 40+ failures)

**Symptoms**: Rust detects additional licenses not in Python output:

- `warranty-disclaimer` appearing unexpectedly
- `unknown-license-reference` over-detection
- `proprietary-license` extra matches
- Exception components alongside combined expressions (e.g., `gpl-2.0 WITH exception` AND `gpl-2.0` separately)

**Root Cause**: False positive filtering and containment filtering differ.

**Files to Fix**:

- `src/license_detection/match_refine.rs:249-388` - `filter_contained_matches()`, `filter_overlapping_matches()`
- `src/license_detection/match_refine.rs:698-775` - `filter_false_positive_license_lists_matches()`
- `src/license_detection/unknown_match.rs` - Unknown license thresholds

**Evidence from Suites**:

- lic1 Pattern 1 (~25 failures)
- lic2 Pattern 3 (~20 failures)
- lic3 Pattern 4 (~5 failures)
- lic4 Pattern 4 (~6 failures)

---

### Pattern C: Missing Detection - Complete Failure (Estimated 20+ failures)

**Symptoms**: Rust returns `[]` where Python detects licenses.

**Examples**:

- `isc_only.txt` - ISC license reference not detected
- `warranty-disclaimer_1.txt` - Short warranty text not matched
- `lgpl_21.txt` - LGPL reference not matched
- `mit_additions_1.c` - MIT with modifications not matched

**Root Cause**:

1. Short license reference rules may be filtered as "too short"
2. Modified license text may not match due to sequence matching thresholds
3. Some rules may not be loaded or indexed correctly

**Files to Fix**:

- `src/license_detection/match_refine.rs:62-84` - `filter_too_short_matches()`
- `src/license_detection/seq_match.rs` - Fuzzy matching thresholds
- `src/license_detection/index/builder.rs` - Rule loading

**Evidence from Suites**:

- lic1 Pattern 2 (~15 failures)
- lic2 Pattern 2 (~7 failures)
- lic3 Pattern 5 (~3 failures)
- lic4 Pattern 2 (~5 failures)

---

### Pattern D: Expression Combination Mismatch (Estimated 25+ failures)

**Symptoms**:

- License expressions combined with wrong operators
- Missing parentheses or extra parentheses
- Complex nested expressions not simplified correctly

**Examples**:

- Expected `A OR B OR C`, Got `A OR (B AND C)`
- Expected `license`, Got `license-fallback`
- Expected `A WITH B`, Got `A WITH B, A` (components not filtered)

**Root Cause**:

1. `group_matches_by_region()` may split/combine differently
2. `combine_expressions()` may not match Python's logic
3. Exception containment not handled (`A WITH B` should subsume `A`)

**Files to Fix**:

- `src/license_detection/detection.rs:149-206` - `group_matches_by_region()`
- `src/license_detection/expression.rs:628-666` - `combine_expressions()`

**Evidence from Suites**:

- lic1 Pattern 3 (~12 failures)
- lic3 Pattern 3 (~5 failures)
- lic4 Pattern 3, 5 (~9 failures)

---

### Pattern E: UTF-8/Binary File Handling (16 failures)

**Symptoms**: Test fails with "stream did not contain valid UTF-8"

**Files Affected**:

- `.class` files (Java bytecode)
- `.pdf` files
- `.gif` files
- Files with encoding issues (e.g., `ï¿œ` characters)

**Root Cause**: `fs::read_to_string()` fails on non-UTF-8 content. Python handles binary files with text extraction.

**Fix**: Use `fs::read()` with lossy UTF-8 conversion or add text extraction for binary formats.

**Files to Fix**:

- `src/license_detection/golden_test.rs:110-116` - File reading logic

**Evidence from Suites**:

- lic1 Pattern 4 (4 failures)
- lic2 Pattern 5 (5 failures)
- lic3 Pattern 6 (2 failures)
- lic4 Pattern 6 (5 failures)

---

## Priority Fix Order

### Priority 1: Match Merging Logic (Highest Impact)

This is the single largest source of failures across all suites.

**Action Items**:

1. Read Python's `merge_matches()` at `match.py:869-1068`
2. Implement `idistance_to()` method for index-based distance
3. Add `max_rule_side_dist` threshold: `min((rule_length // 2) or 1, max_dist)`
4. Update `merge_overlapping_matches()` to use distance-based merging
5. Fix `remove_duplicate_detections()` to dedupe by location, not expression

**Test Cases to Verify**:

- `lic1/gpl-2.0-plus_33.txt` - 6 expected vs 1 actual
- `lic2/bsd-new_17.txt` - duplicate detection expected
- `lic3/mit_18.txt` - 3 expected vs 1 actual
- `lic4/gpl-2.0-plus_and_gpl-2.0-plus.txt` - 2 vs 1

---

### Priority 2: False Positive/Containment Filtering

Second largest impact, especially for extra detections.

**Action Items**:

1. Compare `filter_contained_matches()` with Python
2. Implement exception containment (if `A WITH B` exists, filter standalone `A`)
3. Review `filter_false_positive_license_lists_matches()` thresholds
4. Tighten unknown license matching in `unknown_match.rs`

**Test Cases to Verify**:

- `lic1/COPYING.gplv3` - extra gpl variants
- `lic2/apache-1.1_1.txt` - extra `mx4j`
- `lic3/lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt` - exploded components
- `lic4/ijg.txt` - extra warranty-disclaimer

---

### Priority 3: Short License Reference Detection

Causes complete detection failures for simple license tags.

**Action Items**:

1. Verify ISC, LGPL, warranty-disclaimer rules exist in index
2. Check if `filter_too_short_matches()` is incorrectly filtering valid rules
3. Review rule loading for license tag/reference rules

**Test Cases to Verify**:

- `lic4/isc_only.txt` - empty vs `isc`
- `lic4/warranty-disclaimer_1.txt` - empty vs `warranty-disclaimer`
- `lic4/lgpl_21.txt` - empty vs `lgpl-2.0-plus`

---

### Priority 4: Expression Combination

Affects complex multi-license files.

**Action Items**:

1. Compare `group_matches_by_region()` with Python's `group_matches()`
2. Review `combine_expressions()` for operator precedence issues
3. Add expression normalization for known patterns

**Test Cases to Verify**:

- `lic1/eclipse-openj9_html2.html` - nested OR/AND
- `lic3/lzma-sdk-original.txt` - complex exception expression
- `lic4/airo.c` - "both X and Y" dual-license parsing

---

### Priority 5: UTF-8/Binary Handling

Infrastructure fix, not license logic.

**Action Items**:

1. Change `fs::read_to_string()` to `fs::read()` with lossy conversion
2. Consider adding text extraction for PDF/class files
3. Add graceful error handling for non-UTF-8 files

---

## Key Code Files Summary

| File | Priority | Patterns | Lines to Focus |
|------|----------|----------|----------------|
| `match_refine.rs` | 1, 2 | A, B, C | `merge_overlapping_matches()` 128-229, `filter_contained_matches()` 249-388, `filter_too_short_matches()` 62-84 |
| `detection.rs` | 1, 4 | A, D | `remove_duplicate_detections()` 891-909, `group_matches_by_region()` 149-206 |
| `expression.rs` | 4 | D | `combine_expressions()` 628-666 |
| `unknown_match.rs` | 2 | B | Threshold tuning |
| `golden_test.rs` | 5 | E | File reading 110-116 |

---

## Python Reference Files

| Python File | Lines | Purpose |
|-------------|-------|---------|
| `match.py` | 869-1068 | `merge_matches()` - distance-based merging |
| `match.py` | 800-910 | Match filtering logic |
| `detection.py` | 1162-1380 | `is_false_positive()` |
| `detection.py` | group functions | Detection grouping |

---

## Recommended Implementation Approach

1. **Start with Priority 1** - This will have the largest impact on test scores
2. **Implement incrementally** - Make one change, run tests, verify improvement
3. **Use debug tests** - Add specific debug tests for representative cases
4. **Compare Python output** - Run Python on same test files to understand expected behavior
5. **Track progress** - Run full golden test suite after each major change

---

## Debug Commands

```bash
# Run single suite
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Run specific debug test
cargo test --release -q --lib debug_gpl_12 -- --nocapture

# Run all golden tests
cargo test --release -q --lib license_detection::golden_test

# Compare with Python
cd reference/scancode-toolkit
./scancode -clp testdata/licensedcode/data/datadriven/lic1/gpl_19.txt
```
