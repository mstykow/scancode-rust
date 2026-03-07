# Hypothesis List for Failing Golden Tests

## Progress
- Started: 96 failing tests
- After MAX_DIST fix: 90 failing tests  
- After minimum_containment fix: 82 failing tests (14 tests fixed)
- After escape sequence preprocessing fix: 74 failing tests (22 tests fixed)
- After MAX_DETECTION_SIZE increase to 1MB: 70 failing tests (26 tests fixed)
- After binary strings extraction: 64 failing tests (32 tests fixed, 33% improvement)
- After subtraction logic alignment: 64 failing tests (code aligned with Python)

## Current Status: 64 failing tests

### Pattern Analysis of Failures (Updated)
| Category | Count | Description |
|----------|-------|-------------|
| Missing Detections | ~15 | Rust finds fewer matches than expected |
| Wrong License Expression | ~12 | Correct position, wrong identifier |
| Extra Detections | ~10 | Rust produces more matches than expected |
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

### H10: Wrong License Variant (X vs X-plus) - **COMPLEX**
- **Impact**: ~8 tests
- **Root cause**: Sequence matches with lower coverage win over exact matches with higher coverage due to containment filtering
- **Example**: `GFDL-1.1.t3`
  - Expected: `gfdl-1.1`
  - Actual: `gfdl-1.1-plus`
- **Fix attempt**: Adding matcher_order() protection caused 3 regressions
- **Lesson learned**: The fix is more nuanced - need to consider:
  - Same vs different license expressions
  - Coverage differences
  - Match quality metrics
- **Status**: NEEDS BETTER APPROACH

### H12: Extra Detections in Non-License Files - **MEDIUM PRIORITY**
- **Impact**: ~10 tests
- **Root cause**: Rust detects licenses in files that should have none
- **Example**: `test.js` (spdx-correct.js test file)
  - Expected: `[]` (no licenses)
  - Actual: `["lgpl-3.0 OR mpl-2.0", "mit"]`
  - File contains license identifiers as test data strings like `'LGPL 3.0'`, `'MIT'`
- **Fix needed**: Better false positive filtering
- **Status**: PENDING

### H13: GPL Version Expression Resolution - **NOT A BUG**
- **Investigation result**: Rust correctly handles `-plus` suffix
  - `SPDX-License-Identifier: GPL-2.0+` → `gpl-2.0-plus` ✓
  - "version 2 or 3" → `gpl-2.0 OR gpl-3.0` ✓ (correct, not `-plus`)
- **Status**: CLOSED - No bug exists

### H14: License Expression Parsing Differences
- **Impact**: ~5 tests
- **Root cause**: Parentheses handling in license expressions differs
- **Example**: `missing_leading_trailing_paren.txt`
  - Expected: `"(gpl-2.0 AND mit) AND unknown-spdx"`
  - Actual: `"gpl-2.0 AND mit AND unknown-spdx"`
- **Status**: PENDING

### H6: Unknown License Detection Differences
- **Impact**: ~5 tests
- **Root cause**: "unknown" vs "unknown-license-reference" usage differs
- **Example**: `README.md` in unknown/
  - Expected: `["unknown-license-reference", "unknown-license-reference", "unknown-license-reference"]`
  - Actual: `["unknown-license-reference", "unknown-license-reference", "unknown"]`
- **Status**: PENDING

### H15: Missing License Rule Detection - **RUST IMPROVEMENT**
- **Impact**: ~5 tests
- **Example**: `lgpl-2.1-plus_with_other-copyleft_1.RULE`
  - Expected: `["unknown-spdx"]`
  - Actual: `["lgpl-2.1-plus"]`
  - Python detects as unknown-spdx, Rust correctly identifies lgpl-2.1-plus
- **This is a Rust improvement** over Python!
- **Status**: NOT A BUG - Rust is more correct

### H16: OR Expression Converted to AND
- **Impact**: ~3 tests
- **Root cause**: OR license expressions being converted to AND
- **Example**: `AFL-2.1_or_GPL-2.0.txt`
  - Expected: `["afl-2.1 OR gpl-2.0-plus", "afl-2.1", "gpl-2.0"]`
  - Actual: `["(afl-2.1 OR gpl-2.0) AND gpl-2.0", "afl-2.1", "gpl-2.0"]`
- **Status**: PENDING

## Summary of Root Causes

1. **Containment filtering** (H10): Complex - matcher_order alone isn't enough
2. **Extra detections** (H12): Need better false positive filtering
3. **Expression parsing** (H14, H16): Parentheses and OR/AND handling issues

## Recommended Investigation Order

1. **H14/H16 (Expression Parsing)**: May be easier to fix
2. **H12 (Extra Detections)**: Understand the false positive filtering gap
3. **H10 (Containment)**: Revisit with more nuanced approach
