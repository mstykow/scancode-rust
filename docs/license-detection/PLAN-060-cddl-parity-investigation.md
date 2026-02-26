# PLAN-060: CDDL Rule Selection Parity Investigation

## Status: FIX ATTEMPTED (Partial Improvement)

## Fix Applied

**Commit**: `d8de83f0`

1. Enhanced `surround()` in `models.rs` to check both qspan AND ispan bounds
2. Added `qoverlap > 0` check in `match_refine.rs` before merging surrounded matches

**Result**: Marginal improvement (lic1: 251→252 passed). The fix doesn't fully resolve the issue because the root cause is deeper - Python's sequence matching doesn't create CDDL 1.1 fragmented matches in the first place.

## Remaining Issue

Rust's `seq_match` creates 6 fragmented CDDL 1.1 matches that get merged and compete with CDDL 1.0. Python produces different intermediate results.

## Problem Statement

CDDL 1.0 test files are incorrectly matching CDDL 1.1 rules. Rust diverges from Python in CDDL rule selection.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["cddl-1.0 OR gpl-2.0"]` | `["cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0"]` |

---

## Investigation Findings

### Divergence Point

**File**: `src/license_detection/match_refine.rs`
**Function**: `filter_overlapping_matches()`
**Line**: 583-586

### Root Cause Analysis

#### Phase 2: Near-Duplicate Detection

Both Python and Rust create similar candidates during near-duplicate detection:
- CDDL 1.0 (rid=9698): resemblance=1.000
- CDDL 1.1 (rid=9757): resemblance=0.800

#### Sequence Matching Creates Fragmented Matches

Rust creates **6 fragmented CDDL 1.1 matches** during sequence matching:
```
Match 22: cddl-1.1 ... coverage=3.4%, start=0, end=10
Match 23: cddl-1.1 ... coverage=20.0%, start=18, end=77
Match 24: cddl-1.1 ... coverage=11.9%, start=81, end=116
Match 25: cddl-1.1 ... coverage=1.0%, start=118, end=121
Match 26: cddl-1.1 ... coverage=7.5%, start=121, end=143
Match 27: cddl-1.1 ... coverage=42.7%, start=144, end=270
```

#### Merge Step Creates Competing Match

During `merge_overlapping_matches()` (match_refine.rs:161-304), CDDL 1.1's fragmented matches are merged via the `surround` condition (line 251-266):

```rust
if current.surround(&next) {
    let combined = combine_matches(&current, &next);
    if combined.qspan().len() == combined.ispan().len() {
        rule_matches[i] = combined;
        rule_matches.remove(j);
        continue;
    }
}
```

This creates a merged CDDL 1.1 match:
- `start=0, end=270, matched_length=255, hilen=52`

#### Filter Overlapping Discards CDDL 1.0

At `filter_overlapping_matches()` line 583-586:

```
CDDL 1.1 (current): qstart=0, end=270, matched_length=255, hilen=52
CDDL 1.0 (next): qstart=18, end=270, matched_length=252, hilen=51

overlap_ratio_to_next (CDDL 1.0): 0.972 >= 0.90 (extra_large_next=true)
current_len(255) >= next_len(252): true

Line 583 condition: extra_large_next && current_len >= next_len = true
Result: CDDL 1.0 is DISCARDED
```

### Key Intermediate Data

| Step | CDDL 1.0 | CDDL 1.1 |
|------|----------|----------|
| Phase 2 matches | 2 matches | 6 fragmented matches |
| After merge | 2 matches (unchanged) | 1 merged match |
| After filter_contained | 1 match (start=18) | 1 match (start=0) |
| After filter_overlapping | **DISCARDED** | **KEPT** |

### Why Python Gets It Right

Python also has the same `filter_overlapping_matches` logic, but the critical difference is:

1. **Python does NOT create the same fragmented CDDL 1.1 matches** - the seq_match algorithm produces different intermediate results
2. The CDDL 1.0 match has `match_coverage=100.0%` in Python's final result, suggesting it wins before any overlap filtering

---

## Proposed Fix

The divergence appears to be in the sequence matching algorithm, not the filtering logic. The issue is that CDDL 1.1's fragmented matches are being created and merged incorrectly.

### Investigation Required

1. **Check Python's seq_match output** - does Python create CDDL 1.1 matches for this file?
2. **Compare the `surround` merge condition** - the Rust implementation might be too aggressive in merging
3. **Check ispan alignment** - the condition `combined.qspan().len() == combined.ispan().len()` might be incorrectly satisfied

### Specific Fix Location

**File**: `src/license_detection/match_refine.rs`
**Function**: `merge_overlapping_matches()`
**Lines**: 251-266 (surround merge condition)

The `surround` condition is merging matches that shouldn't be merged. The CDDL 1.1 fragmented matches have gaps (positions not matched), but the merge is combining them into a single match that competes with CDDL 1.0.

---

## Key Files to Investigate

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/match_refine.rs:251-266` | `licensedcode/match.py:998-1010` | Surround merge condition |
| `src/license_detection/seq_match.rs` | `licensedcode/match_seq.py` | Sequence matching |
| `src/license_detection/models.rs:512-516` | `licensedcode/match.py:621-630` | `surround()` function |

---

## Success Criteria

1. Identify exact divergence point between Rust and Python
2. Document root cause
3. Implement fix that achieves parity
4. All 8 CDDL tests pass
