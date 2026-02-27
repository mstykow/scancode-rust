# PLAN-083: gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt Investigation

## Status: NEEDS DEEPER INVESTIGATION

**Previous hypothesis was WRONG**: The proposed fix (prefer Aho matches by coverage) would break Python parity.

**Validation findings**:
- Python's `filter_overlapping_matches` does NOT consider coverage or matcher type
- Python only uses: qspan.start, hilen, matched_length, matcher_order
- Rust correctly implements Python's overlap filtering logic
- The issue is WHY Python doesn't generate `_419.RULE` as a seq match

**Next investigation needed**:
1. Compare candidate selection between Python and Rust for this query
2. Check if `_419.RULE` is in the top 70 candidates for both
3. Investigate sequence alignment results

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["gpl-3.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus AND free-unknown", "mit-modern", "gpl-2.0-plus", "gpl-2.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0"]` (8) | `["gpl-3.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus AND free-unknown", "mit-modern", "gpl-2.0-plus", "gpl-2.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0"]` (9) |

**Issue**: Extra `lgpl-2.1-plus` detection in Rust output.

## Investigation Summary

### Python Reference Matches

```
lines=5-9: gpl-3.0-plus (rule=gpl-3.0-plus_9.RULE)
lines=13-17: lgpl-2.1-plus (rule=lgpl-2.1-plus_24.RULE)  <-- FOUND BY PYTHON
lines=22-26: lgpl-2.1-plus (rule=lgpl-2.1-plus_24.RULE)
lines=33-37: lgpl-2.1-plus AND free-unknown (rule=lgpl-2.1-plus_and_free-unknown_1.RULE)
lines=39-53: mit-modern (rule=mit-modern_3.RULE) [seq match]
lines=57-61: gpl-2.0-plus (rule=gpl-2.0-plus_71.RULE)
lines=65-69: gpl-2.0-plus (rule=gpl-2.0-plus_71.RULE)
lines=71-74: lgpl-2.1 AND gpl-2.0 AND gpl-3.0 (rule=lgpl-2.1_and_gpl-2.0_and_gpl-3.0.RULE)
```

### Rust Actual Matches

```
lines=5-9: gpl-3.0-plus (rule=gpl-3.0-plus_9.RULE)
lines=13-13: lgpl-2.1-plus (rule=lgpl-2.1-plus_108.RULE)  <-- EXTRA, CONTAINED BY MISSING MATCH
lines=14-25: lgpl-2.1-plus (rule=lgpl-2.1-plus_419.RULE)  <-- SHOULD NOT EXIST (seq match)
lines=22-26: lgpl-2.1-plus (rule=lgpl-2.1-plus_24.RULE)   <-- CORRECT
lines=33-37: lgpl-2.1-plus AND free-unknown (rule=lgpl-2.1-plus_and_free-unknown_1.RULE)
lines=39-53: mit-modern (rule=mit-modern_3.RULE)
lines=57-61: gpl-2.0-plus (rule=gpl-2.0-plus_71.RULE)
lines=65-69: gpl-2.0-plus (rule=gpl-2.0-plus_71.RULE)
lines=71-74: lgpl-2.1 AND gpl-2.0 AND gpl-3.0 (rule=lgpl-2.1_and_gpl-2.0_and_gpl-3.0.RULE)
```

## Root Cause (Updated)

**`filter_overlapping_matches` is incorrectly discarding the Aho-Corasick exact match (`lgpl-2.1-plus_24.RULE`) in favor of a sequence match (`lgpl-2.1-plus_419.RULE`) with lower coverage.**

### Detailed Analysis

1. **Aho-Corasick DOES find `lgpl-2.1-plus_24.RULE` at lines 13-17**:
   - Tokens 72-120, 48 matched tokens, 100% coverage
   - Matcher: `2-aho`

2. **Sequence matching produces `lgpl-2.1-plus_419.RULE` at lines 14-25**:
   - Tokens 78-162, 84 matched tokens, 70% coverage
   - Matcher: `3-seq`

3. **Both matches overlap** (tokens 78-120 overlap region)

4. **`filter_contained_matches` keeps both** (neither is fully contained in the other)

5. **`filter_overlapping_matches` discards the Aho match** because:
   - The seq match is longer (84 tokens vs 48 tokens)
   - The overlap ratio logic prefers longer matches
   - BUT this is WRONG because the Aho match has 100% coverage vs 70% for the seq match

### The Problem in `filter_overlapping_matches`

The function at `src/license_detection/match_refine.rs:549-797` sorts matches by:
1. `qstart` (ascending)
2. `hilen` (descending - high-value tokens)
3. `matched_length` (descending)
4. `matcher_order` (ascending - hash=1, aho=2, seq=3)

When comparing overlapping matches, the logic considers:
- `current_len >= next_len` (longer match wins)
- `current_hilen >= next_hilen` (more high-value tokens wins)

**The issue**: When comparing `lgpl-2.1-plus_24.RULE` (aho, 48 tokens) with `lgpl-2.1-plus_419.RULE` (seq, 84 tokens):
- The seq match is longer, so it wins
- But the aho match has 100% coverage vs 70% for seq

**The fix should prioritize exact matches (aho) with 100% coverage over sequence matches with partial coverage.**

### Code Location

The issue is in `filter_overlapping_matches` at lines 643-665:

```rust
if extra_large_next && current_len_val >= next_len_val {
    // Discard next if current is longer or equal
    discarded.push(matches.remove(j));
    continue;
}

if extra_large_current && current_len_val <= next_len_val {
    // Discard current if next is longer or equal
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

This logic doesn't consider match coverage. An aho match with 100% coverage should win over a seq match with 70% coverage even if the seq match is longer.

## Files to Fix

1. `src/license_detection/match_refine.rs` - `filter_overlapping_matches` function

## Recommended Fix

Add a check for match coverage when comparing overlapping matches:

1. If one match is an exact match (aho, 100% coverage) and the other is a sequence match (partial coverage), prefer the exact match
2. Consider adding a check for `match_coverage == 100.0` or `matcher == "2-aho"` as a tiebreaker

Example fix approach:

```rust
// Prefer exact matches (aho with 100% coverage) over sequence matches
let current_is_exact = matches[i].matcher == "2-aho" && matches[i].match_coverage >= 99.9;
let next_is_exact = matches[j].matcher == "2-aho" && matches[j].match_coverage >= 99.9;

if current_is_exact && !next_is_exact && next_len_val > current_len_val {
    // Keep the exact match even if the seq match is longer
    discarded.push(matches.remove(j));
    continue;
}
```

## Test Command

```bash
cargo test --lib test_plan_083 -- --nocapture
```

## Verification

After the fix:
1. `lgpl-2.1-plus_24.RULE` at lines 13-17 should be kept (aho match with 100% coverage)
2. `lgpl-2.1-plus_108.RULE` at lines 13-13 should be filtered as contained by `_24`
3. `lgpl-2.1-plus_419.RULE` at lines 14-25 should NOT appear
4. Total match count should be 8 (matching Python)
