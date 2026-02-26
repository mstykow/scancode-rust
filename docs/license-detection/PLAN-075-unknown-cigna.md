# PLAN-075: Unknown Detection - cigna-go-you-mobile-app-eula.txt

## Status: DETAILED IMPLEMENTATION PLAN READY

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| 8 matches | 16 matches (2x more) |

**Expected**: `["proprietary-license", "proprietary-license", "unknown-license-reference", "warranty-disclaimer", "proprietary-license", "warranty-disclaimer", "unknown-license-reference", "unknown"]`

**Actual**: `["proprietary-license", "proprietary-license", "unknown", "unknown", "unknown-license-reference", "unknown", "unknown", "warranty-disclaimer", "unknown", "unknown", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "unknown-license-reference", "unknown"]`

## Root Cause Analysis

### Key Finding: Missing `split_weak_matches()` Function

The root cause is that **Rust does NOT call `split_weak_matches()` before unknown detection**.

### Python Behavior (Correct)

Python's `idx.match()` at `reference/scancode-toolkit/src/licensedcode/index.py:1082-1118`:

```python
if unknown_licenses:
    good_matches, weak_matches = match.split_weak_matches(matches)  # PHASE 2

    # Compute uncovered regions from GOOD matches only
    original_qspan = Span(0, len(qry.tokens) - 1)
    good_qspans = (mtch.qspan for mtch in good_matches)
    good_qspan = Span().union(*good_qspans)
    unmatched_qspan = original_qspan.difference(good_qspan)

    # Run unknown detection on EACH unmatched subspan as separate QueryRun
    unknown_matches = []
    for unspan in unmatched_qspan.subspans():
        unquery_run = query.QueryRun(query=qry, start=unspan.start, end=unspan.end)
        unknown_match = match_unknown.match_unknowns(
            idx=self,
            query_run=unquery_run,
            automaton=self.unknown_automaton,
        )
        if unknown_match:
            unknown_matches.append(unknown_match)

    # Filter invalid contained unknown matches
    unknown_matches = match.filter_invalid_contained_unknown_matches(
        unknown_matches=unknown_matches,
        good_matches=good_matches,
    )

    matches.extend(unknown_matches)
    matches.extend(weak_matches)  # Re-add weak matches at end
```

### `split_weak_matches()` Function (match.py:1740-1765)

```python
def split_weak_matches(matches):
    """
    Return filtered list of kept matches and weak matches.
    Filters out:
    1. Short seq matches (len <= SMALL_RULE, coverage <= 25%)
    2. Matches where rule.has_unknown == True
    """
    kept = []
    discarded = []

    for match in matches:
        if (match.matcher == MATCH_SEQ
            and match.len() <= SMALL_RULE
            and match.coverage() <= 25
        ) or match.rule.has_unknown:  # <-- KEY: filter unknown-license-reference
            discarded_append(match)
        else:
            kept_append(match)

    return kept, discarded
```

### `rule.has_unknown` Property (models.py)

```python
@property
def has_unknown(self):
    return self.license_expression and 'unknown' in self.license_expression
```

### Why This Matters

1. `unknown-license-reference` matches have `"unknown"` in their `license_expression`
2. Python treats these as "weak" and excludes them from the "good_matches" list
3. The regions covered by `unknown-license-reference` become **eligible for unknown detection**
4. Python then runs `match_unknowns()` on those regions and creates proper `unknown` matches
5. Result: `unknown-license-reference` regions are replaced with `unknown` matches

### Rust Behavior (Incorrect)

Rust's `detect()` at `src/license_detection/mod.rs:278-281`:

```rust
let unknown_matches = unknown_match(&self.index, &query, &all_matches);  // Uses ALL matches
let filtered_unknown_matches = filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
all_matches.extend(filtered_unknown_matches);
```

**Problems**:
1. No `split_weak_matches()` call - `unknown-license-reference` matches stay in `all_matches`
2. Unknown matching runs on tiny gaps between matches (not on larger unmatched regions)
3. Regions covered by `unknown-license-reference` are excluded from unknown detection
4. Creates spurious "unknown" matches in gaps that are too small

### Why Spurious Unknown Matches Appear

1. `find_unmatched_regions()` finds ALL gaps between matches
2. Many small gaps (1-3 tokens) pass `MIN_REGION_LENGTH = 5` check
3. `create_unknown_match()` has thresholds but:
   - `UNKNOWN_NGRAM_LENGTH * 4 = 24` is only checked after ngram matching
   - `hispan < 5` check allows many spurious matches through
4. Python avoids this by running `match_unknowns()` on **larger unmatched regions** (not tiny gaps)

---

## Implementation Plan

### Change 1: Add `has_unknown()` method to Rule struct

**File**: `src/license_detection/models.rs`

**Location**: After the `Rule` struct definition (around line 150)

**Code to add**:
```rust
impl Rule {
    /// Check if this rule's license expression contains "unknown".
    ///
    /// Rules like "unknown-license-reference" have "unknown" in their expression.
    /// These are considered "weak" matches and should be excluded from coverage
    /// calculations before unknown license detection.
    ///
    /// Corresponds to Python: `rule.has_unknown` property (models.py)
    pub fn has_unknown(&self) -> bool {
        self.license_expression.contains("unknown")
    }
}
```

### Change 2: Add `split_weak_matches()` function

**File**: `src/license_detection/match_refine.rs`

**Location**: After `filter_invalid_contained_unknown_matches()` (around line 62)

**Code to add**:
```rust
/// Split matches into "good" and "weak" matches.
///
/// Weak matches are:
/// 1. Short sequence matches (len <= SMALL_RULE, coverage <= 25%)
/// 2. Matches where the license expression contains "unknown"
///
/// This is called BEFORE unknown license detection to ensure that
/// matches to "unknown-license-reference" and similar rules don't
/// prevent those regions from being detected as true unknown licenses.
///
/// # Arguments
/// * `matches` - Slice of LicenseMatch to split
///
/// # Returns
/// Tuple of (good_matches, weak_matches)
///
/// Based on Python: `split_weak_matches()` (match.py:1740-1765)
pub fn split_weak_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    const SMALL_RULE: usize = 5;

    let mut good = Vec::new();
    let mut weak = Vec::new();

    for m in matches {
        let is_weak_seq = m.matcher == "3-seq"
            && m.matched_length <= SMALL_RULE
            && m.match_coverage <= 25.0;

        let has_unknown = m.license_expression.contains("unknown");

        if is_weak_seq || has_unknown {
            weak.push(m.clone());
        } else {
            good.push(m.clone());
        }
    }

    (good, weak)
}
```

### Change 3: Update `mod.rs` to export `split_weak_matches`

**File**: `src/license_detection/mod.rs`

**Location**: Line 61-62 (the `pub use match_refine::` block)

**Current code**:
```rust
pub use match_refine::{
    filter_invalid_contained_unknown_matches, merge_overlapping_matches, refine_matches,
};
```

**Change to**:
```rust
pub use match_refine::{
    filter_invalid_contained_unknown_matches, merge_overlapping_matches, refine_matches,
    split_weak_matches,
};
```

### Change 4: Update detection pipeline in `detect()`

**File**: `src/license_detection/mod.rs`

**Location**: Lines 278-281

**Current code**:
```rust
let unknown_matches = unknown_match(&self.index, &query, &all_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
all_matches.extend(filtered_unknown_matches);
```

**Change to**:
```rust
// Split matches into "good" and "weak" before unknown detection.
// Weak matches (e.g., unknown-license-reference) are excluded from
// coverage calculation so their regions can be detected as unknown.
// Corresponds to Python: index.py:1082-1118
let (good_matches, weak_matches) = split_weak_matches(&all_matches);

// Run unknown matching only on regions not covered by good_matches
let unknown_matches = unknown_match(&self.index, &query, &good_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &good_matches);

// Reconstruct all_matches: good + unknown + weak
// Note: weak_matches are added back at the end (Python: index.py:1117-1118)
all_matches = good_matches;
all_matches.extend(filtered_unknown_matches);
all_matches.extend(weak_matches);
```

### Change 5: Add unit test for `split_weak_matches()`

**File**: `src/license_detection/match_refine.rs`

**Location**: In the `#[cfg(test)] mod tests` block (around line 1548)

**Code to add**:
```rust
#[test]
fn test_split_weak_matches_no_weak() {
    let matches = vec![create_test_match("#1", 1, 10, 100.0, 100.0, 100)];
    let (good, weak) = split_weak_matches(&matches);
    assert_eq!(good.len(), 1);
    assert_eq!(weak.len(), 0);
}

#[test]
fn test_split_weak_matches_has_unknown() {
    let mut m = create_test_match("#1", 1, 10, 100.0, 100.0, 100);
    m.license_expression = "unknown-license-reference".to_string();

    let (good, weak) = split_weak_matches(&[m]);
    assert_eq!(good.len(), 0);
    assert_eq!(weak.len(), 1);
}

#[test]
fn test_split_weak_matches_short_seq_low_coverage() {
    let mut m = create_test_match("#1", 1, 3, 50.0, 20.0, 100); // len=3, coverage=20%
    m.matcher = "3-seq".to_string();

    let (good, weak) = split_weak_matches(&[m]);
    assert_eq!(good.len(), 0);
    assert_eq!(weak.len(), 1);
}

#[test]
fn test_split_weak_matches_mixed() {
    let m1 = create_test_match("#1", 1, 10, 100.0, 100.0, 100); // good

    let mut m2 = create_test_match("#2", 20, 30, 100.0, 100.0, 100);
    m2.license_expression = "unknown-license-reference".to_string(); // weak

    let mut m3 = create_test_match("#3", 40, 50, 100.0, 100.0, 100); // good

    let (good, weak) = split_weak_matches(&[m1, m2, m3]);
    assert_eq!(good.len(), 2);
    assert_eq!(weak.len(), 1);
}
```

---

## Test Strategy

### 1. Unit Tests

Run the new unit tests:
```bash
cargo test split_weak_matches -- --nocapture
```

### 2. Golden Test for This File

Run the specific golden test:
```bash
cargo test test_cigna_go_you_mobile_app_eula -- --nocapture
```

Or run all unknown golden tests:
```bash
cargo test golden_tests::test_golden_unknown -- --nocapture
```

### 3. Verify Match Counts

After fix, the Rust output should match Python:
- 8 matches (not 16)
- No spurious "unknown" matches between other matches

### 4. Regression Tests

Run the full golden test suite to ensure no regressions:
```bash
cargo test golden_tests -- --nocapture
```

### 5. Compare Related Plans

This fix should also resolve:
- PLAN-076: cisco.txt (same root cause)
- PLAN-077: citrix.txt (same root cause)
- PLAN-079: scea.txt (same root cause)

---

## Summary of Changes

| File | Lines | Change |
|------|-------|--------|
| `src/license_detection/models.rs` | ~150 | Add `Rule::has_unknown()` method |
| `src/license_detection/match_refine.rs` | ~62 | Add `split_weak_matches()` function |
| `src/license_detection/match_refine.rs` | ~1548 | Add unit tests for `split_weak_matches()` |
| `src/license_detection/mod.rs` | 61-62 | Export `split_weak_matches` |
| `src/license_detection/mod.rs` | 278-281 | Update pipeline to use `split_weak_matches()` |

---

## Related Files

- `src/license_detection/unknown_match.rs` - Unknown match detection
- `src/license_detection/mod.rs` - Detection pipeline orchestration
- `src/license_detection/match_refine.rs` - Match refinement functions
- `reference/scancode-toolkit/src/licensedcode/match_unknown.py` - Python reference
- `reference/scancode-toolkit/src/licensedcode/index.py:1082-1118` - Python unknown detection pipeline
- `reference/scancode-toolkit/src/licensedcode/match.py:1740-1765` - Python `split_weak_matches()` reference
