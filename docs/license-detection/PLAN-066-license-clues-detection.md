# PLAN-066: License Clues Detection Test

## Status: RESOLVED

## Problem Statement

`test_analyze_detection_license_clues` was failing.

## Root Cause

The test was not correctly setting up match data for `has_correct_license_clue_matches()` which requires BOTH:
1. 100% match coverage (via `is_correct_detection`)
2. `is_license_clue: true` flag

## Fix

Updated test to set both conditions:

```rust
let mut m = create_test_match_with_params(
    "mit", "2-aho", 1, 10,
    100.0, 100, 100, 100.0, 100,  // 100% coverage
    "mit.LICENSE",
);
m.is_license_clue = true;  // Required for license clue detection
```

## Python Reference

Python's `has_correct_license_clue_matches()` at `detection.py:1265-1272`:
```python
def has_correct_license_clue_matches(license_matches):
    return is_correct_detection(license_matches) and all(
        match.rule.is_license_clue for match in license_matches
    )
```

And `is_correct_detection()` requires `coverage() == 100`.

## Files Changed

- `src/license_detection/detection.rs:4053-4070` (test)

## Tests Fixed

- `test_analyze_detection_license_clues`
