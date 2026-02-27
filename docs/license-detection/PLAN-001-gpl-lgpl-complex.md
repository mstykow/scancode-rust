# PLAN-001: gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt

## Status: ROOT CAUSE IDENTIFIED

## Test File
`testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt`

## Issue
Extra `lgpl-2.1-plus` detection caused by spurious seq match spanning gap.

**Expected:** `["gpl-3.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus AND free-unknown", "mit-modern", "gpl-2.0-plus", "gpl-2.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0"]`

**Actual:** `["gpl-3.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus AND free-unknown", "mit-modern", "gpl-2.0-plus", "gpl-2.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0"]`

## Root Cause

The seq match `lgpl-2.1-plus_419.RULE` at lines 14-25 incorrectly spans across two separate LGPL blocks with unrelated content between them:

1. **First LGPL block** (lines 13-17): "This library is free software..."
2. **Gap** (lines 18-21): Copyright info and "Files: lib/ifd.h" header
3. **Second LGPL block** (lines 22-26): Same license text as first block

This spurious seq match causes the aho match at lines 13-17 to be discarded in `filter_overlapping_matches`.

## Investigation Findings

### Pipeline Stage Analysis

| Stage | What Happens |
|-------|--------------|
| Aho Match | Correct matches at lines 13-17 (`lgpl-2.1-plus_24.RULE`) and 22-26 |
| Seq Match | Spurious match at lines 14-25 (`lgpl-2.1-plus_419.RULE`) with 70% coverage |
| Merge | No change in match count |
| Filter Contained | Correct match at 13-17 still present |
| Filter Overlapping | **BUG**: Match at 13-17 discarded because seq match overlaps |

### The Spurious Match

The seq match `lgpl-2.1-plus_419.RULE` at lines 14-25 (tokens 78-162, coverage 70%):
- Starts at token 78 (line 14) - inside first LGPL block
- Ends at token 162 (line 25) - inside second LGPL block
- Spans across 8 lines of unrelated content (lines 18-21)

### Why Python Doesn't Have This Issue

Python's seq matching either:
1. Does not produce this match at all (different candidate selection/alignment), OR
2. Filters it out before final results

### Potential Fixes

1. **Fix seq matching**: Prevent matches that span across unrelated content
2. **Fix overlap filtering**: Give preference to aho matches over seq matches when they overlap
3. **Add gap detection**: Reject seq matches that have large gaps in qspan

## Investigation Tests

Created at `src/license_detection/investigation/gpl_lgpl_complex_test.rs`:
- `test_plan_001_divergence_point` - Traces full pipeline, identifies spurious seq match
- `test_plan_001_expected_behavior` - Documents expected vs actual behavior

## Next Steps

1. Investigate why seq matching produces match spanning gap
2. Compare Python's seq matching for same input
3. Implement fix (likely in seq_match.rs or match_refine.rs)
4. Verify fix with golden test
