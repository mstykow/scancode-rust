# Hypothesis List for 82 Failing Golden Tests

## Progress
- Started: 96 failing tests
- After MAX_DIST fix: 90 failing tests  
- After minimum_containment fix: 82 failing tests (**14 tests fixed, 14.6% improvement**)

## Active Hypotheses

### H1: QueryRun Splitting Disabled
- **Impact**: ~25 tests (missing detections)
- **Root cause**: QueryRun splitting is disabled, files with 4+ blank lines between licenses don't get separate matches
- **Status**: PENDING

### H2: Multi-Occurrence Deduplication
- **Impact**: ~25 tests (missing detections of same license at different locations)
- **Root cause IDENTIFIED**: Containment filtering removes 100% Aho matches when covered by larger seq match
- **Example**: dojo.js expected 4 matches, got 3
- **Status**: INVESTIGATED - Need to understand Python's detection grouping

### H3: CC-BY-SA vs CC-BY-NC-SA - **FIXED**
- **Root cause**: NC-SA got higher candidate_resemblance than SA despite matching less text
- **Fix**: Added minimum_containment check to candidate selection
- **Result**: CC-BY-SA tests now pass

### H4: German Text ß Character
- **Impact**: ~15 tests
- **Status**: PYTHON PARITY ACHIEVED (Python has same issue)

### H5: Extra Matches Being Created - **INVESTIGATED**
- **Impact**: ~15 tests (Rust produces more matches than expected)
- **Status**: ROOT CAUSES IDENTIFIED

#### H5-A: options.c - Missing rule gpl-2.0-plus_412.RULE
- **Root cause**: Rule 412 has `minimum_coverage: 70` and `ignorable_urls`
- **Python matches**: Lines 679-681 with rule gpl-2.0-plus_412.RULE (minimum_coverage 70)
- **Rust matches**: 3 separate small rules (gpl-2.0-plus_225, gpl-2.0-plus_780, gpl-1.0-plus_155)
- **Issue**: Rust's sequence matcher doesn't find rule 412 as a candidate, falls back to smaller rules
- **Fix needed**: Improve sequence matching to handle ignorable_urls

#### H5-B: BSD-3-Clause_AND_CC0-1.0.txt - Sequence matching misses rule
- **Root cause**: Rust uses Aho-Corasick for exact matches, missing approximate match Python finds
- **Python**: Uses bsd-new_303.RULE via sequence matching (3-seq)
- **Rust**: Uses 2 separate Aho matches (bsd-new_302, bsd-new_304)
- **Fix needed**: Improve sequence candidate selection

#### H5-C: warranty-disclaimer over-matching
- **Root cause**: Multiple small warranty-disclaimer rules match where larger rules should
- **Fix needed**: Better overlap filtering or minimum_coverage enforcement

### H6: Unknown License Detection Differences
- **Impact**: ~5 tests
- **Status**: PENDING

### H7: Match Ordering Differences
- **Impact**: ~5 tests
- **Status**: PENDING

### H8: Binary File Detection
- **Impact**: ~2 tests
- **Status**: PENDING

## Summary of Extra Matches Root Causes

1. **Sequence matching misses rules that Python finds**:
   - Rust falls back to smaller Aho-Corasick matches
   - Results in more matches instead of one combined match

2. **ignorable_urls not properly handled**:
   - Rules with ignorable_urls should match with/without URL punctuation
   - Rust doesn't generate URL variants for matching

3. **minimum_coverage filtering in sequence matching**:
   - Rules with higher minimum_coverage may be filtered out incorrectly
   - Need to verify candidate selection respects minimum_coverage

## Investigation Protocol
1. Pick top 3 hypotheses
2. Launch parallel subagent investigations
3. Each investigation:
   - Analyze specific failing test cases
   - Compare Python vs Rust behavior
   - Identify root cause and fix location
   - Recommend implementation approach
4. Verify findings with Python reference
5. Create detailed implementation plan
6. Implement and test
7. Commit if golden tests improve
