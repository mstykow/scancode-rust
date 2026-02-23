# PLAN-037: Add Post-Phase `merge_matches()` Calls

**Date**: 2026-02-23
**Status**: Planning Complete - Implementation Pending
**Priority**: HIGH
**Related**: PLAN-024, PLAN-029 (Section 2.4)

## Executive Summary

Python's license detection calls `merge_matches()` after each matching phase to deduplicate overlapping matches before the next phase runs. Rust only merges at the end of `refine_matches()`, causing overlapping matches from different phases to remain as separate matches, leading to duplicate expressions and incorrect containment filtering.

**Expected Impact**: ~200+ test improvements across external tests

---

## 1. Problem Description

### 1.1 Observed Behavior

When running golden tests, we observe:

- Duplicate license expressions in final output
- Multiple matches for the same license in overlapping positions
- Incorrect containment filtering (larger matches not filtering smaller ones)
- Score calculation affected by duplicate matches

### 1.2 Root Cause

Rust accumulates all matches from different phases (hash, SPDX-LID, Aho-Corasick, sequence) into `all_matches` vector without merging between phases. These overlapping matches then proceed through `refine_matches()` where they may not be properly deduplicated due to subtle timing differences.

---

## 2. Current State Analysis

### 2.1 Rust Implementation (mod.rs:117-271)

```rust
// Phase 1: Hash matching
let hash_matches = hash_match(&self.index, &whole_run);
// ... process matched_qspans ...
all_matches.extend(hash_matches);  // NO MERGE

// Phase 1b: SPDX-LID matching
let spdx_matches = spdx_lid_match(&self.index, &query);
all_matches.extend(spdx_matches);  // NO MERGE

// Phase 1c: Aho-Corasick matching
let aho_matches = aho_match(&self.index, &whole_run);
all_matches.extend(aho_matches);  // NO MERGE

// Phase 2: Near-duplicate detection
let near_dupe_matches = seq_match_with_candidates(...);
all_matches.extend(near_dupe_matches);  // NO MERGE

// Phase 3: Regular sequence matching
let seq_matches = seq_match(&self.index, &whole_run);
all_matches.extend(seq_matches);  // NO MERGE

// Phase 4: Query run matching
// ... multiple query_runs processed ...
all_matches.extend(matches);  // NO MERGE

// Phase 5: Unknown matching
let unknown_matches = unknown_match(&self.index, &query, &all_matches);
all_matches.extend(filtered_unknown_matches);  // NO MERGE

// ONLY NOW - merge happens inside refine_matches()
let refined = refine_matches(&self.index, all_matches, &query);
```

**Location**: `src/license_detection/mod.rs:117-271`

### 2.2 Python Reference (index.py:1010-1058)

```python
matchers = [
    Matcher(function=get_spdx_id_matches, ...),
    Matcher(function=self.get_exact_matches, ...),  # Aho-Corasick
    Matcher(function=approx, ...),  # Sequence matching
]

already_matched_qspans = []
for matcher in matchers:
    # Get matches from this matcher
    matched = matcher.function(
        qry,
        matched_qspans=already_matched_qspans,
        existing_matches=matches,
        deadline=deadline,
    )

    # MERGE IMMEDIATELY after each phase
    matched = match.merge_matches(matched)  # <-- KEY DIFFERENCE
    matches.extend(matched)

    # Subtract matched positions for license_text
    for mtch in matched:
        if (mtch.rule.is_license_text
            and mtch.rule.length > 120
            and mtch.coverage() > 98):
            qry.subtract(mtch.qspan)

    # Track 100% coverage matches
    already_matched_qspans.extend(
        mtch.qspan for mtch in matched if mtch.coverage() == 100)
```

**Location**: `reference/scancode-toolkit/src/licensedcode/index.py:1010-1058`

### 2.3 Key Difference

| Aspect | Python | Rust |
|--------|--------|------|
| When merge happens | After each phase | Only at end in `refine_matches()` |
| Number of merges | 3-4 times during matching | 3 times in refine_matches |
| Overlap detection | Immediate | Delayed |
| Score accuracy | Accurate per-phase | Accumulated duplicates affect scoring |

---

## 3. Equivalence Analysis

### 3.1 Is Rust's `merge_overlapping_matches()` Equivalent to Python's `merge_matches()`?

**YES** - After PLAN-024 implementation, they are functionally equivalent.

Both functions implement:

1. Distance-based merging threshold (`max_rule_side_dist`)
2. Grouping by `rule_identifier`
3. Merge conditions: `qcontains`, `surround`, `is_after`, overlap with alignment
4. `qdistance_to()` and `idistance_to()` checks
5. Ispan alignment verification (`qoverlap == ioverlap`)

**Location**: `src/license_detection/match_refine.rs:159-299`

### 3.2 Verification

The function was rewritten in PLAN-024 to match Python's logic:

```rust
// Rust (match_refine.rs:159-299)
fn merge_overlapping_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    // Distance threshold
    let max_rule_side_dist = (rule_length / 2).clamp(1, MAX_DIST);

    // Check both qdistance and idistance
    if current.qdistance_to(&next) > max_rule_side_dist
        || current.idistance_to(&next) > max_rule_side_dist {
        break;
    }

    // All merge conditions match Python...
}
```

```python
# Python (match.py:869-1068)
max_rule_side_dist = min((rule_length // 2) or 1, max_dist)

if (current_match.qdistance_to(next_match) > max_rule_side_dist
    or current_match.idistance_to(next_match) > max_rule_side_dist):
    break
```

**Conclusion**: We can use the existing `merge_overlapping_matches()` function for post-phase merging.

---

## 4. Proposed Changes

### 4.1 Overview

Add `merge_overlapping_matches()` calls after each matching phase in the `detect()` function.

### 4.2 Detailed Changes

**File**: `src/license_detection/mod.rs`

**Current Code (lines 117-249)**:

```rust
let mut all_matches = Vec::new();
let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

// Phase 1: Hash, SPDX, Aho-Corasick matching
{
    let whole_run = query.whole_query_run();
    let hash_matches = hash_match(&self.index, &whole_run);
    for m in &hash_matches {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(hash_matches);
}

{
    let spdx_matches = spdx_lid_match(&self.index, &query);
    // ... similar processing ...
    all_matches.extend(spdx_matches);
}

{
    let aho_matches = aho_match(&self.index, &whole_run);
    // ... similar processing ...
    all_matches.extend(aho_matches);
}

// ... more phases ...
```

**Proposed Code**:

```rust
use crate::license_detection::match_refine::merge_overlapping_matches;

let mut all_matches = Vec::new();
let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

// Phase 1a: Hash matching
{
    let whole_run = query.whole_query_run();
    let hash_matches = hash_match(&self.index, &whole_run);

    // MERGE immediately after hash matching
    let merged_hash = merge_overlapping_matches(&hash_matches);

    for m in &merged_hash {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(merged_hash);
}

// Phase 1b: SPDX-LID matching
{
    let spdx_matches = spdx_lid_match(&self.index, &query);

    // MERGE immediately after SPDX-LID matching
    let merged_spdx = merge_overlapping_matches(&spdx_matches);

    for m in &merged_spdx {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(merged_spdx);
}

// Phase 1c: Aho-Corasick matching
{
    let whole_run = query.whole_query_run();
    let aho_matches = aho_match(&self.index, &whole_run);

    // MERGE immediately after Aho-Corasick matching
    let merged_aho = merge_overlapping_matches(&aho_matches);

    for m in &merged_aho {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(merged_aho);
}

// Phase 2: Near-duplicate detection
{
    let whole_run = query.whole_query_run();
    let near_dupe_candidates = compute_candidates_with_msets(
        &self.index,
        &whole_run,
        true,
        MAX_NEAR_DUPE_CANDIDATES,
    );

    if !near_dupe_candidates.is_empty() {
        let near_dupe_matches =
            seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);

        // MERGE immediately after near-duplicate matching
        let merged_near_dupe = merge_overlapping_matches(&near_dupe_matches);

        for m in &merged_near_dupe {
            if m.end_token > m.start_token {
                let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                query.subtract(&span);
                matched_qspans.push(span);
            }
        }

        all_matches.extend(merged_near_dupe);
    }
}

// Phase 3: Regular sequence matching
{
    let whole_run = query.whole_query_run();
    let seq_matches = seq_match(&self.index, &whole_run);

    // MERGE immediately after sequence matching
    let merged_seq = merge_overlapping_matches(&seq_matches);

    all_matches.extend(merged_seq);
}

// Phase 4: Query run matching
// ... (merge after collecting query_run_matches) ...

// Phase 5: Unknown matching
// (This phase is special - needs matches from all previous phases unmerged)
let unknown_matches = unknown_match(&self.index, &query, &all_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);

// MERGE unknown matches before adding
let merged_unknown = merge_overlapping_matches(&filtered_unknown_matches);
all_matches.extend(merged_unknown);

// Final refinement still includes merge as safety
let refined = refine_matches(&self.index, all_matches, &query);
```

### 4.3 Make `merge_overlapping_matches()` Public

**File**: `src/license_detection/match_refine.rs`

**Current**: Function is private (no `pub`)

**Change**: Add `pub` to make it accessible from `mod.rs`

```rust
/// Merge overlapping and adjacent matches for the same rule.
///
/// Based on Python: `merge_matches()` (match.py:869-1068)
/// Uses distance-based merging with multiple merge conditions.
pub fn merge_overlapping_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    // ... existing implementation ...
}
```

**Also update**: `src/license_detection/mod.rs` imports

```rust
pub use match_refine::merge_overlapping_matches;
```

---

## 5. Alternative: Cross-Phase Merging

### 5.1 Consideration

Should we also merge across phases? For example, should a hash match be merged with an overlapping Aho-Corasick match?

**Python Behavior**: No. Python's `merge_matches()` only merges matches within the same `matched` list from a single phase. Cross-phase merging happens in `refine_matches()`.

### 5.2 Recommendation

**Do NOT merge across phases**. This matches Python's behavior where:

- Each phase's matches are merged internally
- Cross-phase merging happens in `refine_matches()`

This is the correct approach because:

1. Matches from different phases may have different matcher priorities
2. The `matcher_order()` affects sort order and merge decisions
3. `refine_matches()` handles cross-phase deduplication

---

## 6. Performance Considerations

### 6.1 Additional Merge Calls

| Location | Current | Proposed |
|----------|---------|----------|
| Hash match | 0 | 1 |
| SPDX-LID match | 0 | 1 |
| Aho-Corasick match | 0 | 1 |
| Near-duplicate match | 0 | 1 |
| Sequence match | 0 | 1 |
| Unknown match | 0 | 1 |
| `refine_matches()` | 3 | 3 |
| **Total** | 3 | 9 |

### 6.2 Performance Impact

**Expected**: Minimal to negligible impact because:

1. **Merge is O(n log n)**: Sorting dominates, and sorting small lists is fast
2. **Early deduplication reduces later work**: Fewer matches in `refine_matches()` means faster filtering
3. **Typical match counts**: Most phases produce <100 matches; merge is fast on small lists
4. **No I/O**: Merge is pure computation on in-memory data

**Benchmark Recommendation**:

```bash
# Before implementation
cargo test --release -q --lib license_detection::golden_test 2>&1 | grep -E "passed|failed|time"

# After implementation
cargo test --release -q --lib license_detection::golden_test 2>&1 | grep -E "passed|failed|time"
```

### 6.3 Mitigation Strategy

If performance becomes an issue:

1. **Skip merge for empty/single matches**:

   ```rust
   let merged = if matches.len() < 2 {
       matches
   } else {
       merge_overlapping_matches(&matches)
   };
   ```

2. **Use inline caching** for the merge function

3. **Profile before optimizing**: Measure actual impact before adding complexity

---

## 7. Test Requirements

### 7.1 Unit Tests

**File**: `src/license_detection/match_refine.rs` (tests module)

Add tests for multi-phase merge behavior:

```rust
#[test]
fn test_merge_handles_empty_input() {
    let matches: Vec<LicenseMatch> = vec![];
    let merged = merge_overlapping_matches(&matches);
    assert!(merged.is_empty());
}

#[test]
fn test_merge_handles_single_match() {
    let m = create_test_match("mit", 1, 10, 95.0, 100);
    let merged = merge_overlapping_matches(&[m]);
    assert_eq!(merged.len(), 1);
}

#[test]
fn test_merge_deduplicates_overlapping_same_rule() {
    let m1 = create_test_match_with_tokens("#1", 0, 20, 20);
    let m2 = create_test_match_with_tokens("#1", 10, 30, 20); // Overlaps

    let merged = merge_overlapping_matches(&[m1, m2]);
    // Should merge into one
    assert_eq!(merged.len(), 1);
}

#[test]
fn test_merge_keeps_separate_different_rules() {
    let m1 = create_test_match_with_tokens("mit", 0, 20, 20);
    let m2 = create_test_match_with_tokens("apache", 10, 30, 20); // Different rule

    let merged = merge_overlapping_matches(&[m1, m2]);
    // Should NOT merge - different rules
    assert_eq!(merged.len(), 2);
}
```

### 7.2 Integration Tests

**File**: `src/license_detection/mod.rs` (tests module)

Test that phases produce properly merged results:

```rust
#[test]
fn test_detect_merges_hash_matches() {
    // Create test content that produces multiple hash matches
    // Verify they are merged before being added to all_matches
}

#[test]
fn test_detect_merges_aho_matches() {
    // Create test content with overlapping Aho-Corasick matches
    // Verify proper merging
}
```

### 7.3 Golden Test Validation

**Per TESTING_STRATEGY.md**: Run full golden test suite and compare results.

```bash
# Run all golden tests
cargo test --release -q --lib license_detection::golden_test 2>&1 | tail -50

# Run specific failing tests for validation
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_external 2>&1 | tail -20
```

### 7.4 Expected Test Improvements

Based on PLAN-029 analysis:

| Test Suite | Current Failures | Expected After Fix |
|------------|------------------|-------------------|
| lic1 | ~60 | ~50 |
| lic2 | ~57 | ~45 |
| lic3 | ~37 | ~30 |
| lic4 | ~51 | ~40 |
| external | ~200+ | ~100 |

---

## 8. Risk Assessment

### 8.1 Risks

| Risk | Severity | Probability | Mitigation |
|------|----------|-------------|------------|
| Performance regression | Low | Low | Benchmark before/after |
| Breaking passing tests | Medium | Low | Run full suite after each change |
| Incorrect merge logic | Low | Very Low | PLAN-024 already validated merge function |
| Missing edge cases | Medium | Medium | Comprehensive unit tests |

### 8.2 Rollback Plan

If issues arise:

1. Revert changes to `mod.rs`
2. Keep `merge_overlapping_matches()` public (harmless)
3. Document findings for future investigation

---

## 9. Implementation Order

1. **Step 1**: Make `merge_overlapping_matches()` public in `match_refine.rs`
2. **Step 2**: Update imports in `mod.rs`
3. **Step 3**: Add merge after hash_match phase
4. **Step 4**: Run tests, verify no regression
5. **Step 5**: Add merge after spdx_lid_match phase
6. **Step 6**: Run tests
7. **Step 7**: Add merge after aho_match phase
8. **Step 8**: Run tests
9. **Step 9**: Add merge after near_dupe phase
10. **Step 10**: Run tests
11. **Step 11**: Add merge after seq_match phase
12. **Step 12**: Run tests
13. **Step 13**: Add merge for unknown matches
14. **Step 14**: Run full golden test suite
15. **Step 15**: Document results

---

## 10. Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/match_refine.rs` | Make `merge_overlapping_matches()` public |
| `src/license_detection/mod.rs` | Add merge calls after each phase (lines 117-249) |

---

## 11. References

- **Python `match_query()`**: `reference/scancode-toolkit/src/licensedcode/index.py:966-1080`
- **Python `merge_matches()`**: `reference/scancode-toolkit/src/licensedcode/match.py:869-1068`
- **Rust `merge_overlapping_matches()`**: `src/license_detection/match_refine.rs:159-299`
- **Rust `detect()` function**: `src/license_detection/mod.rs:117-271`
- **PLAN-024**: Match merging implementation details
- **PLAN-029 Section 2.4**: Analysis of missing post-phase merge
- **TESTING_STRATEGY.md**: Test requirements and validation approach

---

## 12. Acceptance Criteria

- [ ] `merge_overlapping_matches()` is public and accessible
- [ ] Merge is called after each matching phase
- [ ] Unit tests pass for merge function
- [ ] Golden test suite shows improvement (no regressions)
- [ ] Performance is acceptable (no significant slowdown)
- [ ] Code is documented with references to Python implementation
