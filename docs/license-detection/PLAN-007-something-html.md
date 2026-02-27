# PLAN-007: should_detect_something.html

## Status: FOUND DIVERGENCE - Extra match at line 205

## Test File
`testdata/license-golden/datadriven/lic4/should_detect_something.html`

## Issue Summary

**Golden test compares match expressions (not detection expressions).**

Expected (from YAML): `["sun-sissl-1.1", "mit", "sun-sissl-1.1", "sun-sissl-1.1", "apache-2.0"]` (5 matches)

Actual (from Rust): 6 matches with extra `sun-sissl-1.1` at line 205

## Root Cause

### Python Output (4 detections, 5 matches):
| Detection | License | Lines | Matches |
|-----------|---------|-------|---------|
| 1 | sun-sissl-1.1 | 7-7 | 1 match: line 7 |
| 2 | mit | 30-30 | 1 match: line 30 |
| 3 | sun-sissl-1.1 | 195-494 | 2 matches: lines 195-494, 207 |
| 4 | apache-2.0 | 528-530 | 1 match: lines 528-530 |

### Rust Output (4 detections, 6 matches):
| Detection | License | Lines | Matches |
|-----------|---------|-------|---------|
| 1 | sun-sissl-1.1 | 7-7 | 1 match: line 7 |
| 2 | mit | 30-30 | 1 match: line 30 |
| 3 | sun-sissl-1.1 | 195-494 | **3 matches: lines 195-494, 205, 207** ← EXTRA at 205 |
| 4 | apache-2.0 | 528-530 | 1 match: lines 528-530 |

## Divergence Point

**Line 205 contains:** `<P><FONT COLOR="#cc6600"><B>Sun Industry Standards Source License - Version 1.1</B></FONT><BR>`

**Rule `sun-sissl-1.1_4.RULE`** is a license reference rule that matches:
- Text: `Sun Industry Standards Source License Version 1 1`
- `is_license_reference: yes`
- `relevance: 100`

### Why Python Doesn't Match at Line 205

**Hypothesis:** Python's Aho-Corasick matcher likely filters out matches that are:
1. Inside a larger match region (lines 195-494 covers line 205)
2. Overlapping with another match

However, Rust's Aho-Corasick finds this match and after `merge_overlapping_matches`:
- The match at line 7 is kept (separate detection)
- The match at line 205 is merged INTO the larger match at lines 195-494

**Key insight:** The issue is that Rust's `merge_overlapping_matches` doesn't properly filter matches that are fully contained within other matches of the SAME license expression.

## Investigation Evidence

From `test_plan_007_refine_filters`:
```
After merge: 1816 matches
  sun-sissl-1.1: 44 matches
    lines 195-207, tokens 806-834, rule=sun-sissl-1.1.RULE  <-- contains line 205
    lines 205-205, tokens 815-823, rule=sun-sissl-1.1_4.RULE  <-- EXTRA
```

The match at line 205 (tokens 815-823) is FULLY CONTAINED within the match at lines 195-207 (tokens 806-834).

## Next Steps to Fix

1. Check Python's `merge_overlapping_matches` behavior for same-expression matches
2. Possibly need to filter matches that are contained within other matches of the same license expression
3. The filter should happen AFTER merge, not before

## Failing Tests Created

1. `test_plan_007_line_205_match_should_not_exist` - Documents the bug
2. `test_plan_007_sun_sissl_match_count_matches_python` - Documents expected match count

## Files Created
- `src/license_detection/investigation/something_html_test.rs` - Complete investigation suite
