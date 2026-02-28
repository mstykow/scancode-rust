# PLAN-013: unknown/cigna-go-you-mobile-app-eula.txt

## Status: ✅ PARTIALLY FIXED

## Test File
`testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt`

## Issue
**Expected:** `["proprietary-license", "proprietary-license", "unknown-license-reference", "warranty-disclaimer", "proprietary-license", "warranty-disclaimer", "unknown-license-reference", "unknown"]`
**Actual:** `["proprietary-license", "proprietary-license", "unknown", "warranty-disclaimer", "warranty-disclaimer", "warranty-disclaimer", "unknown-license-reference", "unknown"]`

## Differences
- Position 2: Expected `unknown-license-reference`, Actual `unknown`
- Position 4: Expected `proprietary-license`, Actual `warranty-disclaimer`
- Extra `warranty-disclaimer`, missing `proprietary-license`

## Root Cause Analysis

### 1. The Rule IS Loaded and IS Approx-Matchable

**Rule: `unknown-license-reference_118.RULE`**
- License expression: `unknown-license-reference`
- Text: "By downloading, copying, installing or using the software you agree to this license. If you do not agree to this license, do not download, install, copy or use the software."
- Token count: 30 (not small, not tiny)
- `is_license_reference: true` but `is_small: false`, so it IS approx-matchable
- **Confirmed in `approx_matchable_rids`**

### 2. The Rule PASSES All Threshold Checks

Step 1 of candidate selection checks:
- ✅ Intersection size: 18 tokens (min_matched_length_unique: 4) - PASS
- ✅ High token intersection: 4 tokens (min_high_matched_length_unique: 3) - PASS

### 3. The Rule FAILS to Make Top Candidates Due to Low Resemblance

**The Problem: Candidate ranking truncates at `top_n * 10 = 100` candidates**

Rule 118's scores:
- `resemblance`: 0.035857 (0.036)
- `amplified_resemblance`: 0.001286 (0.001) 
- `containment`: 0.90

Competing rules (from debug output):
- `warranty-disclaimer_103.RULE`: resemblance=0.022, matched_length=248
- `warranty-disclaimer_104.RULE`: resemblance=0.009, matched_length=157
- `license-intro_40.RULE`: resemblance=0.000, matched_length=10

**The sorting is by:**
1. `is_highly_resemblant` (false for all these)
2. `containment` (0.90 for rule 118 vs 1.0 for others)
3. `resemblance` (0.001 for rule 118 vs higher for others)
4. `matched_length` (18 for rule 118 vs higher for others)

Rule 118 ranks very low due to its low resemblance and is truncated by the `top_n * 10` limit.

### 4. Why Python Detects This

Python uses `top=50` by default in `compute_candidates()`, which means `top * 10 = 500` candidates pass through step 1. 

Rust uses `MAX_NEAR_DUPE_CANDIDATES = 10` for regular candidates, meaning only 100 candidates pass through.

**This is the critical difference!** Rule 118's low resemblance ranks it outside the top 100 in Rust but inside the top 500 in Python.

## Proposed Fix

### Option A: Increase Candidate Limit (Recommended)

Change `MAX_NEAR_DUPE_CANDIDATES` to match Python's default:

```rust
// In src/license_detection/seq_match.rs
pub const MAX_NEAR_DUPE_CANDIDATES: usize = 50;  // Match Python's default
```

This would keep `50 * 10 = 500` candidates, matching Python's behavior.

### Option B: Adjust Sorting Weights

Weight `matched_length` higher relative to `resemblance` so rules with good token overlap but low resemblance (like rule 118) rank higher.

### Option C: Special Handling for License Reference Rules

Rules with `is_license_reference: true` that have high containment (>0.8) could be given priority in candidate selection.

## Implementation Plan

1. **Change `MAX_NEAR_DUPE_CANDIDATES` from 10 to 50** in `src/license_detection/seq_match.rs`
2. **Verify** by running `test_plan_013_rust_detection`
3. **Check for regressions** by running the full golden test suite

## Risk Analysis

- **Low Risk**: Increasing candidate limit only affects ranking, not correctness
- **Performance Impact**: Slight increase in memory/time for processing more candidates
- **Benefit**: Better parity with Python's detection behavior

## Investigation Files Created

- `src/license_detection/investigation/rule_118_test.rs` - Detailed candidate selection analysis

## Success Criteria
- [x] Investigation test file created
- [x] Python reference output documented
- [x] Rust debug output added for all pipeline stages
- [x] Exact divergence location identified: Candidate truncation at step 1
- [x] Root cause documented: Candidate limit too low
- [ ] Fix proposed and implemented
- [ ] Golden test passes
