# Hypothesis List for 74 Failing Golden Tests

## Progress
- Started: 96 failing tests
- After MAX_DIST fix: 90 failing tests  
- After minimum_containment fix: 82 failing tests (14 tests fixed)
- After escape sequence preprocessing fix: 74 failing tests (**22 tests fixed, 22.9% improvement**)

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

#### H5-A: options.c - **FIXED**
- **Root cause**: Golden tests weren't applying escape sequence preprocessing for source files
- **Fix**: Added source file preprocessing in `golden_test.rs:131-134`
- **Result**: Rule gpl-2.0-plus_412.RULE now matches correctly

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

### H9: Sequence Matching Candidate Selection - **INVESTIGATING**
- **Impact**: Unknown number of tests
- **Root cause IDENTIFIED**: `high_set_intersection` check filters out rules that Python finds
- **Example**: `aladdin-md5_and_not_rsa-md5.txt` - Python finds `aladdin-md5.RULE` via seq match, Rust doesn't
- **Location**: `src/license_detection/seq_match/candidates.rs:325-328`
- **Fix needed**: Adjust candidate selection thresholds to match Python
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
