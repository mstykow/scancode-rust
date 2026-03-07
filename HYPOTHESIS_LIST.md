# Hypothesis List for Failing Golden Tests

## Progress
- Started: 96 failing tests
- After MAX_DIST fix: 90 failing tests  
- After minimum_containment fix: 82 failing tests (14 tests fixed)
- After escape sequence preprocessing fix: 74 failing tests (22 tests fixed)
- After MAX_DETECTION_SIZE increase to 1MB: 70 failing tests (26 tests fixed)
- After binary strings extraction: 64 failing tests (32 tests fixed, 33% improvement)

## Current Status: 64 failing tests

### Pattern Analysis of Failures (Updated)
| Category | Count | Description |
|----------|-------|-------------|
| Missing Detections | ~15 | Rust finds fewer matches than expected |
| Wrong License Expression | ~12 | Correct position, wrong identifier |
| Extra Detections | ~10 | Rust produces more matches than expected |
| GPL Version Resolution | ~8 | Wrong "or later" handling |
| Unknown License Handling | ~5 | "unknown" vs "unknown-license-reference" |
| Match Ordering | ~6 | Order differs from Python |
| Wrong License Type | ~8 | Completely wrong license detected |

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
- **Impact**: ~15 tests (missing detections)
- **Root cause**: QueryRun splitting is disabled, causes 37 regressions when enabled
- **Status**: DEFERRED - regressions need to be understood first
- **Examples**:
  - `net-snmp-license.txt`: Expected 18 matches, got 4
  - `pkg.c`: Expected "gpl-2.0 OR lgpl-2.0", got separate matches

### H10: Wrong License Variant (X vs X-plus) - **HIGH PRIORITY**
- **Impact**: ~8 tests
- **Root cause**: Sequence matches with lower coverage win over exact matches with higher coverage due to containment filtering
- **Example**: `GFDL-1.1.t3`
  - Expected: `gfdl-1.1`
  - Actual: `gfdl-1.1-plus`
  - The `gfdl-1.1-plus_5.RULE` sequence match "contains" the exact `gfdl-1.1_11.RULE` match
  - Containment filtering removes the correct match
- **Location**: `src/license_detection/match_refine/handle_overlaps.rs:filter_contained_matches()`
- **Fix needed**: Consider match coverage/type when filtering contained matches
- **Status**: READY TO FIX

### H12: Extra Detections in Non-License Files - **NEW**
- **Impact**: ~10 tests
- **Root cause**: Rust detects licenses in files that should have none
- **Example**: `test.js` (spdx-correct.js test file)
  - Expected: `[]` (no licenses)
  - Actual: `["lgpl-3.0 OR mpl-2.0", "mit"]`
  - File contains license identifiers as test data strings like `'LGPL 3.0'`, `'MIT'`
  - Python ignores these as they're in string literals, Rust detects them
- **Fix needed**: Either:
  1. Context-aware detection (ignore strings in code)
  2. Better false positive filtering
  3. Minimum match length threshold
- **Status**: INVESTIGATING

### H13: GPL Version Expression Resolution - **NEW**
- **Impact**: ~8 tests
- **Root cause**: Rust produces "X OR Y" instead of "X-plus" for GPL licenses
- **Example**: `gpl-2.0-plus_33.txt`
  - Expected: `["gpl-2.0-plus", "gpl-2.0-plus", "gpl-1.0-plus", "gpl-1.0-plus", "gpl-2.0-plus", "gpl-1.0-plus"]`
  - Actual: `["gpl-2.0-plus", "gpl-2.0-plus", "gpl-2.0 OR gpl-3.0", "gpl-2.0 OR gpl-3.0"]`
- **Root cause hypothesis**:
  - Python has logic to normalize "GPL-2.0 OR GPL-3.0" to "GPL-2.0-plus"
  - Rust may be missing this normalization or applying it inconsistently
- **Status**: PENDING INVESTIGATION

### H14: License Expression Parsing Differences - **NEW**
- **Impact**: ~5 tests
- **Root cause**: Parentheses handling in license expressions differs
- **Example**: `missing_leading_trailing_paren.txt`
  - Expected: `"(gpl-2.0 AND mit) AND unknown-spdx"`
  - Actual: `"gpl-2.0 AND mit AND unknown-spdx"`
- **Fix needed**: Verify expression parsing preserves parentheses structure
- **Status**: PENDING

### H6: Unknown License Detection Differences
- **Impact**: ~5 tests
- **Root cause**: "unknown" vs "unknown-license-reference" usage differs
- **Example**: `README.md` in unknown/
  - Expected: `["unknown-license-reference", "unknown-license-reference", "unknown-license-reference"]`
  - Actual: `["unknown-license-reference", "unknown-license-reference", "unknown"]`
- **Fix needed**: Unify unknown license type handling
- **Status**: PENDING

### H15: Missing License Rule Detection - **NEW**
- **Impact**: ~5 tests
- **Root cause**: Specific license rules aren't being matched
- **Example**: `lgpl-2.1-plus_with_other-copyleft_1.RULE`
  - File content: `SPDX-License-Identifier: LGPL-2.1+ The author added a static linking exception...`
  - Expected: `["unknown-spdx"]`
  - Actual: `["lgpl-2.1-plus"]`
  - Python detects as unknown-spdx, Rust correctly identifies lgpl-2.1-plus
  - This may be a **Rust improvement** over Python!
- **Status**: NEEDS VERIFICATION - May not be a bug

### H16: OR Expression Converted to AND - **NEW**
- **Impact**: ~3 tests
- **Root cause**: OR license expressions being converted to AND
- **Example**: `AFL-2.1_or_GPL-2.0.txt`
  - Expected: `["afl-2.1 OR gpl-2.0-plus", "afl-2.1", "gpl-2.0"]`
  - Actual: `["(afl-2.1 OR gpl-2.0) AND gpl-2.0", "afl-2.1", "gpl-2.0"]`
- **Fix needed**: Investigate why OR becomes AND in first match
- **Status**: PENDING

## Summary of Root Causes

1. **GPL version normalization** (H13): Need to normalize "GPL-2.0 OR GPL-3.0" to "GPL-2.0-plus"
2. **Containment filtering** (H10): Sequence matches incorrectly win over exact matches
3. **Extra detections** (H12): Need context-aware detection or better filtering
4. **Expression parsing** (H14, H16): Parentheses and OR/AND handling issues

## Recommended Fix Order

1. **H10 (Wrong License Variant)**: High impact, clear fix location
2. **H13 (GPL Version Resolution)**: High impact, likely normalization issue
3. **H12 (Extra Detections)**: Medium impact, may need architectural changes
4. **H14/H16 (Expression Parsing)**: Low impact, edge cases

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
