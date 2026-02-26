# PLAN-076: Unknown Detection - cisco.txt

## Status: IMPLEMENTATION PLANNED

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/cisco.txt`

| Expected | Actual |
|----------|--------|
| `["unknown", "warranty-disclaimer", "unknown"]` (3) | `["unknown-license-reference", "unknown-license-reference", "unknown", "unknown", "warranty-disclaimer", "warranty-disclaimer", "unknown", "unknown"]` (8) |

**Issue**: Wrong detection type (unknown-license-reference vs unknown) and doubled counts.

## Root Cause Analysis

### Python Behavior (Correct)

Python's `idx.match()` with `unknown_licenses=True`:

1. **Phase 1 - Regular matching**: Detects `unknown-license-reference` matches from rule `license-intro_21.RULE` (lines 1-1, 3-3) and `warranty-disclaimer` from `warranty-disclaimer_21.RULE` (lines 20-22)

2. **Phase 2 - `split_weak_matches()`**: Filters out matches where `rule.has_unknown == True`:
   - `unknown-license-reference` matches have `has_unknown=True` (because "unknown" is in license_expression)
   - These are moved to the "weak" list and set aside

3. **Phase 3 - Unknown matching**: Runs `match_unknowns()` on the **uncovered regions** (positions not covered by "good" matches)
   - Since `warranty-disclaimer` is the only "good" match, the uncovered regions are lines 1-19 and 24-32
   - These are detected as `unknown` license matches

4. **Final result**: `["unknown", "warranty-disclaimer", "unknown"]`

### Rust Behavior (Incorrect)

Rust's `detect()`:

1. **Phase 1 - Regular matching**: Same as Python, detects `unknown-license-reference` and `warranty-disclaimer`

2. **MISSING Phase 2 - `split_weak_matches()`**: Rust does NOT filter out matches with `has_unknown` expressions
   - The `unknown-license-reference` matches remain in `all_matches`

3. **Phase 3 - Unknown matching**: Runs `unknown_match()` but with incorrect coverage calculation
   - The `unknown-license-reference` matches are considered "known", so those regions are excluded
   - But they're also included in final output

4. **Result**: Both `unknown-license-reference` AND `unknown` matches appear, plus duplicates from region grouping

### Key Difference

**Python's `split_weak_matches()` function** (match.py:1740-1765):

```python
def split_weak_matches(matches):
    """
    Return a filtered list of kept LicenseMatch matches and a list of weak
    matches given a `matches` list of LicenseMatch by considering shorter
    sequence matches with a low coverage or match to unknown licenses. These are
    set aside before "unknown license" matching.
    """
    from licensedcode.match_seq import MATCH_SEQ

    kept = []
    kept_append = kept.append
    discarded = []
    discarded_append = discarded.append

    for match in matches:
        # always keep exact matches
        if (match.matcher == MATCH_SEQ
            and match.len() <= SMALL_RULE
            and match.coverage() <= 25
        ) or match.rule.has_unknown:

            discarded_append(match)
        else:
            kept_append(match)

    return kept, discarded
```

The `rule.has_unknown` property (models.py:1861-1867):

```python
@property
def has_unknown(self):
    """
    Return True if any of this rule licenses is an unknown license.
    """
    return self.license_expression and 'unknown' in self.license_expression
```

**Python's usage in `idx.match()`** (index.py:1082-1118):

```python
if unknown_licenses:
    good_matches, weak_matches = match.split_weak_matches(matches)
    # collect the positions that are "good matches" to exclude from
    # matching for unknown_licenses. Create a Span to check for unknown
    # based on this.
    original_qspan = Span(0, len(qry.tokens) - 1)
    good_qspans = (mtch.qspan for mtch in good_matches)
    good_qspan = Span().union(*good_qspans)

    unmatched_qspan = original_qspan.difference(good_qspan)

    # for each subspan, run unknown license detection
    unknown_matches = []
    for unspan in unmatched_qspan.subspans():
        # ... run match_unknowns on each unspan ...

    unknown_matches = match.filter_invalid_contained_unknown_matches(
        unknown_matches=unknown_matches,
        good_matches=good_matches,
    )

    matches.extend(unknown_matches)
    # reinject weak matches and let refine matches keep the bests
    matches.extend(weak_matches)
```

## Missing Implementation in Rust

1. **`Rule.has_unknown` property**: Rust's `Rule` struct does not have a `has_unknown` method

2. **`split_weak_matches()` function**: Rust's detection pipeline does not call this function before unknown matching

---

## Detailed Implementation Plan

### Step 1: Add `has_unknown()` method to `LicenseMatch`

**File**: `src/license_detection/models.rs`

**Location**: Add method to `impl LicenseMatch` block (around line 415)

**Implementation**:

```rust
/// Check if this match's license expression contains "unknown".
///
/// Matches with "unknown" in their license_expression (e.g., "unknown-license-reference")
/// are considered "weak" and set aside before unknown detection.
///
/// Corresponds to Python: `rule.has_unknown` property (models.py:1861-1867)
pub fn has_unknown(&self) -> bool {
    self.license_expression.contains("unknown")
}
```

**Why on `LicenseMatch` and not `Rule`**:
- The check is based on `license_expression`, which is already copied to `LicenseMatch`
- `LicenseMatch` is what we're iterating over in the pipeline
- Avoids needing to look up the rule from the index

### Step 2: Implement `split_weak_matches()` function

**File**: `src/license_detection/match_refine.rs`

**Location**: Add after `filter_invalid_contained_unknown_matches()` function (around line 62)

**Implementation**:

```rust
/// Split matches into "good" and "weak" based on match quality and unknown status.
///
/// Matches are considered weak if:
/// 1. They are sequence matches ("3-seq") with:
///    - Matched length <= SMALL_RULE (15 tokens), AND
///    - Match coverage <= 25%
/// 2. OR they have "unknown" in their license expression
///
/// Weak matches are set aside before unknown license detection so they don't
/// block unknown detection in their regions.
///
/// # Arguments
/// * `matches` - Slice of LicenseMatch to split
///
/// # Returns
/// Tuple of (kept_matches, weak_matches)
///
/// Based on Python: `split_weak_matches()` (match.py:1740-1765)
pub fn split_weak_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    use crate::license_detection::rules::thresholds::SMALL_RULE;
    use crate::license_detection::seq_match::MATCH_SEQ;

    let mut kept = Vec::new();
    let mut weak = Vec::new();

    for m in matches {
        // Condition 1: Short seq matches with low coverage
        let is_weak_seq = m.matcher == MATCH_SEQ
            && m.matched_length <= SMALL_RULE
            && m.match_coverage <= 25.0;

        // Condition 2: Has "unknown" in license expression
        let has_unknown = m.has_unknown();

        if is_weak_seq || has_unknown {
            weak.push(m.clone());
        } else {
            kept.push(m.clone());
        }
    }

    (kept, weak)
}
```

**Key details**:
- `SMALL_RULE` constant is already defined in `rules/thresholds.rs` (value: 15)
- `MATCH_SEQ` constant is already defined in `seq_match.rs` (value: "3-seq")
- Uses `matched_length` (token count, not character length) - matches Python's `match.len()`
- Uses `match_coverage` (percentage 0-100) - matches Python's `match.coverage()`

### Step 3: Update detection pipeline in `mod.rs`

**File**: `src/license_detection/mod.rs`

**Location**: Around lines 274-283 in `LicenseDetectionEngine::detect()`

**Current code**:
```rust
// Merge all sequence matches ONCE (like Python's approx matcher)
let merged_seq = merge_overlapping_matches(&seq_all_matches);
all_matches.extend(merged_seq);

let unknown_matches = unknown_match(&self.index, &query, &all_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
all_matches.extend(filtered_unknown_matches);

let refined = refine_matches(&self.index, all_matches, &query);
```

**Updated code**:
```rust
// Merge all sequence matches ONCE (like Python's approx matcher)
let merged_seq = merge_overlapping_matches(&seq_all_matches);
all_matches.extend(merged_seq);

// Split weak matches before unknown detection
// Weak matches (e.g., unknown-license-reference) are set aside
// so they don't block unknown detection in their regions
let (good_matches, weak_matches) = split_weak_matches(&all_matches);

// Run unknown matching only on regions not covered by good_matches
let unknown_matches = unknown_match(&self.index, &query, &good_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &good_matches);

// Combine: good_matches + unknown_matches + weak_matches
// The weak matches are re-injected so refine_matches can keep the best
all_matches = good_matches;
all_matches.extend(filtered_unknown_matches);
all_matches.extend(weak_matches);

let refined = refine_matches(&self.index, all_matches, &query);
```

**Critical ordering** (matches Python index.py:1082-1118):
1. Split matches into good and weak
2. Run unknown matching against `good_matches` only
3. Filter invalid contained unknowns against `good_matches` only
4. Combine: good + unknown + weak
5. Run refine_matches on the combined list

### Step 4: Update imports in `match_refine.rs`

Add to the imports at the top of the file:

```rust
use crate::license_detection::rules::thresholds::SMALL_RULE;
use crate::license_detection::seq_match::MATCH_SEQ;
```

### Step 5: Update imports in `mod.rs`

Add `split_weak_matches` to the imports from `match_refine`:

```rust
use match_refine::{
    filter_invalid_contained_unknown_matches,
    group_matches_by_region,
    merge_overlapping_matches,
    refine_matches,
    sort_matches_by_line,
    split_weak_matches,  // ADD THIS
};
```

---

## Understanding `has_unknown`

### What it means:
- A match has `has_unknown=true` if its `license_expression` string contains the substring "unknown"
- This catches expressions like:
  - `unknown-license-reference`
  - `unknown`
  - `mit AND unknown`
  - Any expression with "unknown" in it

### Why it matters:
- Rules with "unknown" in their expression are "intro" or "reference" rules
- They detect patterns like "This software is licensed under an unknown license"
- These should NOT block unknown detection - the region should still be analyzed for unknown licenses
- Setting them aside allows proper unknown detection

### Example from cisco.txt:
- Rule `license-intro_21.RULE` has `license_expression: "unknown-license-reference"`
- Python: This match is moved to `weak_matches`, so lines 1-3 are NOT covered by "good" matches
- Result: Unknown detection runs on lines 1-3, detecting as `unknown`
- Rust (current): This match stays in `all_matches`, blocking unknown detection on those lines

---

## Test Strategy

### Unit Tests

**File**: `src/license_detection/match_refine.rs` (add to `#[cfg(test)] mod tests`)

```rust
#[test]
fn test_split_weak_matches_has_unknown() {
    // Match with "unknown" in license_expression should be weak
    let match_unknown = LicenseMatch {
        license_expression: "unknown-license-reference".to_string(),
        matcher: "2-aho".to_string(),
        matched_length: 20,
        match_coverage: 100.0,
        ..Default::default()
    };

    let (kept, weak) = split_weak_matches(&[match_unknown.clone()]);
    assert!(weak.iter().any(|m| m.license_expression == "unknown-license-reference"));
    assert!(kept.is_empty());
}

#[test]
fn test_split_weak_matches_short_seq_low_coverage() {
    // Short seq match with low coverage should be weak
    let match_short = LicenseMatch {
        license_expression: "mit".to_string(),
        matcher: "3-seq".to_string(),
        matched_length: 10,  // <= SMALL_RULE (15)
        match_coverage: 20.0, // <= 25%
        ..Default::default()
    };

    let (kept, weak) = split_weak_matches(&[match_short.clone()]);
    assert!(weak.iter().any(|m| m.matcher == "3-seq"));
    assert!(kept.is_empty());
}

#[test]
fn test_split_weak_matches_good_match_kept() {
    // Good match should be kept
    let match_good = LicenseMatch {
        license_expression: "mit".to_string(),
        matcher: "2-aho".to_string(),
        matched_length: 50,
        match_coverage: 100.0,
        ..Default::default()
    };

    let (kept, weak) = split_weak_matches(&[match_good.clone()]);
    assert!(kept.iter().any(|m| m.license_expression == "mit"));
    assert!(weak.is_empty());
}

#[test]
fn test_split_weak_matches_mixed() {
    let matches = vec![
        LicenseMatch {
            license_expression: "unknown-license-reference".to_string(),
            matcher: "2-aho".to_string(),
            matched_length: 20,
            match_coverage: 100.0,
            ..Default::default()
        },
        LicenseMatch {
            license_expression: "warranty-disclaimer".to_string(),
            matcher: "2-aho".to_string(),
            matched_length: 30,
            match_coverage: 100.0,
            ..Default::default()
        },
    ];

    let (kept, weak) = split_weak_matches(&matches);
    assert_eq!(kept.len(), 1);
    assert_eq!(weak.len(), 1);
    assert_eq!(kept[0].license_expression, "warranty-disclaimer");
    assert_eq!(weak[0].license_expression, "unknown-license-reference");
}
```

### Golden Test Verification

After implementation, run:

```bash
cargo test golden_tests::test_golden_unknown -- --nocapture
```

**Expected**: `cisco.txt` should produce `["unknown", "warranty-disclaimer", "unknown"]`

### Manual Testing

```bash
# Run on cisco.txt directly
cargo run -- testdata/license-golden/datadriven/unknown/cisco.txt -o /tmp/cisco-output.json

# Check the license expressions in the output
jq '.files[0].licenses[].license_expression' /tmp/cisco-output.json
```

Expected output:
```json
"unknown"
"warranty-disclaimer"
"unknown"
```

---

## Related Files

| File | Purpose |
|------|---------|
| `src/license_detection/mod.rs` | Main detection pipeline - add split_weak_matches call |
| `src/license_detection/match_refine.rs` | Add split_weak_matches() function |
| `src/license_detection/models.rs` | Add has_unknown() method to LicenseMatch |
| `src/license_detection/rules/thresholds.rs` | SMALL_RULE constant (already exists) |
| `src/license_detection/seq_match.rs` | MATCH_SEQ constant (already exists) |
| `reference/scancode-toolkit/src/licensedcode/match.py` | Python split_weak_matches() reference |
| `reference/scancode-toolkit/src/licensedcode/index.py` | Python pipeline usage reference |

---

## Implementation Checklist

- [ ] Add `has_unknown()` method to `LicenseMatch` in `models.rs`
- [ ] Add `split_weak_matches()` function to `match_refine.rs`
- [ ] Update imports in `match_refine.rs` (SMALL_RULE, MATCH_SEQ)
- [ ] Update imports in `mod.rs` (split_weak_matches)
- [ ] Update detection pipeline in `mod.rs::detect()`
- [ ] Add unit tests for `split_weak_matches()`
- [ ] Add unit tests for `has_unknown()`
- [ ] Run golden tests to verify cisco.txt fix
- [ ] Run full test suite to ensure no regressions

---

## Edge Cases to Consider

1. **Empty matches list**: `split_weak_matches(&[])` should return `(vec![], vec![])`

2. **All matches are weak**: Result should be `(vec![], vec![...])`

3. **No weak matches**: Result should be `([...], vec![])`

4. **Hash matches (matcher == "1-hash")**: Should always be kept (not seq, not unknown)

5. **Aho matches (matcher == "2-aho")**: Should be kept unless has_unknown

6. **Exact matches with unknown**: Should be moved to weak (e.g., exact match on "unknown-license-reference" rule)

---

## Python Reference

### `split_weak_matches()` location
- File: `reference/scancode-toolkit/src/licensedcode/match.py`
- Lines: 1740-1765

### `has_unknown` property location
- File: `reference/scancode-toolkit/src/licensedcode/models.py`
- Lines: 1861-1867

### Usage in `idx.match()`
- File: `reference/scancode-toolkit/src/licensedcode/index.py`
- Lines: 1082-1118

### Constants
- `SMALL_RULE = 15` in `reference/scancode-toolkit/src/licensedcode/__init__.py:20`
- `MATCH_SEQ` is imported from `licensedcode.match_seq`
