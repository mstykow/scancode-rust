# Hypothesis List for Failing Golden Tests

## Progress
- Started: 96 failing tests
- After MAX_DIST fix: 90 failing tests  
- After minimum_containment fix: 82 failing tests (14 tests fixed)
- After escape sequence preprocessing fix: 74 failing tests (22 tests fixed)
- After MAX_DETECTION_SIZE increase to 1MB: 70 failing tests (26 tests fixed)
- After binary strings extraction: 64 failing tests (32 tests fixed, 33% improvement)

## Current Status: 64 failing tests

### Pattern Analysis of Failures
- **Missing Detections**: ~18 failures (Rust finds fewer matches than Python)
- **Wrong License Expression**: ~18 failures (correct position, wrong identifier)
- **GPL Version Resolution**: ~8 failures (wrong "or later" handling)
- **Unknown License Handling**: ~5 failures (wrong unknown type)
- **Missing License Rules**: ~3 failures (no rule in SPDX data)
- **Extra Detections**: ~4 failures

## Fixed Hypotheses

### H2: Content Size Limit - **FIXED**
- **Root cause**: 100KB limit truncated large files
- **Fix**: Increased MAX_DETECTION_SIZE to 1MB
- **Result**: 4 tests fixed

### H3: CC-BY-SA vs CC-BY-NC-SA - **FIXED**
- **Root cause**: NC-SA got higher candidate_resemblance
- **Fix**: Added minimum_containment check to candidate selection
- **Result**: CC-BY-SA tests now pass

### H5-A: options.c Escape Sequences - **FIXED**
- **Root cause**: Golden tests weren't applying escape sequence preprocessing
- **Fix**: Added source file preprocessing in golden_test.rs
- **Result**: Rule gpl-2.0-plus_412.RULE now matches correctly

### H11: Binary File Detection - **FIXED**
- **Root cause**: Rust skipped binaries, Python extracts ASCII strings
- **Fix**: Added extract_ascii_strings() function
- **Result**: 6 tests fixed

## Active Hypotheses

### H1: QueryRun Splitting Disabled - **DEFERRED**
- **Impact**: ~18 tests (missing detections)
- **Root cause**: QueryRun splitting is disabled, causes 37 regressions when enabled
- **Status**: DEFERRED - regressions need to be understood first

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

### H10: Wrong License Expression - Sequence Match Wins Over Exact Match - **NEW**
- **Impact**: ~20 tests (including GFDL-1.1.t3)
- **Root cause IDENTIFIED**: Sequence matches with lower coverage win over exact Aho matches with higher coverage due to containment filtering
- **Example**: `GFDL-1.1.t3`
  - `gfdl-1.1-plus_5.RULE`: 68.6% coverage, lines 2-8, 3-seq match (WRONG)
  - `gfdl-1.1_11.RULE`: 100% coverage, lines 2-4, 2-aho match (CORRECT)
  - The plus_5 match spans more text (tokens 5-77) and "contains" the gfdl_11 match (tokens 5-28)
  - Containment filtering removes gfdl_11 because it's contained in plus_5
  - Python correctly returns gfdl-1.1, not gfdl-1.1-plus
- **Location**: `src/license_detection/match_refine/handle_overlaps.rs:filter_contained_matches()`
- **Fix needed**: 
  1. Consider match coverage when deciding which match to keep in containment
  2. Or prevent sequence matches from winning over more accurate exact matches
  3. Check if Python has different candidate selection that prevents plus_5 from matching
- **Status**: INVESTIGATING

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
