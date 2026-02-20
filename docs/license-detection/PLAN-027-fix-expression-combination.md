# PLAN-027: Fix Expression Combination Logic

**Date**: 2026-02-20
**Status**: Partially Implemented
**Priority**: 4 (Pattern D - ~60 failures in lic1)
**Related**: PLAN-023-failure-analysis-summary.md

## Executive Summary

License expressions are being combined incorrectly in ~60 golden test cases (lic1 suite). The issues fall into three categories:

1. **Missing matches** - Deduplication/removal filtering too aggressive
2. **Extra matches** - Component licenses not being filtered alongside WITH expressions  
3. **Wrong match count** - Mismatches in number of matches vs expected

**NOTE**: After code analysis, the "outer parentheses" issue described in the original plan does NOT exist in the current implementation. The `expression_to_string_internal()` function correctly handles top-level expressions without adding outer parentheses.

## Current Implementation Status

### Already Implemented

1. **`licensing_contains()` function** (expression.rs:444-506) - Handles expression-based subsumption including WITH expressions.

2. **`licensing_contains_match()` helper** (match_refine.rs:452-457) - Wraps `licensing_contains()` for use in filtering.

3. **`filter_overlapping_matches()` uses `licensing_contains_match()`** (match_refine.rs:551-606) - The overlap filtering already includes expression-based subsumption checks.

4. **Expression rendering is correct** - `expression_to_string_internal()` correctly handles parentheses. Tests verify this.

### NOT Yet Implemented

1. **Expression subsumption in `filter_contained_matches()`** - This function (match_refine.rs:319-353) uses only token-based `qcontains()` and does NOT use `licensing_contains()`.

## Problem Analysis

### Python's `combine_expressions()` Behavior

Located in `packagedcode/utils.py:136-148` (wraps `license_expression` library):

```python
def combine_expressions(
    expressions,
    relation='AND',
    unique=True,
    licensing=_LICENSING,
):
    if not licensing:
        raise Exception('combine_expressions: cannot combine...')
    return expressions and str(le_combine_expressions(expressions, relation, unique, licensing)) or None
```

**Key Behavior**: Python uses `combine_expressions()` only in specific code paths:

- `get_detected_license_expression()` at `detection.py:1594` combines expressions with AND
- The actual expression is derived from **individual rule matches** - if a rule has `mit OR apache-2.0`, that OR comes from the rule itself

### Python's `group_matches()` Behavior

Located in `detection.py:1820-1868`:

```python
def group_matches(license_matches, lines_threshold=LINES_THRESHOLD):
    group_of_license_matches = []

    for license_match in license_matches:
        if not group_of_license_matches:
            group_of_license_matches.append(license_match)
            continue

        previous_match = group_of_license_matches[-1]
        is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold

        if previous_match.rule.is_license_intro:
            group_of_license_matches.append(license_match)
        elif license_match.rule.is_license_intro:
            yield group_of_license_matches
            group_of_license_matches = [license_match]
        elif license_match.rule.is_license_clue:
            yield group_of_license_matches
            yield [license_match]
            group_of_license_matches = []
        elif is_in_group_by_threshold:
            group_of_license_matches.append(license_match)
        else:
            yield group_of_license_matches
            group_of_license_matches = [license_match]

    if group_of_license_matches:
        yield group_of_license_matches
```

**Key Behaviors**:

1. Uses `start_line <= prev_end_line + threshold` (inclusive)
2. License intros force next match into same group
3. License clues are always separate groups
4. Otherwise groups by line proximity

### Rust's Current Implementation

**`group_matches_by_region()` at `detection.rs:149-206`**:

- Uses `should_group_together()` which checks `line_gap <= LINES_THRESHOLD`
- This is semantically equivalent to Python's approach
- Correct handling of `is_license_intro` and `is_license_clue`

**`expression_to_string_internal()` at `expression.rs:559-592`**:

- Correctly handles parentheses based on precedence
- **No outer parentheses issue** - when `parent_prec=None`, OR/AND expressions are NOT wrapped in parentheses
- The precedence check `parent_prec.is_some_and(|p| p != Precedence::Or)` correctly returns false for None

**`combine_expressions()` at `expression.rs:628-666`**:

- Parses expressions, combines with AND/OR, optionally simplifies
- Returns string representation
- Works correctly for combining expressions

**`determine_license_expression()` at `detection.rs:651-663`**:

- Always uses `CombineRelation::And` to combine match expressions
- This matches Python's behavior in `get_detected_license_expression()`

**`filter_contained_matches()` at `match_refine.rs:319-353`**:

- Uses `qcontains()` for token-based containment
- Does NOT use `licensing_contains()` for expression-based subsumption

**`licensing_contains()` at `expression.rs:444-520`**:

- Handles expression containment including WITH expressions
- `gpl-2.0 WITH exception` contains `gpl-2.0`
- This is used in `filter_overlapping_matches()` but NOT in `filter_contained_matches()`

## Root Causes Identified

### Issue 1: Component License Filtering (WITH Expression Subsumption)

**Location**: `match_refine.rs:319-353` - `filter_contained_matches()`

**Problem**: When `gpl-2.0 WITH classpath-exception-2.0` is detected, standalone `gpl-2.0` matches are not filtered as contained/subsumed in this function.

**Evidence from golden tests**:

- `gpl-2.0-plus_4.txt`: Expected `["gpl-2.0-plus AND free-unknown"]`, Actual `["gpl-2.0-plus AND free-unknown", "gpl-2.0-plus"]`
- `gpl-2.0_and_lgpl-2.0-plus.txt`: Expected `["gpl-2.0 AND lgpl-2.0-plus AND gpl-2.0-plus"]`, Actual has extra `gpl-2.0-plus`

**Root Cause**: `filter_contained_matches()` uses `qcontains()` (token span containment) but not `licensing_contains()` (expression subsumption). While `filter_overlapping_matches()` does use expression subsumption, `filter_contained_matches()` runs first and may not filter these cases properly.

### Issue 2: Deduplication Too Aggressive

**Location**: `match_refine.rs:319-353` and the entire refinement pipeline

**Problem**: Some matches are being removed when they shouldn't be.

**Evidence from golden tests**:

- `gpl-2.0_82.RULE`: Expected `["gpl-2.0", "gpl-2.0", "gpl-2.0"]`, Actual `["gpl-2.0"]`
- `gpl-2.0_complex.txt`: Expected `["gpl-2.0", "gpl-2.0"]`, Actual `["gpl-2.0"]`

**Root Cause**: The `simplify_expression()` function deduplicates license keys within AND/OR expressions. When combining expressions with `unique=true`, duplicates are removed. This may be removing legitimate duplicate matches at different file positions.

### Issue 3: Rule Match Count Mismatches

**Location**: Detection and filtering pipeline

**Problem**: Some tests expect different numbers of matches than are being produced.

**Evidence**:

- `gpl-2.0_and_gpl-2.0-plus.txt`: Expected 9 matches, Actual 6 matches
- `gpl-2.0_and_gpl-2.0_and_gpl-2.0-plus.txt`: Expected 6 matches, Actual 4 matches

**Root Cause**: Complex interaction between detection, filtering, and deduplication. May involve rule matching, containment filtering, or overlap resolution.

## Implementation Plan

### Phase 1: Fix WITH Expression Subsumption in Containment Filtering

**File**: `src/license_detection/match_refine.rs`

**Current Code** (`match_refine.rs:319-353`):

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // ... uses qcontains() only ...
}
```

**Changes**:

1. Modify `filter_contained_matches()` to also consider expression subsumption:

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // ... existing token-based containment ...
    
    // Additionally check expression subsumption:
    // If match A has expression "X WITH Y" and match B has expression "X",
    // then A subsumes B
}
```

1. Add helper function (already exists as `licensing_contains_match()`):

```rust
fn expression_subsumes_match(container: &LicenseMatch, contained: &LicenseMatch) -> bool {
    if container.license_expression.is_empty() || contained.license_expression.is_empty() {
        return false;
    }
    licensing_contains(&container.license_expression, &contained.license_expression)
}
```

**Test Cases**:

- `gpl-2.0-plus_4.txt` - should not have extra `gpl-2.0-plus` alongside `gpl-2.0-plus AND free-unknown`
- `gpl-2.0_and_lgpl-2.0-plus.txt` - should not have extra `gpl-2.0-plus`
- Unit test: `gpl-2.0 WITH exception` should subsume `gpl-2.0`

### Phase 2: Investigate and Fix Deduplication Logic

**File**: `src/license_detection/match_refine.rs` and `src/license_detection/detection.rs`

**Analysis Required**:

1. The issue is that identical matches at DIFFERENT locations in the file should be kept, but the current logic may be merging them.

2. Python's behavior: Each occurrence of a license in the text produces a separate match.

3. Current Rust behavior may be incorrectly deduplicating based on expression alone.

**Investigation Steps**:

1. Compare Python and Rust output for `gpl-2.0_complex.txt`
2. Check if matches at different line positions are being merged
3. Determine if the issue is in `filter_contained_matches()`, `filter_overlapping_matches()`, or the expression simplification

**Potential Fix Locations**:

- `simplify_expression()` in expression.rs - may be deduplicating too aggressively
- Sorting in `filter_contained_matches()` uses `start_token`, `hilen`, `matched_length`, `rule_identifier` - matches at different positions should have different `start_token` values
- Detection grouping logic in `detection.rs`

### Phase 3: Expression Rendering Tests (Already Passing)

**Status**: The expression rendering tests at `expression.rs:1287-1641` already verify:

- `test_expression_to_string_or_inside_and` - OR inside AND gets parens
- `test_expression_to_string_and_inside_or` - AND inside OR gets parens
- `test_expression_to_string_nested_or_no_parens` - Nested OR without outer parens
- `test_expression_to_string_nested_and_no_parens` - Nested AND without outer parens
- `test_expression_to_string_with_no_outer_parens` - WITH without outer parens

**No changes needed** - the parentheses logic is correct.

## Test Cases to Verify

| File | Suite | Expected | Root Cause | Fix Phase |
|------|-------|----------|------------|-----------|
| `gpl-2.0-plus_4.txt` | lic1 | `["gpl-2.0-plus AND free-unknown"]` | WITH subsumption | Phase 1 |
| `gpl-2.0_and_lgpl-2.0-plus.txt` | lic1 | Single expression | WITH subsumption | Phase 1 |
| `gpl-2.0_82.RULE` | lic1 | 3 matches | Deduplication | Phase 2 |
| `gpl-2.0_complex.txt` | lic1 | 2 matches | Deduplication | Phase 2 |
| `fsf-free_and_fsf-free_and_fsf-free.txt` | lic1 | 3 matches | Deduplication | Phase 2 |

## Step-by-Step Implementation Order

### Step 1: Add Expression Subsumption to Containment Filtering

1. Modify `filter_contained_matches()` to use `licensing_contains()`
2. Add unit tests for WITH expression subsumption
3. Run golden tests to verify fixes

### Step 2: Investigate and Fix Deduplication

1. Debug specific test cases to understand why matches are being removed
2. Compare Python output for same files
3. Fix the filtering logic to preserve matches at different locations
4. Add unit tests for multi-location same-license scenarios
5. Run golden tests to verify fixes

### Step 3: Run Full Test Suite

1. Run all golden tests: `cargo test --release -q --lib license_detection::golden_tests`
2. Verify no regressions in passing tests
3. Document any intentional behavioral differences from Python

## Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/match_refine.rs` | Phase 1: Add expression subsumption to containment filtering, Phase 2: Fix deduplication |
| `src/license_detection/expression.rs` | Unit tests for subsumption cases (if needed) |

## Verification Commands

```bash
# Run specific golden test suites
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic2
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic3
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic4

# Run expression tests
cargo test --release -q --lib expression

# Run all golden tests
cargo test --release -q --lib license_detection::golden_tests
```

## Success Criteria

1. `gpl-2.0-plus_4.txt` passes without extra `gpl-2.0-plus` match
2. `gpl-2.0_82.RULE` passes with 3 separate matches
3. `gpl-2.0_complex.txt` passes with 2 matches
4. No regressions in passing tests
5. Overall golden test pass rate improves by ~30-40 tests

## Estimated Impact

- Phase 1 (WITH subsumption in filter_contained_matches): ~5-10 tests fixed
- Phase 2 (Deduplication): ~15-25 tests fixed

**Total Estimated Fix**: ~20-35 tests (covering most of Pattern D failures related to expression/match issues)

## Appendix: What Was Correct in Original Plan

1. ✅ Python's `combine_expressions()` uses AND by default
2. ✅ Python's `group_matches()` logic correctly described
3. ✅ Rust's `group_matches_by_region()` matches Python behavior
4. ✅ The `determine_license_expression()` uses AND for combination
5. ✅ WITH expression subsumption is missing from `filter_contained_matches()` (STILL NEEDS IMPLEMENTATION)

## Appendix: What Was Incorrect in Original Plan

1. ❌ **"Unnecessary outer parentheses"** - This issue does NOT exist. The `expression_to_string_internal()` correctly handles top-level expressions:
   - When `parent_prec=None`, the check `parent_prec.is_some_and(|p| p != Precedence::Or)` returns false
   - Existing tests verify this: `test_expression_to_string_nested_or_no_parens`, `test_expression_to_string_nested_and_no_parens`

2. ❌ **"Wrong operators (AND vs OR)"** - The plan misidentified the AND/OR combination issue. The OR in expressions like `mit OR apache-2.0` comes from the **rule's license_expression field**, not from the combination logic. Rules can have OR expressions, and those are preserved through the detection pipeline.

3. ❌ **`plantuml_license_notice.txt` issue** - This test expects `mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus`, which would come from a rule with that expression, not from combining separate matches.

4. ❌ **Line number references for Python code** - `combine_expressions()` is at `packagedcode/utils.py:136`, not `license_expression/__init__.py:1746`

## Appendix: Implementation Changes Since Plan Creation

1. **`licensing_contains()` implemented** - expression.rs:444-506
2. **`licensing_contains_match()` helper added** - match_refine.rs:452-457
3. **`filter_overlapping_matches()` now uses expression subsumption** - match_refine.rs:551-606
4. **Line numbers shifted** - `filter_contained_matches()` is now at match_refine.rs:319-353

## Appendix: Missing Test Coverage

The plan should add unit tests for:

1. Expression subsumption: `"gpl-2.0 WITH classpath-exception-2.0"` subsumes `"gpl-2.0"`
2. Expression subsumption: `"mit AND apache-2.0"` subsumes `"mit"` (should be false)
3. Multi-location matches: Same license at different positions should produce separate matches
