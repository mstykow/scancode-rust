# PLAN-067: Unknown Reference Filters in Detection

## Status: RESOLVED

## Problem Statement

`test_create_detection_from_group_unknown_reference_filters` expected 1 match but got 2.

## Root Cause

Test expectation was incorrect. Python stores raw unfiltered matches in `detection.matches`, while filtering only applies to `matches_for_expression` for license expression computation.

## Fix

Corrected test expectation:

```rust
// detection.matches stores RAW matches (matching Python behavior)
// filtering only applies to matches_for_expression for license computation
assert_eq!(detection.matches.len(), 2);  // Changed from 1
```

## Python Reference

Python at `detection.py:256,261` stores raw matches:
```python
detection = cls(
    matches=matches,  # Original unfiltered matches
    license_expression=str(license_expression),
    ...
)
```

And `get_detected_license_expression()` uses separate `matches_for_expression` variable (lines 1494-1533).

## Files Changed

- `src/license_detection/detection.rs:4659-4665` (test)

## Tests Fixed

- `test_create_detection_from_group_unknown_reference_filters`
