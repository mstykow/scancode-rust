# PLAN-064: Wrong Detection Investigation

## Status: PARTIAL FIX APPLIED - Further Investigation Needed

## Problem Statement

Completely different license expression is detected instead of the expected one.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/cpl-1.0_in_html.html`

| Expected | Actual |
|----------|--------|
| `["cpl-1.0"]` | `["unknown-license-reference"]` |

---

## Investigation Results

### Python Reference Behavior

Python correctly detects `cpl-1.0` with:
- **ONE large match**: lines 4-119
- **Score**: 96.65
- **Match coverage**: 96.65%
- **Matcher**: 3-seq
- **Rule**: cpl-1.0.LICENSE

### Rust Behavior

Rust detects incorrectly:
- **Final result**: `unknown-license-reference` at line 119 only
- **Near-dupe candidates**: 10 found, with `cpl-1.0` at top (resemblance=0.996, containment=1.0)
- **Near-dupe matches**: 607 small partial matches created
- **After refinement**: Only 6 matches remain with wrong licenses

### Root Cause: Sequence Matching Creates Many Small Matches Instead of One Large Match

**The Issue**: Rust's sequence matching creates 607 tiny partial matches (coverage 0.3% to 2.0%) instead of one large combined match (96.65% coverage).

**Example of Rust's CPL matches**:
```
cpl-1.0 (lines 4-4, score=0.35, coverage=0.3%)
cpl-1.0 (lines 13-13, score=1.69, coverage=1.7%)
cpl-1.0 (lines 16-16, score=0.12, coverage=0.1%)
... (hundreds more tiny matches)
```

**Python's expected behavior**:
```
cpl-1.0 (lines 4-119, score=96.65, coverage=96.65%)
```

---

## Divergence Point

### File: `src/license_detection/seq_match.rs`

**Function**: `seq_match_with_candidates()` or `match_blocks()`

The sequence matching algorithm is not:
1. Finding the full license text as one contiguous match, OR
2. Properly combining small matches into larger combined matches

### File: `src/license_detection/match_refine.rs`

**Function**: `merge_matches_by_rule()` / `combine_matches()`

The merging logic may not be correctly combining small adjacent matches into one large match.

---

## Key Differences

| Aspect | Python | Rust |
|--------|--------|------|
| Matches created | 1 large match | 607 tiny matches |
| Match coverage | 96.65% | 0.1% - 2.0% each |
| Lines matched | 4-119 | Individual lines |
| Final detection | cpl-1.0 | unknown-license-reference |

---

## Hypothesis

The issue appears to be in how `match_blocks()` finds matches in the presence of HTML markup. The HTML tags break the token sequence, causing the algorithm to find many small matches in HTML tag-free regions instead of finding the full license text.

Python may handle this differently by:
1. Using a different algorithm for finding long matches across HTML boundaries
2. Better merging of adjacent/nearby matches
3. Different handling of `matchables` in the presence of markup

---

## Investigation Test File

Created: `src/license_detection/wrong_detection_investigation_test.rs`

Run with:
```bash
cargo test test_cpl_10_html_full_pipeline_debug --lib -- --nocapture
```

---

## Next Steps

1. **Compare `match_blocks()` implementation**:
   - Check if Python's seq.py differs in how it handles interrupted sequences
   - Check `find_longest_match()` behavior with HTML-marked text

2. **Compare match merging**:
   - Check `merge_matches_by_rule()` logic
   - Check `combine_matches()` conditions for combining adjacent matches

3. **Check `matchables` handling**:
   - HTML tags may be excluded from matchables
   - This could prevent finding longer matches

---

## Key Files to Compare

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `seq_match.rs:find_longest_match()` | `seq.py:match_blocks()` | Finding longest match |
| `seq_match.rs:match_blocks()` | `seq.py:match_blocks()` | All matching blocks |
| `match_refine.rs:merge_matches_by_rule()` | `match.py:merge_matches()` | Combining matches |
| `match_refine.rs:combine_matches()` | `match.py:combine()` | Match combination logic |

---

## Success Criteria

1. ~~Identify why cpl-1.0 is not detected~~ ✓ (ROOT CAUSE: Many small matches instead of one large)
2. ~~Document root cause~~ ✓ (Sequence matching creates fragmented matches)
3. Implement fix in sequence matching or match merging
4. All 6 wrong detection tests pass

---

## Specific Fix

### Root Cause

**Missing `high_postings` filtering** in `seq_match_with_candidates()` and `seq_match()`.

**Python filters** (index.py:850-854):
```python
high_postings = self.high_postings_by_rid[rid]
high_postings = {
    tid: postings for tid, postings in high_postings.items()
        if tid in high_intersection}  # Filter to intersection tokens only
```

**Rust bug** (seq_match.rs lines 666 and 800):
```rust
let high_postings = index.high_postings_by_rid.get(&rid);  // NO FILTERING!
```

### Fix Location

**File**: `src/license_detection/seq_match.rs`

**Lines to change**:
- Line ~666 in `seq_match()`
- Line ~800 in `seq_match_with_candidates()`

### Fix Code

```rust
// Filter high_postings to only include tokens in high_set_intersection
let high_postings: HashMap<u16, Vec<usize>> = index.high_postings_by_rid
    .get(&rid)
    .map(|hp| {
        hp.iter()
            .filter(|(tid, _)| candidate.high_set_intersection.contains(tid))
            .map(|(&tid, postings)| (tid, postings.clone()))
            .collect()
    })
    .unwrap_or_default();
```

### Why This Fixes It

1. `high_set_intersection` contains only high-value tokens present in **both** query and rule
2. Filtering restricts sequence matching anchors to these tokens
3. This produces fewer, larger matches instead of many tiny matches
4. The CPL-1.0 match will then cover lines 4-119 instead of 607 tiny matches

---

## Fix Applied (Commit TBD)

**Result**: Partial improvement.

| Metric | Before Fix | After Fix |
|--------|------------|-----------|
| CPL-1.0 matches | 607 tiny matches | 6 refined matches |
| CPL-1.0 coverage | Fragmented | Still fragmented (10.7% + 11.3%) |
| IBMPL-1.0 coverage | N/A | 43.1% (incorrectly higher than CPL) |
| Golden tests | 3593 passed | 3593 passed (unchanged) |

**Remaining Issue**: The CPL-1.0 license is still being detected incorrectly because:
1. IBMPL-1.0 gets higher coverage (43.1%) than CPL-1.0 (10.7%) 
2. These are very similar licenses and sequence matching prefers IBMPL
3. The matches aren't being properly merged or cross-rule deduplicated

**Further investigation needed**:
- Why does IBMPL get higher coverage when the text says "Common Public License" (CPL)?
- Should matches from different rules for the same region be deduplicated by candidate score?
- Python produces 96.65% CPL coverage - how does it avoid the IBMPL false positive?

---

## Second Root Cause Identified

**In `filter_overlapping_matches`** (match_refine.rs:520-717):

When matches from different licenses overlap, Rust prefers higher `hilen` (high-value token length). Since IBMPL's rule is 2.5x longer, its matches have higher `hilen`, causing CPL to be discarded.

**Evidence**:
```
candidate: cpl-1.0 (resemblance=0.996, containment=1.000)  ← Better candidate
candidate: ibmpl-1.0 (resemblance=0.864, containment=0.993)

Group 1 result:
  ibmpl-1.0 (coverage=43.1%, lines=4-100)  ← Wins due to hilen
  cpl-1.0 (coverage=10.7%, lines=13-47)    ← Truncated
```

**The bug**: Candidate scores (resemblance/containment) are not used when filtering cross-license overlaps.

---

## Proposed Fix (Not Yet Implemented)

**Option 1**: Store candidate scores in `LicenseMatch` and use for cross-license tie-breaking.

**Option 2**: Check for license name in matched text (HTML says "COMMON PUBLIC LICENSE" → CPL).

**Files to change**:
- `src/license_detection/models.rs`: Add `candidate_resemblance` and `candidate_containment` fields
- `src/license_detection/seq_match.rs`: Populate these fields when creating matches
- `src/license_detection/match_refine.rs`: Use these scores in `filter_overlapping_matches`
