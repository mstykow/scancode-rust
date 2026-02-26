# PLAN-072: Has Extra Words Coverage Fix

## Status: RESOLVED

## Problem Statement

`test_has_extra_words_true` was failing.

## Root Cause

Same as PLAN-065: `has_extra_words()` was using `m.icoverage() * 100.0` which computes coverage dynamically from token positions that weren't set in test matches.

## Fix

Use `m.match_coverage` (pre-computed coverage) directly:

```rust
let score_coverage_relevance = m.match_coverage * m.rule_relevance as f32 / 100.0;
```

## Python Reference

Python's `calculate_query_coverage_coefficient()` uses pre-computed `coverage()` value, not dynamically computed.

## Files Changed

- `src/license_detection/detection.rs:295-300`

## Tests Fixed

- `test_has_extra_words_true`
- `test_analyze_detection_extra_words` (same fix)
