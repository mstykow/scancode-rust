# Scoring and Confidence Calculation Audit

This document compares the scoring and confidence calculation algorithms between Python ScanCode Toolkit and the Rust implementation, identifying differences and potential behavioral variations.

## Executive Summary

**Overall Status**: ✅ Core formulas match, minor implementation differences exist

The Rust implementation correctly replicates Python's scoring formulas with these notes:
- Match score calculation is **functionally equivalent** 
- Coverage calculation is **identical**
- Detection score calculation **differs in weighting** (needs investigation)
- Relevance handling is **correct**
- Threshold filtering logic is **present but applied differently**

---

## 1. Match Score Calculation

### Python Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:592-619`

```python
def score(self):
    """
    Return the score for this match as a rounded float between 0 and 100.
    """
    # relevance is a number between 0 and 100. Divide by 100
    relevance = self.rule.relevance / 100
    if not relevance:
        return 0

    qmagnitude = self.qmagnitude()

    if not qmagnitude:
        return 0

    # FIXME: this should exposed as an q/icoverage() method instead
    query_coverage = self.len() / qmagnitude
    rule_coverage = self._icoverage()
    
    if query_coverage < 1 and rule_coverage < 1:
        # use rule coverage in this case
        return round(rule_coverage * relevance * 100, 2)
    
    return round(query_coverage * rule_coverage * relevance * 100, 2)
```

### Rust Implementation

**File**: `src/license_detection/match_refine/merge.rs:241-260`

```rust
fn compute_match_score(m: &LicenseMatch, query: &Query) -> f32 {
    let relevance = m.rule_relevance as f32 / 100.0;
    if relevance < 0.001 {
        return 0.0;
    }

    let qmagnitude = m.qmagnitude(query);
    if qmagnitude == 0 {
        return 0.0;
    }

    let query_coverage = m.len() as f32 / qmagnitude as f32;
    let rule_coverage = m.icoverage();

    if query_coverage < 1.0 && rule_coverage < 1.0 {
        return (rule_coverage * relevance * 100.0).round();
    }

    (query_coverage * rule_coverage * relevance * 100.0).round()
}
```

### Comparison

| Aspect | Python | Rust | Status |
|--------|--------|------|--------|
| Formula (normal case) | `query_coverage × rule_coverage × relevance × 100` | Same | ✅ Match |
| Formula (both < 1) | `rule_coverage × relevance × 100` | Same | ✅ Match |
| Rounding | `round(..., 2)` | `.round()` | ⚠️ Different |
| Relevance check | `if not relevance: return 0` | `if relevance < 0.001: return 0.0` | ✅ Equivalent |
| Zero qmagnitude check | `if not qmagnitude: return 0` | Same | ✅ Match |

**Behavioral Difference**: 
- Python uses `round(value, 2)` which rounds to 2 decimal places
- Rust uses `.round()` which rounds to nearest integer
- **Impact**: Minor differences in final score values (e.g., 99.5 vs 100.0)

**Recommendation**: Use `(value * 100.0).round() / 100.0` in Rust to match Python's 2 decimal place rounding.

---

## 2. Coverage Calculation

### Python Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:472-486`

```python
def _icoverage(self):
    """
    Return the coverage of this match to the matched rule as a float between
    0 and 1.
    """
    if not self.rule.length:
        return 0
    return self.len() / self.rule.length

def coverage(self):
    """
    Return the coverage of this match to the matched rule as a rounded float
    between 0 and 100.
    """
    return round(self._icoverage() * 100, 2)
```

### Rust Implementation

**File**: `src/license_detection/models/license_match.rs:329-334`

```rust
pub fn icoverage(&self) -> f32 {
    if self.rule_length == 0 {
        return 0.0;
    }
    self.len() as f32 / self.rule_length as f32
}
```

**Coverage is stored directly as percentage (0-100) in `match_coverage` field.**

### Comparison

| Aspect | Python | Rust | Status |
|--------|--------|------|--------|
| Formula | `len() / rule.length` | Same | ✅ Match |
| Zero-length handling | `if not self.rule.length: return 0` | Same | ✅ Match |
| Percentage conversion | `× 100`, rounded to 2 decimals | Stored as percentage | ✅ Match |
| Internal use | `_icoverage()` returns 0-1 | `icoverage()` returns 0-1 | ✅ Match |

**Status**: ✅ **Identical behavior**

---

## 3. Relevance Handling

### Python Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/models.py:2573-2619`

```python
def compute_relevance(length):
    """
    Return a computed ``relevance`` given a ``length`` and a threshold.
    The relevance is a integer between 0 and 100.
    """
    if length > 18:
        return 100
    return {
        0: 0,
        1: 5,
        2: 11,
        3: 16,
        4: 22,
        5: 27,
        6: 33,
        7: 38,
        8: 44,
        9: 50,
        10: 55,
        11: 61,
        12: 66,
        13: 72,
        14: 77,
        15: 83,
        16: 88,
        17: 94,
        18: 100,
    }.get(length, 100)
```

Relevance is loaded from rule data or computed based on rule length.

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:387-396`

```python
def relevance(self):
    """
    Return the ``relevance`` of this detection.
    """
    return compute_relevance(self.rules_length())
```

### Rust Implementation

Relevance is loaded from rule data during index building (`Rule.relevance: u8`).

**File**: `src/license_detection/models/license_match.rs:62`

```rust
pub rule_relevance: u8,  // 0-100
```

### Comparison

| Aspect | Python | Rust | Status |
|--------|--------|------|--------|
| Relevance range | 0-100 (integer) | 0-100 (u8) | ✅ Match |
| Storage | Rule attribute | Rule attribute | ✅ Match |
| Usage in score | `relevance / 100` | `relevance as f32 / 100.0` | ✅ Match |

**Status**: ✅ **Identical behavior**

---

## 4. Detection Score Calculation

### Python Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:398-409`

```python
def score(self):
    """
    Return the score for this detection as a rounded float between 0 and 100.

    This is computed as the sum of the underlying matches score weighted
    by the length of a match to the overall detection length.
    """
    length = self.length
    weighted_scores = (m.score() * (m.len() / length) for m in self.matches)
    return min([round(sum(weighted_scores), 2), 100])
```

### Rust Implementation

**File**: `src/license_detection/detection/analysis.rs:360-376`

```rust
pub fn compute_detection_score(matches: &[LicenseMatch]) -> f32 {
    if matches.is_empty() {
        return 0.0;
    }

    let total_weight: f32 = matches.iter().map(|m| m.match_coverage).sum();
    if total_weight == 0.0 {
        return 0.0;
    }

    let weighted_score: f32 = matches
        .iter()
        .map(|m| m.score * m.match_coverage * m.rule_relevance as f32 / 100.0)
        .sum();

    (weighted_score / total_weight).min(100.0)
}
```

### Comparison

| Aspect | Python | Rust | Status |
|--------|--------|------|--------|
| Weighting factor | `m.len() / length` | `m.match_coverage` | ❌ **Different** |
| Score contribution | `m.score() × weight` | `m.score × match_coverage × relevance / 100` | ❌ **Different** |
| Normalization | `sum(weighted_scores)` | `weighted_score / total_weight` | ❌ **Different** |
| Rounding | `round(..., 2)` | `.min(100.0)` | ⚠️ Different |

**CRITICAL BEHAVIORAL DIFFERENCE**:

**Python formula**:
```
detection_score = sum(match.score × (match.len() / detection_length))
```
- Weights by **match length relative to total detection length**
- Does NOT include relevance in detection score
- Simple weighted average

**Rust formula**:
```
detection_score = sum(match.score × match_coverage × relevance / 100) / sum(match_coverage)
```
- Weights by **match coverage**
- Includes relevance in weighting
- Coverage-weighted average

**Impact**: This will produce **different detection scores** in many cases, especially:
- When matches have different coverage levels
- When matches have different relevance values
- When matches have very different lengths

**Example**:
- Match 1: score=95, len=100, coverage=100%, relevance=100
- Match 2: score=30, len=10, coverage=50%, relevance=80

Python detection score: `(95 × 100/110) + (30 × 10/110) = 86.36 + 2.73 = 89.09`

Rust detection score: `(95 × 100 × 1.0 + 30 × 50 × 0.8) / (100 + 50) = (9500 + 1200) / 150 = 71.33`

**Recommendation**: Align Rust implementation with Python's length-weighted formula.

---

## 5. Threshold Filtering

### Python Implementation

**min_score filtering**:

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:1590-1619`

```python
def filter_matches_below_minimum_score(matches, min_score=100, ...):
    """
    Return a filtered list ... by removing matches scoring below the provided ``min_score``.
    """
    if not min_score:
        return matches, []

    kept = []
    discarded = []
    for match in matches:
        if match.score() < min_score:
            match.discard_reason = reason
            discarded.append(match)
        else:
            kept.append(match)
    return kept, discarded
```

**minimum_coverage filtering**:

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:1551-1587`

```python
def filter_below_rule_minimum_coverage(matches, ...):
    """
    Return a filtered list ... by removing matches that have a coverage 
    below a rule-defined minimum coverage.
    """
    kept = []
    discarded = []
    for match in matches:
        # always keep exact matches
        if match.matcher != MATCH_SEQ:
            kept.append(match)
            continue

        if match.coverage() < match.rule.minimum_coverage:
            match.discard_reason = reason
            discarded.append(match)
        else:
            kept.append(match)
    return kept, discarded
```

### Rust Implementation

**File**: `src/license_detection/match_refine/filter_low_quality.rs:68-90`

```rust
pub(crate) fn filter_below_rule_minimum_coverage(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    matches.iter().filter(|m| {
        if m.matcher != "3-seq" {
            return true;
        }

        let rid = m.rid;
        if let Some(rule) = index.rules_by_rid.get(rid)
            && let Some(min_cov) = rule.minimum_coverage
        {
            return m.match_coverage >= min_cov as f32;
        }
        true
    }).cloned().collect()
}
```

**Note**: `min_score` filtering is handled at detection classification level:

**File**: `src/license_detection/detection/analysis.rs:440-453`

```rust
pub(super) fn classify_detection(detection: &LicenseDetection, min_score: f32) -> bool {
    let score = compute_detection_score(&detection.matches);
    let meets_score_threshold = score >= min_score - 0.01;
    let not_false_positive = !is_false_positive(&detection.matches);
    meets_score_threshold && not_false_positive
}
```

### Comparison

| Aspect | Python | Rust | Status |
|--------|--------|------|--------|
| min_score filtering | At match refinement | At detection classification | ⚠️ **Different stage** |
| min_coverage filtering | Per-match, only for seq matches | Same | ✅ Match |
| Exact match handling | Always kept | Always kept | ✅ Match |
| Threshold comparison | `match.score() < min_score` | `score >= min_score - 0.01` | ⚠️ Fuzzy comparison |

**BEHAVIORAL DIFFERENCE**:

**Python**: Filters **individual matches** based on score at refinement stage
- Applied in `refine_matches()` at line 2819-2820
- Low-scoring matches are discarded before detection assembly

**Rust**: Filters **entire detections** based on combined score at classification stage  
- Applied in `classify_detection()`
- Low-scoring detections are not created as valid detections

**Impact**: 
- Different behavior when `min_score` is configured
- Python may include matches that contribute to a high-scoring detection even if they individually have low scores
- Rust may exclude entire detections if the combined score is low

**Recommendation**: Clarify whether filtering should happen at match or detection level.

---

## 6. qmagnitude Calculation

### Python Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:488-527`

```python
def qmagnitude(self):
    """
    Return the maximal query length represented by this match start and end
    in the query. This number represents the full extent of the matched
    query region including matched, unmatched AND unknown tokens, but
    excluding STOPWORDS.
    """
    query = self.query
    qspan = self.qspan
    qmagnitude = self.qregion_len()

    if query:
        # Compute a count of unknown tokens that are inside the matched range
        unknowns_pos = qspan & query.unknowns_span
        qspe = qspan.end
        unknowns_pos = (pos for pos in unknowns_pos if pos != qspe)
        qry_unkxpos = query.unknowns_by_pos
        unknowns_in_match = sum(qry_unkxpos[pos] for pos in unknowns_pos)

        # update the magnitude by adding the count of unknowns in the match.
        qmagnitude += unknowns_in_match

    return qmagnitude
```

### Rust Implementation

**File**: `src/license_detection/models/license_match.rs:273-290`

```rust
pub fn qmagnitude(&self, query: &crate::license_detection::query::Query) -> usize {
    let qregion_len = self.qregion_len();
    let positions: Vec<usize> = if let Some(qspan_positions) = &self.qspan_positions {
        qspan_positions.clone()
    } else {
        (self.start_token..self.end_token).collect()
    };
    if positions.is_empty() {
        return qregion_len;
    }
    let max_pos = *positions.iter().max().unwrap_or(&0);
    let unknowns_in_match: usize = positions
        .iter()
        .filter(|&&pos| pos != max_pos)
        .filter_map(|&pos| query.unknowns_by_pos.get(&Some(pos as i32)))
        .sum();
    qregion_len + unknowns_in_match
}
```

### Comparison

| Aspect | Python | Rust | Status |
|--------|--------|------|--------|
| Base calculation | `qregion_len()` | Same | ✅ Match |
| Unknown handling | Count unknowns in matched range | Same | ✅ Match |
| Exclusion of end position | `pos != qspe` | `pos != max_pos` | ✅ Match |
| Stopword handling | "excluding STOPWORDS" | Not explicit | ⚠️ Verify |

**Status**: ✅ **Likely equivalent**, but verify stopword handling

---

## 7. Summary of Differences

### Critical Issues

1. **Detection Score Formula** (HIGH PRIORITY)
   - Python: Length-weighted average of match scores
   - Rust: Coverage-weighted with relevance
   - **Impact**: Different detection scores in multi-match scenarios
   - **File**: `src/license_detection/detection/analysis.rs:360-376`

### Minor Issues

2. **Rounding Precision** (LOW PRIORITY)
   - Python: 2 decimal places
   - Rust: Nearest integer
   - **Impact**: Minor score differences (< 1.0)
   - **File**: `src/license_detection/match_refine/merge.rs:256,259`

3. **Threshold Filtering Stage** (MEDIUM PRIORITY)
   - Python: Filters matches
   - Rust: Filters detections
   - **Impact**: Different behavior with min_score parameter
   - **File**: `src/license_detection/detection/analysis.rs:440-453`

### Verified Correct

- ✅ Match score calculation (normal case)
- ✅ Match score calculation (both coverages < 1)
- ✅ Coverage calculation
- ✅ Relevance handling
- ✅ qmagnitude calculation (likely)
- ✅ minimum_coverage filtering per match

---

## 8. Recommendations

### Priority 1: Fix Detection Score Calculation

Align Rust with Python's length-weighted formula:

```rust
pub fn compute_detection_score(matches: &[LicenseMatch]) -> f32 {
    if matches.is_empty() {
        return 0.0;
    }

    let total_length: usize = matches.iter().map(|m| m.matched_length).sum();
    if total_length == 0 {
        return 0.0;
    }

    let weighted_score: f32 = matches
        .iter()
        .map(|m| m.score * (m.matched_length as f32 / total_length as f32))
        .sum();

    weighted_score.min(100.0)
}
```

### Priority 2: Verify min_score Filtering Behavior

Decide whether filtering should happen at:
- Match level (Python's approach)
- Detection level (Rust's current approach)
- Both levels

### Priority 3: Align Rounding

Use 2-decimal-place rounding to match Python:

```rust
fn round_to_2_decimals(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}
```

---

## 9. Testing Recommendations

Create test cases to verify:

1. **Detection score with multiple matches** of different lengths/coverages
2. **Match score rounding** at boundaries (e.g., 99.95 → 100.0)
3. **min_score filtering** at both match and detection levels
4. **Relevance impact** on scores (especially relevance=0, relevance=50)
5. **Edge cases**: Empty matches, zero-length rules, zero qmagnitude

---

## References

- Python match.py: `reference/scancode-toolkit/src/licensedcode/match.py`
- Python detection.py: `reference/scancode-toolkit/src/licensedcode/detection.py`
- Python models.py: `reference/scancode-toolkit/src/licensedcode/models.py`
- Rust license_match.rs: `src/license_detection/models/license_match.rs`
- Rust merge.rs: `src/license_detection/match_refine/merge.rs`
- Rust analysis.rs: `src/license_detection/detection/analysis.rs`
- Rust filter_low_quality.rs: `src/license_detection/match_refine/filter_low_quality.rs`
