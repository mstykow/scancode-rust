# PLAN-079: Unknown Detection - scea.txt

## Status: DETAILED PLAN READY

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/scea.txt`

| Expected | Actual |
|----------|--------|
| 5 matches (Python with `--unknown-licenses`) | 12 matches (Rust) |

**Expected (Python with `--unknown-licenses`)**:
- `scea-1.0` (line 1)
- `scea-1.0` (line 7)
- `unknown` (lines 7-22)
- `unknown` (lines 22-31)
Plus 1 license_clue: `unknown-license-reference` (line 1)

**Actual (Rust)**:
- `scea-1.0` (line 1)
- `unknown-license-reference` (line 1) - **EXTRA - should be license_clue**
- `scea-1.0` (line 7)
- `unknown` (lines 7-14) - **WRONG SPAN**
- `unknown-license-reference` (line 17) - **EXTRA**
- `unknown-license-reference` (line 17) - **EXTRA DUPLICATE**
- `unknown` (lines 19-22) - **WRONG SPAN**
- `unknown-license-reference` (line 22) - **EXTRA**
- `unknown` (lines 22-22) - **EXTRA**
- `unknown` (lines 22-22) - **EXTRA DUPLICATE**
- `unknown` (lines 22-26) - **WRONG SPAN**
- `unknown` (lines 26-31) - **WRONG SPAN**

## Root Causes Identified

### 1. Missing `split_weak_matches` Logic (SHARED WITH PLAN-076)

**Python** (`index.py:1082-1118`):
```python
if unknown_licenses:
    good_matches, weak_matches = match.split_weak_matches(matches)
    # Run unknown matching only on gaps from good_matches
    unknown_matches = match_unknown.match_unknowns(...)
    matches.extend(unknown_matches)
    matches.extend(weak_matches)  # Re-add weak matches after
```

**Python** (`match.py:1740-1765`):
```python
def split_weak_matches(matches):
    for match in matches:
        if (match.matcher == MATCH_SEQ and match.len() <= SMALL_RULE and match.coverage() <= 25
        ) or match.rule.has_unknown:  # "unknown" in license_expression
            discarded_append(match)
        else:
            kept_append(match)
    return kept, discarded
```

**Rust Issue**: Rust does NOT separate weak matches before unknown matching. This means:
- Rust includes `unknown-license-reference` matches in the "covered" positions
- Python excludes them, allowing unknown matching to run on those regions
- Result: Extra `unknown-license-reference` detections in Rust

**Impact**: High - Affects both PLAN-076 and PLAN-079

### 2. Different Unknown Match Merging

**Python** (`match_unknown.py:150-152`):
```python
qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
qspan = Span().union(*qspans)  # MERGE overlapping ngrams into ONE span
```

Python's `Span.union()` merges overlapping/adjacent positions into a single span:
- If ngrams at positions 10-15 and 13-20 overlap, they merge to 10-20
- This produces ONE large unknown match covering the union of all matched ngrams

**Rust** (`unknown_match.rs:127-145`):
```rust
for region in unmatched_regions {
    // Creates separate match for EACH unmatched region
    // Does NOT merge ngram matches within regions
}
```

**Impact**: Python produces 2 large unknown matches (7-22, 22-31), Rust produces 5 smaller fragmented ones

### 3. Missing `license_clues` Handling

**Python**: Matches with `unknown-license-reference` expression that overlap with known licenses become `license_clues` (not full detections).

**Rust**: Creates full detections for `unknown-license-reference` matches.

**Impact**: Medium - Affects output format but not detection count

## Dependency Analysis

### PLAN-076 Shared Root Cause

PLAN-076 (`cisco.txt`) and PLAN-079 (`scea.txt`) share **Root Cause 1** (missing `split_weak_matches`).

**Implementation Order**: Fix `split_weak_matches` once, both plans benefit.

### Other Plans Potentially Affected

- PLAN-077 (`citrix.txt`) - Likely same root cause
- PLAN-078 (`qt-commercial.txt`) - Likely same root cause
- PLAN-080 (`ucware.txt`) - Likely same root cause
- PLAN-073 (`readme.txt`), PLAN-074 (`cclrc.txt`), PLAN-075 (`cigna.txt`) - May also benefit

## Detailed Implementation Plan

### Step 1: Add `has_unknown` Property to Rule (DEPENDENCY: None)

**File**: `src/license_detection/models.rs`

**Location**: Add to `impl Rule` block (after existing methods)

**Implementation**:
```rust
impl Rule {
    /// Check if this rule's license_expression contains "unknown".
    ///
    /// Rules with "unknown" in their expression are considered "weak" matches
    /// and are set aside before unknown license detection runs.
    ///
    /// Corresponds to Python: `rule.has_unknown` property (models.py)
    pub fn has_unknown(&self) -> bool {
        self.license_expression.contains("unknown")
    }
}
```

**Tests**: Add unit test in `models.rs`:
```rust
#[test]
fn test_rule_has_unknown() {
    let rule_with_unknown = Rule {
        license_expression: "unknown-license-reference".to_string(),
        // ... other fields
    };
    assert!(rule_with_unknown.has_unknown());

    let rule_without_unknown = Rule {
        license_expression: "mit".to_string(),
        // ... other fields
    };
    assert!(!rule_without_unknown.has_unknown());
}
```

### Step 2: Implement `split_weak_matches()` Function (DEPENDENCY: Step 1)

**File**: `src/license_detection/match_refine.rs`

**Location**: Add after `filter_invalid_contained_unknown_matches()` (around line 62)

**Implementation**:
```rust
/// Split matches into "good" and "weak" matches.
///
/// Weak matches are:
/// - Short sequence matches (len <= SMALL_RULE, coverage <= 25%)
/// - Matches where the rule's license_expression contains "unknown"
///
/// These are set aside before unknown license detection runs,
/// then re-added afterward.
///
/// # Arguments
/// * `matches` - Slice of LicenseMatch to split
/// * `index` - LicenseIndex to access rule properties
///
/// # Returns
/// Tuple of (good_matches, weak_matches)
///
/// Based on Python: `split_weak_matches()` (match.py:1740-1765)
pub fn split_weak_matches(
    matches: &[LicenseMatch],
    index: &LicenseIndex,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    const SMALL_RULE: usize = 12;  // Python: match.py:45

    let mut good = Vec::new();
    let mut weak = Vec::new();

    for m in matches {
        // Get rule to check has_unknown
        let rule = index.rules_by_rid.get(m.rid);

        // Check for weak sequence match (short + low coverage)
        let is_weak_seq = m.matcher == "3-seq"
            && m.matched_length <= SMALL_RULE
            && m.match_coverage <= 25.0;

        // Check for unknown expression (key condition for PLAN-076/079)
        let has_unknown = rule.map(|r| r.has_unknown()).unwrap_or(false);

        if is_weak_seq || has_unknown {
            weak.push(m.clone());
        } else {
            good.push(m.clone());
        }
    }

    (good, weak)
}
```

**Tests**: Add unit tests:
```rust
#[cfg(test)]
mod tests_split_weak_matches {
    use super::*;

    #[test]
    fn test_split_by_has_unknown() {
        // Test that matches with has_unknown go to weak
    }

    #[test]
    fn test_split_by_weak_seq() {
        // Test that short/low-coverage seq matches go to weak
    }

    #[test]
    fn test_keep_good_matches() {
        // Test that good matches stay in good list
    }
}
```

### Step 3: Update Detection Pipeline (DEPENDENCY: Step 2)

**File**: `src/license_detection/mod.rs`

**Location**: Around line 276-281, modify the unknown matching section

**Current Code** (lines 276-281):
```rust
let unknown_matches = unknown_match(&self.index, &query, &all_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
all_matches.extend(filtered_unknown_matches);
```

**New Code**:
```rust
// Split weak matches before unknown detection
// This is critical: matches with "unknown" in expression should NOT
// be considered "known" when computing uncovered regions
let (good_matches, weak_matches) = split_weak_matches(&all_matches, &self.index);

// Run unknown matching only on regions NOT covered by good_matches
let unknown_matches = unknown_match(&self.index, &query, &good_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &good_matches);

// Reconstruct final match list: good + unknown + weak
// Order matches Python: matches.extend(unknown_matches); matches.extend(weak_matches)
all_matches = good_matches;
all_matches.extend(filtered_unknown_matches);
all_matches.extend(weak_matches);
```

**Also update**: Export `split_weak_matches` at line 61:
```rust
pub use match_refine::{
    filter_invalid_contained_unknown_matches, merge_overlapping_matches, refine_matches,
    split_weak_matches,  // ADD THIS
};
```

### Step 4: Fix Unknown Match Merging (DEPENDENCY: None, but should be done after Step 3)

**File**: `src/license_detection/unknown_match.rs`

**Problem**: Current implementation creates one match per unmatched region. Python merges overlapping ngram positions first using `Span.union()`.

**Python Approach** (`match_unknown.py:150-152`):
```python
# matched_ngrams is an iterator of (qstart, qend) tuples
qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
qspan = Span().union(*qspans)  # MERGE ALL into ONE span
```

**Implementation Strategy**:

Option A: Use existing `Span` struct in `src/license_detection/spans.rs`
- Modify `unknown_match()` to collect all ngram positions first
- Merge using `Span::union_span()` or similar
- Create ONE unknown match from the merged span

Option B: Simpler approach - merge adjacent/overlapping ngram positions inline

**Recommended**: Option A - Use existing Span infrastructure

**Modified `unknown_match()` logic**:
```rust
pub fn unknown_match(
    index: &LicenseIndex,
    query: &Query,
    known_matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    // ... existing setup code ...

    let covered_positions = compute_covered_positions(query, known_matches);
    let unmatched_regions = find_unmatched_regions(query_len, &covered_positions);

    let automaton = &index.unknown_automaton;

    // NEW: Collect all ngram match positions first
    let mut all_ngram_positions: Vec<usize> = Vec::new();

    for region in &unmatched_regions {
        let positions = collect_ngram_positions_in_region(
            &query.tokens,
            region.0,
            region.1,
            automaton,
        );
        all_ngram_positions.extend(positions);
    }

    // NEW: Merge positions into span (like Python's Span.union())
    let merged_span = Span::from_iterator(all_ngram_positions);

    // NEW: Create ONE unknown match from merged span (not one per region)
    if let Some(match_result) = create_unknown_match_from_span(
        index, query, &merged_span, automaton
    ) {
        vec![match_result]
    } else {
        vec![]
    }
}
```

**Helper function**:
```rust
/// Collect all positions where ngrams match in a region.
fn collect_ngram_positions_in_region(
    tokens: &[u16],
    start: usize,
    end: usize,
    automaton: &AhoCorasick,
) -> Vec<usize> {
    // Returns positions covered by matching ngrams
}
```

**Tests**:
```rust
#[test]
fn test_unknown_match_merges_ngrams() {
    // Test that overlapping/adjacent ngrams merge into one match
}

#[test]
fn test_unknown_match_produces_correct_span() {
    // Test span covers the union of all matched positions
}
```

### Step 5: Implement `license_clues` Handling (DEPENDENCY: Step 3, Lower Priority)

**Problem**: Matches with `unknown-license-reference` that overlap with known licenses should become clues, not detections.

**Python Logic** (implied from behavior):
- After detection, check for `unknown-license-reference` matches
- If they overlap with a "real" license match, mark as `is_license_clue = true`
- Clues are reported separately, not in main detection list

**Implementation Location**: `src/license_detection/detection.rs`

**Implementation**:
```rust
/// Mark unknown-license-reference matches that overlap with known licenses as clues.
///
/// When a match has license_expression containing "unknown" and overlaps
/// with a known license match, it should be reported as a license_clue
/// rather than a full detection.
fn mark_overlapping_unknown_as_clue(matches: &mut [LicenseMatch]) {
    // Sort by start position
    // For each match with has_unknown:
    //   Check if it overlaps with any non-unknown match
    //   If so, set is_license_clue = true
}
```

**Note**: This may require changes to `LicenseDetection` struct to support a separate `license_clues` field.

### Step 6: Integration Testing (DEPENDENCY: All Steps)

**Test Command**:
```bash
cargo test golden_tests::test_golden_unknown -- --nocapture
```

**Expected Results After Fix**:

| File | Expected Expressions |
|------|---------------------|
| `scea.txt` | `scea-1.0`, `scea-1.0`, `unknown`, `unknown` (+ 1 clue) |
| `cisco.txt` | `unknown`, `warranty-disclaimer`, `unknown` |

**Manual Verification**:
```bash
# Run against test file
cargo run -- testdata/license-golden/datadriven/unknown/scea.txt

# Check JSON output has correct number of detections
```

## Implementation Order

1. **Step 1** - Add `has_unknown` property (trivial, no dependencies)
2. **Step 2** - Implement `split_weak_matches()` (depends on Step 1)
3. **Step 3** - Update detection pipeline (depends on Step 2) - **FIXES PLAN-076**
4. **Step 4** - Fix unknown match merging (independent) - **FIXES SPAN FRAGMENTATION**
5. **Step 5** - Implement license_clues (optional, lower priority)
6. **Step 6** - Integration testing

## Risk Assessment

### High Risk Areas
- **Step 3**: Pipeline changes affect all detection results - needs thorough testing
- **Step 4**: Span merging logic is complex - edge cases with overlapping ngrams

### Mitigation
- Run full golden test suite after each step
- Compare output with Python reference for multiple test files
- Add regression tests for any bugs found

## Success Criteria

1. `scea.txt` produces exactly 5 matches (not 12)
2. `cisco.txt` produces `["unknown", "warranty-disclaimer", "unknown"]`
3. No regressions in other golden tests
4. `license_clues` properly reported (if Step 5 implemented)

## Related Files

- `src/license_detection/mod.rs` - Main detection pipeline (Step 3)
- `src/license_detection/match_refine.rs` - Match refinement functions (Step 2)
- `src/license_detection/models.rs` - Rule and LicenseMatch structs (Step 1)
- `src/license_detection/unknown_match.rs` - Unknown matching (Step 4)
- `src/license_detection/spans.rs` - Span utilities (Step 4)
- `reference/scancode-toolkit/src/licensedcode/match.py` - Python `split_weak_matches()` reference
- `reference/scancode-toolkit/src/licensedcode/match_unknown.py` - Python `match_unknowns()` reference
- `reference/scancode-toolkit/src/licensedcode/index.py` - Python pipeline reference
