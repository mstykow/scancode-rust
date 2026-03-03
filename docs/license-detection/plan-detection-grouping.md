# Plan: Fix Detection Grouping Issues

## Status: IMPLEMENTED (Partial - Does Not Solve Detection Count Issues)

### Implementation Attempt (2026-03-03)

**What was done:**
- Modified `determine_license_expression()` to use `combine_expressions()` instead of naive string join
- Modified `determine_spdx_expression()` similarly
- Added proper imports for `combine_expressions` and `CombineRelation`

**Result:** The fix was implemented correctly but **did not reduce the golden test failure count**.

**Why it didn't help:**
- The detection count issue is NOT caused by expression combination
- Investigation revealed the real problem is in **contained match filtering** (`filter_contained_matches`)
- Matches at non-overlapping locations are being incorrectly marked as "contained" and removed
- See `plan-duplicate-detection.md` for the actual root cause

**Recommendation:** This plan should be considered complete (the fix is correct), but the detection count issues require a different fix.

---

## Verification Status: PASS (Updated)

This plan has been verified against the actual codebase and Python reference.

### Verification Results

| Check | Status | Notes |
|-------|--------|-------|
| Root cause identified correctly | PASS | Bug is in `determine_license_expression()` doing naive string join |
| Code locations correct | PASS | Line numbers and function names verified |
| `combine_expressions()` exists | PASS | Found at `expression/simplify.rs:421-459` with 80+ tests |
| Python reference matches | PASS | Python calls `combine_expressions(unique=True)` |
| Missing aspects identified | FIXED | Updated plan to use existing infrastructure |

### Key Corrections Made

1. **Simplified root cause**: The bug is simply that `determine_license_expression()` doesn't use the existing `combine_expressions()` function. No new code needed.

2. **Effort estimate reduced**: From 12-17 hours to 2-6 hours (most likely 2-3 hours).

3. **Removed unnecessary phases**: Phase 2 (grouping fix) and Phase 3 (refinement fix) are now secondary review only.

4. **Fixed detection count**: hauppauge.txt expects 5 detections, not 6 (verified against golden test file).

5. **Answered open question**: The `expression` module was reviewed and found to have the complete solution.

---

## Problem Statement

License matches that should create separate detections are being incorrectly grouped together into single detections with AND expressions. This causes:
- Incorrect detection count (fewer detections than expected)
- Incorrect expression assembly (AND when should be separate)
- Golden test failures against Python reference

### Example Failures

**Example 1: hauppauge.txt**
- Expected: 5 separate detections
  - `["proprietary-license", "proprietary-license", "unknown-license-reference", "hauppauge-firmware-eula", "hauppauge-firmware-oem"]`
- Actual: Merged into fewer detections with AND expressions

**Example 2: GFDL cases**
- Expected: Multiple separate detections
  - `["gfdl-1.1", "gfdl-1.3-invariants-only", "gfdl-1.3-invariants-only"]` (3 detections)
- Actual: `["gfdl-1.1-plus", "gfdl-1.3-invariants-only", "gfdl-1.3-invariants-only"]` (merged incorrectly)

## Root Cause Analysis

### Key Finding 1: `determine_license_expression()` Does Not Use Existing Infrastructure

The bug is in `src/license_detection/detection/analysis.rs:377-400`:

```rust
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    // ...
    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    // Get unique expressions preserving order
    let mut unique: Vec<&str> = Vec::new();
    for expr in &expressions {
        if !unique.contains(expr) {
            unique.push(expr);
        }
    }

    if unique.len() == 1 {
        Ok(unique[0].to_string())
    } else {
        Ok(unique.join(" AND "))  // <-- ALWAYS joins with AND
    }
}
```

**Problem**: This function does naive string concatenation with AND instead of using the existing `combine_expressions()` function.

### Key Finding 2: `combine_expressions()` Already Exists in Codebase

**IMPORTANT**: The Rust codebase already has a complete `combine_expressions()` implementation at `src/license_detection/expression/simplify.rs:421-459`:

```rust
pub fn combine_expressions(
    expressions: &[&str],
    relation: CombineRelation,
    unique: bool,
) -> Result<String, ParseError> {
    // Parses each expression
    // Combines using proper AND/OR semantics
    // Optionally deduplicates
    // Returns simplified expression
}
```

This function:
1. Parses license expressions into an AST
2. Combines using `LicenseExpression::and()` or `LicenseExpression::or()`
3. Deduplicates when `unique=true`
4. Returns properly formatted expression string

**The fix is simply to USE this existing function in `determine_license_expression()`!**

### Python Reference Behavior

Python's `get_detected_license_expression()` at `detection.py:1468-1602`:

```python
def get_detected_license_expression(...):
    # ...
    combined_expression = combine_expressions(
        expressions=[match.rule.license_expression for match in matches_for_expression],
        licensing=get_licensing(),
    )
    return detection_log, str(combined_expression)
```

Python's `combine_expressions()` (from `license-expression` library) is called with:
- `unique=True` - deduplicate license keys
- `relation='AND'` - combine with AND (default)

Examples from Python codebase (`detection.py:451-455`, `detection.py:882-887`):
```python
license_expression = combine_expressions(
    [self.license_expression, match.license_expression],
    unique=True,
    licensing=licensing,
)
```

### Root Cause: `determine_license_expression()` Not Using Existing Infrastructure

The **primary bug** is that `determine_license_expression()` performs naive string concatenation instead of using the existing `combine_expressions()` function that was built for this exact purpose.

### Why This Causes Detection Grouping Issues

When `determine_license_expression()` incorrectly combines expressions with AND, the downstream effects are:
1. Multiple license matches get combined into one expression string
2. The detection count becomes incorrect
3. Golden tests fail because output doesn't match Python

### Why Detection Count Matters

The hauppauge.txt example expects 5 separate detections because the file contains:
1. Line 1-3: "END-USER FIRMWARE LICENSE AGREEMENT" → proprietary-license
2. Line 10-18: License intro text → proprietary-license  
3. Line 20-53: End-user license terms → unknown-license-reference
4. Line 1-128: Full end-user EULA → hauppauge-firmware-eula
5. Line 132-280: OEM/IHV/ISV license → hauppauge-firmware-oem

These are detected at different line ranges and should create separate detections based on:
- The `LINES_THRESHOLD = 4` proximity rule
- The `is_license_intro` flag handling in grouping

### Secondary Issue: Match Grouping Logic May Need Review

After fixing `determine_license_expression()`, if issues persist, the grouping logic in `grouping.rs:21-64` may need review. However, the current implementation appears to match Python's `group_matches()` at `detection.py:1820-1868`:
- Same LINES_THRESHOLD = 4
- Same is_license_intro handling
- Same is_license_clue handling
- Same proximity-based grouping

## Python vs Rust Behavior Comparison

### Python's Approach

1. **Group matches by proximity** (`detection.py:1820`)
   - Same line threshold (LINES_THRESHOLD = 4)
   - License intro/clue handling
   - Yields separate groups for distant matches

2. **Create detection from group** (`detection.py:178-237` in Rust)
   - Analyze detection category
   - Filter license intros/references
   - Call `combine_expressions()` with `unique=True`

3. **`combine_expressions()` from `license-expression` library**:
   - Parses each expression into AST
   - Combines using proper AND/OR semantics
   - Deduplicates when `unique=True`
   - Returns simplified expression

### Rust's Current Approach

1. **Group matches by proximity** - CORRECT (matches Python)
2. **Create detection from group** - CORRECT structure
3. **Determine expression** - BUG: Naive string concatenation with AND
   - Should use existing `combine_expressions()` from `expression/simplify.rs`
   - Currently does string join without parsing
   - No deduplication using the `simplify_expression()` logic

### The Fix

Replace `determine_license_expression()` implementation:

```rust
// BEFORE (buggy):
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    // ... naive string join ...
    Ok(unique.join(" AND "))
}

// AFTER (correct):
use crate::license_detection::expression::{combine_expressions, CombineRelation};

pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    if matches.is_empty() {
        return Err("No matches to determine expression from".to_string());
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    combine_expressions(&expressions, CombineRelation::And, true)
        .map_err(|e| format!("Failed to combine expressions: {}", e))
}
```

Same fix applies to `determine_spdx_expression()`.

## Code Locations Requiring Changes

### Primary Change (Required)

#### `src/license_detection/detection/analysis.rs`

**Function**: `determine_license_expression()` (lines 377-400)
**Issue**: Naive string concatenation with AND instead of using `combine_expressions()`
**Fix Required**: Replace implementation with call to existing `combine_expressions()` function

**Function**: `determine_spdx_expression()` (lines 407-440)  
**Issue**: Same naive string concatenation bug
**Fix Required**: Same fix - use `combine_expressions()` with SPDX expressions

### Supporting Code (Already Exists, No Changes Needed)

#### `src/license_detection/expression/simplify.rs`

**Function**: `combine_expressions()` (lines 421-459)
**Status**: Already implemented and tested
**Capabilities**:
- Parses expressions into AST
- Combines with AND/OR semantics
- Deduplicates when `unique=true`
- Has 80+ unit tests

#### `src/license_detection/expression/mod.rs`

**Status**: Exports `combine_expressions` and `CombineRelation`
**No changes needed**

### Secondary Review (Only If Primary Fix Doesn't Resolve Issues)

#### `src/license_detection/detection/grouping.rs`

**Function**: `group_matches_by_region_with_threshold()` (lines 21-64)
**Status**: Appears to match Python's `group_matches()` behavior
**Review Only**: Verify line threshold and intro/clue handling match Python exactly

#### `src/license_detection/match_refine/handle_overlaps.rs`

**Status**: Has sophisticated overlap handling with `licensing_contains()` check
**Review Only**: Ensure matches to different licenses aren't merged incorrectly

## Implementation Steps

### Phase 1: Fix `determine_license_expression()` (Critical - Primary Fix)

This is the root cause fix. The `combine_expressions()` function already exists.

**Step 1.1**: Add import to `analysis.rs`

```rust
use crate::license_detection::expression::{combine_expressions, CombineRelation};
```

**Step 1.2**: Replace `determine_license_expression()` implementation

```rust
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    if matches.is_empty() {
        return Err("No matches to determine expression from".to_string());
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    combine_expressions(&expressions, CombineRelation::And, true)
        .map_err(|e| format!("Failed to combine expressions: {}", e))
}
```

**Step 1.3**: Apply same fix to `determine_spdx_expression()`

```rust
pub fn determine_spdx_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    if matches.is_empty() {
        return Err("No matches to determine SPDX expression from".to_string());
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression_spdx.as_str())
        .collect();

    combine_expressions(&expressions, CombineRelation::And, true)
        .map_err(|e| format!("Failed to combine SPDX expressions: {}", e))
}
```

**Estimated effort**: 30 minutes

### Phase 2: Test and Validate

**Step 2.1**: Run unit tests for expression combination

```bash
cargo test expression::simplify
```

**Step 2.2**: Run license detection golden tests

```bash
cargo test --release license_detection_golden
```

**Step 2.3**: Count failures before and after

```bash
# Count failing golden test cases
cargo test --release -q --lib license_detection::golden_test 2>&1 | \
  grep "failed, 0 skipped" | sed 's/.*, \([0-9]*\) failed,.*/\1/' | paste -sd+ | bc
```

**Step 2.4**: Run specific failing test cases

```bash
cargo test --release hauppauge
```

**Estimated effort**: 1-2 hours

### Phase 3: Review Grouping Logic (Only If Issues Persist)

If the primary fix doesn't resolve all issues, investigate:

**Step 3.1**: Compare grouping behavior with Python
- Trace through `group_matches_by_region()` with test cases
- Verify matches are grouped/detected correctly

**Step 3.2**: Add debug logging
- Log groups created before expression determination
- Compare with Python's group_matches() output

**Estimated effort**: 2-3 hours (if needed)

## Test Cases to Verify the Fix

### Existing Unit Tests (No Changes Needed)

The `combine_expressions()` function already has comprehensive tests in `expression/simplify.rs`:
- `test_combine_expressions_empty`
- `test_combine_expressions_single`
- `test_combine_expressions_two_and`
- `test_combine_expressions_two_or`
- `test_combine_expressions_multiple_and`
- `test_combine_expressions_with_duplicates_unique`
- `test_combine_expressions_with_duplicates_not_unique`
- `test_combine_expressions_complex_with_simplification`
- `test_combine_expressions_parse_error`
- `test_combine_expressions_with_existing_and`
- `test_combine_expressions_with_existing_or`

### New Unit Tests for `determine_license_expression()`

Add tests in `detection/analysis.rs`:

```rust
#[test]
fn test_determine_expression_single() {
    let matches = vec![create_test_match("mit")];
    let result = determine_license_expression(&matches).unwrap();
    assert_eq!(result, "mit");
}

#[test]
fn test_determine_expression_same_license_deduped() {
    let matches = vec![
        create_test_match("mit"),
        create_test_match("mit"),
    ];
    let result = determine_license_expression(&matches).unwrap();
    assert_eq!(result, "mit");  // Deduplicated
}

#[test]
fn test_determine_expression_unrelated_licenses() {
    let matches = vec![
        create_test_match("mit"),
        create_test_match("apache-2.0"),
    ];
    let result = determine_license_expression(&matches).unwrap();
    assert_eq!(result, "mit AND apache-2.0");
}

#[test]
fn test_determine_expression_complex_expressions() {
    let matches = vec![
        create_test_match("mit OR apache-2.0"),
        create_test_match("gpl-2.0"),
    ];
    let result = determine_license_expression(&matches).unwrap();
    assert!(result.contains("mit"));
    assert!(result.contains("apache-2.0"));
    assert!(result.contains("gpl-2.0"));
}
```

### Integration Tests

**hauppauge.txt test**
- Expected: 5 separate detections
- Run: `cargo test --release hauppauge`
- Verify detection expressions match Python output

**Full golden test suite**
- Run: `cargo test --release license_detection_golden`
- Target: Zero failures (or significantly reduced)

## Risk Assessment

### Low Risk (Primary Fix)

1. **Using existing `combine_expressions()`**
   - Risk: Minimal - function is already implemented and tested
   - Mitigation: 80+ existing unit tests, well-documented

2. **Expression parsing edge cases**
   - Risk: Already handled in existing `expression` module
   - Mitigation: Tests for complex expressions, duplicates, OR/AND/WITH

### Medium Risk (Only If Grouping Review Needed)

1. **Grouping logic changes**
   - Risk: May break existing correct groupings
   - Mitigation: Extensive unit tests in `grouping.rs`, compare with Python behavior

2. **Performance impact**
   - Risk: Parsing expressions is more expensive than string join
   - Mitigation: Profile before/after, but impact should be negligible

### Areas Not Expected to Need Changes

1. **Match refinement**: `handle_overlaps.rs` already has `licensing_contains()` checks
2. **Detection log and metadata**: Unaffected by expression fix

## Dependencies

All dependencies already exist in the codebase:
- `src/license_detection/expression/mod.rs` - Exports `combine_expressions`, `CombineRelation`
- `src/license_detection/expression/simplify.rs` - Implementation of `combine_expressions()`
- `src/license_detection/expression/parse.rs` - Expression parsing
- `src/license_detection/models.rs` - LicenseMatch definition

**No new dependencies required.**

## Success Criteria

1. **Golden test pass rate**: 100% (or significantly improved from current failures)
2. **Detection count matches Python**: For all test files
3. **Expression assembly correct**: Uses proper `combine_expressions()` logic
4. **Performance**: No significant regression (<5% slowdown)
5. **Code quality**: Passes `cargo clippy`, well-documented

## Estimated Total Effort

- Phase 1 (Expression fix): 30 minutes
- Phase 2 (Testing): 1-2 hours
- Phase 3 (Grouping review - if needed): 2-3 hours

**Total**: 2-6 hours (most likely just 2-3 hours for the fix + testing)

## Alternative Approaches Considered

### Alternative 1: Use External License Expression Library

**Pros**: 
- Well-tested, handles edge cases

**Cons**: 
- Adds external dependency
- No Rust equivalent of Python's `license-expression` library with identical behavior

**Decision**: Not needed - existing `expression` module is sufficient

### Alternative 2: Port Python's `combine_expressions` Exactly

**Pros**: 
- Guaranteed same behavior

**Cons**: 
- Python's `combine_expressions` delegates to `license-expression` Python library
- Rust implementation already exists and is tested

**Decision**: Not needed - Rust `combine_expressions()` already implements the same logic

### Alternative 3: Change Grouping Threshold

**Pros**: 
- Simple change

**Cons**: 
- Doesn't fix root cause
- Threshold is already correct per Python (`LINES_THRESHOLD = 4`)

**Decision**: Not recommended - the bug is in expression combination, not grouping

## Open Questions

1. **Should related licenses be simplified?**
   - Example: `gfdl-1.1 AND gfdl-1.1-plus` → currently NOT simplified by Rust
   - Python's `license-expression` library may do this
   - Current Rust `combine_expressions()` with `unique=true` only deduplicates identical keys
   - **Answer**: Start with current behavior; add simplification if needed after testing

2. **What about OR expressions in input?**
   - Example: Match with `"mit OR apache-2.0"` expression
   - Current `combine_expressions()` handles this correctly
   - Tests exist in `test_combine_expressions_complex_with_simplification`

3. ~~**How does `expression` module work currently?**~~
   - **Answered**: Already reviewed - it has full `combine_expressions()` implementation

## Next Steps

1. **Implement Phase 1 fix** (immediate, 30 min)
   - Add import for `combine_expressions`
   - Replace `determine_license_expression()` implementation
   - Replace `determine_spdx_expression()` implementation

2. **Test the fix** (priority 1, 1-2 hours)
   - Run existing unit tests
   - Run golden tests
   - Compare with Python output

3. **Document any differences** (ongoing)
   - If behavior differs from Python, document why
   - Add tests demonstrating correctness

## References

- Python `detection.py`: `reference/scancode-toolkit/src/licensedcode/detection.py`
- Python `group_matches()`: Lines 1820-1868
- Python `get_detected_license_expression()`: Lines 1468-1602
- Rust `grouping.rs`: `src/license_detection/detection/grouping.rs`
- Rust `analysis.rs`: `src/license_detection/detection/analysis.rs`
- Golden tests: `src/license_detection/golden_test.rs`
- Test data: `testdata/license-golden/datadriven/`
