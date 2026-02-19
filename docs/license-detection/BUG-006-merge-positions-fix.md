# BUG-006: merge_overlapping_matches Doesn't Properly Update Positions

## Status: Planning Complete - Ready for Implementation

---

## 1. Root Cause Analysis

### The Problem

When matches are merged in Rust's `merge_overlapping_matches()`, the following fields are NOT properly updated:

1. **`rule_start_token`** - Left unchanged (keeps value from first match)
2. **`start_token` / `end_token`** - Not updated (query-side positions)
3. **`matched_token_positions`** - Not merged (stays `None`)
4. **`hilen`** - Not recomputed (takes max instead of sum)

### Python vs Rust Comparison

| Field | Python `combine()` | Rust `merge_overlapping_matches()` | Issue |
|-------|--------------------|-----------------------------------|-------|
| `qspan` | `Span(self.qspan \| other.qspan)` (union) | Not updated | Missing |
| `ispan` | `Span(self.ispan \| other.ispan)` (union) | Not updated | Missing |
| `hispan` | `Span(self.hispan \| other.hispan)` (union) | `max()` instead of union | Wrong logic |
| `matched_length` | `len(qspan)` after union | `max()` instead of updated | Wrong logic |
| `score` | Recomputed from coverage | `max()` instead of recompute | Wrong logic |

### Why Python's Span Union Matters

Python's `Span` is backed by `intbitset` - a **set-like structure** that can represent **non-contiguous positions**.

```python
# Python example
Span([1, 2, 3]) | Span([5, 6])  # Results in Span([1, 2, 3, 5, 6])
```

When two matches merge:

- **Query side (qspan)**: Union of positions where tokens matched in the query text
- **Rule side (ispan)**: Union of positions where tokens matched in the rule text

Rust's current `ispan()` returns `Range<usize>` which **cannot** represent gaps:

```rust
pub fn ispan(&self) -> Range<usize> {
    self.rule_start_token..self.rule_start_token + self.matched_length
}
```

### The Critical Insight

Python's `combine()` (lines 638-687) creates the UNION of positions:

```python
combined = LicenseMatch(
    rule=self.rule,
    qspan=Span(self.qspan | other.qspan),  # UNION
    ispan=Span(self.ispan | other.ispan),  # UNION  
    hispan=Span(self.hispan | other.hispan),  # UNION
    ...
)
```

This means after merge:

- `len(match.qspan)` = total unique matched positions (may be sum of both matches minus overlap)
- `match.ispan` contains all rule positions that matched (could have gaps)
- `match.hispan` contains all high-value positions matched

---

## 2. Recommended Solution: Option B (Store ispan as a field)

### Why Option B

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| A: Change `ispan()` to return `Vec<usize>` | Handles non-contiguous | Breaks all callers | Rejected |
| **B: Store `ispan` and `qspan` as fields** | Set during merge, minimal API change | More memory | **Recommended** |
| C: Track merged status | Minimal struct changes | Complex logic | Rejected |
| D: Keep `Range<usize>` | Simple | Incorrect for non-contiguous | Rejected |

**Recommendation**: Store `ispan` and `qspan` as `Option<Vec<usize>>` fields on `LicenseMatch`. This allows:

1. Non-contiguous positions after merge
2. Correct union computation during merge
3. Minimal API change (computed methods for non-merged matches)

---

## 3. Step-by-Step Implementation

### Phase 1: Add New Fields to LicenseMatch

**File**: `src/license_detection/models.rs` (~line 280)

```rust
/// Token positions matched in the query text.
/// None means contiguous range [start_token, end_token).
/// Some(positions) contains exact positions for non-contiguous matches (after merge).
#[serde(skip)]
pub qspan_positions: Option<Vec<usize>>,

/// Token positions matched in the rule text.
/// None means contiguous range [rule_start_token, rule_start_token + matched_length).
/// Some(positions) contains exact positions for non-contiguous matches (after merge).
#[serde(skip)]
pub ispan_positions: Option<Vec<usize>>,
```

**Update Default implementation** (~line 324):

```rust
matched_token_positions: None,
qspan_positions: None,
ispan_positions: None,
hilen: 0,
```

### Phase 2: Update `ispan()` and `qspan()` Methods

**File**: `src/license_detection/models.rs` (~lines 466-472)

```rust
pub fn ispan(&self) -> Vec<usize> {
    if let Some(positions) = &self.ispan_positions {
        positions.clone()
    } else {
        (self.rule_start_token..self.rule_start_token + self.matched_length).collect()
    }
}

pub fn qspan(&self) -> Vec<usize> {
    if let Some(positions) = &self.qspan_positions {
        positions.clone()
    } else {
        (self.start_token..self.end_token).collect()
    }
}

/// Returns true if this match has non-contiguous positions (was merged).
pub fn has_gaps(&self) -> bool {
    self.qspan_positions.is_some() || self.ispan_positions.is_some()
}
```

### Phase 3: Update `len()` to Use qspan_positions

**File**: `src/license_detection/models.rs` (~line 366)

```rust
pub fn len(&self) -> usize {
    if let Some(positions) = &self.qspan_positions {
        positions.len()
    } else if let Some(positions) = &self.matched_token_positions {
        positions.len()
    } else {
        self.end_token.saturating_sub(self.start_token)
    }
}
```

### Phase 4: Update `merge_overlapping_matches()` to Compute Unions

**File**: `src/license_detection/match_refine.rs` (~lines 126-180)

Replace the merge logic to properly combine spans:

```rust
fn merge_overlapping_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    if matches.len() == 1 {
        return matches.to_vec();
    }

    let mut grouped: HashMap<String, Vec<&LicenseMatch>> = HashMap::new();
    for m in matches {
        grouped
            .entry(m.rule_identifier.clone())
            .or_default()
            .push(m);
    }

    let mut merged = Vec::new();

    for (_rid, rule_matches) in grouped {
        if rule_matches.len() == 1 {
            merged.push(rule_matches[0].clone());
            continue;
        }

        let mut sorted_matches: Vec<_> = rule_matches.into_iter().collect();
        sorted_matches.sort_by_key(|m| (m.start_token, std::cmp::Reverse(m.matched_length)));

        let mut accum = sorted_matches[0].clone();

        for next_match in sorted_matches.into_iter().skip(1) {
            // Check if matches should be merged (overlap or adjacent in query)
            let should_merge = accum.end_token >= next_match.start_token;
            
            if should_merge {
                // Compute union of qspan positions
                let accum_qspan: HashSet<usize> = accum.qspan().into_iter().collect();
                let next_qspan: HashSet<usize> = next_match.qspan().into_iter().collect();
                let merged_qspan: Vec<usize> = accum_qspan.union(&next_qspan).copied().collect();
                
                // Compute union of ispan positions
                let accum_ispan: HashSet<usize> = accum.ispan().into_iter().collect();
                let next_ispan: HashSet<usize> = next_match.ispan().into_iter().collect();
                let merged_ispan: Vec<usize> = accum_ispan.union(&next_ispan).copied().collect();
                
                // Compute union of hispan positions (for hilen)
                let accum_hispan: HashSet<usize> = (accum.rule_start_token..accum.rule_start_token + accum.hilen)
                    .filter(|&p| accum.ispan().contains(&p))
                    .collect();
                let next_hispan: HashSet<usize> = (next_match.rule_start_token..next_match.rule_start_token + next_match.hilen)
                    .filter(|&p| next_match.ispan().contains(&p))
                    .collect();
                
                // Update accum with merged data
                let new_start_token = merged_qspan.iter().min().copied().unwrap_or(accum.start_token);
                let new_end_token = merged_qspan.iter().max().copied().unwrap_or(accum.end_token) + 1;
                let new_rule_start_token = merged_ispan.iter().min().copied().unwrap_or(accum.rule_start_token);
                
                // Sort positions
                let mut sorted_qspan = merged_qspan;
                sorted_qspan.sort();
                let mut sorted_ispan = merged_ispan;
                sorted_ispan.sort();
                
                accum.start_token = new_start_token;
                accum.end_token = new_end_token;
                accum.rule_start_token = new_rule_start_token;
                accum.matched_length = sorted_qspan.len();
                accum.hilen = sorted_ispan.len(); // Sum of high-value positions
                accum.start_line = accum.start_line.min(next_match.start_line);
                accum.end_line = accum.end_line.max(next_match.end_line);
                accum.score = accum.score.max(next_match.score);
                accum.qspan_positions = Some(sorted_qspan);
                accum.ispan_positions = Some(sorted_ispan);
            } else {
                merged.push(accum);
                accum = next_match.clone();
            }
        }
        merged.push(accum);
    }

    merged
}
```

### Phase 5: Update `filter_matches_missing_required_phrases()` to Handle Merged Matches

**File**: `src/license_detection/match_refine.rs` (~lines 860-880)

The `ispan()` method now returns `Vec<usize>`, so update the containment check:

```rust
let ispan = m.ispan(); // Now returns Vec<usize>
let ispan_set: HashSet<usize> = ispan.iter().copied().collect();

// Check if all required phrase spans are contained in ispan
let all_contained = ikey_spans
    .iter()
    .all(|span| {
        (span.start..span.end)
            .all(|pos| ispan_set.contains(&pos))
    });
```

### Phase 6: Update All Match Creators (If Needed)

Most match creators don't need changes because they create contiguous matches where:

- `qspan_positions = None` (computed from start_token..end_token)
- `ispan_positions = None` (computed from rule_start_token..rule_start_token+matched_length)

However, verify these files set `qspan_positions: None` and `ispan_positions: None`:

- `src/license_detection/seq_match.rs`
- `src/license_detection/aho_match.rs`
- `src/license_detection/hash_match.rs`
- `src/license_detection/spdx_lid.rs`
- `src/license_detection/unknown_match.rs`

---

## 4. Testing Strategy

### Unit Tests

**File**: `src/license_detection/match_refine.rs` (add to tests module)

```rust
#[test]
fn test_merge_updates_qspan_union() {
    // Match 1: tokens 0-10 in query, positions 0-10 in rule
    let m1 = create_test_match_with_tokens("#1", 0, 10, 10);
    // Match 2: tokens 5-15 in query, positions 5-15 in rule  
    let m2 = create_test_match_with_tokens("#1", 5, 15, 10);
    
    let merged = merge_overlapping_matches(&[m1, m2]);
    
    assert_eq!(merged.len(), 1);
    // qspan should be union: 0-15 (all positions covered)
    let qspan = merged[0].qspan();
    assert_eq!(qspan.len(), 16); // 0-15 inclusive = 16 positions
    // Verify positions 0-15 are all present
    for i in 0..=15 {
        assert!(qspan.contains(&i));
    }
}

#[test]
fn test_merge_updates_ispan_union() {
    let m1 = create_test_match_with_rule_start("#1", 0, 10, 10, 0);
    let m2 = create_test_match_with_rule_start("#1", 5, 15, 10, 5);
    
    let merged = merge_overlapping_matches(&[m1, m2]);
    
    assert_eq!(merged.len(), 1);
    let ispan = merged[0].ispan();
    // ispan should be union of rule positions
    assert!(ispan.len() >= 10);
}

#[test]
fn test_merge_non_contiguous_qspan() {
    // Match 1: tokens 0-5 in query
    let m1 = create_test_match_with_tokens("#1", 0, 5, 5);
    // Match 2: tokens 10-15 in query (gap between 5-10)
    let m2 = create_test_match_with_tokens("#1", 10, 15, 5);
    
    // These should NOT merge (not adjacent/overlapping)
    let merged = merge_overlapping_matches(&[m1, m2]);
    assert_eq!(merged.len(), 2);
}

#[test]
fn test_merge_adjacent_qspan() {
    // Match 1: tokens 0-10 in query
    let m1 = create_test_match_with_tokens("#1", 0, 10, 10);
    // Match 2: tokens 10-20 in query (adjacent, end_token == start_token)
    let m2 = create_test_match_with_tokens("#1", 10, 20, 10);
    
    let merged = merge_overlapping_matches(&[m1, m2]);
    
    // Adjacent matches should merge
    assert_eq!(merged.len(), 1);
    let qspan = merged[0].qspan();
    assert_eq!(qspan.len(), 20);
}
```

### Integration Tests

Run golden tests before and after the fix:

```bash
cargo test --release -q --lib license_detection::golden_test
```

Expected behavior:

- Some tests may now pass that were failing (correct merge detection)
- No tests should regress (merge is more accurate now)

### Specific Test Cases

1. **Merge with overlap**: Two matches overlap by 5 tokens → merged qspan should have correct union
2. **Merge adjacent**: Two matches touch at boundary → merged qspan should cover both
3. **Merge with gap**: Two matches separated by gap → should NOT merge
4. **Merge different rules**: Matches from different rules → should NOT merge
5. **ispan after merge**: Verify `filter_matches_missing_required_phrases` works with merged ispan

---

## 5. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance regression (HashSet for unions) | Medium | Low | Profile with large files; consider BTreeSet for sorted output |
| Test failures in golden tests | Medium | Medium | Run full test suite; compare outputs with Python |
| API breakage for ispan() return type | Low | Medium | `Vec<usize>` is more flexible; callers iterate anyway |
| Memory increase from new fields | Low | Low | Fields are `Option<Vec>`; only set after merge |

---

## 6. Estimated Effort

| Task | Time | Complexity |
|------|------|------------|
| Phase 1: Add new fields | 30 min | Low |
| Phase 2: Update ispan/qspan methods | 30 min | Low |
| Phase 3: Update len() | 15 min | Low |
| Phase 4: Update merge_overlapping_matches | 2 hours | High |
| Phase 5: Update required_phrases filter | 1 hour | Medium |
| Phase 6: Verify match creators | 30 min | Low |
| Write unit tests | 1.5 hours | Medium |
| Run golden tests and fix issues | 1 hour | Medium |
| **Total** | **7-8 hours** | |

---

## 7. Implementation Order

1. **Phase 1 & 2 together**: Add fields and update methods
2. **Phase 3**: Update len()
3. **Phase 4**: Core merge logic (most complex)
4. **Phase 5**: Update required_phrases filter
5. **Phase 6**: Verify match creators set fields correctly
6. **Write tests**: Unit tests for merge behavior
7. **Run golden tests**: Verify no regressions
8. **Clean up**: Remove unused code, update docs

---

## 8. Verification Checklist

After implementation:

- [ ] `cargo build` succeeds without warnings
- [ ] `cargo test` passes all tests
- [ ] `cargo clippy` passes without warnings
- [ ] Golden test results show improvement or no regression
- [ ] Manual testing with files that have merged matches
- [ ] `ispan()` and `qspan()` return correct positions after merge
- [ ] `filter_matches_missing_required_phrases` works with merged matches
- [ ] Memory usage is acceptable (run with large file)

---

## 9. Appendix: Python Span Class Details

From `reference/scancode-toolkit/src/licensedcode/spans.py`:

```python
class Span(Set):
    """
    Represent ranges of integers as a set of integers.
    A Span is hashable and not meant to be modified once created.
    """
    
    def __init__(self, *args):
        # Can be created from:
        # - Span(start, end) - range of integers
        # - Span([1, 2, 3]) - list of integers (can be non-contiguous)
        self._set = intbitset(...)  # Backed by efficient bitset
    
    def __or__(self, other):
        return Span(self._set.union(other._set))  # UNION
    
    def __len__(self):
        return len(self._set)  # Count of positions
    
    @property
    def start(self):
        return self._set[0]  # Minimum position
    
    @property
    def end(self):
        return self._set[-1]  # Maximum position
    
    def magnitude(self):
        return self.end - self.start + 1  # Span extent (may be > len)
    
    def density(self):
        return len(self) / self.magnitude()  # Ratio of filled positions
```

Key insight: `Span` can represent non-contiguous positions, and its `len()` counts actual positions, while `magnitude()` measures the full extent. The `|` operator creates a union of positions.
