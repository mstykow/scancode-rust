# Inverted Index for Candidate Pre-Filtering

**Status**: Beyond-parity optimization (not implemented in Python reference)

## Summary

Add a true inverted index (`token_id -> Set<rid>`) to skip rules entirely during candidate selection, rather than iterating all rules and computing set intersection for each.

---

## Current Approach

### Python Reference

**File**: `reference/scancode-toolkit/src/licensedcode/match_set.py`, lines 277-297

```python
for rid, rule in enumerate(idx.rules_by_rid):
    if rid not in matchable_rids:
        continue

    scores_vectors, high_set_intersection = compare_token_sets(
        qset=qset,
        iset=sets_by_rid[rid],  # Set intersection computed per rule
        ...
    )
```

Python iterates ALL matchable rules and computes set intersection for each one. Rules with empty intersection are filtered out, but the intersection computation still happens.

### Rust Implementation

**File**: `src/license_detection/seq_match/candidates.rs`

```rust
for (rid, rule) in index.rules_by_rid.iter().enumerate() {
    if !index.approx_matchable_rids.contains(&rid) {
        continue;
    }

    let intersection: HashSet<u16> = query_set.intersection(rule_set).copied().collect();
    if intersection.is_empty() {
        continue;
    }
    // ...
}
```

Rust follows the same approach: iterate all rules, compute intersection, filter.

---

## Proposed Improvement

### Add Inverted Index: `rids_by_high_tid`

Add a mapping from high-value token IDs to the rule IDs that contain them:

```rust
// In LicenseIndex (src/license_detection/index/mod.rs)
/// Inverted index: high-value token ID -> set of rule IDs containing it.
/// Enables O(1) candidate pre-filtering instead of O(n_rules) iteration.
pub rids_by_high_tid: HashMap<u16, HashSet<usize>>,
```

### Modified Candidate Selection

```rust
// In seq_match/candidates.rs
fn compute_candidates(query: &Query, index: &LicenseIndex, top_n: usize) -> Vec<Candidate> {
    let query_high_tids: HashSet<u16> = query.tokens
        .iter()
        .filter(|&&tid| tid < index.len_legalese)
        .copied()
        .collect();

    // Pre-filter: collect only rules that share at least one high token
    let candidate_rids: HashSet<usize> = query_high_tids
        .iter()
        .flat_map(|tid| index.rids_by_high_tid.get(tid).unwrap_or(&HashSet::new()))
        .copied()
        .collect();

    // Now only iterate pre-filtered rules
    for rid in candidate_rids {
        let rule = &index.rules_by_rid[rid];
        // ... rest of similarity computation
    }
}
```

### Index Construction

Build during index creation in `src/license_detection/index/builder/mod.rs`:

```rust
let mut rids_by_high_tid: HashMap<u16, HashSet<usize>> = HashMap::new();

for (rid, rule_tokens) in rules_token_ids.iter().enumerate() {
    for &tid in rule_tokens {
        if tid < len_legalese {
            rids_by_high_tid.entry(tid).or_default().insert(rid);
        }
    }
}
```

---

## Expected Benefits

| Metric                     | Current           | After Improvement            |
| -------------------------- | ----------------- | ---------------------------- |
| Rules checked per file     | O(n_rules) ~1000+ | O(matching_rules) ~10-100    |
| Set intersections computed | All rules         | Only candidate rules         |
| Memory overhead            | None              | HashMap<u16, HashSet<usize>> |

For a typical source file with few/no license matches, the inverted index could skip 90%+ of rules without any intersection computation.

---

## Trade-offs

**Memory**: Additional HashMap storing rule ID sets per token. With ~4000 legalese tokens and ~1000 rules, worst case is ~4 million entries (but sparse in practice).

**Complexity**: Slightly more complex index construction and maintenance.

**Consistency**: Must be kept in sync with `sets_by_rid` if rules are added/removed.

---

## Verification Strategy

1. Add metrics to count rules checked before/after filtering
2. Benchmark on diverse test cases:
   - License files (should check many rules)
   - Source code (should check few rules)
   - Non-license text (should check minimal rules)
3. Ensure output unchanged vs current implementation

---

## Relevant Files

| File                                                       | Purpose                  |
| ---------------------------------------------------------- | ------------------------ |
| `src/license_detection/seq_match/candidates.rs`            | Candidate selection loop |
| `src/license_detection/index/mod.rs`                       | LicenseIndex struct      |
| `src/license_detection/index/builder/mod.rs`               | Index construction       |
| `reference/scancode-toolkit/src/licensedcode/match_set.py` | Python reference         |

---

## Notes

This is a **beyond-parity optimization**. The Python reference does not implement this, so it's not required for feature parity. However, it could provide significant performance improvements for large codebases with many small files.

Consider implementing after achieving parity and establishing performance baselines.
