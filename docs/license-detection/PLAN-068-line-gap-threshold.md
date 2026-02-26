# PLAN-068: Line Gap Threshold Grouping

## Status: RESOLVED

## Problem Statement

`test_group_matches_just_past_line_gap_threshold` was failing - expected 2 groups but got 1.

## Root Cause

Test had incorrect boundary condition. Python uses `<=` for line gap comparison:
```python
is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold
```

With threshold 4 and gap 4, `4 <= 4` is true → matches are grouped.
The test intended to test separation, so gap must be 5 (exceeds threshold).

## Fix

Changed match2's start_line from 9 to 10:

```rust
let match2 = create_test_match(10, 14, "2-aho", "mit.LICENSE");  // Was (9, 13)
// Gap is now 5 (10-5=5), which exceeds threshold 4
```

## Python Reference

Python at `detection.py:1836`:
```python
is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold
```

## Files Changed

- `src/license_detection/detection.rs:1265-1272` (test)

## Tests Fixed

- `test_group_matches_just_past_line_gap_threshold`
