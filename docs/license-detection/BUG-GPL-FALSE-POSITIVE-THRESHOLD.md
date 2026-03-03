# Bug: GPL False Positive Threshold Incorrect

## Status: Fixed

**Fix commit:** f8fa98a4 (reduces failing golden tests from 133 to 121)

## Summary

The Rust implementation incorrectly filters out GPL license matches with `rule_length <= 3` as false positives, while Python only filters those with `rule_length == 1`. This causes golden test failures for files containing short GPL license references like `EXPORT_SYMBOL_GPL`.

## Affected Files

- `src/license_detection/detection/analysis.rs` - Contains the buggy `is_false_positive()` function
- Golden tests: `gpl-2.0-plus_29.txt`, and potentially others

## Root Cause

Two different `is_false_positive` concepts exist in the codebase with different semantics:

### 1. Rule-level `is_false_positive` flag (Correct)
- Location: `src/license_detection/match_refine/filter_low_quality.rs:filter_false_positive_matches()`
- Uses `index.false_positive_rids` populated from rule file's `is_false_positive` boolean field
- This matches Python's `filter_false_positive_matches()` which checks `match.rule.is_false_positive`

### 2. Detection-level `is_false_positive` heuristic (Buggy)
- Location: `src/license_detection/detection/analysis.rs:is_false_positive()`
- Uses heuristics to classify detections as false positives
- **Bug**: GPL check uses wrong threshold

## The Bug

**Python** (`reference/scancode-toolkit/src/licensedcode/detection.py`):
```python
all_match_rule_length_one = all(
    match_rule_length == 1
    for match_rule_length in match_rule_length_values
)
# ...
if is_gpl and all_match_rule_length_one:
    return True
```

**Rust** (`src/license_detection/detection/analysis.rs`):
```rust
// Check 2: GPL with short rule length
if is_gpl
    && matches
        .iter()
        .all(|m| m.rule_length <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD)  // 3!
{
    return true;
}
```

The Rust code uses `<= 3` while Python uses `== 1`.

## Impact

For `gpl-2.0_kernel_export_symbol_gpl.RULE`:
- Rule text: `EXPORT_SYMBOL_GPL`
- Tokenized: `["export", "symbol", "gpl"]` = **3 tokens**
- Python: `all_match_rule_length_one` = False (3 != 1) → NOT filtered
- Rust: `rule_length <= 3` = True (3 <= 3) → **incorrectly filtered out**

## Test Case Evidence

### gpl-2.0-plus_29.txt
- **Expected**: `["gpl-2.0-plus", "gpl-2.0"]`
- **Actual**: `["gpl-2.0-plus"]`
- The `gpl-2.0` detection (from `EXPORT_SYMBOL_GPL` on line 161) is incorrectly filtered

## Proposed Fix

Change lines 126-131 in `src/license_detection/detection/analysis.rs` from:

```rust
// Check 2: GPL with short rule length
if is_gpl
    && matches
        .iter()
        .all(|m| m.rule_length <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD)
{
    return true;
}
```

To:

```rust
// Check 2: GPL with rule_length == 1 (matching Python's all_match_rule_length_one)
if is_gpl && all_rule_length_one
{
    return true;
}
```

The variable `all_rule_length_one` is already computed earlier in the function (line 111):
```rust
let all_rule_length_one = rule_length_values.iter().all(|&l| l == 1);
```

## Verification

After the fix, the golden test for `gpl-2.0-plus_29.txt` should pass with:
- Detection 1: `gpl-2.0-plus` (from lines 29-32)
- Detection 2: `gpl-2.0` (from line 161, `EXPORT_SYMBOL_GPL`)

## Related Code Paths

1. `engine.detect()` calls `post_process_detections()`
2. `post_process_detections()` calls `filter_detections_by_score()`
3. `filter_detections_by_score()` calls `classify_detection()`
4. `classify_detection()` calls `is_false_positive()` (the buggy function)

## Note on Debug Script Consistency

The debug script (`src/bin/debug_license_detection.rs`) produces the same results as the golden test because both go through `post_process_detections()`. The discrepancy initially observed was due to comparing different test cases, not a difference between the debug script and the engine.

## References

- Python source: `reference/scancode-toolkit/src/licensedcode/detection.py:1162-146` (`is_false_positive` function)
- Rust source: `src/license_detection/detection/analysis.rs:72-146` (`is_false_positive` function)
