# Implementation Plan: Missing Duplicate Detections

## Status: VERIFIED - Root Cause Confirmed (Pipeline Difference)

### Key Finding (2026-03-03, Verified 2026-03-03)

**The real issue is NOT `filter_contained_matches()` - it's a pipeline difference between Python and Rust.**

After thorough verification:

**Root Cause:** Python's `idx.match()` returns raw matches after `refine_matches()`, while Rust's `detect()` groups matches into detections via `group_matches_by_region()`.

**Evidence:**
1. **Python pipeline** (index.py:1131-1139):
   ```python
   matches, _discarded = match.refine_matches(matches=matches, ...)
   matches.sort()
   return matches  # Returns RAW MATCHES - no grouping!
   ```

2. **Rust pipeline** (mod.rs:319-336):
   ```rust
   let refined = refine_matches(&self.index, refined_matches, &query);
   sort_matches_by_line(&mut sorted);
   let groups = group_matches_by_region(&sorted);  // <-- GROUPING HAPPENS HERE
   let detections: Vec<LicenseDetection> = groups.iter()...
   ```

3. **Golden test comparison**:
   - Python test extracts from `matches` (raw, no grouping)
   - Rust test extracts from `detection.matches` (after grouping)
   - This is a fundamental mismatch in test approach!

**The kernel test case** (`gpl-2.0_or_bsd-new_intel_kernel.c`):
- GPL section: lines 4-15
- BSD section: lines 17-45
- Line gap: 17 - 15 = 2 lines (WITHIN `LINES_THRESHOLD = 4`)
- Result: All matches grouped into ONE detection, but expected 4 separate expressions

**Why `filter_contained_matches()` is NOT the issue:**
1. Unit test `test_filter_contained_matches_token_positions_non_overlapping` PASSES (line 681-689)
2. Break condition analysis shows Python and Rust are equivalent:
   - Python: `if next_match.qend > current_match.qend: j += 1; break`
   - Rust: `if next.end_token > current.end_token { break; }`
   - Both result in same final state after outer loop increment
3. The `qcontains()` method correctly returns `false` for non-overlapping matches

**What needs to be done:**
1. Add a `detect_matches()` method that returns raw matches (like Python's `idx.match()`)
2. Update golden tests to use `detect_matches()` instead of `detect()`
3. Keep `detect()` for production use (grouping into detections is correct for end users)

**Recommendation:** Implement `detect_matches()` to match Python's `idx.match()` behavior for accurate golden test comparison.

---

**Created:** 2026-03-03  
**Priority:** High  
**Category:** License Detection Correctness  
**Verified:** 2026-03-03 - Root cause confirmed: pipeline difference (Python returns raw matches, Rust groups into detections)

## Executive Summary

Rust incorrectly merges/deduplicates license matches that Python keeps as separate detections. When the same license appears multiple times in a file at different locations, Python creates multiple detections (one per location), but Rust creates only one detection (merged/deduplicated).

**Example:**
```
File: gpl-2.0_or_bsd-new_intel_kernel.c

Python expected: ["bsd-new OR gpl-2.0", "gpl-2.0", "bsd-new", "bsd-new"]
Rust actual:     ["bsd-new OR gpl-2.0"]  (missing 3 detections)
```

**Affected Files:**
- Kernel files with `MODULE_LICENSE` and `EXPORT_SYMBOL_GPL` macros
- Python license files (python-*.txt) where the license text appears twice
- Multi-license files where the same license appears at different locations
- Files with `ms-pl` appearing 3 times but only 1 detected

## Verification Summary

### Code Analysis Performed

1. **Rust `filter_contained_matches()`** (`handle_overlaps.rs:40-108`):
   - Break condition: `if next.end_token > current.end_token { break; }`
   - This is equivalent to Python's `if next_match.qend > current_match.qend: break`
   - Unit test confirms non-overlapping matches are NOT filtered

2. **Python `filter_contained_matches()`** (`match.py:1075-1184`):
   - Returns matches after containment filtering
   - No grouping step - returns raw matches

3. **Rust `group_matches_by_region()`** (`grouping.rs:7-64`):
   - Groups matches where `line_gap <= LINES_THRESHOLD` (4)
   - This is correct behavior for production use
   - But causes mismatch with Python golden tests

4. **Rust `detect()` pipeline** (`mod.rs:319-336`):
   - Calls `group_matches_by_region()` after refinement
   - Returns grouped detections, not raw matches

### Key Insight

The issue is NOT a bug in filtering or containment logic. It's a **design difference**:
- Python's `idx.match()` returns raw matches for testing
- Rust's `detect()` returns grouped detections for production

Both are correct, but the golden tests compare against Python's raw matches while using Rust's grouped detections.

### Recommended Fix

Add `detect_matches()` method that mirrors Python's `idx.match()` behavior (return raw matches without grouping). Use this for golden test comparison. Keep `detect()` unchanged for production use.

## Specific Code Locations Needing Changes

### Primary Change: Add `detect_matches()` Method

**File:** `src/license_detection/mod.rs`  
**Location:** After the existing `detect()` method (around line 340)

```rust
/// Detect licenses and return raw matches (like Python's idx.match()).
///
/// This method returns matches after refinement, WITHOUT grouping into detections.
/// Use this for testing and comparison with Python's idx.match() output.
/// For production use, prefer detect() which returns grouped detections.
pub fn detect_matches(&self, text: &str, unknown_licenses: bool) -> Result<Vec<LicenseMatch>> {
    // Same logic as detect() but return refined matches instead of grouped detections
    // ... implementation ...
    let refined = refine_matches(&self.index, refined_matches, &query);
    sort_matches_by_line(&mut refined);
    Ok(refined)  // Return raw matches
}
```

### Secondary Change: Update Golden Test

**File:** `src/license_detection/golden_test.rs`  
**Lines:** 160-168

```rust
// Current:
let detections = engine.detect(&text, unknown_licenses)?;

// Change to:
let matches = engine.detect_matches(&text, unknown_licenses)?;
let actual: Vec<&str> = matches
    .iter()
    .map(|m| m.license_expression.as_str())
    .collect();
```

### No Changes Needed

The following do NOT need changes (contrary to earlier analysis):
- `filter_contained_matches()` - Already correct, unit tests pass
- `qcontains()` - Already correct for non-overlapping matches
- `group_matches_by_region()` - Behavior is correct for production

## Implementation Steps

### Step 1: Add `detect_matches()` Method

Add a new method to `LicenseDetectionEngine` that returns raw matches (like Python's `idx.match()`):

```rust
/// Detect licenses and return raw matches (like Python's idx.match()).
/// This is for golden test comparison - production code should use detect().
pub fn detect_matches(&self, text: &str, unknown_licenses: bool) -> Result<Vec<LicenseMatch>> {
    // ... same as detect() but return matches after refine_matches() ...
    // Do NOT call group_matches_by_region()
    let refined = refine_matches(&self.index, refined_matches, &query);
    sort_matches_by_line(&mut refined);
    Ok(refined)  // Return raw matches, not detections
}
```

**Location:** `src/license_detection/mod.rs`

### Step 2: Update Golden Test to Use `detect_matches()`

Update the golden test to use the new method:

```rust
// Before (current):
let detections = engine.detect(&text, unknown_licenses)?;
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();

// After (fixed):
let matches = engine.detect_matches(&text, unknown_licenses)?;
let actual: Vec<&str> = matches
    .iter()
    .map(|m| m.license_expression.as_str())
    .collect();
```

**Location:** `src/license_detection/golden_test.rs:160-168`

### Step 3: Verify Fix with Unit Test

Add a unit test that verifies raw matches match Python's expected behavior:

```rust
#[test]
fn test_kernel_file_returns_four_matches() {
    let content = include_str!("../../testdata/license-golden/.../gpl-2.0_or_bsd-new_intel_kernel.c");
    let engine = LicenseDetectionEngine::new(...).unwrap();
    let matches = engine.detect_matches(content, false).unwrap();
    
    let expressions: Vec<_> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    
    assert_eq!(expressions, vec!["bsd-new OR gpl-2.0", "gpl-2.0", "bsd-new", "bsd-new"]);
}
```

### Step 4: Run Golden Test Suite

After changes, run the full golden test suite to verify alignment with Python:

```bash
cargo test --lib license_detection::golden_test
```

### Step 5: Document the Pipeline Difference

Update documentation to explain the difference between `detect()` and `detect_matches()`:
- `detect()` - Returns grouped detections (for production use)
- `detect_matches()` - Returns raw matches (for Python parity testing)

## Test Cases to Verify the Fix

### Unit Test: Non-overlapping Matches Kept

**Location:** `src/license_detection/match_refine/handle_overlaps.rs` (already exists)

The existing test `test_filter_contained_matches_token_positions_non_overlapping` (line 681-689) verifies that non-overlapping matches are NOT filtered:

```rust
#[test]
fn test_filter_contained_matches_token_positions_non_overlapping() {
    let m1 = create_test_match_with_tokens("#1", 0, 10, 10);
    let m2 = create_test_match_with_tokens("#2", 20, 30, 10);  // Non-overlapping
    let matches = vec![m1, m2];

    let (filtered, _) = filter_contained_matches(&matches);

    assert_eq!(filtered.len(), 2);  // Both kept!
}
```

This test PASSES, confirming `filter_contained_matches()` is correct.

### Unit Test: Detection Grouping Behavior

**Location:** `src/license_detection/detection/grouping.rs` (already exists)

Test `test_group_matches_within_threshold` (line 210-217) shows grouping behavior:

```rust
#[test]
fn test_group_matches_within_threshold() {
    let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
    let match2 = create_test_match(6, 10, "2-aho", "mit.LICENSE");  // gap=1
    let matches = vec![match1, match2];
    let groups = group_matches_by_region(&matches);
    assert_eq!(groups.len(), 1);  // Grouped together!
}
```

### Integration Test: Golden Test Alignment

**Location:** `src/license_detection/golden_test.rs`

After implementing `detect_matches()`, the golden test should pass:

```rust
// Test: gpl-2.0_or_bsd-new_intel_kernel.c
// Expected: ["bsd-new OR gpl-2.0", "gpl-2.0", "bsd-new", "bsd-new"]
// Before fix: ["bsd-new OR gpl-2.0"] (matches grouped into 1 detection)
// After fix: ["bsd-new OR gpl-2.0", "gpl-2.0", "bsd-new", "bsd-new"] (raw matches)
```

### Regression Test: Existing Passes Still Pass

Run full golden test suite to ensure no regressions:

```bash
cargo test --lib license_detection::golden_test
```

Count passing tests before and after to verify no regressions.

## Testing Strategy

This fix follows the project's multi-layered testing approach (see `docs/TESTING_STRATEGY.md`):

### Layer 1: Unit Tests (Already Passing)

- `test_filter_contained_matches_token_positions_non_overlapping` - Verifies filtering is correct
- `test_group_matches_within_threshold` - Verifies grouping behavior
- These tests confirm the individual components work correctly

### Layer 2: Golden Tests (Need Fix)

The golden tests compare Rust output against Python reference. The current mismatch is:
- Python's `idx.match()` returns raw matches
- Rust's `detect()` returns grouped detections

**Fix:** Add `detect_matches()` to match Python's behavior for accurate comparison.

### Layer 3: Integration Tests

The `detect()` method (with grouping) remains the production API. After the fix:
- `detect_matches()` - For golden test parity with Python
- `detect()` - For production use (returns grouped detections)

### Quality Gates

Before marking complete:
- [ ] Unit tests pass (`cargo test --lib`)
- [ ] Golden tests pass with `detect_matches()`
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)

## Related Documents

- [0017-phase1-duplicate-detection-plan.md](0017-phase1-duplicate-detection-plan.md) - Original analysis
- [0015-filter-dupes-regressions.md](0015-filter-dupes-regressions.md) - Related regression investigation
- [PLAN-019-unique-detection.md](PLAN-019-unique-detection.md) - Unique detection design
- [TESTING_STRATEGY.md](../TESTING_STRATEGY.md) - Multi-layered testing approach

## Next Steps

1. **Implement `detect_matches()`** - Add method to return raw matches like Python's `idx.match()`
2. **Update golden tests** - Use `detect_matches()` for accurate Python comparison
3. **Run full golden test suite** - Verify alignment with Python reference
4. **Document the difference** - Explain `detect()` vs `detect_matches()` in API docs
