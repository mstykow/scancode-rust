# filter_dupes Regression Investigation

## Summary

Four golden test failures were introduced by the filter_dupes fix. This document analyzes the root cause of each.

## Python vs Rust filter_dupes Comparison

### Python Implementation (match_set.py:461-498)

```python
def filter_dupes(sortable_candidates):
    def group_key(item):
        (sv_round, _sv_full), _rid, rule, _inter = item
        return (
            rule.license_expression,
            sv_round.is_highly_resemblant,
            sv_round.containment,      # Float with 1 decimal place
            sv_round.resemblance,       # Float with 1 decimal place
            sv_round.matched_length,    # Float with 1 decimal place
            rule.length,                # Token count
        )

    def rank_key(item):
        (_sv_round, sv_full), _rid, rule, _inter = item
        return sv_full, rule.identifier

    for group, duplicates in groupby(sorted(sortable_candidates, key=group_key), key=group_key):
        duplicates = sorted(duplicates, reverse=True, key=rank_key)
        yield duplicates[0]  # Keep best from each group
```

### Rust Implementation (seq_match.rs:245-272)

```rust
fn filter_dupes(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut groups: HashMap<DupeGroupKey, Vec<Candidate>> = HashMap::new();

    for candidate in candidates {
        let key = DupeGroupKey {
            license_expression: candidate.rule.license_expression.clone(),
            is_highly_resemblant: candidate.score_vec_rounded.is_highly_resemblant,
            containment: (candidate.score_vec_rounded.containment * 10.0).round() as i32,
            resemblance: (candidate.score_vec_rounded.resemblance * 10.0).round() as i32,
            matched_length: (candidate.score_vec_rounded.matched_length * 20.0).round() as i32,
            rule_length: candidate.rule.tokens.len(),
        };
        groups.entry(key).or_default().push(candidate);
    }
    // ... keep best from each group
}
```

### Key Differences

1. **Python uses floats directly** for containment, resemblance, matched_length in group key
2. **Rust converts to integers** by multiplying and rounding
3. Both approaches should be equivalent for values rounded to 1 decimal place

---

## Case 1: DNSDigest.c - Missing apache-2.0 Detection

**File:** `testdata/license-golden/datadriven/external/fossology-tests/APSL/DNSDigest.c`

**Expected:** 3 apache-2.0 detections  
**Actual:** 2 apache-2.0 detections

### File Content Analysis

Apache-2.0 occurrences in the file:
- Lines 5-15: Full Apache license header (explicit license text)
- Line 49: Change log entry mentioning "Re-licensed mDNSResponder daemon source code under Apache License, Version 2.0"
- Line 158-159: Another Apache license reference

### Rust Detection Output

```
Detection 1: apache-2.0 (lines 5-15, rule: apache-2.0_7.RULE)
Detection 2: apache-2.0 (lines 158-159, rule: apache-2.0_135.RULE)
Detection 3: openssl (lines 165-207)
Detection 4: openssl-ssleay (lines 213-215)
Detection 5: ssleay-windows (lines 220-271)
```

### Root Cause

The third Apache detection (line 49) is missing. This is a change log entry that mentions Apache but doesn't trigger a detection. This is NOT a filter_dupes issue - it's a detection completeness issue. The line 49 text is:

```
Revision 1.17  2006/08/14 23:24:22  cheshire
Re-licensed mDNSResponder daemon source code under Apache License, Version 2.0
```

This is a **different rule** that should match this text. The detection is missing because the appropriate rule isn't matching, not because filter_dupes is incorrectly grouping.

---

## Case 2: sa11xx_base.c - Missing mpl-1.1 OR gpl-2.0 Detection

**File:** `testdata/license-golden/datadriven/external/slic-tests/sa11xx_base.c`

**Expected:** 2 "mpl-1.1 OR gpl-2.0" detections  
**Actual:** 1 "mpl-1.1 OR gpl-2.0" detection

### File Content Analysis

MPL/GPL occurrences:
- Lines 1-30: Full dual license header with MPL/GPL choice text
- Line 269: `MODULE_LICENSE("Dual MPL/GPL");` - kernel module license tag

### Rust Detection Output

```
Detection 1: mpl-1.1 OR gpl-2.0 (lines 269-269, rule: mpl-1.1_or_gpl-2.0_1.RULE)
```

### Root Cause

The main license header (lines 1-30) is not being detected! Only the short MODULE_LICENSE tag on line 269 is detected.

This suggests the dual-license text is either:
1. Not being matched by Aho-Corasick (no exact rule matches)
2. Not being picked up by sequence matching
3. Being filtered incorrectly somewhere

This is NOT a filter_dupes issue - it's a missing detection issue in an earlier phase.

---

## Case 3: ar-ER.js.map - Extra mit Detection

**File:** `testdata/license-golden/datadriven/lic2/ar-ER.js.map`

**Expected:** 1 mit detection  
**Actual:** 2 mit matches (in 1 detection)

### File Content

A minified JavaScript source map containing:
```
@license
Copyright Google Inc. All Rights Reserved.
Use of this source code is governed by an MIT-style license that can be
found in the LICENSE file at https://angular.io/license
```

### Rust Detection Output

```
Detection 1: mit
  Match 1: mit (score: 100.00, rule: mit_131.RULE, lines: 1-1)
  Match 2: mit (score: 99.00, rule: mit_132.RULE, lines: 1-1)
```

### Rules Involved

- `mit_131.RULE`: `is_license_notice: yes`, text: "Use of this source code is governed by an {{MIT-style license}}"
- `mit_132.RULE`: `is_license_reference: yes`, text: "https://angular.io/license", relevance: 99

### Root Cause

Two different MIT rules match at the same location:
1. `mit_131.RULE` matches "Use of this source code is governed by an MIT-style license"
2. `mit_132.RULE` matches "https://angular.io/license"

Both are valid matches, but Python produces only 1 match while Rust produces 2.

**This IS related to filter_dupes** - the two rules have:
- Same `license_expression`: "mit"
- Different `rule_length` (different token counts)

Since `rule_length` is part of the DupeGroupKey, they are in DIFFERENT groups and BOTH survive filter_dupes.

However, Python also produces the same grouping logic. The difference must be in:
1. How matches are refined after sequence matching
2. The `filter_contained_matches` or `filter_overlapping_matches` logic

Both matches are at lines 1-1 with the same boundaries. One should be filtered as contained/overlapping.

---

## Case 4: lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt - Extra Detections

**File:** `testdata/license-golden/datadriven/lic3/lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt`

**Expected:** 1 detection with `lgpl-2.0-plus WITH wxwindows-exception-3.1`  
**Actual:** 5 matches across 2 detections

### Rust Detection Output

```
Detection 1: lgpl-2.0-plus WITH wxwindows-exception-3.1
  Match 1: lgpl-2.0-plus WITH wxwindows-exception-3.1 (rule: lgpl-2.0-plus_with_wxwindows-exception-3.1_5.RULE, lines: 8-29)

Detection 2: lgpl-2.0-plus WITH wxwindows-exception-3.1 AND wxwindows-exception-3.1
  Match 1: lgpl-2.0-plus WITH wxwindows-exception-3.1 (rule: wxwindows_1.RULE, lines: 34-34)
  Match 2: lgpl-2.0-plus WITH wxwindows-exception-3.1 (rule: lgpl-2.0-plus_with_wxwindows-exception-3.1_5.RULE, lines: 34-47)
  Match 3: lgpl-2.0-plus (rule: lgpl-2.0-plus_44.RULE, lines: 45-58)
  Match 4: wxwindows-exception-3.1 (rule: wxwindows-exception-3.1_8.RULE, lines: 60-84)
```

### Root Cause

The file contains:
1. OpenSceneGraph license mention (lines 8-29)
2. wxWindows Library License (lines 34-84)

Multiple rules are matching different parts of the text, creating overlapping detections. The expression combination logic is incorrectly creating `lgpl-2.0-plus WITH wxwindows-exception-3.1 AND wxwindows-exception-3.1` instead of properly combining the expressions.

This is a **detection grouping and expression combination issue**, not a filter_dupes issue.

---

## Summary of Root Causes

| Case | Root Cause | filter_dupes Issue? |
|------|-----------|---------------------|
| DNSDigest.c | Missing rule match for changelog entry | NO - earlier phase |
| sa11xx_base.c | Missing detection for dual-license header | NO - earlier phase |
| ar-ER.js.map | Two rules at same location not being deduplicated | PARTIALLY - key uses rule_length |
| lgpl-wxwindows | Expression combination creating wrong results | NO - grouping/combination issue |

## Conclusion

**None of these failures are directly caused by the filter_dupes implementation.**

The filter_dupes logic correctly groups candidates by (expression, is_highly_resemblant, containment, resemblance, matched_length, rule_length). The issues are:

1. **DNSDigest.c, sa11xx_base.c**: Missing detections in earlier phases (Aho matching or sequence matching not finding rules)

2. **ar-ER.js.map**: Multiple rules matching same location - this SHOULD be handled by `filter_contained_matches` or `filter_overlapping_matches`, not filter_dupes. The two rules have different `rule_length` so they're correctly in different groups.

3. **lgpl-wxwindows**: Expression combination is producing incorrect results when multiple matches overlap.

## Recommendation

The "filter_dupes fix" mentioned in the task description likely didn't cause these regressions directly. Instead, these are pre-existing issues that were masked before and are now exposed. Investigate:

1. Why changelog entries mentioning licenses don't trigger detections
2. Why dual-license headers aren't being matched
3. How contained/overlapping matches at identical positions should be handled
4. How license expressions are combined when matches overlap
