# PLAN-007: should_detect_something.html

## Status: OPEN - NEEDS FIX

## Test File
`testdata/license-golden/datadriven/lic4/should_detect_something.html`

## Issue
Extra `sun-sissl-1.1` detection.

**Expected:** `["sun-sissl-1.1", "mit", "sun-sissl-1.1", "sun-sissl-1.1", "apache-2.0"]` (5 matches)
**Actual:** `["sun-sissl-1.1", "mit", "sun-sissl-1.1", "sun-sissl-1.1", "sun-sissl-1.1", "apache-2.0"]` (6 matches)

## Root Cause

**Line 205** contains a license reference that matches `sun-sissl-1.1_4.RULE`. This match is **fully contained within** the larger match at lines 195-494.

### Python vs Rust
- **Python:** Returns 5 matches (filtered out contained match at line 205)
- **Rust:** Returns 6 matches (keeps contained match at line 205)

The match at line 205 (tokens 815-823) is fully contained within the match at lines 195-494 (tokens 806-3180) - both are `sun-sissl-1.1`.

## The Problem

Rust's `filter_contained_matches` doesn't filter matches of the **same license expression** when one is contained within another because of a bug in `qcontains`:

### Detailed Analysis

1. **BIG match** (lines 195-494, tokens 806-3180):
   - `qspan_positions`: Some (sparse, contains only matched positions)
   - Range: 806..=3179, but with GAPS
   - Positions 815-822 are NOT in qspan_positions (gap in matched tokens)

2. **Line 205 match** (lines 205-205, tokens 815-823):
   - `qspan_positions`: None (aho match, uses range check)

3. **Bug in `qcontains`** (src/license_detection/models.rs:540-542):
   ```rust
   if let (Some(self_positions), None) = (&self.qspan_positions, &other.qspan_positions) {
       let self_set: HashSet<usize> = self_positions.iter().copied().collect();
       return (other.start_token..other.end_token).all(|p| self_set.contains(&p));
   }
   ```
   - Checks if ALL positions in `(815..823)` are in `self_positions`
   - But `self_positions` is SPARSE (only matched tokens, not all positions)
   - So positions 815-822 are NOT in the set, and `qcontains` returns `false`

4. **Python's approach** (reference/scancode-toolkit/src/licensedcode/spans.py:200-201):
   ```python
   def __contains__(self, other):
       if isinstance(other, Span):
           return self._set.issuperset(other._set)
   ```
   - Python's `qcontains` delegates to `other.qspan in self.qspan`
   - `Span.__contains__` checks `self._set.issuperset(other._set)`
   - When `other` has no qspan_positions, Python creates a Span from range(start, end)
   - The issuperset check then checks if ALL of `other`'s positions are in `self`'s positions

## Fix Required

### Location
`src/license_detection/models.rs:540-543`

### Current Code (BUGGY)
```rust
if let (Some(self_positions), None) = (&self.qspan_positions, &other.qspan_positions) {
    let self_set: HashSet<usize> = self_positions.iter().copied().collect();
    return (other.start_token..other.end_token).all(|p| self_set.contains(&p));
}
```

### Fixed Code
```rust
if let (Some(_), None) = (&self.qspan_positions, &other.qspan_positions) {
    let (self_qstart, self_qend) = self.qspan_bounds();
    let (other_qstart, other_qend) = other.qspan_bounds();
    return self_qstart <= other_qstart && self_qend >= other_qend;
}
```

### Explanation

The fix uses `qspan_bounds()` instead of checking individual positions:

1. **`qspan_bounds()`** (models.rs:666-678) returns `(min_position, max_position + 1)` from qspan_positions, or `(start_token, end_token)` if None
2. For the BIG match: returns `(806, 3180)` (min/max of sparse positions)
3. For the line 205 match: returns `(815, 823)` (from start_token/end_token since qspan_positions is None)
4. Check: `806 <= 815 && 3180 >= 823` → `true` → match is contained → filter it out

This matches Python's behavior: checking if `other`'s span is contained within `self`'s span by comparing bounds.

## Verification

### Test Command
```bash
cargo test test_plan_007 --lib -- --nocapture
```

### Expected Result
All 12 tests pass, including:
- `test_plan_007_filter_contained_matches_same_license` - Currently failing, will pass
- `test_plan_007_full_detection` - Should show 5 detections instead of 6

### Golden Test
```bash
cargo test test_license_golden_lic4 --lib
```

Should produce expected output with 5 license matches.

## Potential Regressions

### Risk: Over-filtering

**Concern:** Using bounds-based containment might filter matches that shouldn't be filtered.

**Mitigation:** The test suite has comprehensive coverage:
- `test_plan_007_qspan_containment_debug` - Validates the fix doesn't break other containment checks
- Existing tests in `match_refine_test.rs` cover various containment scenarios

### Risk: Different behavior for same-expression vs different-expression containment

**Concern:** Python may have different behavior for contained matches with different license expressions.

**Analysis:** Python's `filter_contained_matches` (match.py:~1400) uses `qcontains` regardless of license expression. The `qcontains` fix applies uniformly.

**Verification:** Run full test suite to ensure no regressions:
```bash
cargo test license_detection --lib
```

## Implementation Steps

1. Edit `src/license_detection/models.rs` line 540-543
2. Replace the HashSet-based position check with `qspan_bounds()` range check
3. Run `cargo test test_plan_007 --lib -- --nocapture` to verify fix
4. Run `cargo test license_detection --lib` to check for regressions
5. Run `cargo clippy` to ensure code quality

## Key Files

- `src/license_detection/models.rs:540-543` - `qcontains()` method - **FIX LOCATION**
- `src/license_detection/models.rs:666-678` - `qspan_bounds()` helper
- `src/license_detection/match_refine.rs:363-419` - `filter_contained_matches()` - Uses qcontains
- `src/license_detection/investigation/something_html_test.rs` - Investigation tests

## Investigation Tests

Key tests in `src/license_detection/investigation/something_html_test.rs`:
- `test_plan_007_filter_contained_matches_same_license` - Failing test that will pass once fixed
- `test_plan_007_qspan_containment_debug` - Debug test showing the containment issue
- `test_plan_007_full_detection` - End-to-end detection test
