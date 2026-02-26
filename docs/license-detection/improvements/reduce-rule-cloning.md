# Reduce Rule Cloning in Sequence Matching

**Status**: Planned

## Goal

Eliminate unnecessary cloning of `Rule` structs during candidate selection in sequence matching, reducing memory allocations and improving performance.

## Problem

**File**: `src/license_detection/seq_match.rs`, lines 317, 444

```rust
step1_candidates.push((svr, svf, rid, rule.clone(), high_set_intersection));
```

The `Rule` struct contains 30+ fields including:
- `identifier: String`
- `license_expression: String`
- `text: String` (can be several KB for full license texts)
- `tokens: Vec<u16>` (hundreds/thousands of elements)
- Many `Option<Vec<String>>` fields

Each candidate selection clones the entire rule, creating significant memory pressure when checking thousands of rules per file.

---

## Clone Locations Analysis

| File | Line | Context |
|------|------|---------|
| `seq_match.rs` | 317 | `compute_candidates_with_msets()` - step1 candidates |
| `seq_match.rs` | 444 | `select_candidates()` - Candidate struct |
| `seq_match.rs` | 1013 | Test helper `add_test_rule()` - intentional |
| `index/builder.rs` | 943-945 | Test code - intentional |
| `models.rs` | 1072 | Test verifying Clone trait - intentional |

**Only lines 317 and 444 need fixing** - the others are test code where cloning is acceptable.

---

## Rule Fields Actually Used in Candidates

| Field | Used For |
|-------|----------|
| `relevance` | Score calculation |
| `license_expression` | LicenseMatch.license_expression |
| `identifier` | LicenseMatch.rule_identifier |
| `referenced_filenames` | LicenseMatch.referenced_filenames |
| `is_license_intro` | LicenseMatch flag |
| `is_license_clue` | LicenseMatch flag |
| `is_license_reference` | LicenseMatch flag |
| `is_license_tag` | LicenseMatch flag |
| `is_license_text` | LicenseMatch flag |
| `is_from_license` | LicenseMatch flag |

**Fields NOT used from cloned Rule:**
- `text` (large String - several KB)
- `tokens` (large Vec<u16> - hundreds/thousands of elements)
- Many other fields...

---

## Python Reference Comparison

**Python uses references, not clones.** In `match_set.py:297`:

```python
sortable_candidates_append((scores_vectors, rid, rule, high_set_intersection))
```

The `rule` is a **reference** to the rule object in `idx.rules_by_rid`. Python objects are always heap-allocated and passed by reference.

**Neither proposed option causes problematic drift from Python.** Both maintain reference semantics.

---

## Implementation Options

### Option A: Use `Arc<Rule>` (More Invasive)

Wrap rules in `Arc<Rule>` at index construction. Clone only the `Arc` pointer (O(1)).

**Files requiring changes:**
- `models.rs` - Rule struct
- `index/mod.rs` - `rules_by_rid: Vec<Arc<Rule>>`
- `index/builder.rs` - wrap in Arc
- `seq_match.rs` - Candidate struct
- `hash_match.rs`, `aho_match.rs`, `spdx_lid.rs`, `match_refine.rs` - all accessors

**Pros:** Thread-safe, enables future parallelization
**Cons:** High invasiveness (7+ files), Arc overhead on every access

### Option B: Store `rid` Only (Recommended - Less Invasive)

Remove `rule: Rule` from `Candidate` struct. Look up rule from index when needed via `rid`.

**Files requiring changes:**
- `seq_match.rs` only

**Changes:**

1. Remove `rule: Rule` from `Candidate` struct (line 88)
2. Remove `rule.clone()` at lines 317, 444
3. In `seq_match_with_candidates()`, add lookup:
   ```rust
   let rule = &index.rules_by_rid[candidate.rid];
   ```

**Pros:** Minimal change surface, zero memory overhead, no Arc overhead
**Cons:** Requires index reference in matching functions (already available)

---

## Comparison

| Criteria | Option A (Arc) | Option B (rid only) |
|----------|----------------|---------------------|
| Invasiveness | High (7+ files) | Low (1 file) |
| Risk | Higher | Lower |
| Performance gain | Good | Same or better |
| Memory overhead | Small (Arc metadata) | None |
| Python parity | Similar | Closer (explicit lookup) |

---

## Recommended Approach

**Implement Option B first** because:
1. Changes localized to `seq_match.rs` only
2. No changes to core index structure
3. Lower risk, easier to validate
4. Can migrate to Option A later if parallelization needed
5. Closer to Python's reference semantics

---

## Measurement Plan

### Before Implementation

```rust
// Add temporarily to seq_match.rs
static CLONE_COUNT: std::sync::atomic::AtomicUsize = AtomicUsize::new(0);

// At line 317
CLONE_COUNT.fetch_add(1, Ordering::Relaxed);
step1_candidates.push((svr, svf, rid, rule.clone(), high_set_intersection));
```

Run benchmark and log clone count.

### After Implementation

Clone count should be 0 (only test code clones remain).

### Memory Measurement

```bash
# Before/after comparison
valgrind --tool=dhat target/release/scancode-rust testdata/ -o /dev/null

# Or use /usr/bin/time
/usr/bin/time -v target/release/scancode-rust testdata/ -o /dev/null
# Compare "Maximum resident set size"
```

### Benchmark Cases

| Test Case | Why |
|-----------|-----|
| Small MIT file | Baseline |
| Large file with multiple licenses | Many candidates |
| GPL family files (GPL-2.0, GPL-2.0+, GPL-3.0) | Similar rules = more candidates |

---

## Validation

```bash
# Run all license detection tests
cargo test license_detection --release

# Run golden tests (compare against Python)
cargo test golden --release

# Full scan
cargo run --release -- testdata/ -o output.json
```

---

## Risk Assessment

| Risk | Assessment |
|------|------------|
| Thread safety | No impact (single-threaded currently) |
| Test isolation | No impact (tests have own index instances) |
| Intentional clones | Only in test code, not affected |

---

## Files Affected

| File | Changes |
|------|---------|
| `src/license_detection/seq_match.rs` | Remove `rule` from Candidate, add lookup logic |

No changes needed to:
- `models.rs`
- `index/mod.rs`
- `index/builder.rs`
- Other matchers

---

## Estimated Effort

**2-4 hours** for Option B implementation and validation.
