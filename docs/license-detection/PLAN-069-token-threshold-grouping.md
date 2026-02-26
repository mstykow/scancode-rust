# PLAN-069: Token Threshold Not Used for Grouping

## Status: RESOLVED

## Problem Statement

`test_grouping_separates_by_token_threshold` expected 2 groups (separation by token threshold) but got 1 group.

## Root Cause

Test assumption was incorrect. Python's `group_matches()` ONLY uses line threshold for grouping, NOT token threshold.

## Fix

Corrected test to expect 1 group with appropriate message:

```rust
assert_eq!(
    groups.len(),
    1,
    "Should group when line gap (2) is within threshold (4) - token gap is not used for grouping"
);
```

## Python Reference

Python's `group_matches()` at `detection.py:1820-1868`:
```python
def group_matches(license_matches, lines_threshold=LINES_THRESHOLD):
    # ...
    is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold
    # ... NO token threshold check
```

The function only takes `lines_threshold` as parameter, no token threshold.

## Files Changed

- `src/license_detection/detection.rs:1419-1428` (test)

## Tests Fixed

- `test_grouping_separates_by_token_threshold`
