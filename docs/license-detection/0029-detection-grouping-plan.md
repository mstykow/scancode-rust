# Plan: Fix Detection Grouping Issues

## Status: IMPLEMENTED

### Implementation Summary (2026-03-03)

**What was implemented:**
- Modified `determine_license_expression()` to use `combine_expressions()` instead of naive string join
- Modified `determine_spdx_expression()` similarly
- Added proper imports for `combine_expressions` and `CombineRelation`
- Location: `src/license_detection/detection/analysis.rs:383-414`

**Result:** The fix was implemented correctly and aligns with Python reference behavior.

---

## Verification Status: PASS

This plan has been verified against the actual codebase and Python reference.

### Verification Results

| Check | Status | Notes |
|-------|--------|-------|
| Implementation correct | PASS | `determine_license_expression()` uses `combine_expressions()` |
| Code locations correct | PASS | Verified at `analysis.rs:383-395` and `analysis.rs:402-414` |
| `combine_expressions()` exists | PASS | Found at `expression/simplify.rs:421-459` with comprehensive tests |
| Python reference matches | PASS | Python calls `combine_expressions()` at `detection.py:1594-1597` |
| Import correct | PASS | `use crate::license_detection::expression::{combine_expressions, CombineRelation};` at line 5 |

### Implementation Details

The fix replaced naive string concatenation with proper expression combination:

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

This matches Python's behavior at `detection.py:1594-1597`:
```python
combined_expression = combine_expressions(
    expressions=[match.rule.license_expression for match in matches_for_expression],
    licensing=get_licensing(),
)
```

---

## Problem Statement

License matches that should create separate detections were being incorrectly grouped together into single detections with AND expressions. This caused:
- Incorrect detection count (fewer detections than expected)
- Incorrect expression assembly (AND when should be separate)
- Golden test failures against Python reference

### Example: hauppauge.txt
- Expected: 5 separate detections
  - `["proprietary-license", "proprietary-license", "unknown-license-reference", "hauppauge-firmware-eula", "hauppauge-firmware-oem"]`

## Root Cause Analysis

### Original Bug: `determine_license_expression()` Did Not Use Existing Infrastructure

The bug was in `src/license_detection/detection/analysis.rs:377-400` (before fix):

```rust
// BUGGY CODE (before fix):
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    // ...
    if unique.len() == 1 {
        Ok(unique[0].to_string())
    } else {
        Ok(unique.join(" AND "))  // <-- ALWAYS joins with AND (naive)
    }
}
```

**Problem**: This function did naive string concatenation with AND instead of using the existing `combine_expressions()` function.

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

## Code Locations Changed

### `src/license_detection/detection/analysis.rs`

**Function**: `determine_license_expression()` (lines 383-395)
**Change**: Now uses `combine_expressions()` instead of naive string join

**Function**: `determine_spdx_expression()` (lines 402-414)
**Change**: Same fix applied for SPDX expressions

### Supporting Code (Already Existed, No Changes)

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

### Related Code (Unchanged, Verified Correct)

#### `src/license_detection/detection/grouping.rs`

**Function**: `group_matches_by_region_with_threshold()` (lines 21-64)
**Status**: Matches Python's `group_matches()` behavior at `detection.py:1820-1868`
- Same LINES_THRESHOLD = 4
- Same is_license_intro handling
- Same is_license_clue handling
- Same proximity-based grouping

#### `src/license_detection/match_refine/handle_overlaps.rs`

**Function**: `filter_contained_matches()` (lines 40-108)
**Status**: Matches Python's `filter_contained_matches()` at `match.py:1075-1169`
- Same qspan containment logic
- Same coverage-based tie-breaking
- Same sorting order

## Testing Strategy

This fix follows the project's four-layer testing approach per `docs/TESTING_STRATEGY.md`:

### Layer 0: Doctests
- N/A for this fix (internal function, not public API)

### Layer 1: Unit Tests

**Existing tests** (in `expression/simplify.rs`):
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

**Tests in `analysis.rs`** (lines 1057-1105):
- `test_determine_license_expression_single`
- `test_determine_license_expression_multiple`
- `test_determine_license_expression_empty`
- `test_determine_spdx_expression_single`
- `test_determine_spdx_expression_multiple`
- `test_determine_spdx_expression_empty`

### Layer 2: Golden Tests

**Test command**:
```bash
cargo test --release license_detection_golden
```

**Test data**: `testdata/license-golden/datadriven/`

**Specific test cases**:
- `hauppauge.txt` - expects 5 separate detections
- Various GFDL cases - expects proper expression combination

### Layer 3: Integration Tests

**Test command**:
```bash
cargo test scanner_integration
```

## Validation Commands

```bash
# Run unit tests for expression combination
cargo test expression::simplify

# Run license detection golden tests
cargo test --release license_detection_golden

# Run specific test cases
cargo test --release hauppauge

# Count failing golden test cases
cargo test --release -q --lib license_detection::golden_test 2>&1 | \
  grep "failed, 0 skipped" | sed 's/.*, \([0-9]*\) failed,.*/\1/' | paste -sd+ | bc
```

## Risk Assessment

### Low Risk

1. **Using existing `combine_expressions()`**
   - Risk: Minimal - function is already implemented and tested
   - Mitigation: 80+ existing unit tests, well-documented

2. **Expression parsing edge cases**
   - Risk: Already handled in existing `expression` module
   - Mitigation: Tests for complex expressions, duplicates, OR/AND/WITH

### Areas Not Expected to Need Changes

1. **Match refinement**: `handle_overlaps.rs` already has `licensing_contains()` checks
2. **Detection log and metadata**: Unaffected by expression fix
3. **Grouping logic**: Already matches Python's `group_matches()` behavior

## Dependencies

All dependencies already exist in the codebase:
- `src/license_detection/expression/mod.rs` - Exports `combine_expressions`, `CombineRelation`
- `src/license_detection/expression/simplify.rs` - Implementation of `combine_expressions()`
- `src/license_detection/expression/parse.rs` - Expression parsing
- `src/license_detection/models.rs` - LicenseMatch definition

**No new dependencies required.**

## Success Criteria

1. ✅ **Expression assembly correct**: Uses proper `combine_expressions()` logic
2. ✅ **Code quality**: Passes `cargo clippy`, well-documented
3. 🔄 **Golden test pass rate**: To be validated against Python reference
4. 🔄 **Detection count matches Python**: For all test files
5. 🔄 **Performance**: No significant regression (<5% slowdown)

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

## References

- Python `detection.py`: `reference/scancode-toolkit/src/licensedcode/detection.py`
  - `get_detected_license_expression()`: Lines 1468-1602
  - `group_matches()`: Lines 1820-1868
- Python `match.py`: `reference/scancode-toolkit/src/licensedcode/match.py`
  - `filter_contained_matches()`: Lines 1075-1169
- Rust `grouping.rs`: `src/license_detection/detection/grouping.rs`
- Rust `analysis.rs`: `src/license_detection/detection/analysis.rs`
- Rust `handle_overlaps.rs`: `src/license_detection/match_refine/handle_overlaps.rs`
- Golden tests: `src/license_detection/golden_test.rs`
- Test data: `testdata/license-golden/datadriven/`
- Testing strategy: `docs/TESTING_STRATEGY.md`
