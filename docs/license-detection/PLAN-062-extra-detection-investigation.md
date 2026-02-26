# PLAN-062: Extra Detections Investigation

## Status: FIX ATTEMPTED - CAUSED REGRESSION (Reverted)

## Fix Attempt

A fix was attempted to improve containment logic for sparse qspan matches, but it caused major regression:
- lic1: 252 → 211 passed (-41)
- lic2: 807 → 738 passed (-69)

The fix was reverted. A different approach is needed.

## Root Cause Analysis

### Problem Statement

Additional unexpected license expressions are detected. Expected N expressions, got N+1 or more.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/gfdl-1.1-en_gnome_1.RULE`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["gfdl-1.1", "gfdl-1.1-plus"]` | `["gfdl-1.1", "gfdl-1.1-plus", "other-copyleft", "gfdl-1.3-no-invariants-only"]` (4 expressions) |

### The Core Issue: Sparse Qspan in Near-Duplicate Matches

The extra `other-copyleft` detections occur because:

1. **Rust's near-duplicate sequence matching produces sparse qspan matches**:
   - GFDL-1.1 near-dupe match spans tokens 1-74 but only has 29 tokens in its qspan
   - Missing tokens: 5-35 (gap), 41-53 (gap) - these are "unmatched" tokens in the query

2. **The `other-copyleft` Aho matches land in these gaps**:
   - `other-copyleft_fsf_address_3.RULE` matches tokens 41-55
   - `other-copyleft_4.RULE` matches tokens 55-74
   - These are NOT contained in the GFDL match's qspan (tokens 41-53 are missing)

3. **The `qcontains` check fails**:
   - `qcontains` checks if all of other's qspan tokens are in self's qspan
   - GFDL qspan: `[1, 2, 3, 4, 36, 37, 38, 39, 40, 54, ...]` (missing 41-53)
   - other-copyleft tokens 41-55 are NOT all in GFDL's qspan
   - Result: `qcontains(other-copyleft at 41-55) = false`

4. **Python's behavior differs**:
   - Python produces a single `gfdl-1.1` match covering lines 1-608 with 99.03% coverage
   - The match covers the entire document, including the header lines 1-20
   - `other-copyleft` matches at lines 11-15 ARE contained within this larger match

### Why Rust's GFDL Match Starts at Line 20

The investigation revealed:

1. **Near-duplicate matching produces multiple fragmented GFDL matches**:
   - `gfdl-1.1`: lines 1-15, tokens 1-74, len=29 (sparse!)
   - `gfdl-1.1`: lines 20-487, tokens 78-2242, len=2164 (main match)
   - `gfdl-1.1`: lines 488-550, tokens 2242-2611, len=351
   - `gfdl-1.1`: lines 557-607, tokens 2649-2909, len=228

2. **The first match (lines 1-15) has sparse qspan**:
   - Only 29 matched tokens spread across the 74-token span
   - Tokens 41-53 (where other-copyleft matches) are NOT in the qspan

3. **`filter_contained_matches` fails to remove `other-copyleft`**:
   - `other-copyleft` at tokens 41-55 is NOT contained in GFDL's qspan
   - `other-copyleft` at tokens 55-74 IS contained in GFDL's qspan (token 54+ are present)

### Additional Issue: `gfdl-1.3-no-invariants-only` Matches

The `gfdl-1.3-no-invariants-only` matches at lines 596-600 are also unexpected. These come from:
- Aho-Corasick matching at the end of the document
- These survive filtering because the main GFDL match ends around line 563
- The gap at the end allows these smaller matches to survive

---

## Investigation Files

- `src/license_detection/extra_detection_investigation_test.rs` - Investigation tests
- Test output shows exact token ranges and qspan contents

---

## Key Questions Answered

1. **Are extra matches created from the start, or created by incorrect merging?**
   - Created from the start (Aho matches) but survive filtering due to sparse qspan containment

2. **Does Python have additional filtering for GFDL rules?**
   - No, Python produces a larger, more complete GFDL match that contains the other matches

3. **Are `other-copyleft` and `gfdl-1.3-no-invariants-only` matching incorrectly?**
   - They match correctly; the issue is they should be contained within the GFDL match

---

## Potential Fixes

### Option 1: Fix Near-Duplicate Matching to Produce Complete Matches
- Investigate why near-dupe matching produces sparse qspan matches
- Ensure the main GFDL match covers lines 1-608 like Python does
- This would make `other-copyleft` be contained in the larger match

### Option 2: Improve Containment Logic for Sparse Matches
- When a match has sparse qspan, also check containment using start_token/end_token bounds
- Modify `qcontains` to be more lenient with near-dupe matches

### Option 3: Add Post-Processing to Merge Adjacent Same-Expression Matches
- Merge all `gfdl-1.1` matches into a single match spanning lines 1-608
- This would contain the `other-copyleft` matches

---

## Key Files

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/match_refine.rs` | `licensedcode/match.py` | Filter logic |
| `src/license_detection/seq_match.rs` | `licensedcode/index.py` | Match creation |
| `src/license_detection/models.rs` | `licensedcode/match.py` | `qcontains` implementation |

---

## Success Criteria

1. ~~Identify where extra matches are created or not filtered~~ DONE
2. ~~Document root cause~~ DONE
3. Implement fix
4. All 8 extra detection tests pass

---

## Next Steps

1. Compare Python vs Rust near-duplicate matching algorithm
2. Determine why Python produces complete match (lines 1-608) while Rust produces fragmented matches
3. Implement fix to ensure GFDL match covers the entire license text
