# PLAN-088: kde_licenses_test.txt Investigation

## Status: ANALYSIS COMPLETE

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/kde_licenses_test.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| 15 matches | 14 matches |

**Detailed comparison**:

| Position | Expected | Actual | Issue |
|----------|----------|--------|-------|
| 1 | gpl-2.0 OR gpl-3.0 OR kde-accepted-gpl | gpl-2.0 OR gpl-3.0 OR kde-accepted-gpl | OK |
| 2 | lgpl-2.1 OR lgpl-3.0 OR kde-accepted-lgpl | lgpl-2.1 OR lgpl-3.0 OR kde-accepted-lgpl | OK |
| 3 | gpl-2.0-plus | gpl-2.0-plus | OK |
| 4 | gpl-3.0 | gpl-3.0 | OK |
| 5 | gpl-3.0-plus | gpl-3.0-plus | OK |
| 6 | gpl-3.0-plus | gpl-3.0-plus | OK |
| 7 | gpl-3.0-plus | gpl-3.0-plus | OK |
| 8 | lgpl-2.1 | **lgpl-2.1-plus** | WRONG LICENSE |
| 9 | lgpl-2.1 | bsd-simplified | WRONG LICENSE + MISSING |
| 10 | lgpl-2.1-plus | bsd-simplified AND bsd-new | WRONG LICENSE + MISSING |
| 11 | bsd-simplified AND bsd-new | x11-xconsortium | POSITION SHIFT |
| 12 | x11-xconsortium | x11-xconsortium | OK |
| 13 | x11-xconsortium | x11-xconsortium | OK |
| 14 | mit | mit | OK |
| 15 | mit | mit | OK |

## Root Cause Analysis

### Issue 1: LGPL Tags + Notice (lines 92-109)

**File content**:
```
92:  LGPL 2.1
93:  
94:  LGPL-2.1
95:  
96:  Copyright <year>  <name of author> <e-mail>
97:  
98:  This library is free software; you can redistribute it and/or
99:  modify it under the terms of the GNU Lesser General Public
100: License as published by the Free Software Foundation; either 
101: version 2.1 of the License, or (at your option) any later version.
...
109: License along with this library.  If not, see <http://www.gnu.org/licenses/>.
```

**Rust behavior**:
- Matches `lgpl-2.1-plus` at lines 92-109 using rule `lgpl-2.1-plus_114.RULE`
- This rule includes both the tags ("LGPL 2.1", "LGPL-2.1") AND the notice text
- Score: 94.0, matcher: 3-seq

**Python behavior** (expected):
- Produces **THREE** separate matches for this region:
  1. `lgpl-2.1` - for the "LGPL 2.1" tag (line 92)
  2. `lgpl-2.1` - for the "LGPL-2.1" tag (line 94)
  3. `lgpl-2.1-plus` - for the full notice (lines 96-109)

**Key rule files**:
- `lgpl-2.1-plus_114.RULE`: Matches entire block (tags + notice), expression: `lgpl-2.1-plus`
- `lgpl-2.1_115.RULE`: Tag rule matching "License: LGPL 2.1", expression: `lgpl-2.1`
- `lgpl-2.1_121.RULE`: Reference rule matching "LGPL-2.1", expression: `lgpl-2.1`

### Issue 2: BSD Tags (lines 111-113)

**File content**:
```
111: BSD
112: 
113: BSD-2-Clause
```

**Rust behavior**:
- Matches `bsd-simplified` at lines 111-113 using rule `bsd-simplified_89.RULE`
- This rule matches "BSD BSD-2-Clause" as a license reference
- Score: 100.0, matcher: 2-aho

**Python behavior** (expected):
- Does NOT produce a separate match for the BSD tags
- Only matches `bsd-simplified AND bsd-new` at lines 117-140

**Key rule file**:
- `bsd-simplified_89.RULE`: Matches "BSD BSD-2-Clause", expression: `bsd-simplified`, is_license_reference: yes

## Technical Analysis

### Problem 1: Rule Match Aggregation

The `lgpl-2.1-plus_114.RULE` is a comprehensive rule that matches:
```
LGPL 2.1

LGPL-2.1

This library is free software; you can redistribute it and/or
modify it under the terms of the GNU Lesser General Public
License as published by the Free Software Foundation; either 
version 2.1 of the License, or (at your option) any later version.
...
```

This rule has `minimum_coverage: 50`, meaning it only needs to match 50% of the rule text. However, Rust's seq matcher is matching the entire block as a single `lgpl-2.1-plus` match, while Python appears to:

1. Match the tags separately with `lgpl-2.1` rules
2. Match the notice separately with `lgpl-2.1-plus` rules
3. Keep all three matches in the final output

### Problem 2: Tag vs Notice Distinguishing

The issue is that Rust is preferring the larger, more comprehensive rule (`lgpl-2.1-plus_114.RULE`) over matching the tags separately. This could be due to:

1. **Match filtering**: The tag matches might be getting filtered as "contained" by the larger match
2. **Match scoring**: The larger match might have higher priority/score
3. **Rule precedence**: The seq matcher might prefer larger rules

### Hypothesis

Python likely has special handling for `is_license_tag` and `is_license_notice` rules where:
1. Tag matches are NOT filtered even when contained by a larger notice match
2. The filtering logic considers the rule type when deciding what to keep

## Files to Investigate

1. **`src/license_detection/seq_match.rs`** - How seq matching handles partial coverage rules
2. **`src/license_detection/match_refine.rs`** - `filter_contained_matches()` should preserve tag matches
3. **`src/license_detection/detection.rs`** - Match grouping and output generation

## Recommended Fix Approach

1. **Check Python's filtering logic**: Python's `filter_contained_matches()` likely has special handling for tag matches that prevents them from being filtered when contained by notice matches

2. **Review rule type handling**: The Rust implementation needs to consider `is_license_tag` vs `is_license_notice` when filtering matches

3. **Consider match independence**: Tag matches and notice matches may need to be treated as independent detections even when they overlap spatially

## Next Steps

1. Study Python's `filter_contained_matches()` to understand tag preservation logic
2. Check if `restore_non_overlapping()` in Python handles tag/notice relationships
3. Determine if Rust's seq matcher needs adjustment for partial coverage rules
