# PLAN-013: unknown/cigna-go-you-mobile-app-eula.txt

## Status: VALIDATION COMPLETE - ROOT CAUSE INCORRECT

## Test File
`testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt`

## Issue
**Expected:** `["proprietary-license", "proprietary-license", "unknown-license-reference", "warranty-disclaimer", "proprietary-license", "warranty-disclaimer", "unknown-license-reference", "unknown"]`
**Actual:** `["proprietary-license", "proprietary-license", "unknown", "warranty-disclaimer", "warranty-disclaimer", "warranty-disclaimer", "unknown-license-reference", "unknown"]`

## Differences
- Position 2: Expected `unknown-license-reference`, Actual `unknown`
- Position 4: Expected `proprietary-license`, Actual `warranty-disclaimer`
- Extra `warranty-disclaimer`, missing `proprietary-license`

## Validation Results

### 1. Rule File Verification
**File exists:** `reference/scancode-toolkit/src/licensedcode/data/rules/unknown-license-reference_298.RULE`

**Rule attributes:**
```yaml
license_expression: unknown-license-reference
is_license_notice: yes
relevance: 100
referenced_filenames:
    - Copyright
```

**Rule text:**
```
This SOFTWARE is licensed under the LICENSE provided in the
../Copyright file. By downloading, installing, copying, or otherwise
using the SOFTWARE, you agree to be bound by the terms of that
LICENSE.
```

### 2. Rule Loading Verification
**Result: RULE IS LOADED INTO INDEX**

Test output confirms:
```
Found rule: unknown-license-reference_298.RULE
  license_expression: unknown-license-reference
  text: This SOFTWARE is licensed under the LICENSE provided in the
../Copyright file. By downloading, installing, copying, or otherwise
using the SOFTWARE, you agree to be bound by the terms of that
LICENSE.
  tokens: [6764, 6375, 5482, 2457, 6767, ...]  (32 tokens)
  is_small: false
  is_tiny: false
  is_license_reference: false  (NOTE: is_license_notice=true from frontmatter)
```

### 3. Rule Counts Comparison
- Python rules: 36,475 total
- Python `unknown-license-reference_*` rules: 452 total
- Rust loads rules from same directory and builds index correctly

### 4. Why Rule is NOT Being Matched

**The proposed root cause was INCORRECT.** The rule IS loaded. The issue is:

1. **AHO Matching Requires Exact Token Sequence**
   - The rule text: "This SOFTWARE is licensed under the LICENSE provided in the ../Copyright file. By downloading, installing, copying, or otherwise using the SOFTWARE..."
   - The input text: "By accessing, downloading, copying or otherwise using the Application, You acknowledge that You have read this Agreement..."
   - These do NOT share an exact token sequence match for AHO detection

2. **Sequence Matching - Rule Not in Top Candidates**
   - Regular candidates: 10 rules (warranty-disclaimer variants dominate)
   - The `unknown-license-reference_298.RULE` does not score high enough to be a candidate
   - This is because the overlap between rule text and input text is minimal

3. **Actual Text Overlap is Small**
   - Rule: "By downloading, installing, copying, or otherwise using the SOFTWARE, you agree to be bound by the terms"
   - Input: "By accessing, downloading, copying or otherwise using the Application, You acknowledge that You have read this Agreement, understand it, and agree to be bound by its terms"
   - Shared: "by", "downloading", "copying", "or", "otherwise", "using", "you", "agree", "to", "be", "bound", "by"
   - Missing: "installing" (rule), "accessing" (input), different structure

## Updated Root Cause Analysis

**The issue is NOT a missing rule. The rule IS loaded but doesn't match due to:**

1. **Text similarity is insufficient for sequence matching** - The candidate scoring doesn't rank this rule high enough

2. **No AHO match** - The input text doesn't contain the exact token sequence from the rule

3. **Python may use different detection** - Need to verify HOW Python detects `unknown-license-reference` in this file

## Proposed Fix (Updated)

1. **Run Python detection on the same file** to see:
   - Which rule identifier is matched for `unknown-license-reference`
   - What matcher is used (aho, hash, seq, etc.)
   - What the actual matched text is

2. **If Python uses a DIFFERENT rule**:
   - Find the correct rule that should match
   - Investigate why that rule isn't matching in Rust

3. **If Python uses the SAME rule**:
   - Investigate Python's candidate scoring
   - May need to adjust Rust's scoring algorithm

## Specific Code Changes Needed

1. **Add Python comparison test** to `unknown_cigna_test.rs`:
   - Run Python detection on the same file
   - Compare matched rule identifiers

2. **Check candidate scoring** in `src/license_detection/seq_match.rs`:
   - Verify if `unknown-license-reference_298.RULE` should be a candidate
   - Check containment/resemblance calculations

## Investigation Test File
Created at `src/license_detection/investigation/unknown_cigna_test.rs`

## Success Criteria
- [x] Investigation test file created
- [x] Python reference output documented
- [x] Rust debug output added for all pipeline stages
- [x] Exact divergence location identified
- [x] Root cause documented
- [x] **VALIDATION: Rule IS loaded (proposed fix was incorrect)**
- [ ] Run Python detection to find correct matching rule
- [ ] Fix proposed and implemented

## Risk Analysis
- **Medium Risk**: Detection algorithm differences rather than missing data
- **Impact**: May require scoring algorithm adjustments
- **Complexity**: Need to understand Python's detection behavior for this specific case
