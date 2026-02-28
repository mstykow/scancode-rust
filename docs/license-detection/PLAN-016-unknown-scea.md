# PLAN-016: unknown/scea.txt

## Status: VALIDATED - FIX APPROACH CONFIRMED

## Test File
`testdata/license-golden/datadriven/unknown/scea.txt`

## Issue
**Expected:** `["scea-1.0", "unknown-license-reference", "scea-1.0", "unknown", "unknown"]`
**Actual:** `["scea-1.0", "unknown-license-reference", "scea-1.0", "unknown"]`

## Differences
- **Missing one `unknown` match at the end**
- Python has 5 matches, Rust has 4 matches

## Python Reference Output

```
Total matches: 5
0: scea-1.0 | lines 1-1 | rule=scea-1.0_4.RULE | matcher=2-aho
1: unknown-license-reference | lines 1-1 | rule=unknown-license-reference_332.RULE | matcher=2-aho
2: scea-1.0 | lines 7-7 | rule=scea-1.0_4.RULE | matcher=2-aho
3: unknown | lines 7-22 | rule=license-detection-unknown-* | matcher=6-unknown
4: unknown | lines 22-31 | rule=license-detection-unknown-* | matcher=6-unknown
```

Key observation: Python creates **TWO separate unknown matches**:
- `unknown` at lines 7-22
- `unknown` at lines 22-31

These are **adjacent** (line 22 is both end of match 4 and start of match 5).

## Rust Debug Output

```
=== RUST DETECTIONS ===
Number of detections: 3

Detection 1:
  license_expression: Some("scea-1.0")
  Number of matches: 1
    Match 1:
      license_expression: scea-1.0
      matcher: 2-aho
      lines: 1-1

Detection 2:
  license_expression: Some("unknown-license-reference")
  Number of matches: 1
    Match 1:
      license_expression: unknown-license-reference
      matcher: 2-aho
      lines: 1-1

Detection 3:
  license_expression: Some("scea-1.0 AND unknown")
  Number of matches: 2
    Match 1:
      license_expression: scea-1.0
      matcher: 2-aho
      lines: 7-7
    Match 2:
      license_expression: unknown
      matcher: 5-undetected
      lines: 7-31
```

Rust creates **ONE unknown match** spanning lines 7-31.

---

## Validation Results

### 1. Python Implementation Analysis

**File:** `reference/scancode-toolkit/src/licensedcode/match_unknown.py`

**Key code path (lines 143-152):**
```python
matched_ngrams = get_matched_ngrams(tokens=query_run.tokens, qbegin=query_run.start, ...)
qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
qspan = Span().union(*qspans)
```

**Critical insight:** Python's `match_unknowns()` creates a `qspan` that represents **only the positions where ngrams matched**. The `union()` operation merges overlapping/adjacent ngram positions into contiguous spans.

**File:** `reference/scancode-toolkit/src/licensedcode/index.py` (lines 1091-1109)
```python
unmatched_qspan = original_qspan.difference(good_qspan)
for unspan in unmatched_qspan.subspans():
    unquery_run = query.QueryRun(query=qry, start=unspan.start, end=unspan.end)
    unknown_match = match_unknown.match_unknowns(idx=self, query_run=unquery_run, ...)
```

The splitting into multiple unknown matches occurs because:
1. `unmatched_qspan` represents positions NOT covered by known matches
2. `subspans()` returns contiguous regions from this potentially sparse span
3. Each subspan gets its own call to `match_unknowns()`

**Span behavior (spans.py:454-474):**
```python
def subspans(self):
    """Return a list of Spans creating one new Span for each set of contiguous integer items."""
    return Span.from_ints(self)
```

Uses `itertools.groupby` to split at gaps in integer positions.

### 2. Rust Implementation Analysis

**File:** `src/license_detection/unknown_match.rs`

**Current behavior (lines 127-145):**
```rust
for region in unmatched_regions {
    let start = region.0;
    let end = region.1;
    let ngram_matches = match_ngrams_in_region(&query.tokens, start, end, automaton);
    if ngram_matches < MIN_NGRAM_MATCHES { continue; }
    if let Some(match_result) = create_unknown_match(index, query, start, end, ngram_matches) {
        unknown_matches.push(match_result);
    }
}
```

**Problem:** Rust counts total ngram matches but doesn't track **where** those matches occurred. It creates one match for the entire region regardless of ngram distribution.

**File:** `src/license_detection/spans.rs`
- Existing `Span` struct supports `from_iterator()` which creates contiguous ranges
- Has `union_span()` and other utilities
- **Missing:** `subspans()` equivalent to return individual contiguous ranges

### 3. Root Cause Confirmed

The proposed fix in the plan is correct. The divergence is:

| Aspect | Python | Rust |
|--------|--------|------|
| Tracks ngram positions | Yes (individual qstart/qend) | No (only count) |
| Splits on ngram gaps | Yes (via `union()` then match text extraction) | No |
| Creates multiple matches | Yes, one per contiguous ngram region | No, one per uncovered region |

### 4. Complexity Assessment

**Effort: Medium**

**Required changes:**

1. **Modify `match_ngrams_in_region()`** to return positions instead of just count:
   ```rust
   fn find_ngram_matches(tokens: &[u16], start: usize, end: usize, automaton: &AhoCorasick) 
       -> Vec<(usize, usize)>  // Returns (qstart, qend) tuples
   ```

2. **Add `subspans()` method to `Span`** (or implement inline):
   ```rust
   fn subspans(&self) -> Vec<Range<usize>> {
       // Group contiguous positions, similar to Span.from_ints()
   }
   ```

3. **Update `unknown_match()` to group matches into contiguous spans**:
   ```rust
   let ngram_positions: Vec<(usize, usize)> = find_ngram_matches(...);
   let qspan = Span::from_positions(&ngram_positions);  // Union of all positions
   for subspan in qspan.subspans() {
       // Create unknown match for each contiguous region
   }
   ```

**Existing utilities:**
- `Span::from_iterator()` already creates contiguous ranges from positions
- Tests exist for span operations
- No new external dependencies needed

---

## Specific Code Changes Needed

### Change 1: Return ngram positions instead of count

**File:** `src/license_detection/unknown_match.rs`

Replace `match_ngrams_in_region()` with `find_ngram_matches()`:
```rust
fn find_ngram_matches(
    tokens: &[u16],
    start: usize,
    end: usize,
    automaton: &AhoCorasick,
) -> Vec<(usize, usize)> {
    // Return list of (qstart, qend) for each matched ngram
    // Similar to Python's get_matched_ngrams()
}
```

### Change 2: Add helper to group contiguous positions

**File:** `src/license_detection/unknown_match.rs` or `src/license_detection/spans.rs`

```rust
fn group_contiguous_spans(positions: &[(usize, usize)], ngram_length: usize) -> Vec<(usize, usize)> {
    // Sort by start position
    // Group overlapping/adjacent spans (within ngram_length of each other)
    // Return list of merged (start, end) tuples
}
```

### Change 3: Update `unknown_match()` main function

**File:** `src/license_detection/unknown_match.rs` (lines 127-145)

```rust
for region in unmatched_regions {
    let ngram_matches = find_ngram_matches(&query.tokens, region.0, region.1, automaton);
    
    // Group into contiguous spans
    let contiguous_spans = group_contiguous_spans(&ngram_matches, UNKNOWN_NGRAM_LENGTH);
    
    for span in contiguous_spans {
        // Apply thresholds to each contiguous span
        // Create unknown match if valid
    }
}
```

---

## Estimated Effort

**Total: 2-4 hours**

| Task | Time |
|------|------|
| Modify `match_ngrams_in_region()` to return positions | 30 min |
| Implement `group_contiguous_spans()` helper | 1 hour |
| Update `unknown_match()` to use new logic | 1 hour |
| Update/add tests | 1-1.5 hours |

---

## Risk Analysis
- **Low risk**: This only affects unknown license detection
- **Functional impact**: Users may see more granular unknown matches (improved accuracy)
- **Priority**: Medium - affects feature parity with Python

## Investigation Test File
Created at `src/license_detection/investigation/unknown_scea_test.rs`
