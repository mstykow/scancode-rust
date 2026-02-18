# PLAN-018: Fix Golden Test Flakiness

## Status: In Progress

## Problem Statement

Golden tests are flaky - they produce different results between runs:

- lic4 varies by ±3 tests (280-284 passed)
- lic1 varies by ±2 tests (228-230 passed)
- lic3 varies by ±2 tests (251-253 passed)
- lic2 is stable (777 passed consistently)

This undermines our ability to trust test results and measure progress.

---

## Root Cause Analysis

### Primary Cause: Incomplete Sorting

Rust's `sort_by` is **NOT stable** - equal elements can swap positions arbitrarily. When multiple matches/detections have identical sort keys, the order is non-deterministic, which propagates through the pipeline affecting which matches survive filtering.

### Affected Locations

| Priority | File:Line | Function | Issue |
|----------|-----------|----------|-------|
| **CRITICAL** | match_refine.rs:337 | `filter_overlapping_matches` | Missing tie-breaker for matches with same start_token, hilen, matched_length, matcher_order |
| **CRITICAL** | detection.rs:920 | `rank_detections` | Missing tie-breaker for detections with same score, coverage |
| HIGH | match_refine.rs:203 | `filter_contained_matches` | Missing tie-breaker for matches with same start_token, hilen, matched_length |
| HIGH | detection.rs:940 | `sort_detections_by_line` | Missing tie-breaker for detections with same min_line |
| HIGH | seq_match.rs:62-78 | `ScoresVector::Ord` | Missing `rid` as final tie-breaker |

### Why This Causes Flakiness

1. `filter_overlapping_matches` determines which matches survive vs. get discarded
2. When two matches have identical sort keys, Rust's unstable sort can order them differently
3. Different order → different matches discarded → different final detections
4. Golden tests compare `Vec<&str>` of license expressions - order matters!

---

## Implementation Plan

### Phase 1: Fix Critical Sorts in match_refine.rs

#### Fix 0: merge_overlapping_matches HashMap iteration (line 146) - CRITICAL

**Issue**: HashMap iteration order is non-deterministic. Python sorts by rule.identifier BEFORE grouping.

**Current code:**

```rust
for (_rid, rule_matches) in grouped {
```

**Fixed code:**

```rust
let mut grouped: Vec<_> = grouped.into_iter().collect();
grouped.sort_by(|a, b| a.0.cmp(&b.0));  // Sort by rule_identifier
for (_rid, rule_matches) in grouped {
```

#### Fix 1: filter_overlapping_matches (line 337)

**Current code:**

```rust
matches.sort_by(|a, b| {
    a.start_token
        .cmp(&b.start_token)
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

**Fixed code:**

```rust
matches.sort_by(|a, b| {
    a.start_token
        .cmp(&b.start_token)
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        .then_with(|| a.rule_identifier.cmp(&b.rule_identifier))  // Tie-breaker
});
```

#### Fix 2: filter_contained_matches (line 203)

**Current code:**

```rust
sorted.sort_by(|a, b| {
    a.start_token
        .cmp(&b.start_token)
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
});
```

**Fixed code:**

```rust
sorted.sort_by(|a, b| {
    a.start_token
        .cmp(&b.start_token)
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.rule_identifier.cmp(&b.rule_identifier))  // Tie-breaker
});
```

### Phase 2: Fix Critical Sorts in detection.rs

#### Fix 3: rank_detections (line 920)

**Current code:**

```rust
detections.sort_by(|a, b| {
    score_b.partial_cmp(&score_a).unwrap()
        .then_with(|| coverage_b.partial_cmp(&coverage_a).unwrap())
});
```

**Fixed code:**

```rust
detections.sort_by(|a, b| {
    score_b.partial_cmp(&score_a).unwrap()
        .then_with(|| coverage_b.partial_cmp(&coverage_a).unwrap())
        .then_with(|| a.identifier.cmp(&b.identifier))  // Tie-breaker
});
```

#### Fix 4: sort_detections_by_line (line 940)

**Current code:**

```rust
detections.sort_by(|a, b| {
    min_line_a.cmp(&min_line_b)
});
```

**Fixed code:**

```rust
detections.sort_by(|a, b| {
    min_line_a.cmp(&min_line_b)
        .then_with(|| a.identifier.cmp(&b.identifier))  // Tie-breaker
});
```

### Phase 3: Fix seq_match.rs

#### Fix 5: ScoresVector::Ord (lines 62-78)

Add `rid` as final tie-breaker in the `Ord` implementation for `ScoresVector`.

### Phase 4: Verification

1. Run each flaky test suite 5 times
2. Confirm all runs produce identical results
3. Run all golden tests once to confirm no regressions

---

## Python Reference Alignment

### Verification Results

| Fix | Python Equivalent | Aligned? | Notes |
|-----|------------------|----------|-------|
| **Fix 0: merge_overlapping_matches HashMap** | `match.py:882-884` | **Now YES** | Python sorts by rule.identifier before grouping |
| **Fix 1: filter_overlapping_matches** | `match.py:1220` | **Acceptable** | Python uses stable sort, we add explicit tie-breaker |
| **Fix 2: filter_contained_matches** | `match.py:1099` | **Acceptable** | Same as above |
| **Fix 3: rank_detections** | `detection.py:1011-1012` | **YES** | Python uses identifier as tie-breaker! |
| **Fix 4: sort_detections_by_line** | No direct equivalent | **Reasonable** | Ensures determinism |
| **Fix 5: ScoresVector::Ord** | `match_set.py:480-482` | **YES** | Python uses rule.identifier! |

### Key Findings

1. Python's `list.sort()` is **stable** - it preserves insertion order for equal elements
2. Rust's `slice::sort_by()` is **NOT stable** - equal elements can swap arbitrarily
3. Python often relies on stable sort rather than explicit tie-breakers
4. Adding explicit tie-breakers in Rust achieves the same determinism as Python's stable sort

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Adding tie-breakers changes results | Low | Tie-breakers only affect order of equal elements |
| Python uses different tie-breakers | Medium | Verify against Python reference |
| Performance impact | None | Sorting is O(n log n), tie-breaker is O(1) |

---

## Estimated Effort

| Phase | Time |
|-------|------|
| Phase 1: match_refine.rs fixes | 30 min |
| Phase 2: detection.rs fixes | 30 min |
| Phase 3: seq_match.rs fix | 30 min |
| Phase 4: Verification | 30 min |
| **Total** | **2 hours** |

---

## Verification Commands

```bash
# Run a flaky test multiple times
for i in {1..5}; do
    cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic4 2>&1 | grep "lic4:"
done

# Run all golden tests
cargo test --release -q --lib license_detection::golden_test
```
