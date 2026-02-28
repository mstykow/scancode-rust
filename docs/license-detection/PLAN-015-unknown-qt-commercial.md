# PLAN-015: unknown/qt.commercial.txt

## Status: VALIDATION COMPLETE - FIX CONFIRMED

## Test File
`testdata/license-golden/datadriven/unknown/qt.commercial.txt`

## Issue
**Expected:** `["commercial-license", "commercial-license", "unknown", "unknown-license-reference", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "lgpl-2.0-plus AND gpl-1.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "unknown", "unknown-license-reference", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "commercial-license", "unknown"]`
**Actual:** `["commercial-license", "commercial-license", "unknown", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "lgpl-2.0-plus AND gpl-1.0-plus", "lgpl-2.1 AND gpl-2.0 AND gpl-3.0", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "unknown", "commercial-license", "commercial-license", "unknown"]`

## Differences
- **2 missing `unknown-license-reference` matches** (positions 3 and 8 in expected)
- This causes the entire sequence to shift

## Python Reference Output (with unknown_licenses=true)
```
Total matches: 17
0: commercial-license | lines 1-1 | rule=commercial-option_33.RULE | matcher=2-aho
1: commercial-license | lines 3-3 | rule=commercial-option_33.RULE | matcher=2-aho
2: unknown-license-reference | lines 26-26 | rule=license-intro_25.RULE | matcher=2-aho
3: unknown-license-reference | lines 50-50 | rule=unknown-license-reference_341.RULE | matcher=2-aho
4: lgpl-2.1 AND gpl-2.0 AND gpl-3.0 | lines 175-177 | matcher=2-aho
5: lgpl-2.0-plus AND gpl-1.0-plus | lines 179-179 | matcher=2-aho
6: lgpl-2.1 AND gpl-2.0 AND gpl-3.0 | lines 181-187 | matcher=2-aho
7: unknown | lines 189-207 | matcher=6-unknown
8: unknown-license-reference | lines 197-197 | rule=unknown-license-reference_351.RULE | matcher=2-aho
9-16: commercial-license matches
```

Key observation: ULR at line 197 COEXISTS with unknown match at lines 189-207!

## Rust Debug Output

### Phase 1 Matches
- 69 AHO matches found
- `unknown-license-reference` matches at lines 1, 3, 26, 50, 175, 179, 186, 197, 296-297

### After split_weak_matches
- **All 4 `unknown-license-reference` matches are put in WEAK bucket** (because `has_unknown()` returns true)
- Good matches: 8
- Weak matches: 4

### Unknown Matches Created
- 5 unknown matches created at lines 3-175, 188-234, 236-325, 333-373, 384-403

### After Final refine_matches
- ULR matches at lines 26, 50, 197 are CONTAINED by unknown matches and DISCARDED

## Root Cause Analysis

### Python's qspan for Unknown Matches is a Disjoint Union

In Python (`match_unknown.py:151-152`):
```python
qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
qspan = Span().union(*qspans)
```

The `qspan` is a **union of disjoint spans** representing only the regions where ngrams matched:
```
Unknown at 189-207: qspan=Span(2054, 2059)|Span(2062, 2069)|Span(2071, 2078)|...
```

When checking `qcontains`, the ULR match at line 197 (qspan=2142-2144) falls in a **gap** between these disjoint spans, so:
```python
unknown_189.qcontains(ulr_197)  # Returns False!
```

### Python's Span.__contains__ Implementation

From `spans.py:177-210`:
```python
def __contains__(self, other):
    if isinstance(other, Span):
        return self._set.issuperset(other._set)  # Check ALL positions
```

Python uses an `intbitset` internally. When checking `other.qspan in self.qspan`, it verifies that **every position** in `other.qspan` exists in `self.qspan._set`.

### Rust's qcontains Uses Simple Token Range

In Rust (`unknown_match.rs:330-331`):
```rust
qspan_positions: None,
```

Rust creates unknown matches without `qspan_positions`, so `qcontains` falls back to checking `start_token <= other.start_token && end_token >= other.end_token`, which covers the entire region:
```rust
// Rust: 188 <= 197 && 234 >= 197  => True (contained!)
```

This causes ULR matches to be incorrectly filtered as "contained" by unknown matches.

### Split Weak Matches Issue

Additionally, `split_weak_matches` puts `unknown-license-reference` matches in the weak bucket because `has_unknown()` returns `true` for these rules. This is **correct behavior** matching Python.

However, the final `refine_matches` call in Rust incorrectly filters these weak matches because the unknown matches incorrectly "contain" them.

---

## Validation Results

### 1. Python's qspan Creation (VALIDATED)

**File:** `reference/scancode-toolkit/src/licensedcode/match_unknown.py:143-152`

```python
matched_ngrams = get_matched_ngrams(...)  # Returns (qstart, qend) tuples
qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
qspan = Span().union(*qspans)  # Creates DISJOINT union
```

**Key insight:** Each ngram match is a 6-token span. The union creates disjoint spans where ngrams matched, leaving **gaps** where no ngram matched.

### 2. Python's qcontains Implementation (VALIDATED)

**File:** `reference/scancode-toolkit/src/licensedcode/match.py:444-448`

```python
def qcontains(self, other):
    return other.qspan in self.qspan  # Uses Span.__contains__
```

**File:** `reference/scancode-toolkit/src/licensedcode/spans.py:177-210`

```python
def __contains__(self, other):
    if isinstance(other, Span):
        return self._set.issuperset(other._set)  # All positions must exist
```

**Key insight:** For an unknown match to "contain" a ULR match, the ULR's qspan positions must ALL exist in the unknown match's qspan positions.

### 3. Rust's Current Implementation (VALIDATED)

**File:** `src/license_detection/unknown_match.rs:224-248`

```rust
fn match_ngrams_in_region(...) -> usize {
    // Currently only COUNTS matches, doesn't track positions
    for _ in automaton.find_iter(&region_bytes) {
        match_count += 1;
    }
    match_count
}
```

**File:** `src/license_detection/unknown_match.rs:261-337`

```rust
fn create_unknown_match(..., ngram_count: usize) -> Option<LicenseMatch> {
    // ...
    LicenseMatch {
        // ...
        qspan_positions: None,  // <-- PROBLEM: Not tracking matched positions
        // ...
    }
}
```

**File:** `src/license_detection/models.rs:551-558`

```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    // ...
    // Fallback when qspan_positions is None:
    self.start_token <= other.start_token && self.end_token >= other.end_token
}
```

**Key insight:** Rust discards the ngram match positions, then falls back to simple range containment.

### 4. Relationship to PLAN-014 (free-unknown issue)

**PLAN-014** is a **different issue** related to:
- License-intro filtering behavior
- `free-unknown` rule expression handling
- Unknown detection for undetected regions at file beginning/end

**PLAN-015** (this plan) is specifically about:
- `qspan_positions` not being tracked for unknown matches
- Incorrect containment detection during `refine_matches`

These are **independent issues** that can be fixed separately.

---

## Specific Code Changes Needed

### Change 1: Modify `match_ngrams_in_region` to return positions

**File:** `src/license_detection/unknown_match.rs:224-248`

**Current:**
```rust
fn match_ngrams_in_region(...) -> usize
```

**Required:**
```rust
fn match_ngrams_in_region(
    tokens: &[u16],
    start: usize,
    end: usize,
    automaton: &AhoCorasick,
) -> Vec<(usize, usize)>  // Return (qstart, qend) tuples
```

Must add offset calculation matching Python's `get_matched_ngrams()`:
```python
# Python: match_unknown.py:257-260
offset = unknown_ngram_length - 1  # = 5
for qend, _ in automaton.iter(qtokens):
    qend = qbegin + qend
    qstart = qend - offset
    yield qstart, qend
```

### Change 2: Modify `create_unknown_match` to accept and use positions

**File:** `src/license_detection/unknown_match.rs:261-337`

**Current:**
```rust
fn create_unknown_match(
    index: &LicenseIndex,
    query: &Query,
    start: usize,
    end: usize,
    ngram_count: usize,  // Just a count
) -> Option<LicenseMatch>
```

**Required:**
```rust
fn create_unknown_match(
    index: &LicenseIndex,
    query: &Query,
    start: usize,
    end: usize,
    ngram_positions: Vec<(usize, usize)>,  // Actual positions
) -> Option<LicenseMatch> {
    // Build qspan_positions as union of all matched positions
    let mut qspan_positions: Vec<usize> = Vec::new();
    for (ng_start, ng_end) in &ngram_positions {
        qspan_positions.extend(*ng_start..*ng_end);
    }
    qspan_positions.sort();
    qspan_positions.dedup();
    
    // Validate: len(qspan) >= UNKNOWN_NGRAM_LENGTH * 4 and len(hispan) >= 5
    // ...
    
    LicenseMatch {
        // ...
        qspan_positions: Some(qspan_positions),
        // ...
    }
}
```

### Change 3: Update threshold check

**File:** `src/license_detection/unknown_match.rs:270-287`

Python checks `len(qspan) < unknown_ngram_length * 4` at line 220:
```python
if len(qspan) < unknown_ngram_length * 4 or len(hispan) < 5:
    return
```

Rust currently checks `region_length < UNKNOWN_NGRAM_LENGTH * 4`, which is the entire region. Should check against `qspan_positions.len()` instead.

---

## Edge Cases to Consider

1. **Empty qspan:** If no ngrams match, the function should return None (already handled by returning 0 matches).

2. **Sparse ngram coverage:** The qspan may have significant gaps. This is expected and necessary for correct behavior.

3. **Overlapping ngrams:** Multiple ngrams may overlap. The `dedup()` handles this correctly.

4. **Region boundary calculation:** Python uses `qend` from automaton iteration, Rust must ensure same calculation.

5. **Performance:** Building `qspan_positions` as a Vec<usize> is less efficient than Python's `intbitset`, but the `qcontains` check already handles this correctly via HashSet conversion.

---

## Risk Analysis

**Impact:** Medium - Affects license detection accuracy for files with both unknown license regions and `unknown-license-reference` matches.

**Complexity:** Medium - Requires tracking disjoint spans for unknown matches, similar to Python's `Span.union()` approach.

**Testing:** The existing golden test for qt.commercial.txt will verify the fix once implemented.

## Success Criteria
- [x] Investigation test file created
- [x] Python reference output documented with line numbers
- [x] Rust debug output shows which matches are missing
- [x] Exact divergence location identified
- [x] Root cause documented
- [x] Fix proposed
- [x] Validation completed
- [x] Specific code changes identified
- [x] Edge cases documented
- [ ] Fix implemented
- [ ] Golden test passes
