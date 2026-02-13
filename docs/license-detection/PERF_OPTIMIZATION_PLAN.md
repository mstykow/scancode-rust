# Performance Optimization Plan: Early Exit & Aho HashSet Elimination

## Overview

This plan addresses the top 2 performance opportunities:

1. **Early exit after hash match** - Skip expensive matchers when hash finds 100% coverage
2. **Eliminate HashSet in Aho hot path** - Replace with direct range iteration

## Estimated Impact

- **Early exit**: 1.3x speedup for exact matches (LICENSE files, etc.)
- **HashSet elimination**: 1.2x speedup for all Aho matches
- **Combined**: ~1.5x additional speedup

---

## Change 1: Early Exit After Hash/SPDX Match

### File: `src/license_detection/mod.rs`

### Current Code (lines 105-124)

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let query = Query::new(text, &self.index)?;
    let query_run = query.whole_query_run();

    let mut all_matches = Vec::new();

    let hash_matches = hash_match(&self.index, &query_run);
    all_matches.extend(hash_matches);

    let spdx_matches = spdx_lid_match(&self.index, text);
    all_matches.extend(spdx_matches);

    let aho_matches = aho_match(&self.index, &query_run);
    all_matches.extend(aho_matches);

    let seq_matches = seq_match(&self.index, &query_run);
    all_matches.extend(seq_matches);

    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    all_matches.extend(unknown_matches);
    // ...
}
```

### Proposed Change

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let query = Query::new(text, &self.index)?;
    let query_run = query.whole_query_run();

    let mut all_matches = Vec::new();

    // Run hash and SPDX matchers first (fast)
    let hash_matches = hash_match(&self.index, &query_run);
    all_matches.extend(hash_matches);

    let spdx_matches = spdx_lid_match(&self.index, text);
    all_matches.extend(spdx_matches);

    // Early exit: if hash match found 100% coverage, skip expensive matchers
    // Hash match with 100% coverage means we found an exact license text match
    let has_perfect_match = all_matches.iter().any(|m| m.match_coverage >= 100.0);
    
    if !has_perfect_match {
        // Run Aho-Corasick (moderate cost)
        let aho_matches = aho_match(&self.index, &query_run);
        all_matches.extend(aho_matches);

        // Early exit: check if we have high-coverage matches before running seq_match
        let has_high_coverage = all_matches.iter().any(|m| m.match_coverage >= 90.0);
        
        if !has_high_coverage {
            // Run sequence matcher (most expensive)
            let seq_matches = seq_match(&self.index, &query_run);
            all_matches.extend(seq_matches);
        }
    }

    // Always run unknown matcher for unmatched regions
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    all_matches.extend(unknown_matches);
    // ...
}
```

### Implementation Details

1. **After hash_matches**: Check if any match has `match_coverage >= 100.0`
2. **If true**: Skip `aho_match` and `seq_match` entirely
3. **After aho_matches** (if run): Check if any match has `match_coverage >= 90.0`
4. **If true**: Skip `seq_match` (most expensive)
5. **Always run**: `unknown_match` to detect any remaining unknown licenses

### Trade-offs

- **Pro**: Significant speedup for exact matches (LICENSE files)
- **Pro**: Still catches all licenses
- **Con**: Slightly more complex control flow
- **Con**: Need to verify coverage threshold (100% vs 95% vs 90%)

---

## Change 2: Eliminate HashSet in Aho-Corasick Hot Path

### File: `src/license_detection/aho_match.rs`

### Current Code (lines 100-106)

```rust
for ac_match in automaton.find_iter(&encoded_query) {
    // ...
    let qspan_positions: HashSet<usize> = (qstart..qend).collect();  // ALLOCATION

    let is_entirely_matchable = qspan_positions.iter().all(|pos| matchables.contains(pos));

    if !is_entirely_matchable {
        continue;
    }
    // ...
}
```

### Problem

- For EVERY match (potentially hundreds per file), a new `HashSet<usize>` is allocated
- The HashSet is immediately discarded after checking matchability
- This involves heap allocation + hashing overhead

### Proposed Change

```rust
for ac_match in automaton.find_iter(&encoded_query) {
    // ...
    // Direct range iteration - no allocation
    let is_entirely_matchable = (qstart..qend).all(|pos| matchables.contains(pos));

    if !is_entirely_matchable {
        continue;
    }
    // ...
}
```

### Implementation Details

1. Remove `HashSet` import (line 13) if no longer needed
2. Replace:

   ```rust
   let qspan_positions: HashSet<usize> = (qstart..qend).collect();
   let is_entirely_matchable = qspan_positions.iter().all(|pos| matchables.contains(pos));
   ```

   With:

   ```rust
   let is_entirely_matchable = (qstart..qend).all(|pos| matchables.contains(pos));
   ```

3. The `matchables` HashSet is still used, but we iterate the range and check membership

### Why This Works

- `matchables` is a `HashSet<usize>` containing all matchable positions
- `(qstart..qend).all(|pos| matchables.contains(pos))` checks if every position in range is matchable
- No intermediate allocation needed
- Same O(n) complexity but without heap allocation

### Alternative: Use BitSet (Future Optimization)

If `matchables` were a `BitSet` instead of `HashSet<usize>`:

- O(1) membership check instead of O(1) with hashing
- Better cache locality
- Would require changing `QueryRun::matchables()` return type

---

## Test Plan

### Before Changes

```bash
# Measure baseline
time cargo test test_golden_unknown --release -- --nocapture
```

### After Each Change

```bash
# After change 1 (early exit)
time cargo test test_golden_unknown --release -- --nocapture

# After change 2 (HashSet elimination)  
time cargo test test_golden_unknown --release -- --nocapture
```

### Verification

```bash
# Ensure all tests still pass
cargo test --lib --release

# Ensure clippy is clean
cargo clippy --tests -- -D warnings
```

---

## Implementation Order

1. **Change 2 first** (simpler, no control flow changes)
   - Modify `aho_match.rs`
   - Run tests
   - Measure improvement

2. **Change 1 second** (more complex, needs careful testing)
   - Modify `mod.rs` detect()
   - Test with various file types
   - Verify no regressions in detection accuracy

---

## Files to Modify

| File | Lines | Change |
|------|-------|--------|
| `src/license_detection/mod.rs` | 105-124 | Early exit logic |
| `src/license_detection/aho_match.rs` | 100-106 | Remove HashSet allocation |
