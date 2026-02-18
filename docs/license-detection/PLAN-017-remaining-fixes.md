# PLAN-017: Remaining License Detection Fixes

## Status: Verified Against Python Reference

### Summary

| Test Suite | Passed | Failed | Pass Rate |
|------------|-------|--------|-----------|
| lic1 | 213 | 78 | 73% |
| lic2 | 759 | 94 | 89% |
| lic3 | 242 | 50 | 83% |
| lic4 | 265 | 85 | 76% |
| external | 1935 | 632 | 75% |
| unknown | 2 | 8 | 20% |
| **TOTAL** | **3416** | **947** | **78%** |

---

## Issue 1: Expression Over-Combination

**Status**: ❌ **NOT A BUG** - Current behavior matches Python

### Investigation Finding

The proposed fix was to add subset detection in `collect_unique_and()`. However, verification against Python shows:

**Python's behavior** (`license-expression` library docstring):

```python
>>> str(combine_expressions(('mit WITH foo', 'gpl', 'mit',)))
'mit WITH foo AND gpl AND mit'  # Note: 'mit' appears twice
```

**Python explicitly documents**: "Choices (as in 'MIT or GPL') are kept as-is and not treated as simplifiable. This avoids dropping important choice options in complex expressions which is never desirable."

### Conclusion

- Rust's output of `"(gpl-2.0 OR mit) AND mit"` is **correct behavior matching Python**
- The test expectations may be wrong, OR this is a different issue
- **No fix needed** for expression combination

### Action

Investigate whether the test expectations are correct. Compare Python's actual output for these test files.

---

## Issue 2: Golden Test Comparison Bug ✅ READY

**Status**: ✅ **VERIFIED - Test Bug, Not Detection Bug**

### Root Cause

**File**: `src/license_detection/golden_test.rs:122-125`

The Rust test compares **detection expressions** (combined):

```rust
let actual: Vec<&str> = detections
    .iter()
    .map(|d| d.license_expression.as_deref().unwrap_or(""))
    .collect();
```

But Python's test compares **match expressions** (per-match):

```python
detected_expressions = [match.rule.license_expression for match in matches]
```

### Evidence

Looking at `.yml` expected files:

```yaml
license_expressions:
  - gpl-1.0-plus   # Each entry is ONE MATCH's expression
  - gpl-2.0-plus   # Not a combined detection expression
```

### Fix

**File**: `src/license_detection/golden_test.rs:122-125`

```rust
// BEFORE (wrong):
let actual: Vec<&str> = detections
    .iter()
    .map(|d| d.license_expression.as_deref().unwrap_or(""))
    .collect();

// AFTER (correct - matches Python):
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

### Expected Impact

- **~20+ tests** that were "failing" due to wrong comparison will now pass
- This is a **test bug**, not a detection bug

---

## Issue 3: Missing Detections ✅ READY

**Status**: ✅ **VERIFIED - Remove Wrong Filter**

### Root Cause

**File**: `src/license_detection/match_refine.rs:31-43`

Rust has a custom `filter_short_gpl_matches()` that Python does NOT have:

```rust
fn filter_short_gpl_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    const GPL_SHORT_THRESHOLD: usize = 3;
    matches.iter().filter(|m| {
        let is_gpl = m.license_expression.to_lowercase().contains("gpl");
        let is_short = m.matched_length <= GPL_SHORT_THRESHOLD;
        !(is_gpl && is_short)  // Filters out valid matches!
    }).cloned().collect()
}
```

**Python's approach** (`detection.py:1227`):

- No early GPL filter
- GPL filtering happens in `is_false_positive()` using `rule.length` (not `matched_length`)

### Why It's Wrong

| Metric | Rust uses | Python uses |
|--------|-----------|-------------|
| GPL filter metric | `matched_length` (tokens matched) | `rule.length` (tokens in rule template) |

Example: `License: GPL` has:

- `matched_length = 2` (two tokens matched)
- `rule.length = 2` (two tokens in rule)

Rust incorrectly filters this as "short GPL" when it's a valid license tag.

### Fix

**File**: `src/license_detection/match_refine.rs`

1. **Delete** `filter_short_gpl_matches()` function (lines 31-43)
2. **Remove** call in `refine_matches()` (line ~1002)
3. **Keep** existing GPL check in `is_false_positive()` (detection.rs:350-354) - it's correct

```rust
// DELETE THIS FUNCTION:
fn filter_short_gpl_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> { ... }
```

### Expected Impact

- `gpl-2.0_30.txt` will detect `gpl-1.0-plus` from `License: GPL`
- Other short GPL license tags will be detected correctly

---

## Issue 4: Query Run Stale References ✅ READY

**Status**: ✅ **VERIFIED - Lazy Evaluation Pattern**

### Root Cause

**File**: `src/license_detection/query.rs`

`QueryRun` stores references to `Query.high_matchables`, but `Query.subtract()` creates a NEW `HashSet`, leaving dangling references.

**Python's approach** (`query.py:845-861`):

```python
def __init__(self, query, start, end=None):
    self._high_matchables = None  # Lazy init
    
@property
def high_matchables(self):
    if not self._high_matchables:
        self._high_matchables = intbitset(
            [pos for pos in self.query.high_matchables
             if self.start <= pos <= self.end])
    return self._high_matchables
```

Python lazily evaluates `high_matchables` from parent query.

### Fix

**File**: `src/license_detection/query.rs`

1. Change `QueryRun` to store parent query reference:

```rust
pub struct QueryRun<'a> {
    query: &'a Query<'a>,
    pub start: usize,
    pub end: Option<usize>,
}
```

1. Implement `high_matchables()` and `low_matchables()` as on-demand methods:

```rust
impl<'a> QueryRun<'a> {
    pub fn high_matchables(&self) -> HashSet<usize> {
        self.query.high_matchables
            .iter()
            .filter(|&&pos| pos >= self.start && pos <= self.end.unwrap_or(usize::MAX))
            .copied()
            .collect()
    }
    
    pub fn low_matchables(&self) -> HashSet<usize> {
        self.query.low_matchables
            .iter()
            .filter(|&&pos| pos >= self.start && pos <= self.end.unwrap_or(usize::MAX))
            .copied()
            .collect()
    }
}
```

1. Access other fields via `self.query`: `tokens`, `line_by_pos`, `text`, `index`

### Expected Impact

- Query runs will work correctly when enabled
- Safe to re-enable query runs in detection pipeline

---

## Issue 5: Unknown License Filtering ✅ READY

**Status**: ✅ **VERIFIED - Add Post-Filter**

### Root Cause

**File**: `src/license_detection/mod.rs:223-224`

Python has a post-filter that Rust is missing:

```python
# index.py:1111-1114
unknown_matches = match.filter_invalid_contained_unknown_matches(
    unknown_matches=unknown_matches,
    good_matches=good_matches,
)
```

This filters out unknown matches that are contained within known matches.

### Note: Coverage Calculation is Already Correct

The investigation found that `compute_covered_positions()` is already correct:

- Uses `start_token..end_token` which matches Python's `qregion()` approach
- No changes needed to coverage calculation

### Fix

**File**: `src/license_detection/match_refine.rs` or `unknown_match.rs`

Add new function:

```rust
/// Filter unknown matches that are contained within known matches.
///
/// An unknown match should be discarded if its qspan is fully contained
/// within any known match's qregion.
///
/// Based on Python: filter_invalid_contained_unknown_matches() at match.py:1904-1928
pub fn filter_invalid_contained_unknown_matches(
    unknown_matches: Vec<LicenseMatch>,
    good_matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    unknown_matches
        .into_iter()
        .filter(|unknown| {
            let unknown_start = unknown.start_token;
            let unknown_end = unknown.end_token;
            
            // Check if unknown is contained in any known match's qregion
            !good_matches.iter().any(|good| {
                good.start_token <= unknown_start && good.end_token >= unknown_end
            })
        })
        .collect()
}
```

**File**: `src/license_detection/mod.rs:223-224`

```rust
// BEFORE:
let unknown_matches = unknown_match(&self.index, &query, &all_matches);
all_matches.extend(unknown_matches);

// AFTER:
let unknown_matches = unknown_match(&self.index, &query, &all_matches);
let unknown_matches = filter_invalid_contained_unknown_matches(unknown_matches, &all_matches);
all_matches.extend(unknown_matches);
```

### Expected Impact

- Removes spurious `unknown` and `unknown-license-reference` matches
- Fixes `gpl-2.0-plus_21.txt`, `gpl-2.0_and_gpl-2.0_and_gpl-2.0-plus.txt`

---

## Implementation Order

1. **Issue 2** (Golden test fix) - Simplest, will "fix" many tests immediately
2. **Issue 3** (Remove filter_short_gpl_matches) - Simple deletion
3. **Issue 5** (Add unknown filter) - Add one function and one call
4. **Issue 4** (Query Run lazy eval) - More complex, affects struct layout
5. **Issue 1** (Investigate test expectations) - May not need a fix

---

## Verification Commands

```bash
# Run lic1 tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Run all golden tests
cargo test --release -q --lib license_detection::golden_test

# Format and lint
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```
