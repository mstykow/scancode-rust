# PLAN-012: unknown/README.md

## Status: VALIDATED - FIX NEEDS IMPROVEMENT

## Test File
`testdata/license-golden/datadriven/unknown/README.md`

## Issue
**Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown-license-reference"]`
**Actual:** `["unknown"]`

## Python Reference Output
```
Total matches: 3
  unknown-license-reference | lines 4-4 | rule=unknown-license-reference_344.RULE | matcher=2-aho
  unknown-license-reference | lines 6-6 | rule=unknown-license-reference_341.RULE | matcher=2-aho
  unknown-license-reference | lines 44-44 | rule=unknown-license-reference_348.RULE | matcher=2-aho
```

## Rust Pipeline Debug Output

### PHASE 1 ALL MATCHES
```
Count: 3
  unknown-license-reference at lines 4-4 tokens 29-37
  unknown-license-reference at lines 6-6 tokens 48-51
  unknown-license-reference at lines 44-44 tokens 652-657
```

### AFTER split_weak_matches
```
Good matches: 0
Weak matches: 3
  WEAK: unknown-license-reference at lines 6-6 has_unknown=true
  WEAK: unknown-license-reference at lines 4-4 has_unknown=true
  WEAK: unknown-license-reference at lines 44-44 has_unknown=true
```

### UNKNOWN MATCHES
```
Count: 1
  UNKNOWN: unknown at lines 1-51
```

### AFTER FINAL refine_matches
```
Count: 1
  unknown at lines 1-51
```

## Divergence Location

**Pipeline Stage:** `unknown_match()` function combined with `filter_overlapping_matches()`

**Where:** `src/license_detection/unknown_match.rs` and `src/license_detection/match_refine.rs`

## Root Cause Analysis

### The Problem

1. **Aho matching finds 3 correct matches** (`unknown-license-reference` at lines 4, 6, 44)

2. **`split_weak_matches()` correctly classifies all 3 as "weak"** because they contain "unknown" in their license_expression:
   - `unknown-license-reference`.contains("unknown") = true
   - This matches Python behavior per `match.py:1740-1765`

3. **`unknown_match()` incorrectly creates a match covering the entire document** (lines 1-51):
   - Since `good_matches = []` (all were weak), `covered_positions = {}`
   - `find_unmatched_regions()` returns `[(0, 718)]` (entire document)
   - The document passes the hispan >= 5 threshold (has license-like text)
   - Result: creates one huge `unknown` match

4. **`filter_overlapping_matches()` discards the 3 small matches**:
   - The large `unknown` match (lines 1-51) overlaps 100% with the small matches
   - Per overlap logic, the larger match wins (higher hilen/matched_length)
   - The 3 `unknown-license-reference` matches are discarded

### Why Python Behaves Differently

Python's `match_unknowns()` in `match_unknown.py` has additional thresholds that prevent creating an unknown match when the document already has license-like matches (even if they are "weak"):

1. Python uses `matched_ngrams` from an ngram automaton - only creates unknown if enough ngrams match in the UNMATCHED region
2. Python's `hispan` check considers the unmatched region specifically, not the whole document
3. Most importantly: Python does NOT create an `unknown` match when there are already license matches (even weak ones) covering the license-like content

### The Bug

The Rust `unknown_match()` function doesn't properly account for weak matches when determining what regions are "unmatched". It should:

1. **Consider weak matches as "already matched"** for purposes of unknown detection
2. **OR** skip unknown detection entirely when all matches are weak (contain unknown)

The current logic:
```rust
let covered_positions = compute_covered_positions(query, known_matches);
// known_matches = good_matches, which is empty when all are weak
```

Should be:
```rust
let covered_positions = compute_covered_positions(query, &all_matches_including_weak);
// OR: skip unknown_match if there are any license matches at all
```

## Proposed Fix

### Option A: Include weak matches in covered positions (Preferred)

In `src/license_detection/mod.rs`, change the unknown_match call:

```rust
// Before (current buggy code):
let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
let mut all_matches = good_matches;
if unknown_licenses {
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    // ...
}

// After (fixed):
let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
let mut all_matches = good_matches.clone(); // Clone for unknown_match
all_matches.extend(&weak_matches); // Include weak matches in coverage check
if unknown_licenses {
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    // ...
}
all_matches.clear();
all_matches.extend(good_matches);
all_matches.extend(unknown_matches);
all_matches.extend(weak_matches);
```

Wait, this is wrong. Looking at Python more carefully:

```python
if unknown_licenses:
    good_matches, weak_matches = match.split_weak_matches(matches)
    # ...collect good_qspans from good_matches...
    unmatched_qspan = original_qspan.difference(good_qspan)
    # ...run unknown detection on unmatched regions...
    unknown_matches = ... 
    matches.extend(unknown_matches)
    matches.extend(weak_matches)  # re-inject weak matches AFTER unknown
```

Python runs unknown detection on regions NOT covered by GOOD matches, then adds both unknown_matches and weak_matches to the result. The key is that Python's `match_unknowns()` doesn't create an unknown match for this file.

Looking at Python's `match_unknowns()` thresholds:
- It checks `len(hispan) < 5` but hispan is computed from the unmatched region
- For this file, the unmatched region is the whole document, which should pass...

Actually, I think the real difference is in how Python's `filter_overlapping_matches` handles this. Let me check if Python has special handling for unknown matches...

Looking at Python's `filter_overlapping_matches`, the sorter is:
```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
```

And `matcher_order` for unknown is 6 (highest), while aho is 1 (lowest). So when sorting, unknown matches come LAST.

In the overlap logic, if two matches have the same start position but different matcher_order, the one with lower matcher_order (aho=1) comes first in the sorted list, and the one with higher matcher_order (unknown=6) comes last.

But in Rust, we're sorting by `matcher_order()` ascending, so aho (order 1) comes BEFORE unknown (order 5). This should be the same...

Actually wait - looking at the Rust code more carefully:

```rust
// Rust sorter in filter_overlapping_matches
matches.sort_by(|a, b| {
    a.qstart()
        .cmp(&b.qstart())
        .then_with(|| b.hilen.cmp(&a.hilen))  // Higher hilen first
        .then_with(|| b.matched_length.cmp(&a.matched_length))  // Longer match first
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))  // Lower order first
});
```

vs Python:
```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
```

These are equivalent. Both sort by: start ASC, hilen DESC, len DESC, matcher_order ASC.

The issue is that the large `unknown` match has much higher `hilen` and `matched_length` than the 3 small `unknown-license-reference` matches. So the large unknown match wins in overlap filtering.

Actually, the REAL issue is that Python doesn't create an unknown match at all for this file. Let me verify by running Python with trace...

Actually, I should just implement the fix: the `unknown` match should NOT be created when there are already license matches covering the license-like content in the document.

The simplest fix is to pass ALL matches (including weak) to `unknown_match()` so it can correctly compute covered positions.

## Success Criteria
- [x] Investigation test file created
- [x] Python reference output documented
- [x] Rust debug output added for all pipeline stages
- [x] Exact divergence location identified
- [x] Root cause documented in this plan
- [x] Fix proposed

## Proposed Fix

In `src/license_detection/mod.rs`, pass `merged_matches` (which includes the weak matches) to `unknown_match()` instead of just `good_matches`:

```rust
// Step 2: Split weak from good - Python: index.py:1083
let (good_matches, weak_matches) = split_weak_matches(&merged_matches);

// Step 3: Unknown detection on uncovered regions
// FIX: Pass all matches (good + weak) to unknown_match so it correctly
// computes covered positions. This prevents creating an "unknown" match
// that overlaps with existing license matches (even weak ones).
let mut all_matches_for_unknown = good_matches.clone();
all_matches_for_unknown.extend(weak_matches.clone());

let mut all_matches = good_matches;
if unknown_licenses {
    let unknown_matches = unknown_match(&self.index, &query, &all_matches_for_unknown);
    // ...
}
```

## Risk Analysis
- **Low risk**: This change makes `unknown_match()` aware of weak matches when computing covered positions
- **Backward compatible**: Weak matches are still reinjected after unknown detection
- **Matches Python behavior**: Python implicitly handles this by not creating spurious unknown matches

---

## Validation Results

### Status: NEEDS IMPROVEMENT

### 1. Python Reference Behavior Analysis

**Python code at `index.py:1082-1118`:**

```python
if unknown_licenses:
    good_matches, weak_matches = match.split_weak_matches(matches)
    # ...collect good_qspans from good_matches...
    good_qspans = (mtch.qspan for mtch in good_matches)
    good_qspan = Span().union(*good_qspans)
    unmatched_qspan = original_qspan.difference(good_qspan)
    # ...run unknown detection on unmatched regions...
```

**Key Finding:** Python passes ONLY `good_matches` to compute `good_qspan`, NOT `weak_matches`. The proposed fix to include weak matches in `unknown_match()` is **incorrect** - it would deviate from Python's explicit behavior.

### 2. Root Cause Re-Analysis

Python does NOT create an `unknown` match for this file because `match_unknowns()` returns `None`. Looking at `match_unknown.py:143-223`:

1. Python computes `matched_ngrams` from an ngram automaton on the **unmatched region**
2. If the unmatched region doesn't have enough matching ngrams, it returns `None`
3. Thresholds at line 220: `if len(qspan) < unknown_ngram_length * 4 or len(hispan) < 5: return`

**The REAL issue:** The Rust implementation's `match_ngrams_in_region()` and `create_unknown_match()` functions are not correctly matching Python's ngram matching logic. The Rust code may be finding ngram matches where Python doesn't, OR the hispan calculation differs.

### 3. Fix Assessment

**The proposed fix is WRONG:**
- Passing weak matches to `unknown_match()` would mark positions as "covered" that Python explicitly does NOT mark as covered
- This could cause false negatives in other cases where unknown detection should run

**Correct approach:**
1. Debug WHY Rust's `unknown_match()` creates a match when Python's doesn't
2. Check `match_ngrams_in_region()` - is it returning matches correctly?
3. Check `create_unknown_match()` - is hispan computed correctly?
4. Compare the ngram automaton building between Python and Rust

### 4. Potential Regressions

The proposed fix could cause:
- Unknown matches to NOT be created in documents where they SHOULD be created
- E.g., a document with only weak `unknown-license-reference` matches at the edges, but real unknown license text in the middle

### 5. Improved Fix

The fix should be in `unknown_match.rs`, NOT in `mod.rs`:

1. Verify `create_unknown_match()` thresholds match Python:
   - `len(qspan) < unknown_ngram_length * 4` → `region_length < UNKNOWN_NGRAM_LENGTH * 4`
   - `len(hispan) < 5` → `hispan < 5`

2. Verify ngram matching behavior:
   - Python uses `is_good_tokens_ngram()` to filter ngrams before adding to automaton
   - Rust may be missing this filtering, causing more matches than expected

3. Alternative approach: If all matches are weak (contain "unknown"), consider that the document already has unknown license detection covered by the weak matches, and skip creating a new `unknown` match. But this logic should be explicit, not implicit via covered positions.

### Recommendation

1. **Do NOT implement the proposed fix** - it changes behavior incorrectly
2. Instead, investigate why Rust's `match_ngrams_in_region()` finds matches when Python's `get_matched_ngrams()` apparently doesn't for this specific file
3. Compare ngram automaton construction between Python (`add_ngrams()`, `is_good_tokens_ngram()`) and Rust
