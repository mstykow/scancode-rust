# PLAN-037: Add Post-Phase `merge_matches()` Calls

**Date**: 2026-02-23
**Status**: Planning Complete - Ready for Implementation (Dependencies Identified)
**Priority**: HIGH
**Related**: PLAN-024, PLAN-029 (Sections 2.4, 3.1), PLAN-036
**Prerequisites**: Sort key fix (matcher_order), PLAN-036 (optional)

## Executive Summary

Python's license detection calls `merge_matches()` after each matching phase to deduplicate overlapping matches before the next phase runs. Rust only merges at the end of `refine_matches()`, causing overlapping matches from different phases to remain as separate matches, leading to duplicate expressions and incorrect containment filtering.

**Key Findings During Investigation**:

1. **CRITICAL PREREQUISITE**: The `merge_overlapping_matches()` sort key is missing `matcher_order`, which must be fixed first
2. **MISSING FEATURE**: Python returns immediately after `hash_match()` if a match is found - Rust should do the same
3. **PHASE MAPPING CORRECTED**: Rust's Phases 2-4 (near_dupe, seq, query_runs) collectively correspond to Python's single `approx` matcher
4. **PLAN-036 DEPENDENCY**: The equal ispan selection uses wrong metric (matched_length vs magnitude) - fix recommended but optional

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

## 9. Dependency Analysis (CRITICAL - Read Before Implementation)

### 9.1 Prerequisite: Sort Key Missing `matcher_order`

**CRITICAL ISSUE**: The current `merge_overlapping_matches()` sort key does NOT include `matcher_order`.

**Python Sort Key** (`match.py:882`):
```python
sorter = lambda m: (m.rule.identifier, m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
```

**Current Rust Sort Key** (`match_refine.rs:169-175`):
```rust
sorted.sort_by(|a, b| {
    a.rule_identifier
        .cmp(&b.rule_identifier)
        .then_with(|| a.start_token.cmp(&b.start_token))
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
    // MISSING: .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

**Impact**: Without `matcher_order` in the sort key, matches from different phases may be sorted incorrectly, causing incorrect merge decisions. This is a prerequisite fix that must be done FIRST.

**Fix Location**: `src/license_detection/match_refine.rs:169-175`

**Required Change**:
```rust
sorted.sort_by(|a, b| {
    a.rule_identifier
        .cmp(&b.rule_identifier)
        .then_with(|| a.start_token.cmp(&b.start_token))
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))  // ADD THIS LINE
});
```

### 9.2 Prerequisite: Equal ISpan Selection Fix (PLAN-036)

**CRITICAL ISSUE**: The `merge_overlapping_matches()` function uses `matched_length` instead of `qspan.magnitude()` for equal ispan selection.

**Python** (`match.py:946-970`):
```python
if current_match.ispan == next_match.ispan and current_match.overlap(next_match):
    cqmag = current_match.qspan.magnitude()
    nqmag = next_match.qspan.magnitude()
    if cqmag <= nqmag:  # Smaller magnitude wins
        del rule_matches[j]
```

**Current Rust** (`match_refine.rs:225-234`):
```rust
if current.ispan() == next.ispan() && current.qoverlap(&next) > 0 {
    if current.matched_length >= next.matched_length {  // WRONG: uses matched_length
        rule_matches.remove(j);
```

**Impact**: For non-contiguous matches, `magnitude` != `matched_length`, causing different matches to be kept.

**Resolution**: PLAN-036 addresses this. Either:
1. Implement PLAN-036 first, OR
2. Accept this behavioral difference for now and document it

**Recommendation**: Fix this as part of PLAN-037 implementation since it affects merge correctness.

### 9.3 Phase Mapping Correction

The current plan incorrectly maps Rust phases to Python phases. Here is the CORRECT mapping:

| Python Phase | Python Matcher | Rust Equivalent | Rust Code Location |
|--------------|----------------|-----------------|-------------------|
| Pre-phase | `hash_match()` | `hash_match()` | `mod.rs:129` |
| Matcher 1 | `get_spdx_id_matches` | `spdx_lid_match()` | `mod.rs:144` |
| Matcher 2 | `get_exact_matches` (Aho) | `aho_match()` | `mod.rs:160` |
| Matcher 3 | `approx` (seq) | `seq_match_with_candidates()` + `seq_match()` + query_runs | `mod.rs:186-243` |

**Key Observations**:

1. **Python has 3 matchers in the loop** (SPDX-LID, Aho, seq), each followed by `merge_matches()`
2. **Rust splits sequence matching** into near-duplicate (Phase 2), regular (Phase 3), and query runs (Phase 4)
3. **Rust's Phase 2-4** collectively correspond to Python's single `approx` matcher

**Correct Post-Phase Merge Points for Rust**:

1. After `hash_match()` - **NOT NEEDED** (Python returns early, see below)
2. After `spdx_lid_match()` - **YES** (corresponds to Python matcher 1)
3. After `aho_match()` - **YES** (corresponds to Python matcher 2)
4. After **all** sequence matching (near_dupe + seq + query_runs) - **YES** (corresponds to Python matcher 3)
5. After `unknown_match()` - **MAYBE** (Python doesn't merge unknown, they're handled separately)

### 9.4 Hash Match Early Return Decision

**Python Behavior** (`index.py:987-991`):
```python
if not _skip_hash_match:
    matches = match_hash.hash_match(self, whole_query_run)
    if matches:
        match.set_matched_lines(matches, qry.line_by_pos)
        return matches  # EARLY RETURN - skip all other phases
```

**Current Rust Behavior** (`mod.rs:129-141`):
```rust
let hash_matches = hash_match(&self.index, &whole_run);
for m in &hash_matches {
    // ... track matched_qspans, subtract license_text ...
}
all_matches.extend(hash_matches);  // NO EARLY RETURN - continues to other phases
```

**Question**: Should Rust add early return like Python?

**Analysis**:
- Python returns immediately on hash match because: exact 100% match found, no need for other matchers
- Rust continues to other phases, potentially finding overlapping matches
- This causes duplicate/overlapping matches for hash-matched content

**Recommendation**: **YES, add early return** after hash_match when matches are found.

**Code Change** (`mod.rs:129-141`):
```rust
let hash_matches = hash_match(&self.index, &whole_run);
if !hash_matches.is_empty() {
    // Hash match found - return immediately like Python
    // Set matched lines and return
    let mut matches = hash_matches;
    sort_matches_by_line(&mut matches);
    // ... create detection and return ...
    return Ok(detections);
}
```

**Impact**: This eliminates an entire class of duplicate match issues where hash matches overlap with other matcher results.

### 9.5 Summary: Prerequisites and Implementation Order

**MUST FIX BEFORE PLAN-037**:
1. Add `matcher_order` to sort key in `merge_overlapping_matches()` (single line change)
2. Consider PLAN-036 (magnitude vs matched_length) - can be done in parallel or accepted as known difference

**INCLUDED IN PLAN-037**:
1. Add hash_match early return (not mentioned in original plan)
2. Add merge after spdx_lid_match
3. Add merge after aho_match
4. Add merge after ALL sequence matching (near_dupe + seq + query_runs combined)
5. Unknown match handling (no merge needed - Python doesn't merge these)

---

## 10. Corrected Implementation Order

### Phase 0: Prerequisite Fixes (Do First)

1. **Step 0.1**: Add `matcher_order` to sort key in `merge_overlapping_matches()`
   - File: `src/license_detection/match_refine.rs:169-175`
   - Add: `.then_with(|| a.matcher_order().cmp(&b.matcher_order()))`
   - Run tests: `cargo test --lib license_detection`

2. **Step 0.2**: (Optional) Implement PLAN-036 for magnitude-based selection
   - OR document as known behavioral difference

### Phase 1: Hash Match Early Return

3. **Step 1.1**: Add early return after `hash_match()` when matches found
   - File: `src/license_detection/mod.rs:129-141`
   - Match Python's behavior: return immediately on hash match
   - Run tests: `cargo test --lib license_detection`

### Phase 2: Post-Phase Merge Calls

4. **Step 2.1**: Make `merge_overlapping_matches()` public
   - File: `src/license_detection/match_refine.rs:159`
   - Change `fn` to `pub fn`

5. **Step 2.2**: Add merge after `spdx_lid_match()`
   - Merge before extending `all_matches`
   - Run tests

6. **Step 2.3**: Add merge after `aho_match()`
   - Merge before extending `all_matches`
   - Run tests

7. **Step 2.4**: Add merge after ALL sequence matching
   - This includes: near_dupe + seq_match + query_runs
   - Collect all sequence matches, merge once, then extend `all_matches`
   - Run tests

### Phase 3: Validation

8. **Step 3.1**: Run full golden test suite
   - `cargo test --release -q --lib license_detection::golden_test`

9. **Step 3.2**: Compare results with baseline
   - Document improvements
   - Document any regressions

10. **Step 3.3**: Update this plan with results

---

## 11. Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/match_refine.rs:169-175` | Add `matcher_order` to sort key |
| `src/license_detection/match_refine.rs:159` | Make `merge_overlapping_matches()` public |
| `src/license_detection/mod.rs:129-141` | Add hash_match early return |
| `src/license_detection/mod.rs:117-271` | Add merge calls after SPDX, Aho, and sequence phases |

---

## 12. References

- **Python `match_query()`**: `reference/scancode-toolkit/src/licensedcode/index.py:966-1080`
- **Python `merge_matches()`**: `reference/scancode-toolkit/src/licensedcode/match.py:869-1068`
- **Python hash_match early return**: `reference/scancode-toolkit/src/licensedcode/index.py:987-991`
- **Python sort key with matcher_order**: `reference/scancode-toolkit/src/licensedcode/match.py:882`
- **Rust `merge_overlapping_matches()`**: `src/license_detection/match_refine.rs:159-299`
- **Rust sort key (missing matcher_order)**: `src/license_detection/match_refine.rs:169-175`
- **Rust `detect()` function**: `src/license_detection/mod.rs:117-271`
- **PLAN-024**: Match merging implementation details
- **PLAN-029 Section 2.4**: Analysis of missing post-phase merge
- **PLAN-029 Section 3.1**: Missing `matcher_order` in sort key
- **PLAN-036**: Equal ISpan selection fix (magnitude vs matched_length)
- **TESTING_STRATEGY.md**: Test requirements and validation approach

---

## 13. Acceptance Criteria

### Prerequisites (Must Complete First)
- [ ] `matcher_order` added to sort key in `merge_overlapping_matches()` (match_refine.rs:169-175)
- [ ] (Optional) PLAN-036 implemented for magnitude-based selection OR documented as known difference

### Core Implementation
- [ ] `merge_overlapping_matches()` is public and accessible
- [ ] Hash match early return added (matching Python behavior)
- [ ] Merge called after `spdx_lid_match()` phase
- [ ] Merge called after `aho_match()` phase
- [ ] Merge called after all sequence matching (near_dupe + seq + query_runs)
- [ ] Unknown matches handled correctly (no merge needed per Python behavior)

### Validation
- [ ] All unit tests pass
- [ ] Golden test suite shows improvement (no regressions)
- [ ] Performance is acceptable (no significant slowdown)
- [ ] Code is documented with references to Python implementation

---

## 14. Validation Report (To Be Filled After Implementation)

### Prerequisite Fixes Applied

| Fix | Status | Date | Notes |
|-----|--------|------|-------|
| `matcher_order` in sort key | Pending | - | - |
| PLAN-036 magnitude fix | Pending | - | - |

### Implementation Results

| Phase | Status | Tests Passing | Notes |
|-------|--------|---------------|-------|
| Hash early return | Pending | - | - |
| SPDX-LID merge | Pending | - | - |
| Aho merge | Pending | - | - |
| Sequence merge | Pending | - | - |

### Golden Test Impact

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| lic1 passing | ~60 | - | - |
| lic2 passing | ~57 | - | - |
| lic3 passing | ~37 | - | - |
| lic4 passing | ~51 | - | - |
| external passing | ~200 | - | - |
| **Total** | ~405 | - | - |

### Performance Impact

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Merge calls per scan | 3 | 6-7 | +100-133% |
| Avg scan time | TBD | - | - |

### Issues Found

(To be filled during implementation)
