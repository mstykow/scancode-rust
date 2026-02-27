# PLAN-004: ijg.txt

## Status: ROOT CAUSE IDENTIFIED

## Test File
`testdata/license-golden/datadriven/lic4/ijg.txt`

## Issue
Multiple extra detections: warranty-disclaimer, extra ijg, and free-unknown.

**Expected:** `["ijg"]`

**Actual:** `["ijg", "warranty-disclaimer", "ijg", "free-unknown", "free-unknown"]`

## Root Cause

The engine's aho matching phase subtracts large license text matches from the query (see `mod.rs:198-202`):
```rust
if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
    let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
    query.subtract(&span);
}
```

For ijg.txt, this subtracts two aho matches:
- `ijg_17.RULE` (lines 34-46, rule_length=127, coverage=100%)
- `ijg_8.RULE` (lines 34-59, rule_length=229, coverage=100%)

After this subtraction, when the engine runs near-dupe candidate selection, it returns **0 candidates** because the query no longer has enough tokens in the right places.

Without near-dupe candidates, the engine falls back to regular seq matching (70 candidates), which produces 176 matches. After refine, these become fragmented:
- `ijg_26.RULE` (lines 12-22)
- `warranty-disclaimer_28.RULE` (lines 26-29)
- `free-unknown_63.RULE` (lines 72, 75)

## Python Behavior (Expected)

Python's near-dupe matching finds `ijg.LICENSE` (lines 12-96, coverage=99.56%) as a single match that covers the entire license. This is the correct result.

## What Should Happen

The `ijg.LICENSE` rule should be found via near-dupe matching, which would produce a single large match covering lines 12-96. This match would then survive refine and produce the correct detection.

## Investigation Tests

Created in `src/license_detection/investigation/ijg_test.rs`:

1. `test_ijg_pipeline_trace` - Shows manual pipeline produces correct result
2. `test_ijg_exact_engine_trace` - Shows engine produces 0 near-dupe candidates
3. `test_ijg_query_subtraction_check` - Confirms query subtraction is the cause
4. `test_ijg_near_dupe_with_matched_qspans` - Shows correct candidates without subtraction

## Fix Options

1. **Don't subtract from query for near-dupe**: The subtraction should only affect phases 3+ (regular seq), not phase 2 (near-dupe). Near-dupe matching needs the full query to find the best match.

2. **Run near-dupe BEFORE aho subtraction**: Move near-dupe phase before the aho subtraction step, so it has access to the full query.

3. **Include aho matches in final result**: When aho matches with high coverage are found, ensure they are properly merged with seq matches rather than relying on seq to re-find them.

## Failing Tests

- `test_ijg_full_detection` - Asserts single detection "ijg"
- `test_ijg_should_have_single_detection` - Asserts correct match details
- `test_ijg_no_free_unknown` - Asserts no free-unknown detections
