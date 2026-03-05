# Match Refinement Pipeline: Python vs Rust Comparison

This document compares the match refinement pipeline between Python ScanCode Toolkit
and the Rust implementation, identifying algorithmic differences and potential behavioral
divergences.

## Overview

The match refinement pipeline processes raw license matches through a series of filters
to produce high-quality detections. Both implementations follow the same general structure
but have some differences in implementation details.

### Pipeline Order

**Python** (`refine_matches()` at match.py:2691-2833):
1. Merge matches (optional, default=True)
2. Filter matches missing required phrases
3. Filter spurious matches
4. Filter below rule minimum coverage
5. Filter spurious single-token matches
6. Filter too short matches
7. Filter scattered short matches
8. Filter invalid single-word gibberish
9. Merge matches (always)
10. Filter contained matches
11. Filter overlapping matches
12. Restore non-overlapping (contained)
13. Restore non-overlapping (overlapping)
14. Filter contained matches (again)
15. Filter false positive matches
16. Filter false positive license list matches
17. Filter below minimum score (optional)
18. Merge matches (final, optional)

**Rust** (`refine_matches()` at mod.rs:136-142):
1. Merge overlapping matches
2. Filter matches missing required phrases
3. Filter spurious matches
4. Filter below rule minimum coverage
5. Filter spurious single-token matches
6. Filter too short matches
7. Filter scattered short matches
8. Filter invalid single-word gibberish
9. Merge overlapping matches
10. Filter contained matches
11. Filter overlapping matches
12. Restore non-overlapping (contained)
13. Restore non-overlapping (overlapping)
14. Filter contained matches (again)
15. Filter false positive matches
16. Filter false positive license list matches
17. Merge overlapping matches (final)
18. Filter license references with text match
19. Update match scores

**Differences:**
- Rust does not have `min_score` parameter (always 0)
- Rust adds `filter_license_references_with_text_match` before score update
- Rust always runs merge at the end (Python only if `merge=True`)

---

## 1. False Positive Filtering

### Python Implementation

**Location:** `filter_false_positive_matches()` at match.py:2126-2151

**Logic:**
```python
for match in matches:
    if match.rule.is_false_positive:
        match.discard_reason = reason
        discarded_append(match)
    else:
        kept_append(match)
```

**Key details:**
- Checks `match.rule.is_false_positive` boolean flag
- Sets `discard_reason` to `DiscardReason.FALSE_POSITIVE`
- Returns tuple of (kept, discarded)

### Rust Implementation

**Location:** `filter_false_positive_matches()` at filter_low_quality.rs:391-407

**Logic:**
```rust
for m in matches {
    let rid = m.rid;
    if index.false_positive_rids.contains(&rid) {
        continue;
    }
    filtered.push(m.clone());
}
```

**Key details:**
- Checks `index.false_positive_rids` HashSet for the rule ID
- Does not set discard_reason (no tracking)
- Returns Vec<LicenseMatch> (only kept matches)

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| Data structure | `rule.is_false_positive` boolean | `index.false_positive_rids` HashSet |
| Discard tracking | Sets `discard_reason` | No tracking |
| Return value | (kept, discarded) | kept only |

**Potential behavioral difference:** None - same filtering logic, just different data access pattern.

---

## 2. Contained Match Filtering

### Python Implementation

**Location:** `filter_contained_matches()` at match.py:1075-1184

**Sorting:**
```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
matches = sorted(matches, key=sorter)
```

**Logic:**
1. Sort by: qstart ASC, hilen DESC, len DESC, matcher_order ASC
2. For each pair (current, next):
   - If next.qend > current.qend: break (no overlap possible)
   - If qspans equal: keep higher coverage
   - If current.qcontains(next): discard next
   - If next.qcontains(current): discard current

**Key method:** `qcontains()` checks if one match's qspan is contained in another's.

### Rust Implementation

**Location:** `filter_contained_matches()` at handle_overlaps.rs:40-96

**Sorting:**
```rust
matches.sort_by(|a, b| {
    a.qstart()
        .cmp(&b.qstart())
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

**Logic:**
1. Sort by: qstart ASC, hilen DESC, matched_length DESC, matcher_order ASC
2. For each pair (current, next):
   - If next.end_token > current.end_token: break
   - If qspans equal: keep higher coverage
   - If current.qcontains(next): discard next
   - If next.qcontains(current): discard current

**Key method:** `qcontains()` at models.rs checks token position containment.

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| Sorting key | `-m.len()` | `b.matched_length.cmp(&a.matched_length)` |
| Early break condition | `next_match.qend > current_match.qend` | `next.end_token > current.end_token` |

**Potential behavioral difference:** The sorting uses `len()` in Python and `matched_length` in Rust. These should be equivalent but verify `LicenseMatch.len()` returns `matched_length`.

---

## 3. Overlap Resolution

### Python Implementation

**Location:** `filter_overlapping_matches()` at match.py:1187-1523

**Constants:**
```python
OVERLAP_SMALL = 0.10
OVERLAP_MEDIUM = 0.40
OVERLAP_LARGE = 0.70
OVERLAP_EXTRA_LARGE = 0.90
```

**Logic:**
1. Sort by: qstart ASC, hilen DESC, len DESC, matcher_order ASC
2. Skip pairs where `next_match.qstart >= current_match.qend` (no overlap)
3. Skip if both matches are false positives and `skip_contiguous_false_positive=True`
4. Calculate overlap ratios:
   - `overlap_ratio_to_next = overlap / next_match.len()`
   - `overlap_ratio_to_current = overlap / current_match.len()`
5. Apply tiered filtering:
   - **Extra-large overlap (>= 90%):** Discard shorter match
   - **Large overlap (>= 70%):** Discard shorter match with lower hilen
   - **Medium overlap (>= 40%):** Use licensing_contains and license tag rules
   - **Small overlap (>= 10%):** Use surround check with licensing_contains
6. Sandwich detection: Discard current if 90%+ contained in previous+next union

### Rust Implementation

**Location:** `filter_overlapping_matches()` at handle_overlaps.rs:121-360

**Constants:**
```rust
const OVERLAP_SMALL: f64 = 0.10;
const OVERLAP_MEDIUM: f64 = 0.40;
const OVERLAP_LARGE: f64 = 0.70;
const OVERLAP_EXTRA_LARGE: f64 = 0.90;
```

**Logic:**
Same structure as Python with these additions:
- **Candidate scores:** Checks `candidate_resemblance` and `candidate_containment` for tie-breaking
- Both false positive check uses `index.false_positive_rids`

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| False positive check | `match.rule.is_false_positive` | `index.false_positive_rids.contains(&rid)` |
| Tie-breaking | Length/hilen only | Candidate scores, then hilen |
| License tag check | `current_match.rule.ends_with_license` | `index.rules_by_rid.get(rid).ends_with_license` |

**Potential behavioral differences:**
1. **Candidate score tie-breaking:** Rust checks `candidate_resemblance` and `candidate_containment` before falling back to hilen. This could cause different results when matches have these scores set.
2. **Rule attribute access:** Rust looks up rule from index, Python accesses directly.

---

## 4. Spurious Match Filtering

### Python Implementation

**Location:** `filter_spurious_matches()` at match.py:1768-1836

**Thresholds:**
```python
# For matcher in (MATCH_SEQ, MATCH_UNKNOWN):
if mlen < 10 and (qdens < 0.1 or idens < 0.1): discard
if mlen < 15 and (qdens < 0.2 or idens < 0.2): discard
if mlen < 20 and hilen < 5 and (qdens < 0.3 or idens < 0.3): discard
if mlen < 30 and hilen < 8 and (qdens < 0.4 or idens < 0.4): discard
if qdens < 0.4 or idens < 0.4: discard
```

### Rust Implementation

**Location:** `filter_spurious_matches()` at filter_low_quality.rs:19-56

**Thresholds:** Identical to Python

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| Matcher types | `"3-seq"`, `"5-unknown"` | `"3-seq"`, `"5-unknown"` |
| Density calculation | `m.qdensity()`, `m.idensity()` | `m.qdensity(query)`, `m.idensity()` |

**Note:** Rust passes `query` to `qdensity()` for unknown token calculation. Verify both compute densities the same way.

---

## 5. Required Phrase Checking

### Python Implementation

**Location:** `filter_matches_missing_required_phrases()` at match.py:2154-2322

**Logic:**
1. Check `rule.is_continuous` or `rule.is_required_phrase` first
2. For `is_continuous`: verify match is continuous with no gaps
3. For required phrases (`{{...}}`):
   - Verify all `required_phrase_spans` are in `ispan`
   - Verify qkey_span is continuous (no gaps)
   - Verify no unknown tokens in required phrase region
   - Verify stopwords match between query and rule

**Special case:** Single match is kept unless `is_continuous` or `is_required_phrase`.

### Rust Implementation

**Location:** `filter_matches_missing_required_phrases()` at filter_low_quality.rs:147-324

**Logic:** Same structure, with these differences:
- Uses `query.unknowns_by_pos` HashMap (Python uses dict)
- Uses `query.stopwords_by_pos` HashMap
- Uses `rule.stopwords_by_pos` HashMap

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| Unknown check | `qpos in unknown_by_pos` | `query.unknowns_by_pos.contains_key(&Some(qpos as i32))` |
| Stopword check | `istopwords_by_pos_get(ipos)` | `rule.stopwords_by_pos.get(&ipos)` |

**Potential issue:** Rust uses `Some(qpos as i32)` as key, Python uses `qpos` directly. Verify key types match.

---

## 6. Minimum Coverage Filtering

### Python Implementation

**Location:** `filter_below_rule_minimum_coverage()` at match.py:1551-1587

**Logic:**
```python
for match in matches:
    if match.matcher != MATCH_SEQ:
        kept_append(match)  # Always keep exact matches
        continue
    if match.coverage() < match.rule.minimum_coverage:
        discarded_append(match)
    else:
        kept_append(match)
```

### Rust Implementation

**Location:** `filter_below_rule_minimum_coverage()` at filter_low_quality.rs:68-90

**Logic:**
```rust
matches.iter().filter(|m| {
    if m.matcher != "3-seq" { return true; }
    if let Some(rule) = index.rules_by_rid.get(rid)
        && let Some(min_cov) = rule.minimum_coverage
    {
        return m.match_coverage >= min_cov as f32;
    }
    true
}).cloned().collect()
```

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| Exact match check | `match.matcher != MATCH_SEQ` | `m.matcher != "3-seq"` |
| Coverage field | `match.coverage()` method | `m.match_coverage` field |
| Min coverage | `match.rule.minimum_coverage` | `rule.minimum_coverage` |

**Note:** Rust uses `match_coverage` field directly. Verify this is updated before this filter runs.

---

## 7. Match Merging

### Python Implementation

**Location:** `merge_matches()` at match.py:869-1068

**Constants:**
```python
MAX_DIST = 100  # Default max distance for merging
max_rule_side_dist = min((rule_length // 2) or 1, max_dist)
```

**Logic:**
1. Group matches by `rule.identifier`
2. Sort each group by: qstart ASC, hilen DESC, len DESC, matcher_order ASC
3. For each pair within distance threshold:
   - If equal qspan/ispan: remove duplicate
   - If equal ispan with overlap: keep denser qspan
   - If current.qcontains(next): remove next
   - If next.qcontains(current): remove current
   - If current.surround(next) and aligned: merge
   - If next.is_after(current): merge
   - If overlapping in sequence with equal overlap: merge

### Rust Implementation

**Location:** `merge_overlapping_matches()` at merge.rs:68-216

**Logic:** Same structure with minor differences:
- Uses `HashSet` for span position collection
- Combines matches with `combine_matches()` helper

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| Max distance | `MAX_DIST = 100` | `const MAX_DIST: usize = 100` |
| Distance calc | `current_match.qdistance_to(next_match)` | `current.qdistance_to(&next)` |
| Merge combine | `current_match.update(next_match)` | `rule_matches[i] = combine_matches(&current, &next)` |

---

## 8. Score Calculation

### Python Implementation

**Location:** `LicenseMatch.score()` at match.py:592-619

**Formula:**
```python
relevance = self.rule.relevance / 100
qmagnitude = self.qmagnitude()  # Includes unknowns
query_coverage = self.len() / qmagnitude
rule_coverage = self._icoverage()  # len() / rule.length

if query_coverage < 1 and rule_coverage < 1:
    return round(rule_coverage * relevance * 100, 2)
return round(query_coverage * rule_coverage * relevance * 100, 2)
```

### Rust Implementation

**Location:** `compute_match_score()` at merge.rs:241-260

**Formula:** Identical

```rust
let relevance = m.rule_relevance as f32 / 100.0;
let qmagnitude = m.qmagnitude(query);
let query_coverage = m.len() as f32 / qmagnitude as f32;
let rule_coverage = m.icoverage();

if query_coverage < 1.0 && rule_coverage < 1.0 {
    return (rule_coverage * relevance * 100.0).round();
}
(query_coverage * rule_coverage * relevance * 100.0).round()
```

**Differences:**
| Aspect | Python | Rust |
|--------|--------|------|
| qmagnitude | `self.qmagnitude()` method | `m.qmagnitude(query)` method |
| Rounding | `round(x, 2)` | `x.round()` |
| Return type | `float` | `f32` |

**Potential issue:** Python rounds to 2 decimal places, Rust rounds to nearest integer. This could cause score differences in output.

---

## Summary of Key Differences

### Critical Differences (May Affect Output)

1. **Score rounding:** Python uses 2 decimal places, Rust uses integer rounding. This affects final scores in output.

2. **Candidate scores in overlap resolution:** Rust considers `candidate_resemblance` and `candidate_containment` for tie-breaking during overlap filtering. Python does not have this.

3. **Unknown position keys:** Rust uses `Some(qpos as i32)` for unknown position lookups. Verify this matches Python's direct integer keys.

### Minor Differences (Unlikely to Affect Output)

1. **Data structure access:** Python accesses `match.rule.is_false_positive` directly; Rust uses `index.false_positive_rids.contains(&rid)`.

2. **Return value types:** Some Rust functions return only kept matches, Python returns (kept, discarded) tuples.

3. **Coverage field:** Rust uses `match_coverage` field directly in some places; Python calls `coverage()` method.

### Recommendations

1. **Fix score rounding:** Change Rust's `round()` to round to 2 decimal places for Python parity.

2. **Verify unknown position keys:** Ensure `query.unknowns_by_pos` uses the same key type as Python.

3. **Document candidate scores:** If candidate resemblance/containment are intentional additions, document them in the improvements directory.

4. **Add discard tracking:** Consider adding discard reason tracking for debugging parity with Python.
