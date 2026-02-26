# PLAN-084: gpl-2.0-plus_and_gpl-2.0-plus_and_public-domain.txt Investigation

## Status: FIXED BY PLAN-082

This issue shares the same root cause as PLAN-082: `remove_duplicate_detections()` incorrectly deduplicates file-level detections by identifier.

**Resolution**: Implement PLAN-082 fix. This issue will be automatically resolved.

See `PLAN-082-gpl-2.0-plus-duplicate.txt.md` for the complete implementation plan.

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_public-domain.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["reportbug", "gpl-1.0-plus", "gpl-2.0-plus", "gpl-2.0-plus", "public-domain", "public-domain", "reportbug"]` (7) | `["reportbug", "gpl-1.0-plus", "gpl-2.0-plus", "public-domain", "public-domain"]` (5) |

**Issues**:
1. Missing one `gpl-2.0-plus` detection
2. Missing one `reportbug` detection

## Python Reference Output

```
reportbug: qreg=59-182, lines=12-27, rid=reportbug_1.RULE
gpl-1.0-plus: qreg=206-248, lines=34-39, rid=gpl_77.RULE
gpl-2.0-plus: qreg=259-321, lines=44-50, rid=gpl-2.0-plus_81.RULE
gpl-2.0-plus: qreg=335-397, lines=55-61, rid=gpl-2.0-plus_81.RULE
public-domain: qreg=410-411, lines=66-66, rid=public-domain_77.RULE
public-domain: qreg=416-421, lines=67-67, rid=public-domain_16.RULE
reportbug: qreg=448-571, lines=74-89, rid=reportbug_1.RULE
```

## File Structure Analysis

The file is a Debian copyright file with multiple `Files:` stanzas:

1. **Lines 1-27**: `Files: *` - Contains reportbug license text
2. **Lines 29-39**: `Files: handle_bugscript` - Contains GPL-any reference (detected as gpl-1.0-plus)
3. **Lines 41-50**: `Files: Makefile, */module.mk` - Contains GPL-2+ reference (gpl-2.0-plus)
4. **Lines 52-61**: `Files: test/scaffold.py, test/test_*.py` - Contains GPL-2+ reference (gpl-2.0-plus) - **SAME TEXT AS #3**
5. **Lines 63-68**: `Files: checks/compare_pseudo-pkgs_lists.py` - Contains public-domain text
6. **Lines 70-89**: `Files: reportbug/ui/gtk2_ui.py` - Contains reportbug license text - **SAME TEXT AS #1**

The key insight: Stanzas #3 and #4 contain identical GPL-2+ license text. Stanzas #1 and #6 contain identical reportbug license text. Python correctly detects both instances.

## Match Analysis

### GPL-2.0-plus Matches (Same Rule: gpl-2.0-plus_81.RULE)

| Property | Match 1 | Match 2 |
|----------|---------|---------|
| qspan | 259-321 | 335-397 |
| ispan | 0-62 | 0-62 |
| rule_length | 63 | 63 |
| qdistance_to | 14 | - |
| idistance_to | 0 | - |
| overlap | 0 | - |
| is_after | False | - |

**Key Observation**: Both matches have identical ispan (0-62) but different qspans. This means they match the SAME part of the rule at DIFFERENT positions in the file. This is the case of "same license text appears twice".

### Reportbug Matches (Same Rule: reportbug_1.RULE)

| Property | Match 1 | Match 2 |
|----------|---------|---------|
| qspan | 59-182 | 448-571 |
| ispan | 0-123 | 0-123 |
| rule_length | 124 | 124 |
| qdistance_to | 266 | - |
| idistance_to | 0 | - |

Same pattern: identical ispan, different qspan.

## Merge Logic Analysis

The `merge_overlapping_matches` function (match_refine.rs:196-339) groups matches by rule_identifier and checks several conditions:

1. **Distance check**: `qdistance_to > max_rule_side_dist` (31) → qdistance=14 < 31, so **passes**
2. **Equal match check**: `qspan == qspan && ispan == ispan` → False
3. **Equal ispan with overlap**: `ispan == ispan && overlap > 0` → overlap=0, so **skipped**
4. **Containment check**: `qcontains` → False for both directions
5. **Surround check**: `surround` → False
6. **is_after check**: `m2.is_after(m1)` → False (because istart=0 is NOT >= iend=62)
7. **Overlap in sequence check**: overlap=0, so **skipped**

**Conclusion**: The merge logic should NOT merge these matches. They should remain separate.

## Hypothesis

The matches are being lost in one of these places:

1. **filter_contained_matches** (match_refine.rs:363-419) - Might be incorrectly filtering one match as "contained" in another
2. **filter_overlapping_matches** - Might be incorrectly filtering
3. **Detection grouping** - Might be grouping matches incorrectly

### Suspected Issue: filter_contained_matches

The `filter_contained_matches` function sorts by qstart and then checks if `next.end_token <= current.end_token`. If they have the same end_token but different qstart, the containment check might incorrectly trigger.

**BUT**: The two gpl-2.0-plus matches have different qspans (259-321 vs 335-397), so they should NOT be contained within each other.

### Next Steps

1. Add debug tracing to see where matches are lost
2. Check if the issue is in detection grouping or elsewhere
3. Compare Rust pipeline step-by-step with Python

## Files to Investigate

- `src/license_detection/match_refine.rs` - merge/filter logic
- `src/license_detection/detection.rs` - detection creation and grouping
- `src/license_detection/extra_detection_investigation_test.rs` - pattern for tracing pipeline

## Root Cause Analysis (In Progress)

The golden test compares individual match expressions:
```rust
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

So the issue is that Rust has 5 matches while Python has 7 matches.

### Possible Causes

1. **merge_overlapping_matches** merging duplicates incorrectly
2. **filter_contained_matches** filtering matches incorrectly  
3. **Detection grouping** merging matches from different regions
4. **Pipeline issue** where matches are lost before final output

### Key Insight: Same Rule, Different Positions

Both gpl-2.0-plus matches have:
- Same rule_identifier: `gpl-2.0-plus_81.RULE`
- Same ispan: (0, 62)
- Different qspan: (259-321) vs (335-397)

This means the same license TEXT appears twice in the file, and both should be detected as separate matches.

### Next Step

Run a debug test to trace where matches are lost in the pipeline.
