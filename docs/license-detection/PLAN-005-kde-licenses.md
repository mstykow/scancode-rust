# PLAN-005: kde_licenses_test.txt

## Status: ROOT CAUSE IDENTIFIED

## Test File
`testdata/license-golden/datadriven/lic4/kde_licenses_test.txt`

## Issue
Missing `lgpl-2.1` detections, extra `lgpl-2.1-plus`, extra `bsd-simplified`.

**Expected:** 15 matches (Python output)
**Actual:** 14 matches (Rust output)

## Root Cause Analysis

### Issue 1: lgpl-2.1 matches replaced by sequence match

**Key Finding:** The lgpl-2.1 aho matches ARE found and survive through aho refine, but are filtered out during the final refine after sequence matching.

**Pipeline trace:**

1. **After aho matching**: lgpl-2.1 found at:
   - tokens 655-662 (lines 90-92), coverage=100%
   - tokens 662-665 (lines 94-94), coverage=100%

2. **matched_qspans**: Both lgpl-2.1 matches added (coverage >= 99.99%)

3. **is_matchable check**: Returns `true` because there are still matchable regions elsewhere in the file

4. **Sequence matching runs**: Produces lgpl-2.1-plus at tokens 659-778 (lines 92-109)

5. **After final refine**: lgpl-2.1 aho matches are filtered out, only lgpl-2.1-plus seq remains

**Python vs Rust difference:**
- Python: Returns lgpl-2.1 (aho) at lines 90-92 and 94-94, lgpl-2.1-plus (aho) at lines 98-109
- Rust: Returns lgpl-2.1-plus (seq) at lines 92-109 - this single match REPLACES the three separate matches

**The divergence:**
1. Python's aho matching produces lgpl-2.1-plus at lines 98-109
2. Rust's sequence matching produces lgpl-2.1-plus at lines 92-109 (covering 6 more lines at start)
3. This larger seq match in Rust overlaps with and replaces the lgpl-2.1 aho matches

### Issue 2: bsd-simplified extra detection

**Python behavior:**
- Returns only `bsd-simplified AND bsd-new` at lines 111-140 (seq match)
- No standalone `bsd-simplified` match

**Rust behavior:**
- Returns `bsd-simplified` at lines 111-113 (aho match) - EXTRA
- Returns `bsd-simplified AND bsd-new` at lines 117-140 (aho match)

**Root Cause:**
Similar to issue 1 - aho matching finds a smaller match that should be contained within the larger conjunction match, but it's not being filtered out properly.

## Investigation Tests

Located at: `src/license_detection/investigation/kde_licenses_test.rs`

Key tests:
- `test_kde_licenses_aho_refine_trace` - Traces aho-only pipeline, shows lgpl-2.1 matches survive refine
- `test_kde_licenses_full_pipeline` - Traces full pipeline including matched_qspans
- `test_kde_licenses_lgpl_21_missing` - Failing test asserting 2 lgpl-2.1 matches in final output

## Code References

- `src/license_detection/mod.rs:detect()` - Main detection pipeline
- `src/license_detection/match_refine.rs:filter_overlapping_matches()` - Overlap filtering logic
- `src/license_detection/query.rs:is_matchable()` - Check for remaining matchable tokens
- `reference/scancode-toolkit/src/licensedcode/index.py:match_query()` - Python reference

## Next Steps

1. **Investigate sequence match boundaries:**
   - Why does Rust's seq match start at line 92 while Python's aho starts at line 98?
   - The seq match covers 6 extra lines that should have lgpl-2.1 aho matches

2. **Check filter_overlapping_matches logic:**
   - The lgpl-2.1 aho match at tokens 655-662 overlaps with lgpl-2.1-plus seq at 659-778
   - Which one should be kept? Python keeps both (different regions), Rust keeps only seq

3. **Possible fix:**
   - Ensure seq matching respects already-matched regions (matched_qspans)
   - OR ensure refine/merge preserves smaller non-contained matches
