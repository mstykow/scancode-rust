# Fix Plan: Sequence Match Position Calculation

## Problem Summary

The sequence matcher produces matches with incorrect start positions because it can find matches that overlap with earlier AHO matches from Phase 1.

**Example**:
- Current: cc-by-3.0 starts at line 12 (inside BSD license text)
- Expected: cc-by-3.0 should start at line 36 (actual CC-BY license text)

## Root Cause Analysis

### The Real Issue: Cross-Phase Position Exclusion Failure

The issue is NOT about stale matchables within a single `match_blocks` call. The issue is:

1. **`matched_qspans` from earlier phases (AHO) are used for SKIP decisions but NOT for position exclusion**
   - In `mod.rs:138-220`, `matched_qspans` is built from AHO matches with 100% coverage
   - `matched_qspans` is passed to `is_matchable()` to decide whether to skip phases (line 227, 288)
   - BUT `matched_qspans` is NEVER passed to `seq_match_with_candidates()`

2. **`seq_match_with_candidates()` calls `query_run.matchables(true)` which returns ALL positions**
   - Location: `matching.rs:239-243`
   - This returns ALL matchable positions without excluding already-matched positions
   - Already-matched positions from AHO matches are NOT excluded

3. **This allows sequence matching to find matches overlapping earlier AHO matches**
   - When AHO finds a 100% coverage match at positions 0-100
   - And seq matching runs on a candidate that has similar tokens
   - The seq matcher can extend backward from position 80 into positions 0-100
   - This produces incorrect match positions

### Python vs Rust Comparison

**Python** (`reference/scancode-toolkit/src/licensedcode/index.py:1044-1049`):
```python
# After each AHO match with 100% coverage
if mtch.coverage() == 100:
    query.subtract(mtch.qspan)  # UPDATES the matchable positions!
```

**Python** (`reference/scancode-toolkit/src/licensedcode/query.py:863-871`):
```python
def subtract(self, qspan):
    if qspan:
        self.query.subtract(qspan)
        # INVALIDATES the cached matchables
        self._high_matchables = self.high_matchables.difference_update(qspan)
        self._low_matchables = self.low_matchables.difference_update(qspan)
```

**Key difference**: Python's `query.subtract()` directly modifies the query's matchables. When `query_run.matchables` is accessed later, it returns the updated set. Rust's `Query::subtract()` modifies `high_matchables` and `low_matchables`, but the caller doesn't always call it, and `seq_match_with_candidates` doesn't receive the exclusion information.

### Evidence from Code Flow

**Phase 1c** (`mod.rs:187-221`):
```rust
let refined_aho = match_refine::refine_aho_matches(&self.index, aho_matches, &query);

for m in &refined_aho {
    if (m.match_coverage * 100.0).round() / 100.0 == 100.0
        && m.end_token > m.start_token
    {
        matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
    }
}
// NOTE: query.subtract() is NOT called here!
```

**Phase 2** (`mod.rs:234-261`):
```rust
let near_dupe_matches =
    seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);
// matched_qspans is NOT passed to seq_match_with_candidates!
```

**Inside `seq_match_with_candidates`** (`matching.rs:239-243`):
```rust
let matchables: HashSet<usize> = query_run
    .matchables(true)  // Returns ALL matchables, no exclusion!
    .into_iter()
    .map(|pos| pos - query_run.start)
    .collect();
```

## Solution Design

### Approach: Pass matched_qspans to seq_match_with_candidates

The fix needs to pass the already-matched positions to `seq_match_with_candidates()` so it can exclude them when computing matchables.

**Changes Required**:

1. **Add `matched_qspans` parameter to `seq_match_with_candidates()`**
   - Signature change from:
     ```rust
     pub fn seq_match_with_candidates(
         index: &LicenseIndex,
         query_run: &QueryRun,
         candidates: &[Candidate],
     ) -> Vec<LicenseMatch>
     ```
   - To:
     ```rust
     pub fn seq_match_with_candidates(
         index: &LicenseIndex,
         query_run: &QueryRun,
         candidates: &[Candidate],
         matched_qspans: &[query::PositionSpan],
     ) -> Vec<LicenseMatch>
     ```

2. **Filter matchables to exclude matched_qspans positions**
   - Inside the function, filter the matchables set before using it

3. **Update all call sites** (approximately 20+ locations)

## Implementation Steps

### Step 1: Update function signature and implementation

**File**: `src/license_detection/seq_match/matching.rs`
**Location**: Lines 211-215 (function signature), line 248 (is_matchable call), and lines 239-243 (matchables computation)

**Change function signature** (lines 211-215):
```rust
pub fn seq_match_with_candidates(
    index: &LicenseIndex,
    query_run: &QueryRun,
    candidates: &[Candidate],
    matched_qspans: &[crate::license_detection::query::PositionSpan],
) -> Vec<LicenseMatch> {
```

**Update is_matchable call** (line 248):
```rust
// Pass matched_qspans to check if any positions remain to match
if !query_run.is_matchable(false, matched_qspans) {
    break;
}
```

**Update matchables computation** (lines 239-243):
```rust
// Compute matchables, excluding already-matched positions from earlier phases
let matchables: HashSet<usize> = query_run
    .matchables(true)
    .into_iter()
    .map(|pos| pos - query_run.start)
    .filter(|pos| {
        // Exclude positions that were already matched in earlier phases
        !matched_qspans.iter().any(|span| span.contains(*pos + query_run.start))
    })
    .collect();
```

### Step 2: Update call sites in mod.rs

**File**: `src/license_detection/mod.rs`

**Phase 2 call site** (line 247):
```rust
let near_dupe_matches =
    seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates, &matched_qspans);
```

**Phase 3 call site** (line 274):
```rust
let matches = seq_match_with_candidates(&self.index, &whole_run, &candidates, &matched_qspans);
```

**Phase 4 call site** (line 300):
```rust
let matches =
    seq_match_with_candidates(&self.index, query_run, &candidates, &matched_qspans);
```

**Same changes in detect_matches** (lines 442, 469, 495).

### Step 3: Update call sites in other files

The grep found ~154 matches. The main files needing updates are:

1. **`src/license_detection/mod.rs`** - Main pipeline (6 call sites)
2. **`src/bin/*.rs`** - Debug/tracing binaries (~20+ call sites)
3. **`src/license_detection/investigation/*_test.rs`** - Test files (~15+ call sites)
4. **`src/license_detection/seq_match/mod.rs`** - Unit tests (~10 call sites)

For test files and binaries, pass an empty slice `&[]` if they don't need exclusion logic.

### Step 4: Add import for PositionSpan in matching.rs

**File**: `src/license_detection/seq_match/matching.rs`
**Location**: Line 5 (add import)

```rust
use crate::license_detection::query::PositionSpan;
```

### Complete Diff for Core Changes

```diff
--- a/src/license_detection/seq_match/matching.rs
+++ b/src/license_detection/seq_match/matching.rs
@@ -2,6 +2,7 @@
 
 use crate::license_detection::index::LicenseIndex;
 use crate::license_detection::models::LicenseMatch;
+use crate::license_detection::query::PositionSpan;
 use crate::license_detection::query::QueryRun;
 use std::collections::{HashMap, HashSet};
 
@@ -208,6 +209,7 @@ pub fn seq_match_with_candidates(
     index: &LicenseIndex,
     query_run: &QueryRun,
     candidates: &[Candidate],
+    matched_qspans: &[PositionSpan],
 ) -> Vec<LicenseMatch> {
     let mut matches = Vec::new();
 
@@ -236,9 +238,15 @@ pub fn seq_match_with_candidates(
             let qbegin = 0usize;
             let qfinish = query_tokens.len().saturating_sub(1);
 
-            let matchables: HashSet<usize> = query_run
+            // Compute matchables, excluding already-matched positions from earlier phases
+            // This prevents sequence matching from finding matches that overlap
+            // with AHO matches from Phase 1
+            let matchables: HashSet<usize> = query_run
                 .matchables(true)
                 .into_iter()
                 .map(|pos| pos - query_run.start)
+                .filter(|pos| {
+                    !matched_qspans.iter().any(|span| span.contains(*pos + query_run.start))
+                })
                 .collect();
```

```diff
--- a/src/license_detection/mod.rs
+++ b/src/license_detection/mod.rs
@@ -244,7 +244,7 @@ impl LicenseDetectionEngine {
                 if !near_dupe_candidates.is_empty() {
                     let near_dupe_matches =
-                        seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);
+                        seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates, &matched_qspans);
 
                     for m in &near_dupe_matches {
                         if m.end_token > m.start_token {
@@ -271,7 +271,7 @@ impl LicenseDetectionEngine {
                 );
                 if !candidates.is_empty() {
-                    let matches = seq_match_with_candidates(&self.index, &whole_run, &candidates);
+                    let matches = seq_match_with_candidates(&self.index, &whole_run, &candidates, &matched_qspans);
                     seq_all_matches.extend(matches);
                 }
             }
@@ -297,7 +297,7 @@ impl LicenseDetectionEngine {
                     );
                     if !candidates.is_empty() {
                         let matches =
-                            seq_match_with_candidates(&self.index, query_run, &candidates);
+                            seq_match_with_candidates(&self.index, query_run, &candidates, &matched_qspans);
                         seq_all_matches.extend(matches);
                     }
                 }
@@ -439,7 +439,7 @@ impl LicenseDetectionEngine {
                 if !near_dupe_candidates.is_empty() {
                     let near_dupe_matches =
-                        seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);
+                        seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates, &matched_qspans);
 
                     for m in &near_dupe_matches {
                         if m.end_token > m.start_token {
@@ -466,7 +466,7 @@ impl LicenseDetectionEngine {
                 );
                 if !candidates.is_empty() {
-                    let matches = seq_match_with_candidates(&self.index, &whole_run, &candidates);
+                    let matches = seq_match_with_candidates(&self.index, &whole_run, &candidates, &matched_qspans);
                     seq_all_matches.extend(matches);
                 }
             }
@@ -492,7 +492,7 @@ impl LicenseDetectionEngine {
                 );
                 if !candidates.is_empty() {
                     let matches =
-                        seq_match_with_candidates(&self.index, query_run, &candidates);
+                        seq_match_with_candidates(&self.index, query_run, &candidates, &matched_qspans);
                     seq_all_matches.extend(matches);
                 }
             }
```

## Testing Strategy

### Unit Tests

**File**: `src/license_detection/seq_match/matching.rs`

Update existing tests to pass `&[]` for the new parameter:

```rust
let matches = seq_match_with_candidates(&index, &query_run, &candidates, &[]);
```

### Integration Test

**File**: Create a test that verifies the fix

```rust
#[test]
fn test_seq_match_excludes_matched_qspans() {
    // Test that seq_match_with_candidates excludes positions
    // that were already matched in earlier phases
}
```

### Golden Tests

Run the golden test suite:
```bash
cargo test --release -q --lib license_detection::golden_test
```

## Risk Assessment

### Low Risk

- **Fix is targeted**: Only affects matchable position filtering
- **No algorithm changes**: The matching algorithm itself is unchanged
- **Consistent with Python**: Mirrors Python's position exclusion behavior

### Medium Risk

- **Many call sites**: ~20+ locations need updates
- **Test coverage**: All tests must pass empty slice correctly

### Mitigation

1. Use compiler to find all call sites (signature change will error on all)
2. Run full test suite after changes
3. Compare golden test results with expected behavior

## Success Criteria

1. **Correctness**: Matches don't overlap with earlier AHO matches
2. **No regressions**: All existing tests pass
3. **Golden tests**: Improved accuracy in multi-license files

## Timeline

- **Implementation**: 2 hours (many call sites to update)
- **Testing**: 1 hour
- **Total**: 3 hours

## References

- Rust implementation: `src/license_detection/seq_match/matching.rs:211-348`
- Call sites: `src/license_detection/mod.rs:247, 274, 300, 442, 469, 495`
- Query matchables: `src/license_detection/query/mod.rs:852-861`
- PositionSpan: `src/license_detection/query/mod.rs:16-47`
