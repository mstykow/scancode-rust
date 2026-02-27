# PLAN-002: gpl-2.0-plus_and_mpl-1.0.txt

## Status: ROOT CAUSE IDENTIFIED - NEEDS FIX

## Test File
`testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_mpl-1.0.txt`

## Issue
Should detect `mpl-1.0 OR gpl-2.0-plus` as single expression, but Rust detects them separately.

**Expected:** `["mpl-1.0 OR gpl-2.0-plus"]`
**Actual:** `["mpl-1.0 AND gpl-1.0-plus AND gpl-2.0-plus"]`

## Root Cause Analysis

### Python Behavior (Correct)
Python matches a **combined rule** `mpl-1.0_or_gpl-2.0-plus_2.RULE` via the **seq matcher** (near-duplicate detection):
- Single match: `mpl-1.0 OR gpl-2.0-plus` at lines 1-21
- Matcher: `3-seq`
- Rule: `mpl-1.0_or_gpl-2.0-plus_2.RULE`
- Score: 89.55, coverage: 100%

The combined rule text closely matches the test file content.

### Rust Behavior (Incorrect)
Rust finds **three separate matches** via the **aho matcher**:
1. `mpl-1.0` at lines 3-11 (rule: `mpl-1.0_22.RULE`, 100% coverage)
2. `gpl-1.0-plus` at line 13 (rule: `gpl_bare_word_only.RULE`, 100% coverage) **← EXTRA MATCH**
3. `gpl-2.0-plus` at lines 17-21 (rule: `gpl-2.0-plus_85.RULE`, 100% coverage)

Then combines them with AND: `mpl-1.0 AND gpl-1.0-plus AND gpl-2.0-plus`

### The Key Issue: Seq Matching May Be Skipped

In `src/license_detection/mod.rs:207-211`:
```rust
let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
```

If the aho matches cover all matchable regions, seq matching is SKIPPED. This would explain why the combined rule is not found - the seq matcher never runs because the three individual aho matches cover the entire file.

**Hypothesis**: The `is_matchable()` check considers the file "fully matched" after the three aho matches, causing seq matching to be skipped. But Python still runs the seq matcher and finds the combined rule, which should replace the individual matches.

### Additional Issues

1. **Extra bare-word match**: `gpl_bare_word_only.RULE` matches "GPL:" on line 13. This rule has `relevance: 50` and `is_license_reference: yes`.

2. **Match priority**: When both aho matches (100% coverage) and seq matches (89.55% coverage) exist, Python's seq match should take priority because it represents a coherent combined license notice.

## Investigation Tests

Created `src/license_detection/investigation/gpl_mpl_test.rs` with tests for:
- `test_gpl_mpl_rust_detection` - Shows current failing behavior
- `test_gpl_mpl_aho_matches` - Shows phase 1 aho matches
- `test_gpl_mpl_seq_matches` - Shows near-dupe candidates and seq matches
- `test_gpl_mpl_refine_pipeline` - Shows filtering and grouping
- `test_gpl_mpl_is_matchable_check` - **KEY TEST**: Checks if seq matching is skipped
- `test_gpl_mpl_combined_rule_candidate` - Checks if combined rule is in near-dupe candidates

## Fix Required

### Primary Fix
The seq matcher should ALWAYS run for near-duplicate detection when there are aho matches, regardless of whether the aho matches "cover everything". The `skip_seq_matching` logic at `mod.rs:211` should NOT skip Phase 2 (near-dupe detection) - only Phases 3-4 (regular seq and query runs).

The combined rule `mpl-1.0_or_gpl-2.0-plus_2.RULE` represents a better match (coherent dual-license notice) than the three separate matches. Python's approach is to:
1. Run all matchers (aho + seq)
2. Let the refine/filter pipeline prefer the higher-quality combined match

### Proposed Change
```rust
// Current code (mod.rs:207-218):
let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);

// Proposed change:
// Always run Phase 2 (near-dupe) even if aho matches cover everything
// Only skip Phases 3-4 (regular seq and query runs)
let skip_regular_seq = !whole_run.is_matchable(false, &matched_qspans);

// Phase 2: Near-duplicate detection - ALWAYS RUN
{ /* ... near dupe code ... */ }

// Phases 3-4: Only run if there are still matchable regions
if !skip_regular_seq {
    // Phase 3: Regular seq
    // Phase 4: Query runs
}
```

### Secondary Fix (Optional)
After the primary fix, consider filtering out `is_license_reference` rules with low relevance when a full license text match exists, to avoid the spurious `gpl_bare_word_only` match.

## Key Files

- Detection pipeline: `src/license_detection/mod.rs:207-291`
- Combined rule: `reference/scancode-toolkit/src/licensedcode/data/rules/mpl-1.0_or_gpl-2.0-plus_2.RULE`
- Bare-word rule: `reference/scancode-toolkit/src/licensedcode/data/rules/gpl_bare_word_only.RULE`
- is_matchable check: `src/license_detection/query.rs`
- Match refinement: `src/license_detection/match_refine.rs`

## Next Steps

1. Run `test_gpl_mpl_is_matchable_check` to confirm seq matching is being skipped
2. Implement the fix: Always run Phase 2 (near-dupe) regardless of aho coverage
3. Verify the fix makes the golden test pass
4. Check for regression in other tests
