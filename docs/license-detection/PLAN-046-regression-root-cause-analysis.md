# PLAN-046: Regression Root Cause Analysis

## Status: SUPERSEDED - Issues extracted to dedicated plans

## Summary

This investigation identified root causes of regressions. All actionable issues have been extracted to dedicated plans:

| Issue | New Plan |
|-------|----------|
| `combine_matches` missing validation | PLAN-049 |
| NuGet SPDX pattern | PLAN-050 |
| hispan reconstruction | PLAN-051 |
| `restore_non_overlapping` lines vs tokens | PLAN-030 (existing) |

**Key Finding**: The regressions are NOT caused by bugs in PLAN-044/045 implementations. Instead, they expose deeper differences between Rust and Python that interact with the new code.

---

## Test Case Analysis: `bsd-new_105.txt`

### Expected vs Actual

| Metric | Python | Expected | Rust (Actual) |
|--------|--------|----------|---------------|
| Output | `["bsd-new"]` | `["bsd-new"]` | `["bsd-new", "bsd-x11"]` |
| Rule matched | `bsd-new_375.RULE` | - | `#5350`, `#7091` |
| Coverage | 99.56% | - | 84.29%, 100% |
| Matched length | 228 | - | 177, 204 |

### Critical Discovery

**Rust is matching DIFFERENT rules than Python**:

1. Python matches `bsd-new_375.RULE` - a full license text rule
2. Rust matches rules `#5350` and `#7091` with different rule texts
3. The Rust `bsd-x11` match has 100% coverage because it matches a SHORTER rule text (only partial license)

**This suggests**: Rust's rule loading/indexing differs from Python, causing different candidate rules to be matched.

---

## Root Cause Analysis

### Cause 1: Rule Loading/Indexing Differences - **VERIFIED CORRECT** (HIGH SEVERITY)

**Location**: Rule loading and index building

**Python** (index.py:316-323):

```python
self.rules_by_rid = rules_by_rid = list(rules)
self.rules_by_id = {r.identifier: r for r in self.rules_by_rid}
# ensure that rules are sorted
rules_by_rid.sort()  # Sorts by identifier string (BasicRule has no __lt__)
```

**Rust** (models.rs:199-202, builder.rs:268-269):

```rust
impl Ord for Rule {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identifier.cmp(&other.identifier)  // Sorts by identifier string
    }
}
// ...
all_rules.sort();  // Same sorting as Python
```

**Analysis**: Both Python and Rust sort rules by `identifier` string. This is **consistent**. The rule indices (`#5350`, `#7091`) are just positions in the sorted list and should correspond to the same rules in both implementations.

**Action Required**: Verify that both implementations load the SAME rules from the SAME directories. The difference may be in:

- Different license/rule data directories
- Different file discovery order
- Missing or extra rules in one implementation

---

### Cause 2: Sorting Uses `qspan.start` - **VERIFIED CORRECT** (NOT A BUG)

**Python** (match.py:1097-1100):

```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
```

**Rust** (match_refine.rs:334-340):

```rust
matches.sort_by(|a, b| {
    a.qstart()  // qstart() returns start_token (same as qspan.start)
        .cmp(&b.qstart())
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

**Analysis**: The Rust implementation is **CORRECT**. `qstart()` returns `self.start_token`, which is the first matched token position - the same as Python's `qspan.start`. This is NOT a root cause.

---

### Cause 3: Previous/Next Combined Check - **IMPLEMENTED** (VERIFY CORRECTNESS)

**Python** (match.py:1486-1507):

```python
# check the previous current and next match: discard current if it
# is entirely contained in a combined previous and next and previous
# and next do not overlap
if i:
    previous_match = matches[i - 1]
    if not previous_match.overlap(next_match):
        cpo = current_match.overlap(previous_match)
        cno = current_match.overlap(next_match)
        if cpo and cno:
            overlap_len = cno + cpo
            clen = current_match.len()
            if overlap_len >= (clen * 0.9):
                discarded_append(matches_pop(i))
                i -= 1
                break
```

**Rust** (match_refine.rs:683-700):

```rust
if i > 0 {
    let prev_next_overlap = matches[i - 1].qspan_overlap(&matches[j]);
    if prev_next_overlap == 0 {
        let cpo = matches[i].qspan_overlap(&matches[i - 1]);
        let cno = matches[i].qspan_overlap(&matches[j]);
        if cpo > 0 && cno > 0 {
            let overlap_len = cpo + cno;
            let clen = matches[i].matched_length;
            if overlap_len as f64 >= clen as f64 * 0.9 {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }
        }
    }
}
```

**Analysis**: Rust **HAS** the prev/next combined check. The implementation appears correct. However, verify that `qspan_overlap()` and `overlap()` produce identical results.

---

### Cause 4: `licensing_contains` Implementation Difference (HIGH SEVERITY)

**Python** (models.py:2065-2073):

```python
def licensing_contains(self, other):
    """Return True if this rule licensing contains the other rule licensing."""
    if self.license_expression and other.license_expression:
        return self.licensing.contains(
            expression1=self.license_expression_object,
            expression2=other.license_expression_object,
        )
```

**Rust**: Custom implementation in `expression.rs`

**Problem**:

- Python uses the `license-expression` library which has extensive SPDX expression handling
- Rust has a custom implementation that may miss edge cases
- Complex expressions (WITH, AND/OR, nested) may be handled differently

**Impact**: Expression subsumption checks may produce different results.

**Action Required**: Create comprehensive tests comparing Python's `license-expression` library output with Rust's custom implementation for:

- Simple expressions: `MIT`, `Apache-2.0`
- AND/OR: `MIT AND Apache-2.0`, `MIT OR Apache-2.0`
- WITH: `GPL-2.0 WITH Classpath-exception-2.0`
- Nested: `(MIT OR Apache-2.0) AND BSD-3-Clause`
- LicenseRefs: `LicenseRef-scancode-proprietary`

---

### Cause 5: Break Condition in `filter_contained_matches` (MEDIUM SEVERITY)

**Python** (match.py:1126):

```python
if next_match.qend > current_match.qend:
    break
```

**Rust** (match_refine.rs:349-351):

```rust
if next.end_token > current.end_token {
    break;
}
```

**Analysis**: Both implementations have the same break condition. This is **NOT** a root cause - it's correct behavior.

---

### Cause 6: `filter_contained_matches` Adds `licensing_contains` - **CONFIRMED BUG** (HIGH SEVERITY)

**Python** (match.py:1158-1165):

```python
# remove contained matched spans
if current_match.qcontains(next_match):
    discarded_append(matches_pop(j))
    continue

# remove contained matches the other way
if next_match.qcontains(current_match):
    discarded_append(matches_pop(i))
```

**Rust** (match_refine.rs:364-372):

```rust
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) || licensing_contains_match(&next, &current) {
    discarded.push(matches.remove(i));
```

**Problem**: Python ONLY uses `qcontains` (spatial containment) in `filter_contained_matches`. Rust adds `licensing_contains_match`, which may discard matches that Python would keep.

**Why This Matters**: The `licensing_contains` check belongs in `filter_overlapping_matches` (where Python uses it for MEDIUM overlap cases), NOT in `filter_contained_matches`.

**Action Required**: Remove `licensing_contains_match` from `filter_contained_matches` to match Python behavior.

---

### Cause 7: Different Rules Being Matched (HIGH SEVERITY) - **NEW FINDING**

**Root Cause**: The fundamental issue is that Rust matches DIFFERENT rules than Python for the same input text. This is upstream of the deduplication logic.

**Possible Causes**:

1. **Automaton differences**: The Aho-Corasick automaton may be built or queried differently
2. **Tokenization differences**: Different token handling could produce different match candidates
3. **Hash matching differences**: Hash-based matching may differ
4. **Sequence matching differences**: Approximate matching algorithm may differ
5. **Different rule data**: Missing or extra rules in one implementation

**Evidence**:

- Python matches `bsd-new_375.RULE` with 99.56% coverage
- Rust matches rules `#5350` and `#7091` with different coverage values
- The rules have different text lengths and coverage characteristics

**Action Required**: Add debug logging to trace:

- Which rules are loaded and their identifiers
- Which rules are matched by each strategy (hash, exact/Aho, sequence)
- The coverage calculation for each match

---

## Summary of Findings (Updated)

| Severity | Issue | Location | Status | Impact |
|----------|-------|----------|--------|--------|
| **HIGH** | Different rules matched upstream | Matching pipeline | **NEEDS INVESTIGATION** | Wrong license detection |
| **HIGH** | `licensing_contains` in filter_contained_matches | match_refine.rs:364-372 | **CONFIRMED BUG** | Over-deduplication |
| **HIGH** | `licensing_contains` custom impl | expression.rs | **NEEDS VERIFICATION** | Different subsumption results |
| MEDIUM | ~~Sorting uses wrong field~~ | match_refine.rs | **NOT A BUG** | N/A |
| MEDIUM | ~~Missing prev/next check~~ | match_refine.rs | **IMPLEMENTED** | N/A |

---

## Recommended Next Steps (Prioritized)

### Priority 1: Fix `licensing_contains` in `filter_contained_matches` (Quick Win)

**File**: `src/license_detection/match_refine.rs`

**Change**: Remove `licensing_contains_match` from the containment check:

```rust
// Before:
if current.qcontains(&next) || licensing_contains_match(&current, &next) {

// After:
if current.qcontains(&next) {
```

**Rationale**: Python only uses spatial containment in `filter_contained_matches`. The expression-based containment is used later in `filter_overlapping_matches`.

### Priority 2: Investigate Different Rules Being Matched

**Approach**: Add debug output to trace the matching pipeline:

1. Log all loaded rules with their identifiers and license expressions
2. Log which rules are matched by each strategy (hash, exact, sequence)
3. Compare the rule identifiers matched by Python vs Rust

**Tools**: Create a comparison script that:

- Runs both Python and Rust on the same input
- Outputs the matched rule identifiers
- Highlights differences

### Priority 3: Verify `licensing_contains` Parity

**File**: `src/license_detection/expression.rs`

**Create tests** that compare Python's `license-expression` library with Rust's implementation for:

- Simple expressions: `MIT`, `Apache-2.0`
- Compound expressions: `MIT AND Apache-2.0`, `MIT OR Apache-2.0`
- Exception expressions: `GPL-2.0 WITH Classpath-exception-2.0`
- Nested expressions: `(MIT OR Apache-2.0) AND BSD-3-Clause`
- LicenseRefs: `LicenseRef-scancode-proprietary`

### Priority 4: Verify `qspan_overlap()` Implementation

**File**: `src/license_detection/match_refine.rs`

Verify that `qspan_overlap()` produces identical results to Python's `overlap()` method.

---

## Files to Modify

| File | Change | Priority |
|------|--------|----------|
| `src/license_detection/match_refine.rs` | Remove `licensing_contains_match` from `filter_contained_matches` | P1 |
| `src/license_detection/expression.rs` | Verify `licensing_contains` parity with Python | P3 |
| `src/license_detection/match_refine.rs` | Verify `qspan_overlap()` matches Python's `overlap()` | P4 |

---

## Verification Steps

After implementing fixes:

1. Run the failing golden tests:

   ```bash
   cargo test test_golden_license_detection -- --nocapture
   ```

2. Compare specific test file:

   ```bash
   cargo test test_bsd_new_105 -- --nocapture
   ```

3. Run full golden test suite and compare results:

   ```bash
   cargo test --test golden_test -- --test-threads=1
   ```

---

## Related Documents

- `docs/license-detection/PLAN-044-filter-contained-matches-parity.md`
- `docs/license-detection/PLAN-045-expression-selection-parity.md`
- `docs/license_detection_comparison_report.md`

---

## Python Reference Locations

For verification during implementation:

| Component | Python Location | Rust Location |
|-----------|-----------------|---------------|
| Rule loading | `licensedcode/models.py:1217-1248` | `license_detection/rules.rs` |
| Index building | `licensedcode/index.py:270-577` | `license_detection/index/builder.rs` |
| `filter_contained_matches` | `licensedcode/match.py:1075-1184` | `match_refine.rs:326-380` |
| `filter_overlapping_matches` | `licensedcode/match.py:1187-1515` | `match_refine.rs:513-708` |
| `licensing_contains` | `licensedcode/models.py:2065-2073` | `expression.rs` |
| Match sorting | `licensedcode/match.py:1097-1100` | `match_refine.rs:334-340` |
| Rule sorting | `licensedcode/index.py:323` | `models.rs:199-202` |
