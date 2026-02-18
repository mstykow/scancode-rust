# PLAN-016: Remaining License Detection Fixes

## Status: PARTIALLY IMPLEMENTED - 13 tests fixed

### Summary of Progress

- Baseline: 103 failures
- After first round: 102 failures
- Current: **89 failures**
- **Net improvement: 14 tests fixed**

### Completed Work

**Phase A**: Implemented `matched_qspans` tracking, post-loop logic for `has_unknown_intro_before_detection()`

**Phase B**: Fixed `hilen()`, implemented `qdensity()`/`idensity()` methods

**Phase C**: Implemented 6 missing filters, fixed matcher string bug

**Phase D** (just implemented by subagents):

- **Issue 2**: Fixed Aho-Corasick token boundary bug
- **Issue 5**: Changed to token-based overlap calculation

---

## Current State

| Metric | Value |
|--------|-------|
| lic1 passed | 202 |
| lic1 failed | **89** |

---

## Already Implemented Issues

### Issue 2: False Positive Detections ✅ IMPLEMENTED

**Problem**: Rust detects licenses Python doesn't, especially `cc-by-nc-sa-2.0`.

**Root Cause**: The Aho-Corasick automaton was matching across token boundaries. When encoding `u16` token IDs as little-endian byte pairs, it could accidentally match byte 1 = HIGH byte of token N and byte 2 = LOW byte of token N+1.

**Implementation** (`src/license_detection/aho_match.rs`):

```rust
// Added token alignment check
if byte_start % 2 != 0 {
    continue;
}
```

**Tests Added**:

- `test_aho_match_token_boundary_bug`
- `test_aho_match_single_token_matches_correctly`
- `test_no_token_boundary_false_positives`

---

### Issue 5: GPL Variant Confusion ✅ IMPLEMENTED

**Problem**: Rust detects wrong GPL variants (e.g., `gpl-1.0-plus` when `gpl-2.0-plus` expected).

**Root Cause**: The `filter_overlapping_matches` function used line-based overlap calculation while Python uses token-based (qspan) overlap calculation.

**Implementation**:

1. **Added `qoverlap()` method** (`src/license_detection/models.rs`):

```rust
pub fn qoverlap(&self, other: &LicenseMatch) -> usize {
    if self.start_token == 0 && self.end_token == 0
        && other.start_token == 0 && other.end_token == 0
    {
        // Fallback to line-based
        let start = self.start_line.max(other.start_line);
        let end = self.end_line.min(other.end_line);
        return if start <= end { end - start + 1 } else { 0 };
    }
    // Token-based
    let start = self.start_token.max(other.start_token);
    let end = self.end_token.min(other.end_token);
    end.saturating_sub(start)
}
```

1. **Changed `filter_overlapping_matches`** (`src/license_detection/match_refine.rs`):
   - Sort by `start_token` instead of `start_line`
   - Use `qoverlap()` instead of `calculate_overlap()`
   - Changed termination check to use `end_token`/`start_token`

**Tests Added**:

- `test_filter_contained_matches_gpl_variant_issue`
- `test_filter_contained_matches_gpl_variant_zero_tokens`

---

## Remaining Issues to Implement

### Issue 1: Match Over-Merging (~30+ tests remaining)

**Problem**: Rust combines matches that Python keeps separate.

**Example**: `CRC32.java`

- Expected: `["apache-2.0", "bsd-new", "zlib"]`
- Actual: `["apache-2.0", "bsd-new AND zlib"]`

**Status**: ✅ READY FOR IMPLEMENTATION

#### Root Cause

**Rust's `should_group_together()` uses dual-criteria (token AND line), but Python only uses lines:**

| Aspect | Python | Rust | Issue |
|--------|--------|------|-------|
| Threshold | `LINES_THRESHOLD = 4` | `LINES_GAP_THRESHOLD = 3` | Off by 1 |
| Logic | `start <= end + 4` | `gap <= 3` | Off by 1 |
| Token check | None | `token_gap <= 10` | Extra condition |

#### Implementation Plan

**File**: `src/license_detection/detection.rs`

```rust
// BEFORE:
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    let token_gap = cur.start_token.saturating_sub(prev.end_token);
    
    token_gap <= TOKENS_THRESHOLD && line_gap <= LINES_GAP_THRESHOLD
}

// AFTER:
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    line_gap <= LINES_THRESHOLD  // Use 4, not 3
}
```

Also remove unused `TOKENS_THRESHOLD` and `LINES_GAP_THRESHOLD` constants.

---

### Issue 3A: Combined Rules Not Matched (~10+ tests)

**Problem**: Combined OR rules like `cddl-1.0_or_gpl-2.0-glassfish.RULE` don't match.

**Example**: `cddl-1.0_or_gpl-2.0-glassfish.txt`

- Expected: `["cddl-1.0 OR gpl-2.0"]`
- Actual: `["gpl-2.0 AND cddl-1.0", ...]`

**Status**: ✅ READY FOR IMPLEMENTATION

#### Root Cause (CONFIRMED)

**The bug is in `src/license_detection/query.rs:994`:**

```rust
pub fn matchable_tokens(&self) -> Vec<i32> {
    ...
    let matchables = self.matchables(false);  // BUG: false = only high matchables!
    ...
}
```

Python uses ALL matchables (high + low), resulting in:

- Query set size: **127** tokens, resemblance: **0.945** (passes 0.8 threshold)

Rust uses ONLY high matchables, resulting in:

- Query set size: **28** tokens, resemblance: **0.233** (fails 0.8 threshold)

#### Implementation Plan

**File**: `src/license_detection/query.rs`

```rust
// BEFORE (line 994):
let matchables = self.matchables(false);

// AFTER:
let matchables = self.matchables(true);
```

This is a **one-line fix** that will enable combined rule matching.

---

### Issue 4: Query Run Double-Matching

**Problem**: Enabling query runs causes regression.

**Status**: ✅ READY FOR IMPLEMENTATION

#### Root Cause

`QueryRun` holds stale references to `query.high_matchables` after `subtract()` creates a new HashSet.

#### Implementation Plan

**File**: `src/license_detection/query.rs`

Use lazy evaluation by storing reference to parent `Query`:

```rust
pub struct QueryRun<'q, 'a> {
    query: &'q Query<'a>,
    start: usize,
    end: Option<usize>,
}

impl<'q, 'a> QueryRun<'q, 'a> {
    pub fn high_matchables(&self) -> HashSet<usize> {
        self.query.high_matchables
            .iter()
            .filter(|&&pos| pos >= self.start && pos <= self.end.unwrap_or(usize::MAX))
            .copied()
            .collect()
    }
}
```

---

### Issue 6: Extra Unknown License References

**Problem**: Rust generates extra `unknown-license-reference` matches.

**Status**: ✅ READY FOR IMPLEMENTATION

#### Root Cause

1. **Covered position calculation uses lines instead of tokens**
2. **Missing hispan threshold check** (Python requires `hispan >= 5`)

#### Implementation Plan

**Fix 6A: Token-based Coverage** (`src/license_detection/unknown_match.rs`):

```rust
fn compute_covered_positions(
    _query: &Query,
    known_matches: &[LicenseMatch],
) -> HashSet<usize> {
    let mut covered = HashSet::new();
    for m in known_matches {
        for pos in m.start_token..m.end_token {
            covered.insert(pos);
        }
    }
    covered
}
```

**Fix 6B: Hispan Threshold**:

```rust
fn create_unknown_match(...) -> Option<LicenseMatch> {
    // ...
    let hispan = (start..end)
        .filter(|&pos| query.tokens.get(pos).map_or(false, |&t| (t as usize) < index.len_legalese))
        .count();
    
    if hispan < 5 {
        return None;
    }
    // ...
}
```

---

## Implementation Order (Remaining)

1. **Issue 1** (Grouping threshold) - Simple, should fix ~30 tests
2. **Issue 3A** (matchable_tokens fix) - **One-line fix**, enables combined rules
3. **Issue 6A/6B** (Unknown match filtering) - Simple changes
4. **Issue 4** (Query run lazy evaluation) - More complex, save for last

---

## Verification Commands

```bash
# Run golden tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Format and lint
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```
