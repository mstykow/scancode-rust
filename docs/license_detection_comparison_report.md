# License Detection Pipeline Comparison: Python vs Rust

## Executive Summary

This report documents all identified differences between the Python (ScanCode Toolkit) and Rust implementations of the license detection pipeline, focusing on matching, filtering, and merging operations.

**Overall Status**: The Rust implementation closely mirrors the Python pipeline structure, but has several areas requiring investigation for potential behavioral differences.

---

## 1. Pipeline Order Comparison

### Python `refine_matches` (match.py:2691-2833)

| Step | Function | Lines |
|------|----------|-------|
| 1 | `merge_matches()` | 2719 |
| 2 | `filter_matches_missing_required_phrases()` | 2744 |
| 3 | `filter_spurious_matches()` | 2748 |
| 4 | `filter_below_rule_minimum_coverage()` | 2752 |
| 5 | `filter_matches_to_spurious_single_token()` | 2756 |
| 6 | `filter_too_short_matches()` | 2760 |
| 7 | `filter_short_matches_scattered_on_too_many_lines()` | 2764 |
| 8 | `filter_invalid_matches_to_single_word_gibberish()` | 2768 |
| 9 | `merge_matches()` | 2773 |
| 10 | `filter_contained_matches()` | 2781 |
| 11 | `filter_overlapping_matches()` | 2790 |
| 12 | `restore_non_overlapping()` (contained) | 2794 |
| 13 | `restore_non_overlapping()` (overlapping) | 2800 |
| 14 | `filter_contained_matches()` (second pass) | 2805 |
| 15 | `filter_false_positive_matches()` | 2810 |
| 16 | `filter_false_positive_license_lists_matches()` | 2815 |
| 17 | `merge_matches()` (final) | 2825 |

### Rust `refine_matches` (match_refine.rs:1509-1571)

| Step | Function | Lines |
|------|----------|-------|
| 1 | `merge_overlapping_matches()` | 1519 |
| 2 | `filter_matches_missing_required_phrases()` | 1522 |
| 3 | `filter_spurious_matches()` | 1525 |
| 4 | `filter_below_rule_minimum_coverage()` | 1527 |
| 5 | `filter_matches_to_spurious_single_token()` | 1529 |
| 6 | `filter_too_short_matches()` | 1531 |
| 7 | `filter_short_matches_scattered_on_too_many_lines()` | 1533 |
| 8 | `filter_invalid_matches_to_single_word_gibberish()` | 1535 |
| 9 | `merge_overlapping_matches()` | 1539 |
| 10 | `filter_contained_matches()` | 1541 |
| 11 | `filter_overlapping_matches()` | 1543 |
| 12 | `restore_non_overlapping()` (contained) | 1548 |
| 13 | `restore_non_overlapping()` (overlapping) | 1555 |
| 14 | `filter_contained_matches()` (second pass) | 1560 |
| 15 | `filter_false_positive_matches()` | 1562 |
| 16 | `filter_false_positive_license_lists_matches()` | 1564 |
| 17 | `merge_overlapping_matches()` | 1566 |
| 18 | `update_match_scores()` | 1568 |

**Verdict**: ✅ Pipeline order matches exactly (Rust adds final score update step).

---

## 2. Sorting Criteria Comparison

### Python Sorting (match.py:1097-1100, 1220)

```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
matches = sorted(matches, key=sorter)
```

**Sort order**:

1. `qspan.start` (ascending)
2. `hilen()` (descending)
3. `len()` (descending)
4. `matcher_order` (ascending)

### Rust Sorting (match_refine.rs:405-412, 599-606)

```rust
matches.sort_by(|a, b| {
    a.start_token
        .cmp(&b.start_token)
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        .then_with(|| a.rule_identifier.cmp(&b.rule_identifier))
});
```

**Sort order**:

1. `start_token` (ascending) — **DIFFERENT**: uses `start_token` not `qspan.start`
2. `hilen` (descending)
3. `matched_length` (descending)
4. `matcher_order()` (ascending)
5. `rule_identifier` (ascending) — **ADDITIONAL**: Python doesn't have this tiebreaker

### Difference Analysis

| Aspect | Python | Rust | Severity |
|--------|--------|------|----------|
| Primary sort | `qspan.start` | `start_token` | **HIGH** |
| Tiebreaker | 4 criteria | 5 criteria (adds rule_identifier) | **MEDIUM** |

**Impact**: `qspan.start` may differ from `start_token` when matches have gaps. The `qspan` represents the actual matched token positions, while `start_token` might be a line number proxy. This could change match ordering in edge cases.

---

## 3. Containment Checks (`qcontains`, `ispan`)

### Python `qcontains` (match.py)

Defined on the `LicenseMatch` class, checks if one match's query span contains another.

### Rust `qcontains` (models.rs)

Similar implementation but on the `LicenseMatch` struct.

**Status**: Needs detailed comparison of the span intersection logic.

---

## 4. Expression Subsumption (`licensing_contains`)

### Python Implementation (models.py:2065-2073)

```python
def licensing_contains(self, other):
    """Return True if the license expression of this match contains other."""
    return self.licensing.contains(
        self.license_expression, 
        other.license_expression
    )
```

**Key**: Delegates to `self.licensing.contains()` from the `license-expression` Python library.

### Rust Implementation (expression.rs:444-506)

```rust
pub fn licensing_contains(container: &str, contained: &str) -> bool {
    // Custom implementation with pattern matching on expression types
    match (&simplified_container, &simplified_contained) {
        (LicenseExpression::And { .. }, LicenseExpression::And { .. })
        | (LicenseExpression::Or { .. }, LicenseExpression::Or { .. }) => {
            let container_args = get_flat_args(&simplified_container);
            let contained_args = get_flat_args(&simplified_contained);
            contained_args.iter().all(|c| 
                container_args.iter().any(|ca| expressions_equal(ca, c))
            )
        }
        // ... other cases
    }
}
```

**Key**: Custom Rust implementation using `parse_expression` and `simplify_expression`.

### Difference Analysis

| Aspect | Python | Rust | Severity |
|--------|--------|------|----------|
| Implementation | Delegates to `license-expression` library | Custom implementation | **HIGH** |
| Testing | Battle-tested library | New implementation | **HIGH** |
| Edge cases | Handles all SPDX expression nuances | May miss edge cases | **HIGH** |

**Impact**: This is a **CRITICAL** difference. The `license-expression` library has been extensively tested with complex SPDX expressions including:

- WITH exceptions
- AND/OR combinations
- License refs
- Plus (+) operators
- Nested expressions

The Rust custom implementation may handle these differently.

**Recommendation**: Run comprehensive tests comparing `licensing_contains` outputs for all expression combinations.

---

## 5. Coverage Calculation (`match_coverage`)

### Python Implementation

Uses `match.coverage()` which computes:

```python
coverage = match.len() / match.rule.length
```

### Rust Implementation

Uses `match_coverage` field on `LicenseMatch` struct, computed as `icoverage()`:

```rust
fn icoverage(&self) -> f32 {
    self.matched_length as f32 / self.rule_length as f32
}
```

**Status**: Appears equivalent but needs verification of `len()` vs `matched_length` semantics.

---

## 6. `hilen` Calculation and Usage

### Python `hilen` (match.py:432)

```python
def hilen(self):
    """Return the "high value" length for this match."""
    return self._hilen if hasattr(self, '_hilen') else self.len()
```

### Rust `hilen` (models.rs:370)

```rust
pub fn hilen(&self) -> usize {
    self.hilen
}
```

**Key difference**: Rust stores `hilen` as a field, Python may compute it dynamically.

**Usage**: `hilen` is critical in sorting and tiebreaking for overlap resolution.

---

## 7. Overlap Thresholds

Both implementations use identical thresholds:

| Threshold | Value |
|-----------|-------|
| OVERLAP_SMALL | 0.10 |
| OVERLAP_MEDIUM | 0.40 |
| OVERLAP_LARGE | 0.70 |
| OVERLAP_EXTRA_LARGE | 0.90 |

**Verdict**: ✅ Identical.

---

## 8. `filter_overlapping_matches` Logic Comparison

### Python (match.py:1187-1523)

Key logic branches:

1. EXTRA_LARGE overlap: remove smaller match
2. LARGE overlap: consider hilen tiebreaker
3. MEDIUM overlap: check `licensing_contains`
4. SMALL overlap: check `surround()` + `licensing_contains`
5. Previous/next combined containment check (lines 1486-1507)

### Rust (match_refine.rs:588-790)

Key logic branches:

1. EXTRA_LARGE overlap: remove smaller match
2. LARGE overlap: consider hilen tiebreaker
3. MEDIUM overlap: check `licensing_contains_match`
4. SMALL overlap: check surrounding + `licensing_contains`
5. **MISSING**: Previous/next combined containment check

### Difference Analysis

| Aspect | Python | Rust | Severity |
|--------|--------|------|----------|
| Previous/next combined check | Present (lines 1486-1507) | **MISSING** | **HIGH** |

**Impact**: Python has additional logic to discard a match if it's mostly contained (90%+) in the combination of a previous and next match that don't overlap. This is missing in Rust.

---

## 9. `filter_contained_matches` Logic Comparison

### Python (match.py:1093-1184)

```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)

# Key checks:
# 1. Equal spans: remove lower coverage
# 2. qcontains: remove contained match
```

### Rust (match_refine.rs:374-455)

```rust
matches.sort_by(|a, b| {
    a.start_token.cmp(&b.start_token)
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        .then_with(|| a.rule_identifier.cmp(&b.rule_identifier))
});

// Key checks:
// 1. spans_equal: compare rule_quality
// 2. qcontains OR licensing_contains_match: remove contained
```

### Difference Analysis

| Aspect | Python | Rust | Severity |
|--------|--------|------|----------|
| Primary sort | `qspan.start` | `start_token` | **HIGH** |
| Contained check | `qcontains` only | `qcontains` OR `licensing_contains` | **MEDIUM** |
| Tiebreaker | 4 criteria | 5 criteria | **MEDIUM** |

**Note**: Rust's addition of `licensing_contains_match` in `filter_contained_matches` may be intentional to catch expression-level containment, but Python only uses spatial (`qcontains`) containment here.

---

## 10. `restore_non_overlapping` Logic Comparison

### Python (match.py:1526-1548)

```python
def restore_non_overlapping(matches, discarded):
    all_matched_qspans = Span().union(*(m.qspan for m in matches))
    
    for disc in merge_matches(discarded):
        if not disc.qspan & all_matched_qspans:
            to_keep_append(disc)
        else:
            to_discard_append(disc)
    
    return to_keep, to_discard
```

### Rust (match_refine.rs:796-820)

```rust
fn restore_non_overlapping(
    kept: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    let all_matched_qspans = kept
        .iter()
        .fold(Span::new(), |acc, m| acc.union_span(&match_to_span(m)));

    let merged_discarded = merge_overlapping_matches(&discarded);

    for disc in merged_discarded {
        let disc_span = match_to_span(&disc);
        if !disc_span.intersects(&all_matched_qspans) {
            to_keep.push(disc);
        } else {
            to_discard.push(disc);
        }
    }

    (to_keep, to_discard)
}
```

**Verdict**: ✅ Equivalent logic.

---

## 11. Function Name Mapping

| Python Function | Rust Function | File | Status |
|-----------------|---------------|------|--------|
| `refine_matches` | `refine_matches` | match.py:2691 / match_refine.rs:1509 | ✅ Equivalent |
| `merge_matches` | `merge_overlapping_matches` | match.py / match_refine.rs | ✅ Equivalent |
| `filter_matches_missing_required_phrases` | `filter_matches_missing_required_phrases` | match.py:2154 / match_refine.rs | ⚠️ Needs review |
| `filter_spurious_matches` | `filter_spurious_matches` | match.py:1768 / match_refine.rs:545 | ⚠️ Needs review |
| `filter_below_rule_minimum_coverage` | `filter_below_rule_minimum_coverage` | match.py:1551 / match_refine.rs | ⚠️ Needs review |
| `filter_matches_to_spurious_single_token` | `filter_matches_to_spurious_single_token` | match.py / match_refine.rs | ⚠️ Needs review |
| `filter_too_short_matches` | `filter_too_short_matches` | match.py / match_refine.rs | ⚠️ Needs review |
| `filter_short_matches_scattered_on_too_many_lines` | `filter_short_matches_scattered_on_too_many_lines` | match.py:1940 / match_refine.rs | ⚠️ Needs review |
| `filter_invalid_matches_to_single_word_gibberish` | `filter_invalid_matches_to_single_word_gibberish` | match.py / match_refine.rs | ⚠️ Needs review |
| `filter_contained_matches` | `filter_contained_matches` | match.py:1093 / match_refine.rs:374 | ⚠️ Differences |
| `filter_overlapping_matches` | `filter_overlapping_matches` | match.py:1187 / match_refine.rs:588 | ⚠️ Differences |
| `restore_non_overlapping` | `restore_non_overlapping` | match.py:1526 / match_refine.rs:796 | ✅ Equivalent |
| `filter_false_positive_matches` | `filter_false_positive_matches` | match.py:2124 / match_refine.rs:469 | ⚠️ Needs review |
| `filter_false_positive_license_lists_matches` | `filter_false_positive_license_lists_matches` | match.py:2408 / match_refine.rs:897 | ⚠️ Needs review |
| `licensing_contains` (method) | `licensing_contains` (function) | models.py:2065 / expression.rs:444 | ❌ **Different impl** |
| `hilen` (method) | `hilen` (method) | match.py:432 / models.rs:370 | ⚠️ Needs review |

---

## 12. Summary of Critical Differences

### HIGH Severity

| # | Difference | Location | Impact |
|---|------------|----------|--------|
| 1 | `licensing_contains` implementation | expression.rs vs license-expression lib | May produce different results for complex expressions |
| 2 | Sorting uses `start_token` vs `qspan.start` | match_refine.rs:405, 599 | May reorder matches with gaps |
| 3 | Missing previous/next combined check | filter_overlapping_matches | May keep matches Python would discard |
| 4 | `filter_contained_matches` adds `licensing_contains` | match_refine.rs:439 | May discard matches Python would keep |

### MEDIUM Severity

| # | Difference | Location | Impact |
|---|------------|----------|--------|
| 5 | Additional `rule_identifier` tiebreaker | sorting | Deterministic ordering differs |
| 6 | `hilen` stored vs computed | models.rs | May differ if computed differently |

---

## 13. Recommended Investigation Areas

1. **`licensing_contains` parity testing**: Create test cases for all SPDX expression patterns and compare Python vs Rust outputs.

2. **`qspan.start` vs `start_token`**: Investigate if `start_token` correctly maps to `qspan.start` semantics.

3. **Previous/next combined containment**: Implement the missing logic in Rust's `filter_overlapping_matches`.

4. **Golden tests**: Run detection on diverse test files and compare output JSON structures field-by-field.

5. **`hilen` computation**: Verify that Rust's stored `hilen` matches Python's computed `hilen` in all cases.

---

## 14. Appendix: Key File References

### Python Files

| File | Purpose |
|------|---------|
| `reference/scancode-toolkit/src/licensedcode/detection.py` | Main detection flow, LicenseDetection class |
| `reference/scancode-toolkit/src/licensedcode/match.py` | LicenseMatch class, all filter/merge functions |
| `reference/scancode-toolkit/src/licensedcode/models.py` | Rule class, `licensing_contains` delegation |
| `reference/scancode-toolkit/src/licensedcode/query.py` | Query class, query run building |

### Rust Files

| File | Purpose |
|------|---------|
| `src/license_detection/detection.rs` | Main detection flow, LicenseDetection struct |
| `src/license_detection/match_refine.rs` | All filter/merge functions |
| `src/license_detection/expression.rs` | Expression parsing, `licensing_contains` |
| `src/license_detection/models.rs` | LicenseMatch struct, helper methods |
