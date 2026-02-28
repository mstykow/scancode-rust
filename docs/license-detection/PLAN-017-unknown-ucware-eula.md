# PLAN-017: unknown/ucware-eula.txt

## Status: VALIDATION COMPLETE - ROOT CAUSE IDENTIFIED

## Test File
`testdata/license-golden/datadriven/unknown/ucware-eula.txt`

## Issue
**Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "swrule"]`
**Actual (from main detection):** `["unknown", "warranty-disclaimer", "unknown"]`
**Actual (from investigation test with 70 candidates):** `["unknown-license-reference", "unknown-license-reference", "swrule", "warranty-disclaimer"]`

## Validation Results

### Python Implementation Analysis (match_unknown.py)

Python's `match_unknowns()` function works as follows:

1. **Line 143-152**: It calls `get_matched_ngrams()` which finds all ngram matches from the automaton
2. **Line 151-152**: It builds a union of all matched ngram positions into a `qspan`
3. **Line 220**: Threshold check: `len(qspan) < 24 OR len(hispan) < 5` → skip

**Key insight**: Python matches ngrams against the **full query tokens**, not just the uncovered region. The `QueryRun` is used to define the region boundaries, but `get_matched_ngrams()` searches the entire `tokens` sequence.

### Rust Implementation Analysis (unknown_match.rs)

Rust's `unknown_match()` function:

1. **Lines 184-212**: Correctly finds uncovered regions
2. **Line 136**: Calls `match_ngrams_in_region()` - **THIS IS THE BUG**
3. **Lines 224-248**: `match_ngrams_in_region()` extracts a substring of tokens from the uncovered region and searches ONLY that substring

**The Bug**: Rust only searches for ngrams **entirely within** the uncovered region, while Python searches the **full query** and then filters by region.

### Debug Test Output

From `test_debug_unknown_match_internals`:

```
Region 6: tokens 49-83 (length=34), legalese=9, ngram_matches=0
  -> FAIL: ngram_matches 0 < MIN_NGRAM_MATCHES (3)
```

- **Full query has 25 ngram matches** but **Region 6 has 0 ngram matches**!
- This is because ngrams that span across known matches are not being found
- Python would find these ngrams because it searches the full text

### Why Python Finds More Ngrams

In Python's `get_matched_ngrams()`:
```python
qtokens = tuple(tokens)  # Uses FULL query tokens
for qend, _ in automaton.iter(qtokens):
    qend = qbegin + qend  # Adjusts for region start offset
    qstart = qend - offset
    yield qstart, qend
```

The key difference:
- Python: Searches full `qtokens`, then adjusts positions with `qbegin` offset
- Rust: Searches only `region_tokens = &tokens[start..end]`

An ngram that starts at position 47 and ends at 53 would:
- Python: Be found (automaton iterates over full text)
- Rust: NOT be found (it crosses region boundary at 49)

## Root Cause (REFINED)

**The ngram matching algorithm is fundamentally different:**

| Aspect | Python | Rust |
|--------|--------|------|
| Search scope | Full query tokens | Only uncovered region tokens |
| Cross-boundary ngrams | Found (positions adjusted) | Not found (out of scope) |
| Result | 25 matches in full query | 0 matches in uncovered regions |

**The `UNKNOWN_NGRAM_LENGTH * 4` threshold (24 tokens)** ensures only regions with enough potential for multiple ngram matches qualify. But since Rust searches only the substring, it finds 0 ngram matches even in large regions.

## Proposed Fix

**Option A: Match Python's approach (Recommended)**

1. Modify `match_ngrams_in_region()` to search the full query tokens
2. Filter matches to only those that fall within the uncovered region
3. Adjust match positions by the region's start offset

```rust
fn match_ngrams_in_region(
    tokens: &[u16],      // Full query tokens
    region_start: usize, // Start of uncovered region
    region_end: usize,   // End of uncovered region
    automaton: &AhoCorasick,
) -> usize {
    let query_bytes: Vec<u8> = tokens.iter()
        .flat_map(|tid| tid.to_le_bytes())
        .collect();
    
    let offset = UNKNOWN_NGRAM_LENGTH - 1;
    let mut match_count = 0;
    
    for m in automaton.find_iter(&query_bytes) {
        let qend = m.end() / 2;  // Convert byte position to token position
        let qstart = qend.saturating_sub(offset);
        
        // Only count matches that fall within the uncovered region
        if qstart >= region_start && qend <= region_end {
            match_count += 1;
        }
    }
    
    match_count
}
```

**Option B: Adjust threshold values**

Lower `MIN_NGRAM_MATCHES` to 0 and rely on other thresholds. This is not recommended as it reduces the quality of unknown detection.

## Success Criteria
- [x] Python implementation analyzed
- [x] Rust implementation analyzed  
- [x] Root cause identified: ngram search scope mismatch
- [x] Specific code change documented
- [ ] Fix implemented
- [ ] Tests pass

## Risk Analysis
**Medium risk** - The fix changes core ngram matching logic. Must ensure:
1. Other test files still pass
2. No false positives introduced
3. Performance remains acceptable
