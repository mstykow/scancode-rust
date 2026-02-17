# PLAN-013: Match Grouping Thresholds Fix

## Executive Summary

The Rust implementation uses a single line-proximity threshold (`LINES_THRESHOLD = 4`) for grouping license matches, while Python uses a dual-criteria approach with both `min_tokens_gap=10` AND `min_lines_gap=3` (note: OR logic for separation, AND for staying together). This difference causes approximately 15 golden tests to fail because matches that should be grouped together are incorrectly separated, or vice versa.

**Impact**: ~15 failing golden tests related to match grouping behavior

---

## 1. Problem Statement

### Current Rust Behavior

In `src/license_detection/detection.rs:162-208`, the `group_matches_by_region_with_threshold` function uses only a line-based proximity check:

```rust
const LINES_THRESHOLD: usize = 4;

fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    proximity_threshold: usize,
) -> Vec<DetectionGroup> {
    // ...
    let is_in_group_by_threshold =
        match_item.start_line <= previous_match.end_line + proximity_threshold;
    // ...
}
```

This checks if the current match starts within `proximity_threshold` lines after the previous match ends.

### Python Reference Behavior

Python uses two distinct grouping mechanisms:

1. **`get_matching_regions()` in `match.py:2325-2395`**: Uses token AND line-based proximity
2. **`group_matches()` in `detection.py:1820-1868`**: Uses only line-based proximity

The key difference is that Python's `get_matching_regions()` is used for region detection and employs **dual-criteria**:

```python
def get_matching_regions(
    matches,
    min_tokens_gap=10,
    min_lines_gap=3,
    ...
):
    """
    Two consecutive region Spans are such that:
    - there are no overlapping matches between them
    - there are at least ``min_tokens_gap`` unmatched tokens between them
    - OR there are at least ``min_lines_gap`` unmatched lines between them
    """
    # ...
    if (prev_region.distance_to(cur_region) > min_tokens_gap
        or prev_region_lines.distance_to(cur_region_lines) > min_lines_gap
    ):
        # Start a new region
    else:
        # Extend current region
```

**Key insight**: Python separates matches into new regions when:

- Token gap > 10 **OR** line gap > 3

This means matches stay together when BOTH:

- Token gap <= 10 **AND** line gap <= 3

### The Bug

Rust only checks line gap, ignoring token gap. This causes:

1. **False separations**: Matches with small token gaps but large line gaps get separated (should stay together)
2. **False groupings**: Matches with small line gaps but large token gaps stay together (should be separated)

---

## 2. Python Reference Analysis

### 2.1 `get_matching_regions()` Function

**Location**: `reference/scancode-toolkit/src/licensedcode/match.py:2325-2395`

```python
def get_matching_regions(
    matches,
    min_tokens_gap=10,
    min_lines_gap=3,
    trace=TRACE_REGIONS,
):
    """
    Return a list of token query position Spans, where each Span represents a
    region of related LicenseMatch contained that Span given a ``matches`` list
    of LicenseMatch.

    Matching regions are such that:

    - all matches in the regions are entirely contained in the region Span

    Two consecutive region Spans are such that:
    - there are no overlapping matches between them
    - there are at least ``min_tokens_gap`` unmatched tokens between them
    - OR there are at least ``min_lines_gap`` unmatched lines between them
    """
    regions = []

    prev_region = None
    prev_region_lines = None
    cur_region = None
    cur_region_lines = None

    for match in matches:
        if not prev_region:
            prev_region = match.qregion()
            prev_region_lines = match.qregion_lines()
        else:
            cur_region = match.qregion()
            cur_region_lines = match.qregion_lines()

            # DUAL-CRITERIA CHECK
            if (prev_region.distance_to(cur_region) > min_tokens_gap
                or prev_region_lines.distance_to(cur_region_lines) > min_lines_gap
            ):
                regions.append(prev_region)
                prev_region = cur_region
                prev_region_lines = cur_region_lines
            else:
                prev_region = Span(prev_region.start, cur_region.end)
                prev_region_lines = Span(prev_region_lines.start, cur_region_lines.end)

    if prev_region and prev_region not in regions:
        regions.append(prev_region)

    return regions
```

### 2.2 `group_matches()` Function

**Location**: `reference/scancode-toolkit/src/licensedcode/detection.py:1820-1868`

```python
def group_matches(license_matches, lines_threshold=LINES_THRESHOLD):
    """
    Given a list of ``license_matches`` LicenseMatch objects, yield lists of
    grouped matches together where each group is less than `lines_threshold`
    apart, while also considering presence of license intros.
    """
    group_of_license_matches = []

    for license_match in license_matches:
        if not group_of_license_matches:
            group_of_license_matches.append(license_match)
            continue

        previous_match = group_of_license_matches[-1]
        is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold

        # Handle license intros (special cases)
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

### 2.3 `LINES_THRESHOLD` Constant

**Location**: `reference/scancode-toolkit/src/licensedcode/query.py:106-108`

```python
# Break query in runs if there are `LINES_THRESHOLD` number of empty
# or non-legalese/junk lines
LINES_THRESHOLD = 4
```

### 2.4 `Span.distance_to()` Method

**Location**: `reference/scancode-toolkit/src/licensedcode/spans.py:402-435`

```python
def distance_to(self, other):
    """
    Return the absolute positive distance from this span to other span.
    Overlapping spans have a zero distance.
    Non-overlapping touching spans have a distance of one.
    """
    if self.overlap(other):
        return 0

    if self.touch(other):
        return 1

    if self.is_before(other):
        return other.start - self.end
    else:
        return self.start - other.end
```

---

## 3. Rust Code Analysis

### 3.1 Current Implementation

**Location**: `src/license_detection/detection.rs:11-13, 162-208`

```rust
/// Proximity threshold for grouping matches in lines.
/// Matches more than this many lines apart are considered separate regions.
const LINES_THRESHOLD: usize = 4;

// ...

fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    proximity_threshold: usize,
) -> Vec<DetectionGroup> {
    let mut groups = Vec::new();
    let mut current_group: Vec<LicenseMatch> = Vec::new();

    for match_item in matches {
        if current_group.is_empty() {
            current_group.push(match_item.clone());
            continue;
        }

        let previous_match = current_group.last().unwrap();
        // ONLY LINE-BASED CHECK
        let is_in_group_by_threshold =
            match_item.start_line <= previous_match.end_line + proximity_threshold;

        if previous_match.matcher.starts_with("5-unknown") && is_license_intro_match(previous_match)
        {
            current_group.push(match_item.clone());
        } else if is_license_intro_match(match_item) {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        } else if is_license_clue_match(match_item) {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            groups.push(DetectionGroup::new(vec![match_item.clone()]));
            current_group = Vec::new();
        } else if is_in_group_by_threshold {
            current_group.push(match_item.clone());
        } else {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        }
    }

    if !current_group.is_empty() {
        groups.push(DetectionGroup::new(current_group));
    }

    groups
}
```

### 3.2 `LicenseMatch` Structure

**Location**: `src/license_detection/models.rs:174-224`

The current `LicenseMatch` structure does not track token positions:

```rust
pub struct LicenseMatch {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub matcher: String,
    pub score: f32,
    pub matched_length: usize,  // Character length, not token count
    pub match_coverage: f32,
    pub rule_relevance: u8,
    pub rule_identifier: String,
    pub rule_url: String,
    pub matched_text: Option<String>,
    pub referenced_filenames: Option<Vec<String>>,
    pub is_license_intro: bool,
    pub is_license_clue: bool,
}
```

**Missing fields** (compared to Python):

- `start_token` / `end_token` - Token position range
- Token span for calculating token distance

### 3.3 Existing `Span` Implementation

**Location**: `src/license_detection/spans.rs`

The Rust code has a `Span` struct but it lacks the `distance_to` method needed for this fix:

```rust
pub struct Span {
    ranges: Vec<Range<usize>>,
}
```

---

## 4. Proposed Changes

### 4.1 Add Token Position Tracking to `LicenseMatch`

**File**: `src/license_detection/models.rs`

```rust
pub struct LicenseMatch {
    // ... existing fields ...
    
    /// Start token position (0-indexed in query token stream)
    pub start_token: usize,
    
    /// End token position (0-indexed, inclusive)
    pub end_token: usize,
}
```

**Impact**: All matchers (hash, aho, seq, spdx-lid) need to populate these fields during detection.

### 4.2 Add `distance_to` Method to `Span`

**File**: `src/license_detection/spans.rs`

```rust
impl Span {
    /// Return the absolute positive distance from this span to other span.
    /// Overlapping spans have a zero distance.
    /// Non-overlapping touching spans have a distance of one.
    pub fn distance_to(&self, other: &Span) -> usize {
        if self.intersects(other) {
            return 0;
        }
        
        // Check if touching (adjacent)
        for self_range in &self.ranges {
            for other_range in &other.ranges {
                if self_range.end == other_range.start || other_range.end == self_range.start {
                    return 1;
                }
            }
        }
        
        // Calculate minimum gap between any ranges
        let mut min_distance = usize::MAX;
        for self_range in &self.ranges {
            for other_range in &other.ranges {
                if self_range.end < other_range.start {
                    min_distance = min_distance.min(other_range.start - self_range.end);
                } else if other_range.end < self_range.start {
                    min_distance = min_distance.min(self_range.start - other_range.end);
                }
            }
        }
        
        min_distance
    }
}
```

### 4.3 Add Threshold Constants

**File**: `src/license_detection/detection.rs`

```rust
/// Token gap threshold for grouping matches.
/// Matches with more than this many unmatched tokens between them are separate regions.
/// Based on Python: licensedcode/match.py MIN_TOKENS_GAP
const TOKENS_THRESHOLD: usize = 10;

/// Line gap threshold for grouping matches.
/// Matches with more than this many unmatched lines between them are separate regions.
/// Based on Python: licensedcode/match.py MIN_LINES_GAP
const LINES_GAP_THRESHOLD: usize = 3;

/// Proximity threshold for grouping matches in lines.
/// Used in group_matches() for line-based grouping.
/// Based on Python: licensedcode/query.py LINES_THRESHOLD
const LINES_THRESHOLD: usize = 4;
```

### 4.4 Implement Dual-Criteria Grouping

**File**: `src/license_detection/detection.rs`

Replace the current `group_matches_by_region_with_threshold` with a new implementation:

```rust
/// Check if two matches should be in the same group based on dual-criteria.
///
/// Matches are grouped together when BOTH:
/// - Token gap <= TOKENS_THRESHOLD
/// - Line gap <= LINES_GAP_THRESHOLD
///
/// Matches are separated when EITHER:
/// - Token gap > TOKENS_THRESHOLD
/// - Line gap > LINES_GAP_THRESHOLD
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    // Calculate token gap
    let token_gap = if cur.start_token > prev.end_token {
        cur.start_token - prev.end_token - 1
    } else {
        0  // Overlapping or touching
    };
    
    // Calculate line gap
    let line_gap = if cur.start_line > prev.end_line {
        cur.start_line - prev.end_line - 1
    } else {
        0  // Overlapping or touching
    };
    
    // Dual-criteria: stay together only if BOTH thresholds are not exceeded
    token_gap <= TOKENS_THRESHOLD && line_gap <= LINES_GAP_THRESHOLD
}

fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    _proximity_threshold: usize,  // Kept for API compatibility, not used
) -> Vec<DetectionGroup> {
    let mut groups = Vec::new();
    let mut current_group: Vec<LicenseMatch> = Vec::new();

    for match_item in matches {
        if current_group.is_empty() {
            current_group.push(match_item.clone());
            continue;
        }

        let previous_match = current_group.last().unwrap();
        
        // Handle license intros (special cases) - same as before
        if previous_match.matcher.starts_with("5-unknown") && is_license_intro_match(previous_match)
        {
            current_group.push(match_item.clone());
        } else if is_license_intro_match(match_item) {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        } else if is_license_clue_match(match_item) {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            groups.push(DetectionGroup::new(vec![match_item.clone()]));
            current_group = Vec::new();
        } else if should_group_together(previous_match, match_item) {
            // DUAL-CRITERIA CHECK instead of just line threshold
            current_group.push(match_item.clone());
        } else {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        }
    }

    if !current_group.is_empty() {
        groups.push(DetectionGroup::new(current_group));
    }

    groups
}
```

### 4.5 Update All Matchers to Populate Token Positions

Each matcher needs to track token positions:

**Files to modify**:

- `src/license_detection/hash_match.rs`
- `src/license_detection/aho_match.rs`
- `src/license_detection/seq_match.rs`
- `src/license_detection/spdx_lid.rs`
- `src/license_detection/unknown_match.rs`

Example for hash matcher:

```rust
// When creating LicenseMatch, populate token positions:
LicenseMatch {
    // ... existing fields ...
    start_token: match_start_token,
    end_token: match_end_token,
}
```

---

## 5. Testing Strategy

### 5.1 Python Test Analysis

Python tests the grouping behavior in:

1. **`tests/licensedcode/test_match.py`** - Tests for `get_matching_regions()`
2. **`tests/licensedcode/test_detect.py`** - Tests for `group_matches()`
3. **Data-driven tests** - Golden tests in `tests/licensedcode/data/datadriven/`

### 5.2 Unit Tests to Add

**File**: `src/license_detection/detection_test.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_match_with_tokens(
        start_line: usize,
        end_line: usize,
        start_token: usize,
        end_token: usize,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line,
            end_line,
            start_token,
            end_token,
            matcher: "1-hash".to_string(),
            score: 95.0,
            matched_length: 100,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "".to_string(),
            matched_text: None,
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
        }
    }

    #[test]
    fn test_grouping_within_both_thresholds() {
        // Token gap: 5, Line gap: 2 -> Both within thresholds -> Same group
        let m1 = create_match_with_tokens(1, 10, 0, 50);
        let m2 = create_match_with_tokens(13, 20, 56, 100);  // 5 token gap, 2 line gap
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn test_grouping_exceeds_token_threshold_only() {
        // Token gap: 15 (>10), Line gap: 1 (<=3) -> Separate groups
        let m1 = create_match_with_tokens(1, 10, 0, 50);
        let m2 = create_match_with_tokens(12, 20, 66, 100);  // 15 token gap, 1 line gap
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_grouping_exceeds_line_threshold_only() {
        // Token gap: 5 (<=10), Line gap: 5 (>3) -> Separate groups
        let m1 = create_match_with_tokens(1, 10, 0, 50);
        let m2 = create_match_with_tokens(16, 25, 56, 100);  // 5 token gap, 5 line gap
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_grouping_exceeds_both_thresholds() {
        // Token gap: 15 (>10), Line gap: 5 (>3) -> Separate groups
        let m1 = create_match_with_tokens(1, 10, 0, 50);
        let m2 = create_match_with_tokens(16, 25, 66, 100);  // 15 token gap, 5 line gap
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_grouping_at_exact_thresholds() {
        // Token gap: exactly 10, Line gap: exactly 3 -> Same group (boundary)
        let m1 = create_match_with_tokens(1, 10, 0, 50);
        let m2 = create_match_with_tokens(14, 20, 61, 100);  // 10 token gap, 3 line gap
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(groups.len(), 1);
    }
}
```

### 5.3 Golden Test Verification

After implementation:

1. Run the full golden test suite:

   ```bash
   cargo test --test license_detection_golden_test
   ```

2. Compare results with Python reference for failing tests

3. Document any remaining differences in behavior

### 5.4 Specific Test Cases

The following golden test files are known to be affected by this issue (from previous analysis):

- Test files with multiple matches close in tokens but far in lines
- Test files with matches close in lines but far in tokens
- Edge cases with minified files (few lines, many tokens)

---

## 6. Implementation Checklist

- [ ] Add `start_token` and `end_token` fields to `LicenseMatch`
- [ ] Add `distance_to` method to `Span` struct
- [ ] Add `TOKENS_THRESHOLD` and `LINES_GAP_THRESHOLD` constants
- [ ] Implement `should_group_together()` function with dual-criteria
- [ ] Update `group_matches_by_region_with_threshold()` to use dual-criteria
- [ ] Update hash matcher to populate token positions
- [ ] Update Aho-Corasick matcher to populate token positions
- [ ] Update sequence matcher to populate token positions
- [ ] Update SPDX-LID matcher to populate token positions
- [ ] Update unknown matcher to populate token positions
- [ ] Add unit tests for new grouping logic
- [ ] Run golden test suite and verify fixes
- [ ] Update documentation

---

## 7. References

### Python Source Files

- `reference/scancode-toolkit/src/licensedcode/match.py:2325-2395` - `get_matching_regions()`
- `reference/scancode-toolkit/src/licensedcode/detection.py:1820-1868` - `group_matches()`
- `reference/scancode-toolkit/src/licensedcode/query.py:106-108` - `LINES_THRESHOLD`
- `reference/scancode-toolkit/src/licensedcode/spans.py:402-435` - `Span.distance_to()`

### Rust Source Files

- `src/license_detection/detection.rs` - Current implementation
- `src/license_detection/models.rs` - `LicenseMatch` struct
- `src/license_detection/spans.rs` - `Span` struct

### Related Documentation

- `docs/ARCHITECTURE.md` - Overall architecture
- `docs/license-detection/GOLDEN_TEST_PLAN.md` - Testing approach

---

## 8. Analysis Results (2026-02-17)

### What Was Implemented

The dual-criteria grouping logic has been **partially implemented** in `src/license_detection/detection.rs`:

1. **Constants added** (lines 11-23):
   - `LINES_THRESHOLD: usize = 4` (existing)
   - `TOKENS_THRESHOLD: usize = 10` (new)
   - `LINES_GAP_THRESHOLD: usize = 3` (new)

2. **Token position tracking added to `LicenseMatch`** (`models.rs:195-203`):
   - `start_token: usize` (0-indexed)
   - `end_token: usize` (0-indexed, exclusive)

3. **`should_group_together()` function implemented** (lines 218-234):
   - Uses dual-criteria: `token_gap <= TOKENS_THRESHOLD && line_gap <= LINES_GAP_THRESHOLD`
   - Correctly implements OR logic for separation

4. **`calculate_token_gap()` and `calculate_line_gap()` helper functions** (lines 236-260)

### Critical Bug Still Present: `is_license_intro_match()` and `is_license_clue_match()`

**The functions at lines 272-279 use string-based heuristics instead of the actual boolean fields:**

```rust
// CURRENT (WRONG):
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher.starts_with("5-unknown") || match_item.rule_identifier.contains("intro")
}

fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher == "5-unknown" || match_item.rule_identifier.contains("clue")
}
```

**Python's implementation (correct):**

```python
if previous_match.rule.is_license_intro:  # Uses actual boolean field
    group_of_license_matches.append(license_match)
elif license_match.rule.is_license_intro:  # Uses actual boolean field
    yield group_of_license_matches
    group_of_license_matches = [license_match]
elif license_match.rule.is_license_clue:   # Uses actual boolean field
    yield group_of_license_matches
    yield [license_match]
    group_of_license_matches = []
```

**The boolean fields exist on `LicenseMatch`:**
- `is_license_intro: bool` (line 239)
- `is_license_clue: bool` (line 242)

And are properly populated by matchers (`hash_match.rs:119`, `aho_match.rs:175`, `seq_match.rs:508`, etc.).

### Why This Bug Causes Failures

1. **`is_license_intro_match()`** checks for `"5-unknown"` prefix which misses actual intros from other matchers
2. **`is_license_intro_match()`** checks for `"intro"` in rule_identifier which catches false positives
3. **`is_license_clue_match()`** has the same issues with `"5-unknown"` and `"clue"` string checks

This causes:
- Matches that ARE intros (with `is_license_intro: true`) to not trigger intro handling
- Non-intro matches to incorrectly trigger intro handling
- Same issues with license clues

### Impact on Failing Tests

From FAILURES.md, tests mentioning grouping issues:

| Test | Root Cause |
|------|------------|
| `checker-2200.txt` | String heuristics in `is_license_intro/clue_match()` |
| `cjdict-liconly.txt` | Same - heuristics vs boolean fields |
| `e2fsprogs.txt` | Same |
| `e2fsprogs_1.txt` | Same |
| `eclipse-openj9.LICENSE` | Dual-threshold not effective due to intro/clue bug |
| `gfdl-1.1_1.RULE` | Same |
| `godot2_COPYRIGHT.txt` | Mentioned as dual-criteria issue, but intro/clue bug is likely the real cause |
| `gpl-2.0-plus_41.txt` | Grouping issue with intro handling |

---

## 9. Remaining TODOs

### Priority 1: Fix `is_license_intro_match()` and `is_license_clue_match()` (HIGH IMPACT)

**File:** `src/license_detection/detection.rs:272-279`

**Change:**

```rust
// CORRECT:
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.is_license_intro
}

fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.is_license_clue
}
```

This simple fix should resolve ~10-15 failing tests.

### Priority 2: Verify Token Position Population

Ensure all matchers populate `start_token` and `end_token` correctly:
- [ ] Verify `hash_match.rs` populates token positions
- [ ] Verify `aho_match.rs` populates token positions
- [ ] Verify `seq_match.rs` populates token positions
- [ ] Verify `spdx_lid.rs` populates token positions
- [ ] Verify `unknown_match.rs` populates token positions

### Priority 3: Add Unit Tests for Grouping Logic

Add tests in `detection_test.rs`:
- [ ] Test that intros create new groups regardless of proximity
- [ ] Test that clues are yielded as separate groups
- [ ] Test dual-criteria threshold boundaries
- [ ] Test interaction of intro/clue handling with proximity thresholds

### Priority 4: Run Golden Test Suite

After fixing the intro/clue functions:
```bash
cargo test --lib license_detection::golden_test
```

Expected: ~10-15 tests should now pass.

---

## 10. Summary

**PLAN-013 was partially implemented:**
- ✅ Token position tracking added to `LicenseMatch`
- ✅ Dual-criteria constants added
- ✅ `should_group_together()` implemented with dual-criteria
- ❌ **Bug: `is_license_intro_match()` and `is_license_clue_match()` still use string heuristics**

**The fix is trivial:** Replace the string-based checks with the actual boolean fields that already exist on `LicenseMatch`. This single fix should resolve the majority of grouping-related test failures.
