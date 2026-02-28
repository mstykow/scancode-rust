# PLAN-012: unknown/README.md

## Status: ROOT CAUSE IDENTIFIED

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
```

### UNKNOWN AUTOMATON DEBUG
```
Query tokens: 693
Total ngram matches in document: 12
Hispan (high-value legalese tokens): 124
Region length: 693
Passes length check (>= 24): true
Passes hispan check (>= 5): true
Covered positions from weak matches: 16 tokens
Unmatched regions count: 4
```

## Root Cause Analysis

### The Fundamental Difference

**Python's approach** (`match_unknown.py:143-223`):
1. Get matched ngrams from automaton as `(qstart, qend)` tuples
2. Build `qspan` as **union of all matched ngram positions**
3. Check `len(qspan) < unknown_ngram_length * 4` (i.e., `len(qspan) < 24`)
4. The match covers ONLY the positions with actual ngram matches

**Rust's approach** (`unknown_match.rs:108-148`):
1. Find unmatched regions first
2. Count ngram matches in each region
3. Check `ngram_matches >= MIN_NGRAM_MATCHES (3)`
4. Check `region_length >= UNKNOWN_NGRAM_LENGTH * 4 (24)`
5. Create match covering the **ENTIRE unmatched region**, not just ngram-matched positions

### Why This Causes the Bug

1. **Python**: The 12 ngram matches (each 6 tokens) are scattered across the document. Their union (`qspan`) is likely much less than 24 tokens, so Python returns `None` without creating an unknown match.

2. **Rust**: We find the unmatched region (693 tokens), check if it has >= 3 ngram matches (yes, 12), and if it's >= 24 tokens (yes, 693). Then we create an unknown match covering lines 1-51 (the entire document).

### The Critical Threshold Difference

| Check | Python | Rust |
|-------|--------|------|
| Length check | `len(qspan) < 24` where qspan is **union of matched ngram positions** | `region_length < 24` where region is **entire unmatched area** |
| Hispan check | `len(hispan) < 5` computed from tokens in qspan | `hispan < 5` computed from entire region |

The Rust implementation conflates "unmatched region length" with "matched ngram union length". These are very different things.

### Debug Evidence

From the test output:
- Document has 693 tokens
- 12 ngram matches (each 6 tokens = 72 token-positions, but with overlaps)
- If these 12 matches are scattered, their union could be anywhere from 6 to 72 tokens
- Python checks if union >= 24, likely fails, returns None
- Rust checks if unmatched region (693) >= 24, passes, creates huge match

## Proposed Fix

### Option A: Compute qspan union like Python (Recommended)

Modify `unknown_match.rs` to:

1. Get all matched ngram positions as `(qstart, qend)` tuples
2. Compute the union of these positions (like Python's `Span.union()`)
3. Check if union length >= 24 tokens
4. If passes, create match for the UNION, not the entire unmatched region

```rust
fn unknown_match(
    index: &LicenseIndex,
    query: &Query,
    known_matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    // ... existing setup ...

    for region in unmatched_regions {
        // Get matched ngram positions in this region
        let matched_ngrams = get_matched_ngrams(&query.tokens, region.0, region.1, automaton);

        // Compute union of matched positions (THIS IS THE KEY FIX)
        let qspan = compute_qspan_union(matched_ngrams);

        // Check threshold on the UNION, not the region
        if qspan.len() < UNKNOWN_NGRAM_LENGTH * 4 {
            continue;  // Skip if union is too small
        }

        // Compute hispan from tokens in qspan, not entire region
        let hispan = compute_hispan_from_qspan(&query.tokens, &qspan, index.len_legalese);
        if hispan < 5 {
            continue;
        }

        // Create match for qspan positions, not entire region
        if let Some(match_result) = create_unknown_match_from_qspan(...) {
            unknown_matches.push(match_result);
        }
    }

    unknown_matches
}
```

### Option B: Quick Fix - Pass weak matches to compute covered positions

This was the previously proposed fix but it's incorrect because:
1. Python explicitly uses only `good_matches` for computing `good_qspan`
2. This would deviate from Python's behavior
3. It could cause false negatives in other cases

**Do NOT implement Option B.**

## Implementation Details for Option A

### 1. Add `get_matched_ngrams()` function

Corresponds to Python's `get_matched_ngrams()` at `match_unknown.py:242-260`:

```rust
fn get_matched_ngrams(
    tokens: &[u16],
    start: usize,
    end: usize,
    automaton: &AhoCorasick,
) -> Vec<(usize, usize)> {
    let region_bytes: Vec<u8> = tokens[start..end]
        .iter()
        .flat_map(|tid| tid.to_le_bytes())
        .collect();

    let offset = UNKNOWN_NGRAM_LENGTH - 1;
    let mut matches = Vec::new();

    for m in automaton.find_iter(&region_bytes) {
        let qend = start + m.end() / 2;  // Convert byte position to token position
        let qstart = qend - offset;
        matches.push((qstart, qend));
    }

    matches
}
```

### 2. Add `compute_qspan_union()` function

Corresponds to Python's `Span.union()`:

```rust
fn compute_qspan_union(positions: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if positions.is_empty() {
        return Vec::new();
    }

    // Sort by start position
    let mut sorted: Vec<_> = positions.into_iter().collect();
    sorted.sort_by_key(|p| p.0);

    // Merge overlapping intervals
    let mut merged: Vec<(usize, usize)> = Vec::new();
    let mut current = sorted[0];

    for (start, end) in sorted.into_iter().skip(1) {
        if start <= current.1 {
            // Overlapping or adjacent, merge
            current.1 = current.1.max(end);
        } else {
            merged.push(current);
            current = (start, end);
        }
    }
    merged.push(current);

    merged
}
```

### 3. Compute hispan from qspan tokens

```rust
fn compute_hispan_from_qspan(
    tokens: &[u16],
    qspan: &[(usize, usize)],
    len_legalese: usize,
) -> usize {
    qspan.iter()
        .flat_map(|(start, end)| *start..*end)
        .filter(|&pos| (tokens[pos] as usize) < len_legalese)
        .count()
}
```

## Success Criteria
- [x] Investigation test file created
- [x] Python reference output documented
- [x] Rust debug output added for all pipeline stages
- [x] Exact divergence location identified
- [x] Root cause documented (threshold computed on wrong value)
- [x] Fix proposed with implementation details
- [ ] Implement fix in `unknown_match.rs`
- [ ] Verify test passes

## Risk Analysis
- **Medium risk**: This is a significant change to unknown matching logic
- **Requires testing**: Run full golden test suite after fix
- **Backward compatible**: The fix aligns with Python behavior

## Files to Modify
1. `src/license_detection/unknown_match.rs` - Implement the fix
2. `src/license_detection/investigation/unknown_readme_test.rs` - Update test assertions
