# PLAN-017: unknown/ucware-eula.txt

## Status: ROOT CAUSE REFINED - PIPELINE FLOW ISSUE

## Test File
`testdata/license-golden/datadriven/unknown/ucware-eula.txt`

## Issue
**Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "swrule"]`
**Actual (from investigation test):** `["unknown-license-reference", "unknown-license-reference", "swrule", "warranty-disclaimer"]`
**Actual (from main detection):** `["unknown", "warranty-disclaimer", "unknown"]`

## Refined Root Cause Analysis

### Issue 1: Test Does Not Follow Python Pipeline

The investigation test (`test_plan_017_rust_detection`) does NOT follow Python's pipeline:

**Python Pipeline (index.py:1082-1116):**
```python
# Step 1: Initial refine WITHOUT false positive filtering (line 1074-1080)
matches, _discarded = match.refine_matches(..., filter_false_positive=False, ...)

# Step 2: Split weak from good (line 1083)
good_matches, weak_matches = match.split_weak_matches(matches)

# Step 3: Compute uncovered regions from GOOD matches only (line 1087-1091)
original_qspan = Span(0, len(qry.tokens) - 1)
good_qspan = Span().union(*(mtch.qspan for mtch in good_matches))
unmatched_qspan = original_qspan.difference(good_qspan)

# Step 4: Run unknown detection on each unmatched subspan (line 1095-1109)
for unspan in unmatched_qspan.subspans():
    unknown_match = match_unknown.match_unknowns(...)

# Step 5: Filter contained unknowns, extend matches, reinject weak (line 1111-1118)
```

**Investigation Test Pipeline:**
```rust
// NO initial refine
// NO split_weak_matches
all_matches.extend(hash_match(...));
all_matches.extend(aho_match(...));
all_matches.extend(seq_match(...));
all_matches.extend(unknown_match(&query, &all_matches)); // Uses ALL matches, not good_matches
let refined = refine_matches(...);  // Called AFTER unknown_match
```

### Issue 2: `split_weak_matches` Removes Unknown-License-Reference Matches

Python's `split_weak_matches()` (match.py:1740-1765) removes matches where:
- `match.rule.has_unknown` is true (i.e., `"unknown" in license_expression`)
- OR match is a small/low-coverage seq match

**All `unknown-license-reference` matches are considered "weak"** and are set aside before unknown detection. This is critical because:

1. If `unknown-license-reference` matches stay in `good_matches`, they COVER those regions
2. Unknown detection only runs on UNCOVERED regions
3. With `unknown-license-reference` as "weak", those regions become uncovered
4. Unknown detection then finds `unknown` matches in those regions

### Issue 3: Ngram Search Scope (Previously Documented)

Python's `get_matched_ngrams()` searches the FULL query tokens:
```python
qtokens = tuple(tokens)  # FULL query tokens, not region substring
for qend, _ in automaton.iter(qtokens):
    qend = qbegin + qend  # Adjusts positions with region offset
    qstart = qend - offset
    yield qstart, qend
```

Rust's `match_ngrams_in_region()` searches only the region substring:
```rust
let region_tokens = &tokens[start..end];  // Only region tokens
```

This causes Rust to miss ngrams that cross region boundaries.

## Expected Fix

### Fix 1: Update Test to Follow Python Pipeline

```rust
fn test_plan_017_rust_detection() {
    // ... gather matches ...
    
    // Step 1: Initial refine without FP filtering
    let merged = refine_matches_without_false_positive_filter(&index, all_matches, &query);
    
    // Step 2: Split weak from good
    let (good_matches, weak_matches) = split_weak_matches(&merged);
    
    // Step 3: Unknown detection on uncovered regions
    let unknown_matches = unknown_match(&index, &query, &good_matches);
    let filtered_unknown = filter_invalid_contained_unknown_matches(&unknown_matches, &good_matches);
    
    // Step 4: Combine
    let mut all_matches = good_matches;
    all_matches.extend(filtered_unknown);
    all_matches.extend(weak_matches);
    
    // Step 5: Final refine with FP filtering
    let refined = refine_matches(&index, all_matches, &query);
}
```

### Fix 2: Update `match_ngrams_in_region()` to Search Full Query

```rust
fn match_ngrams_in_region(
    tokens: &[u16],      // Full query tokens
    region_start: usize,
    region_end: usize,
    automaton: &AhoCorasick,
) -> usize {
    let query_bytes: Vec<u8> = tokens.iter()
        .flat_map(|tid| tid.to_le_bytes())
        .collect();
    
    let offset = UNKNOWN_NGRAM_LENGTH - 1;
    let mut match_count = 0;
    
    for m in automaton.find_iter(&query_bytes) {
        let qend = m.end() / 2;
        let qstart = qend.saturating_sub(offset);
        
        // Only count matches within the region
        if qstart >= region_start && qend <= region_end {
            match_count += 1;
        }
    }
    
    match_count
}
```

### Fix 3: Python Returns SPANS, Not Just Count

Python's `get_matched_ngrams()` returns the matched positions:
```python
yield qstart, qend
```

These are then used to build a `qspan`:
```python
qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
qspan = Span().union(*qspans)
```

Rust currently only returns a COUNT. It should return the matched positions to build the span properly.

## Success Criteria
- [x] Python implementation analyzed
- [x] Rust implementation analyzed  
- [x] Root cause refined: pipeline flow + ngram search scope
- [x] Specific code changes documented
- [ ] Fix 1 implemented (test follows Python pipeline)
- [ ] Fix 2 implemented (ngram search scope)
- [ ] Fix 3 implemented (return positions, not count)
- [ ] Tests pass

## Risk Analysis
**Low risk** - The test fix is straightforward. The ngram scope fix aligns with Python behavior and the previous attempt showed it enables detection.
