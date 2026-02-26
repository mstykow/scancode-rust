# PLAN-083: gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt Investigation

## Status: ROOT CAUSE IDENTIFIED

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
lines=14-25: lgpl-2.1-plus (rule=lgpl-2.1-plus_419.RULE)  <-- SHOULD NOT EXIST
lines=22-26: lgpl-2.1-plus (rule=lgpl-2.1-plus_24.RULE)   <-- CORRECT
lines=33-37: lgpl-2.1-plus AND free-unknown (rule=lgpl-2.1-plus_and_free-unknown_1.RULE)
lines=39-53: mit-modern (rule=mit-modern_3.RULE)
lines=57-61: gpl-2.0-plus (rule=gpl-2.0-plus_71.RULE)
lines=65-69: gpl-2.0-plus (rule=gpl-2.0-plus_71.RULE)
lines=71-74: lgpl-2.1 AND gpl-2.0 AND gpl-3.0 (rule=lgpl-2.1_and_gpl-2.0_and_gpl-3.0.RULE)
```

## Root Cause

**Rust's Aho-Corasick matcher is NOT finding `lgpl-2.1-plus_24.RULE` at lines 13-17 (qspan 72-119), while Python does.**

### Detailed Analysis

1. **Python Aho finds** `lgpl-2.1-plus_24.RULE` at:
   - qspan 72-119 → lines 13-17 (query run 1)
   - qspan 135-182 → lines 22-26 (query run 2)

2. **Rust Aho finds**:
   - `lgpl-2.1-plus_108.RULE` at lines 13-13 (qspan 72-75) - a tiny 3-token match
   - `lgpl-2.1-plus_419.RULE` at lines 14-25 (a DIFFERENT rule)
   - `lgpl-2.1-plus_24.RULE` at lines 22-26 - CORRECT

3. **Python containment filtering** correctly filters out `lgpl-2.1-plus_108.RULE` because it's contained within `lgpl-2.1-plus_24.RULE`.

4. **Rust containment filtering** cannot filter `lgpl-2.1-plus_108.RULE` because the containing match (`lgpl-2.1-plus_24.RULE` at lines 13-17) was never found.

### Rule Details

- `lgpl-2.1-plus_24.RULE`: 48 tokens, starts with `[2432, 2403, 4401, 4373, 6614, ...]`
- `lgpl-2.1-plus_419.RULE`: 70 tokens, starts with `[6614, 6615, 5332, 5120, 6225, ...]`
- `lgpl-2.1-plus_108.RULE`: 3 tokens, matches "License: LGPL-2.1+"

### Why Rust doesn't find lgpl-2.1-plus_24 at lines 13-17

**Hypothesis**: The pattern `lgpl-2.1-plus_24.RULE` (48 tokens starting with `[2432, 2403, 4401, 4373, 6614, ...]`) is not being matched by the Aho-Corasick automaton.

Possible causes:
1. **Pattern not in automaton**: The pattern might be missing from `rules_automaton_patterns`
2. **Pattern ID mismatch**: The `pattern_id_to_rid` mapping might be wrong
3. **Tokenization difference**: The query tokens might not match the rule tokens

## Files to Fix

1. `src/license_detection/aho_match.rs` - Aho-Corasick matching
2. `src/license_detection/index/builder.rs` - Automaton building

## Next Steps

See **[PLAN-083-investigation-steps.md](./PLAN-083-investigation-steps.md)** for detailed investigation plan.

### Quick Investigation Summary

1. **Phase 1**: Verify pattern is in automaton (it is - same pattern works at lines 22-26)
2. **Phase 2**: Compare tokenization at lines 13-17 vs 22-26
3. **Phase 3**: Debug Aho-Corasick matching at specific byte positions
4. **Phase 4**: Compare with Python implementation
5. **Phase 5**: Identify and fix the root cause

### Key Insight

The pattern IS in the automaton because Rust finds `lgpl-2.1-plus_24.RULE` at lines 22-26. The issue is specific to the match at lines 13-17. This suggests either:
- Tokenization difference at that location
- Matchables check rejection
- Aho-Corasick configuration issue with overlapping matches

## Test Command

```bash
cargo test --lib test_plan_083 -- --nocapture
```
