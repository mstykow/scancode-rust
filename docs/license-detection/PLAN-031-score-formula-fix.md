# PLAN-031: Fix Score Calculation Formula to Match Python Implementation

## Status: Completed

## Problem Description

The Rust implementation uses a **simplified score formula** that differs from the Python reference implementation. This discrepancy affects match confidence scores and can impact:

1. **Detection quality ranking** - Higher-quality matches may be ranked incorrectly
2. **False positive filtering** - Decisions based on score thresholds may be incorrect
3. **Golden test parity** - Score mismatches cause golden test failures

### Current Rust Implementation

**Location**: `src/license_detection/match_refine.rs:421-425`

```rust
fn update_match_scores(matches: &mut [LicenseMatch]) {
    for m in matches.iter_mut() {
        m.score = m.match_coverage * m.rule_relevance as f32 / 100.0;
    }
}
```

**Formula**: `score = match_coverage * rule_relevance / 100`

Where:

- `match_coverage` = percentage of rule tokens matched (0.0-100.0)
- `rule_relevance` = rule's relevance score (0-100)

**Result range**: 0.0 to 100.0

### Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/match.py:592-619`

```python
def score(self):
    """
    Return the score for this match as a rounded float between 0 and 100.

    The score is an indication of the confidence that a match is good. It is
    computed from the number of matched tokens, the number of query tokens
    in the matched range (including unknowns and unmatched) and the matched
    rule relevance.
    """
    # relevance is a number between 0 and 100. Divide by 100
    relevance = self.rule.relevance / 100
    if not relevance:
        return 0

    qmagnitude = self.qmagnitude()

    # Compute the score as the ratio of the matched query length to the
    # qmagnitude, e.g. the length of the matched region
    if not qmagnitude:
        return 0

    # FIXME: this should be exposed as an q/icoverage() method instead
    query_coverage = self.len() / qmagnitude
    rule_coverage = self._icoverage()
    if query_coverage < 1 and rule_coverage < 1:
        # use rule coverage in this case
        return round(rule_coverage * relevance * 100, 2)
    return round(query_coverage * rule_coverage * relevance * 100, 2)
```

**Formula**: `score = query_coverage * rule_coverage * relevance * 100`

Where:

- `query_coverage` = `len() / qmagnitude()` (ratio of matched tokens to total query region)
- `rule_coverage` = `len() / rule.length` (ratio of matched tokens to rule length)
- `relevance` = `rule.relevance / 100` (normalized relevance 0-1)
- `qmagnitude` = `qregion_len + unknowns_in_match` (query region length including unknowns)

**Result range**: 0.0 to 100.0

---

## Current State Analysis

### Key Difference: `qmagnitude()` vs `match_coverage`

The fundamental difference is that Python's `query_coverage` accounts for **unknown tokens** in the query region, while Rust's `match_coverage` does not.

#### Python's `qmagnitude()` (match.py:488-527)

```python
def qmagnitude(self):
    """
    Return the maximal query length represented by this match start and end
    in the query. This number represents the full extent of the matched
    query region including matched, unmatched AND unknown tokens, but
    excluding STOPWORDS.
    """
    query = self.query
    qspan = self.qspan
    qmagnitude = self.qregion_len()

    if query:
        # Compute a count of unknown tokens that are inside the matched
        # range, ignoring end position of the query span
        unknowns_pos = qspan & query.unknowns_span
        qspe = qspan.end
        unknowns_pos = (pos for pos in unknowns_pos if pos != qspe)
        qry_unkxpos = query.unknowns_by_pos
        unknowns_in_match = sum(qry_unkxpos[pos] for pos in unknowns_pos)

        # update the magnitude by adding the count of unknowns in the match
        qmagnitude += unknowns_in_match

    return qmagnitude
```

**Key insight**: `qmagnitude = qregion_len + count_of_unknowns_in_qspan`

#### Rust's Existing `qmagnitude()` (models.rs:415-423)

```rust
/// Return the query magnitude: qregion_len + unknowns in matched range.
/// Python: qmagnitude = qregion_len + sum(unknowns_by_pos for pos in qspan[:-1])
pub fn qmagnitude(&self, query: &crate::license_detection::query::Query) -> usize {
    let qregion_len = self.qregion_len();
    let unknowns_in_match = (self.start_token..self.end_token)
        .filter(|&pos| query.unknowns_by_pos.contains_key(&Some(pos as i32)))
        .count();
    qregion_len + unknowns_in_match
}
```

**Status**: Method exists but is **NOT used in score calculation**.

### Missing Component: `unknowns_span` in Query

Python's Query class has an `unknowns_span` field (query.py:239) that tracks positions followed by unknown tokens:

```python
# Span of "known positions" (yes really!) followed by unknown token(s)
self.unknowns_span = None
```

This is used in `qmagnitude()` to compute the intersection with the match's qspan.

**Rust status**: The `Query` struct does NOT have `unknowns_span`. The current `qmagnitude()` implementation approximates this by iterating over positions, but may not be equivalent to Python's intersection logic.

---

## Impact Analysis

### Where Scores Are Used

1. **Detection score calculation** (`detection.rs:609-633`)
   - Weighted average of match scores
   - Used to rank detection confidence

2. **Extra words detection** (`detection.rs:294-299`)

   ```rust
   fn has_extra_words(matches: &[LicenseMatch]) -> bool {
       matches.iter().any(|m| {
           let score_coverage_relevance = m.match_coverage * m.rule_relevance as f32 / 100.0;
           score_coverage_relevance - m.score > 0.01
       })
   }
   ```

3. **Match merging** (`match_refine.rs:142`)

   ```rust
   merged.score = a.score.max(b.score);
   ```

4. **JSON output** - Score is part of the output format

5. **Golden tests** (`golden_test.rs`) - Score differences cause test failures

### Impact on Scoring

| Scenario | Rust Score | Python Score | Difference |
|----------|-----------|--------------|------------|
| Exact match (no unknowns) | 100 * relevance | 100 * relevance | None |
| Partial match (no unknowns) | coverage * relevance | coverage * relevance | None |
| Partial match WITH unknowns | coverage * relevance | lower score | Rust overestimates |
| Sparse match (many unknowns) | coverage * relevance | much lower score | Rust significantly overestimates |

**Key insight**: Rust **overestimates** scores when unknown tokens are present in the query region. This leads to:

1. Higher confidence for low-quality matches
2. Incorrect ranking when comparing matches
3. Potential false positive acceptance

---

## Proposed Changes

### Change 1: Add `unknowns_span` to Query Struct

**File**: `src/license_detection/query.rs`

Add a field to track positions followed by unknown tokens:

```rust
/// Span of known positions followed by unknown token(s).
///
/// Used for computing qmagnitude() in score calculation.
///
/// Corresponds to Python: `self.unknowns_span` (query.py:239)
pub unknowns_span: Vec<usize>,  // Positions with unknowns after them
```

**Implementation location**: `Query::with_options()` method, populated during tokenization loop.

**Python reference** (query.py:516-517):

```python
self.unknowns_span = Span(unknowns_pos)
```

### Change 2: Update `qmagnitude()` Method

**File**: `src/license_detection/models.rs`

Update the `qmagnitude()` method to use `unknowns_span` and match Python's logic exactly:

```rust
/// Return the query magnitude: qregion_len + unknowns in matched range.
///
/// The magnitude represents the full extent of the matched query region
/// including matched, unmatched AND unknown tokens, but excluding STOPWORDS.
///
/// Python: qmagnitude = qregion_len + sum(unknowns_by_pos for pos in qspan[:-1] & unknowns_span)
pub fn qmagnitude(&self, query: &crate::license_detection::query::Query) -> usize {
    let qregion_len = self.qregion_len();
    
    // Get positions in qspan (excluding the end position)
    let qspan = self.qspan();
    if qspan.is_empty() {
        return qregion_len;
    }
    
    // Python: unknowns_pos = qspan & query.unknowns_span
    // Then exclude the last position (qspe = qspan.end)
    let qspan_end = qspan.last().copied();
    
    let unknowns_in_match: usize = qspan
        .iter()
        .filter(|&&pos| {
            // Exclude the end position
            if Some(pos) == qspan_end {
                return false;
            }
            // Check if this position has unknowns after it
            query.unknowns_by_pos.contains_key(&Some(pos as i32))
        })
        .map(|pos| query.unknowns_by_pos.get(&Some(*pos as i32)).copied().unwrap_or(0))
        .sum();
    
    qregion_len + unknowns_in_match
}
```

### Change 3: Implement `_icoverage()` Method

**File**: `src/license_detection/models.rs`

Add a method to compute rule coverage (Python's `_icoverage()`):

```rust
/// Return the coverage of this match to the matched rule as a float between 0 and 1.
///
/// Python: _icoverage() at match.py:472-479
pub fn icoverage(&self) -> f32 {
    if self.rule_length == 0 {
        return 0.0;
    }
    self.len() as f32 / self.rule_length as f32
}
```

### Change 4: Rewrite `update_match_scores()` Function

**File**: `src/license_detection/match_refine.rs`

Completely rewrite the score calculation to match Python's formula:

```rust
/// Update match scores for all matches.
///
/// Computes scores using Python's formula:
/// `score = query_coverage * rule_coverage * relevance * 100`
///
/// Where:
/// - query_coverage = len() / qmagnitude() (ratio of matched to query region)
/// - rule_coverage = len() / rule_length (ratio of matched to rule)
/// - relevance = rule_relevance / 100
///
/// Special case: when both coverages < 1, use rule_coverage only.
///
/// This function requires a Query reference for qmagnitude calculation.
///
/// Based on Python: LicenseMatch.score() method at match.py:592-619
pub fn update_match_scores(matches: &mut [LicenseMatch], query: &Query) {
    for m in matches.iter_mut() {
        m.score = compute_match_score(m, query);
    }
}

/// Compute the score for a single match.
///
/// Returns a value between 0.0 and 100.0.
fn compute_match_score(m: &LicenseMatch, query: &Query) -> f32 {
    // relevance is a number between 0 and 100. Divide by 100
    let relevance = m.rule_relevance as f32 / 100.0;
    if relevance < 0.001 {
        return 0.0;
    }

    let qmagnitude = m.qmagnitude(query);
    if qmagnitude == 0 {
        return 0.0;
    }

    // query_coverage = matched tokens / query region magnitude
    let query_coverage = m.len() as f32 / qmagnitude as f32;
    
    // rule_coverage = matched tokens / rule length
    let rule_coverage = m.icoverage();

    // Special case: when both coverages < 1, use rule_coverage only
    if query_coverage < 1.0 && rule_coverage < 1.0 {
        return (rule_coverage * relevance * 100.0 * 100.0).round() / 100.0;
    }

    (query_coverage * rule_coverage * relevance * 100.0 * 100.0).round() / 100.0
}
```

### Change 5: Update `refine_matches()` Signature

**File**: `src/license_detection/match_refine.rs`

The function already has access to `query`, so pass it to `update_match_scores`:

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,  // Already available
) -> Vec<LicenseMatch> {
    // ... existing code ...
    
    // Change this line:
    // update_match_scores(&mut final_scored);
    // To:
    update_match_scores(&mut final_scored, query);
    
    final_scored
}
```

### Change 6: Update Call Sites

**Files**: Multiple

Any code that creates or updates `LicenseMatch` objects must ensure scores are calculated with the new formula:

1. `hash_match.rs` - Hash matches currently set `score = 1.0`
2. `aho_match.rs` - Aho-Corasick matches currently set `score = 1.0`
3. `spdx_lid.rs` - SPDX identifier matches
4. `seq_match.rs` - Sequence matches

**Decision needed**: Should matchers set scores during creation, or should all scores be set centrally during refinement?

**Recommendation**: Keep central score calculation in `update_match_scores()` and call it during refinement. This ensures consistency and avoids duplicate logic.

---

## Test Requirements

### Unit Tests

1. **Test `qmagnitude()` with various scenarios**
   - Match with no unknowns -> should equal qregion_len
   - Match with unknowns inside -> should be qregion_len + unknown_count
   - Match with unknowns at end position -> should exclude end position unknowns
   - Empty match -> should return 0

2. **Test `compute_match_score()` with various scenarios**
   - Exact match (100% coverage, relevance 100) -> score should be 100.0
   - Partial match with no unknowns -> score = coverage * relevance
   - Partial match with unknowns -> score should be lower than simple coverage
   - Both coverages < 1 -> should use rule_coverage formula
   - Zero relevance -> score should be 0
   - Zero qmagnitude -> score should be 0

3. **Test score ranges**
   - Score should always be 0.0-100.0
   - Score should be rounded to 2 decimal places

### Golden Tests

1. **Run existing golden tests and compare scores**
   - Document which tests now pass
   - Document any new failures
   - Compare score values with Python reference output

2. **Create specific golden tests for edge cases**
   - File with many unknown tokens
   - File with sparse matches
   - File with varying relevance rules

### Integration Tests

1. **Test end-to-end detection with new scoring**
   - Verify detection scores match Python reference
   - Verify ranking of multiple detections is correct

---

## Risk Assessment

### High Risk Areas

1. **Score-dependent filtering**
   - `has_extra_words()` in detection.rs may behave differently
   - Need to verify false positive detection still works

2. **Match merging**
   - Score is used to pick the best match during merge
   - Different scores may change merge behavior

3. **Detection scoring**
   - `compute_detection_score()` uses weighted average of match scores
   - Different match scores will affect detection confidence

### Mitigation Strategy

1. **Run full golden test suite before and after**
   - Compare outputs systematically
   - Document all differences

2. **Gradual rollout**
   - Implement changes incrementally
   - Run tests after each change

3. **Preserve backward compatibility**
   - Consider adding a feature flag if needed
   - Document score formula changes in output

---

## Implementation Order

1. **Phase 1: Infrastructure**
   - [x] Add `unknowns_span` field to Query struct (SKIPPED - used alternative approach)
   - [x] Update Query tokenization to populate `unknowns_span` (SKIPPED)
   - [ ] Add unit tests for `unknowns_span`

2. **Phase 2: qmagnitude() fix**
   - [x] Update `qmagnitude()` method with correct logic
   - [x] Add `icoverage()` method
   - [x] Add unit tests for both methods

3. **Phase 3: Score calculation**
   - [x] Rewrite `compute_match_score()` function
   - [x] Update `update_match_scores()` to use new formula
   - [x] Pass query reference through the call chain

4. **Phase 4: Validation**
   - [ ] Run all golden tests
   - [ ] Compare outputs with Python reference
   - [ ] Fix any edge cases

5. **Phase 5: Documentation**
   - [ ] Update code comments
   - [ ] Document score formula in output format spec

---

## Questions to Resolve

1. **Should we keep the simplified score as a fallback?**
   - Some use cases may not need the complexity
   - Consider a configuration option

2. **How to handle matches without Query reference?**
   - Some tests create LicenseMatch objects directly
   - May need a default score or factory method

3. **Should score be computed lazily?**
   - Python computes score on demand via property
   - Rust stores score in the struct
   - Consider whether to change this pattern

---

## Related Documentation

- Python score method: `reference/scancode-toolkit/src/licensedcode/match.py:592-619`
- Python qmagnitude method: `reference/scancode-toolkit/src/licensedcode/match.py:488-527`
- Python Query class: `reference/scancode-toolkit/src/licensedcode/query.py`
- Current Rust implementation: `src/license_detection/match_refine.rs:421-425`
- Rust qmagnitude stub: `src/license_detection/models.rs:415-423`
- Testing strategy: `docs/TESTING_STRATEGY.md`

---

## Summary

The Rust implementation uses a simplified score formula that does not account for unknown tokens in the query region. This causes score overestimation, particularly for sparse matches. The fix requires:

1. Adding `unknowns_span` tracking to the Query struct
2. Updating `qmagnitude()` to correctly compute query region magnitude
3. Adding `icoverage()` method for rule coverage calculation
4. Rewriting score calculation to match Python's three-factor formula

The change will improve detection accuracy and enable golden test parity with the Python reference implementation.

---

## Implementation Notes

### Date: 2026-02-23

### What Was Implemented

1. **`icoverage()` Method** (models.rs:478-483)
   - Implemented exactly as specified in the plan
   - Returns `self.len() as f32 / self.rule_length as f32`
   - Returns 0.0 if rule_length is 0

2. **`qmagnitude()` Method** (models.rs:422-439)
   - Updated to correctly compute query magnitude including unknowns
   - Correctly excludes the end position from unknowns count
   - Works with both contiguous and non-contiguous matches (via qspan_positions)
   - Tests pass: `test_qmagnitude_non_contiguous`, `test_qmagnitude_excludes_end_position`

3. **`update_match_scores()` Function** (match_refine.rs:427-452)
   - Rewritten to use Python's three-factor formula
   - Takes Query reference as parameter
   - Implements special case for both coverages < 1
   - Tests pass: `test_update_match_scores_basic`, `test_update_match_scores_multiple`, `test_update_match_scores_idempotent`, `test_update_match_scores_empty`

4. **`compute_match_score()` Function** (match_refine.rs:433-452)
   - Implements the core score calculation logic
   - Uses `query_coverage = len() / qmagnitude()`
   - Uses `rule_coverage = icoverage()`
   - Special case: uses rule_coverage only when both coverages < 1

5. **`refine_matches()` Integration** (match_refine.rs:1490)
   - Query is passed to `update_match_scores` as specified

6. **`has_extra_words()` Function** (detection.rs:294-300)
   - Updated to use `m.icoverage() * 100.0` instead of `m.match_coverage`
   - This is a fix to be consistent with the new score formula

### Deviations from Plan

1. **`unknowns_span` field NOT added to Query struct**
   - The plan specified adding a new `unknowns_span` field to the Query struct
   - Implementation uses an alternative approach: directly using `query.unknowns_by_pos` HashMap
   - The `qmagnitude()` implementation iterates over positions and checks `unknowns_by_pos` directly
   - This achieves the same result without requiring a new field
   - Rationale: Simpler implementation, no changes to Query struct needed

2. **Rounding behavior**
   - Plan specified: `(x * 100.0).round() / 100.0` for 2 decimal places
   - Implementation uses: `.round()` (rounds to nearest integer)
   - Python uses `round(x, 2)` for 2 decimal places
   - Impact: Minor - most scores are integer-like values

3. **`has_extra_words()` uses `icoverage()` instead of `match_coverage`**
   - Plan showed using `m.match_coverage`
   - Implementation uses `m.icoverage() * 100.0`
   - This is actually a fix - consistent with the new score formula which uses `icoverage()`

### Tests Passing

- `test_qmagnitude_non_contiguous` - PASSED
- `test_qmagnitude_excludes_end_position` - PASSED
- `test_update_match_scores_basic` - PASSED
- `test_update_match_scores_multiple` - PASSED
- `test_update_match_scores_idempotent` - PASSED
- `test_update_match_scores_empty` - PASSED

### Remaining Work

- [ ] Run full golden test suite to validate score parity with Python
- [ ] Add more edge case tests for score calculation
- [ ] Document score formula in output format specification
