# Fix Plan: QueryRun Regression Analysis and Resolution

## Current Status
- **Baseline** (QueryRun disabled): 96 failing tests
- **Current** (QueryRun enabled): 122 failing tests
- **Regression**: +26 additional failures
- **Performance**: ✅ All tests complete in reasonable time
- **Functionality**: ⚠️ Some tests regressed

## Root Cause Categories

### Category A: CC License Misidentification (CRITICAL)
**Files**: CC-BY-SA-1.0.t1, CC-BY-SA-2.0.t1, CC-BY-SA-2.5.t1, CC-BY-3.0.t1
**Expected**: `cc-by-sa-1.0`, `cc-by-sa-2.0`, etc.
**Actual**: `cc-by-nc-sa-1.0`, `cc-by-nc-sa-2.0`, etc. (wrong variant!)

**Root Cause Hypothesis**: QueryRun splitting may be separating license clauses, causing partial matches that match NC variants instead of base licenses.

**Priority**: HIGH - These are serious regressions affecting license identification accuracy.

### Category B: Missing Detections
**Files**: ibmpl-1.0_1.txt, ietf_2.txt, flex-readme.txt
**Pattern**: Rust finds FEWER matches than expected

**Root Cause Hypothesis**: 
- Files with leading blank lines (ibmpl-1.0_1.txt has 10 blank lines)
- QueryRun splitting creates empty runs that get filtered
- License text ends up in a run that doesn't meet matching thresholds

**Priority**: MEDIUM - Need to verify against Python behavior

### Category C: Extra Detections
**Files**: options.c, Autoconf-exception.m4, argparse.c
**Pattern**: Rust finds MORE matches than expected (extra warranty-disclaimer, gpl-1.0-plus, unknown)

**Root Cause Hypothesis**:
- QueryRun boundaries allow low-quality matches to pass
- Whole-file filtering is lost
- Need better filtering within query runs

**Priority**: MEDIUM - May indicate filtering issues

### Category D: Expression Rendering
**Files**: complex3.java
**Issue**: Extra parentheses in expressions
**Status**: Already documented in DIFFERENCES.md #8
**Priority**: LOW - Cosmetic issue

## Investigation Plan

### Step 1: Verify CC License Issue Against Python
**Action**: 
1. Run Python ScanCode on CC-BY-SA-1.0.t1, CC-BY-SA-2.0.t1
2. Compare exact output with Rust
3. Determine if this is:
   - Bug in Rust QueryRun implementation
   - Correct behavior change (Python also returns NC variant)
   - Edge case needing special handling

**Time**: 15 minutes
**Deliverable**: Python vs Rust comparison for CC licenses

### Step 2: Analyze QueryRun Boundaries for CC Files
**Action**:
1. Add debug output to show query runs created for CC-BY-SA-1.0.t1
2. Check if splits happen mid-license-text
3. Identify which run matches the NC variant
4. Understand why NC variant is chosen over correct variant

**Time**: 30 minutes
**Deliverable**: Debug trace showing query run boundaries and match selection

### Step 3: Test Leading Blank Lines Handling
**Action**:
1. Test ibmpl-1.0_1.txt specifically
2. Check if leading blank lines create empty first run
3. Verify Python behavior on same file
4. Fix if needed

**Time**: 20 minutes
**Deliverable**: Understanding of leading/trailing blank line handling

### Step 4: Fix Identified Issues
**Based on investigation results**:
- If CC issue is bug: Fix QueryRun boundary logic
- If missing detections are bug: Fix empty run handling
- If extra detections are bug: Improve filtering

**Time**: 1-2 hours
**Deliverable**: Code fixes with tests

### Step 5: Validate Against Python
**Action**:
1. Run Python on all previously failing tests
2. Run Rust on same tests
3. Verify outputs match (or document intentional differences)

**Time**: 30 minutes
**Deliverable**: Confirmation that Rust matches Python behavior

## Success Criteria
- CC licenses detected correctly (no NC variant misidentification)
- No missing detections compared to Python
- Test count ≤ 96 (baseline) or documented why higher is correct
- All performance improvements maintained
- Code matches Python reference behavior

## Decision Point
If investigation shows QueryRun splitting is fundamentally flawed in its current implementation, we may need to:
1. Roll back to disabled state
2. Implement more carefully with Python parity testing
3. Address each edge case before enabling

However, if issues are minor and fixable, we should fix them to achieve feature parity with Python.
