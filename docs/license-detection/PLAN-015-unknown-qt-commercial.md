# PLAN-015: unknown/qt.commercial.txt

## Status: ROOT CAUSE CONFIRMED - AWAITING IMPLEMENTATION

## Summary
Missing `unknown-license-reference` detections because unknown matches lack `qspan_positions`, causing incorrect containment detection in `filter_contained_matches`.

## Test File
`testdata/license-golden/datadriven/unknown/qt.commercial.txt`

## Issue
**Expected:** `["commercial-license", "commercial-license", "unknown", "unknown-license-reference", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "lgpl-2.0-plus AND gpl-1.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "unknown", "unknown-license-reference", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "commercial-license", "unknown"]`
**Actual:** `["commercial-license", "commercial-license", "unknown", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "lgpl-2.0-plus AND gpl-1.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "commercial-license", "unknown"]`

## Differences
- **2 missing `unknown-license-reference` matches** (positions 4 and 9 in expected)
- These ULR matches should coexist with unknown matches but are being filtered

## Python Reference Output (with unknown_licenses=true)
```
Total matches: 20
0: commercial-license | lines 1-1
1: commercial-license | lines 3-3
2: unknown | lines 3-175 | matcher=6-unknown
3: unknown-license-reference | lines 26-26  <-- COEXISTS with unknown at lines 3-175!
4: lgpl-2.1 AND gpl-2.0 AND gpl-3.0 | lines 175-177
5: lgpl-2.0-plus AND gpl-1.0-plus | lines 179-179
6: lgpl-2.1 AND gpl-2.0 AND gpl-3.0 | lines 181-187
7: unknown | lines 189-207 | matcher=6-unknown
8: unknown-license-reference | lines 197-197  <-- COEXISTS with unknown at lines 189-207!
9-19: commercial-license and unknown matches
```

**Key observation:** ULR at line 197 COEXISTS with unknown match at lines 189-207!

---

## Root Cause Analysis

### The Problem

Python's unknown matches have a **disjoint qspan** - only the positions where ngrams matched, not the entire region. When checking containment, a ULR match in a "gap" between ngram matches returns `False` for `qcontains`.

**Python's qspan for unknown match at lines 189-207:**
```python
qspan = Span(2054, 2059) | Span(2062, 2069) | Span(2071, 2078) | ...
# Disjoint spans with GAPS between them
```

**ULR match at line 197:**
```python
qspan = Span(2142, 2144)  # Falls in a GAP, not in unknown's qspan
```

**Python's qcontains check:**
```python
unknown.qcontains(ulr)  # Returns False (ULR not in unknown's disjoint qspan)
```

### Rust's Incorrect Behavior

Rust creates unknown matches with `qspan_positions: None`, so `qcontains` falls back to simple range check:

```rust
// models.rs:558 (fallback when qspan_positions is None)
self.start_token <= other.start_token && self.end_token >= other.end_token
```

**Rust's qcontains check:**
```rust
// unknown at lines 189-207, ULR at line 197
unknown.qcontains(ulr)  // Returns True (197 is in range 189-207) - WRONG!
```

This causes `filter_contained_matches` to incorrectly discard ULR matches.

### Pipeline Flow

```
1. split_weak_matches:
   - ULR matches -> weak bucket (correct - they have "unknown" in expression)
   - Other matches -> good bucket

2. unknown_match:
   - Creates unknown matches for uncovered regions
   - BUT qspan_positions is None!

3. Reinject weak matches:
   - ULR matches added back to all_matches

4. refine_matches -> filter_contained_matches:
   - Unknown matches incorrectly "contain" ULR matches
   - ULR matches discarded!
```

---

## Why Previous Fix Attempt Failed

The previous investigation noted "fix was implemented to track qspan_positions in unknown matches, but golden test count didn't improve (stayed at 149 failing)".

**Analysis:** This may have been due to:
1. The implementation may have had bugs (incorrect position calculation)
2. Or other tests may have regressed (net same count)
3. The fix commit for PLAN-014 (`has_unknown` change) was actually reverted in a follow-up commit

**Note:** PLAN-014's `has_unknown` fix (`== "unknown"` vs `.contains("unknown")`) is a **separate issue** - it affects which matches go to weak bucket, but doesn't fix the containment issue.

---

## Refined Fix Approach

### Option A: Track qspan_positions in Unknown Matches (Original Proposal)

**Pros:** Most accurate, matches Python exactly
**Cons:** More complex, requires tracking ngram positions

**Implementation:**
1. Modify `match_ngrams_in_region` to return `Vec<(usize, usize)>` of ngram positions
2. Build `qspan_positions` as union of all matched positions
3. Set `qspan_positions: Some(positions)` in `create_unknown_match`

### Option B: Exclude Unknown Matches from Containment Filtering

**Pros:** Simpler, surgical fix
**Cons:** May miss some containment cases

**Implementation:**
In `filter_contained_matches`, skip containment check when the "container" is an unknown match and the "contained" is an `unknown-license-reference` match:

```rust
// Don't filter ULR matches as contained by unknown matches
if current.matcher == "5-undetected" && next.license_expression.contains("unknown-license-reference") {
    j += 1;
    continue;
}
```

### Option C: Check qspan_positions Exists Before Containment Check

**Pros:** Fails safe
**Cons:** May allow some false positives through

**Implementation:**
In `qcontains`, return `false` when self has `qspan_positions: None` and other is a "weak" match type:

```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    // Unknown matches without qspan_positions cannot "contain" other matches
    if self.qspan_positions.is_none() && self.matcher == "5-undetected" {
        return false;
    }
    // ... rest of implementation
}
```

---

## Recommended Approach

**Option A** is the correct fix - track `qspan_positions` properly. This matches Python's behavior exactly.

The implementation should:
1. Track matched ngram positions in `match_ngrams_in_region`
2. Build disjoint qspan from these positions
3. Use the disjoint qspan for containment checks

This ensures ULR matches in "gaps" between ngrams are not incorrectly filtered.

---

## Implementation Details

### Change 1: Modify `match_ngrams_in_region`

**File:** `src/license_detection/unknown_match.rs:224-248`

```rust
fn match_ngrams_in_region(
    tokens: &[u16],
    start: usize,
    end: usize,
    automaton: &AhoCorasick,
) -> Vec<(usize, usize)> {
    // Return (qstart, qend) tuples for each ngram match
    let region_tokens = &tokens[start..end];
    let region_bytes: Vec<u8> = region_tokens
        .iter()
        .flat_map(|tid| tid.to_le_bytes())
        .collect();

    let offset = UNKNOWN_NGRAM_LENGTH - 1;  // = 5
    let mut positions = Vec::new();

    for end_in_region in automaton.find_iter(&region_bytes).map(|m| m.end()) {
        let qend = start + end_in_region / 2;  // Convert byte offset to token offset
        let qstart = qend - offset;
        positions.push((qstart, qend));
    }

    positions
}
```

### Change 2: Modify `create_unknown_match`

```rust
fn create_unknown_match(
    index: &LicenseIndex,
    query: &Query,
    start: usize,
    end: usize,
    ngram_positions: Vec<(usize, usize)>,  // Changed from count
) -> Option<LicenseMatch> {
    // Build disjoint qspan_positions
    let mut qspan_positions: Vec<usize> = Vec::new();
    for (ng_start, ng_end) in &ngram_positions {
        qspan_positions.extend(*ng_start..*ng_end);
    }
    qspan_positions.sort();
    qspan_positions.dedup();

    // Python check: len(qspan) >= UNKNOWN_NGRAM_LENGTH * 4
    if qspan_positions.len() < UNKNOWN_NGRAM_LENGTH * 4 {
        return None;
    }

    // ... rest of implementation

    LicenseMatch {
        // ...
        qspan_positions: Some(qspan_positions),
        // ...
    }
}
```

---

## Edge Cases

1. **Empty qspan:** No ngrams match -> return empty positions -> correctly filtered by threshold check
2. **Sparse ngram coverage:** Gaps are expected and necessary for correct behavior
3. **Overlapping ngrams:** `dedup()` handles correctly
4. **Token offset calculation:** Must match Python's `qend - offset` formula

---

## Success Criteria
- [x] Root cause confirmed
- [x] Python behavior documented
- [x] Rust divergence identified
- [x] Fix approach refined
- [ ] Fix implemented
- [ ] Golden test passes
