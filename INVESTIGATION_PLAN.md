# Investigation Plan: Fix Remaining 96 Failing Golden Tests

## Current Status
- **Baseline**: 96 failing golden test cases
- **Previous fix**: Changed unconditional subtract in Phase 2 near-duplicate matching (102 → 98, now 96)

## Hypotheses to Investigate

### H1: Hash Matching Early Return Issue
**Observation**: `detect_matches()` returns early when hash matches are found (line 384)
**Potential issue**: Hash matching may be returning combined matches instead of letting other matchers find separate matches
**Investigation needed**: 
- Check if mit_25.txt triggers hash match
- Verify if hash match covers both the notice and license text
- Compare with Python's hash matching behavior

### H2: MAX_DIST Threshold Difference  
**Observation**: Rust uses MAX_DIST=100 vs Python's 50
**Potential issue**: Merges matches that Python keeps separate when token gap is 51-100
**Investigation needed**:
- Find test cases where matches have token gaps between 51-100
- Test if reducing MAX_DIST to 50 fixes some failures

### H3: QueryRun Splitting Not Implemented
**Observation**: Rust has QueryRun splitting disabled, Python splits on 4+ empty lines
**Potential issue**: Misses opportunities to find matches in separate regions
**Investigation needed**:
- Check if any failing tests have 4+ empty line gaps
- Test if enabling QueryRun splitting fixes some failures

### H4: Grouping Threshold Edge Case
**Observation**: mit_25.txt has matches with line_gap=4, which equals LINES_THRESHOLD
**Potential issue**: Matches that should be separate are being grouped together
**Investigation needed**:
- Check Python's grouping logic for edge cases
- Verify if Python uses < instead of <=
- Check if there's special handling for license references

### H5: filter_contained_matches() Coverage Issue
**Observation**: 100% coverage AHO matches can be discarded by <100% coverage SEQ matches
**Potential issue**: High-quality matches are being filtered incorrectly
**Investigation needed**:
- Trace filter_contained_matches() for specific failing cases
- Check if Python's filter considers match quality/coverage

### H6: Required Phrase Handling Not Implemented
**Observation**: DIFFERENCES.md notes required phrase handling is not in Rust
**Potential issue**: Matches missing required phrases are not being filtered
**Investigation needed**:
- Check which tests expect required phrase filtering
- Implement required phrase check

### H7: Matcher Order Difference
**Observation**: Aho=2 in Rust vs Aho=1 in Python (matcher order swapped)
**Potential issue**: Different match priority affects which matches are kept
**Investigation needed**:
- Understand why order was swapped
- Check if reverting order fixes issues

## Next Steps

1. Launch parallel investigations for H1, H2, H4 (most likely root causes)
2. Use subagents to trace specific failing test cases
3. When root cause found, verify with Python reference
4. Create implementation plan
5. Implement and validate
