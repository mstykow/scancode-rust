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

### H5: Extra Matches Being Created
- **Impact**: ~15 tests (Rust produces more matches than expected)
- **Examples**: 
  - options.c: expected 2, got 5 matches
  - BSD-3-Clause_AND_CC0-1.0.txt: expected 2, got 3 matches
- **Status**: PENDING

### H6: Unknown License Detection Differences
- **Impact**: ~5 tests
- **Examples**: 
  - README.md: expected "unknown-license-reference" x3, got "unknown-license-reference" x2 + "unknown"
- **Status**: PENDING

### H7: Match Ordering Differences
- **Impact**: ~5 tests
- **Example**: README.html: expected ["bsd-new", "bsd-simplified"], got ["bsd-simplified", "bsd-new"]
- **Status**: PENDING

### H8: Binary File Detection
- **Impact**: ~2 tests (adj.dat)
- **Root cause**: Binary files should extract text or return empty detections
- **Status**: PENDING

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
