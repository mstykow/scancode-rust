# PLAN-065: Extra Words Coverage Calculation

## Status: RESOLVED

## Problem Statement

`test_analyze_detection_extra_words` and `test_has_extra_words_true` were failing.

## Root Cause

The `has_extra_words()` function was using `m.icoverage() * 100.0` to compute coverage, but:
- `icoverage()` returns 0-1 (fraction)
- Multiplying by 100 gives 0-100 range
- However, `icoverage()` computes coverage dynamically from token positions which may not be set in test matches

## Fix

Changed to use `m.match_coverage` directly (the pre-computed coverage field stored as 0-100 percentage).

**Before:**
```rust
let coverage = m.icoverage() * 100.0;
let score_coverage_relevance = coverage * m.rule_relevance as f32 / 100.0;
```

**After:**
```rust
let score_coverage_relevance = m.match_coverage * m.rule_relevance as f32 / 100.0;
```

## Python Reference

Python's `calculate_query_coverage_coefficient()` at `detection.py:1115-1148`:
```python
score_coverage_relevance = (
    license_match.coverage() * license_match.rule.relevance
) / 100
```

Where `coverage()` returns the pre-computed percentage (0-100), not dynamically computed.

## Files Changed

- `src/license_detection/detection.rs:295-300`

## Tests Fixed

- `test_analyze_detection_extra_words`
- `test_has_extra_words_true`
