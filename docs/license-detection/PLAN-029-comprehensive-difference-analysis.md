# PLAN-029: Comprehensive Difference Analysis

**Date**: 2026-02-23
**Status**: Analysis Complete - Implementation Pending
**Priority**: Critical
**Related**: All previous PLANs, golden test failures

## Executive Summary

A thorough analysis of the Rust and Python implementations identified **50+ differences** across 10 pipeline stages. This document consolidates findings from 11 investigation agents into a single reference for prioritizing fixes.

**Current State**: 86.1% golden test pass rate (3756/4363 tests passing)

---

## 1. CRITICAL DIFFERENCES (Fix First)

### 1.1 `restore_non_overlapping()` Uses Wrong Span Type

**Severity**: CRITICAL
**Impact**: ~100+ tests

**Python** (`match.py:1532-1541`):

```python
all_matched_qspans = Span().union(*(m.qspan for m in matches))
if not disc.qspan & all_matched_qspans:  # Uses TOKEN positions
```

**Rust** (`match_refine.rs:688-714`):

```rust
fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)  // Uses LINE positions!
}
```

**Fix**: Change Rust to use token positions (`start_token..end_token`) instead of line positions.

---

### 1.2 Score Calculation Formula Mismatch

**Severity**: CRITICAL
**Impact**: Affects all scoring/ranking decisions

**Python** (`match.py:592-619`):

```python
score = query_coverage * rule_coverage * relevance * 100
# where query_coverage = len() / qmagnitude()
# and qmagnitude includes unknown tokens
```

**Rust** (`match_refine.rs:421-425`):

```rust
m.score = m.match_coverage * m.rule_relevance as f32 / 100.0;
```

**Fix**: Implement full Python formula including `qmagnitude()` with unknown token accounting.

---

### 1.3 SPDX Key Mapping Missing

**Severity**: CRITICAL
**Impact**: ~50+ external tests

**Issue**: `0BSD` should map to `bsd-zero`, `AFL-1.1` to `afl-1.1`, etc.

**Location**: `src/license_detection/spdx_lid.rs:152-179`

**Fix**: Add missing SPDX-to-ScanCode key mappings.

---

### 1.4 BOM (Byte Order Mark) Not Stripped

**Severity**: CRITICAL
**Impact**: Files starting with UTF-8 BOM fail completely

**Location**: `src/license_detection/tokenize.rs`, `src/license_detection/query.rs`

**Fix**: Strip UTF-8 BOM (`\xef\xbb\xbf`) before tokenization.

---

## 2. HIGH SEVERITY DIFFERENCES

### 2.1 Missing Copyright Word Check in `is_false_positive()`

**Severity**: HIGH
**Impact**: False positives not filtered correctly

**Python** (`detection.py:1173-1185`):

```python
copyright_words = ["copyright", "(c)"]
has_copyrights = all(
    any(word in license_match.matched_text().lower() for word in copyright_words)
    for license_match in license_matches
)
if has_copyrights:
    return False
```

**Rust**: Missing this check entirely.

**Location**: `src/license_detection/detection.rs:310-372`

---

### 2.2 `qdensity()` Uses Wrong Metric

**Severity**: HIGH
**Impact**: Spurious match filtering differs

**Python**: Uses `qmagnitude()` which includes unknown tokens
**Rust**: Uses `qregion_len()` without unknown tokens

**Location**: `src/license_detection/models.rs:426-436`

---

### 2.3 Equal Ispan Match Selection Differs

**Severity**: HIGH
**Impact**: Different matches kept when ispans equal

**Python** (`match.py:949-970`):

```python
if current_match.ispan == next_match.ispan and current_match.overlap(next_match):
    if current_match.qspan.magnitude() <= next_match.qspan.magnitude():
        del rule_matches[j]  # Remove match with larger magnitude
```

**Rust** (`match_refine.rs:225-234`):

```rust
if current.ispan() == next.ispan() && current.qoverlap(&next) > 0 {
    if current.matched_length >= next.matched_length {
        rule_matches.remove(j);  // Different criterion!
```

**Fix**: Use `qspan.magnitude()` (span extent) instead of `matched_length` (position count).

---

### 2.4 Missing `merge_matches()` After Each Matching Phase

**Severity**: HIGH
**Impact**: ~200+ external tests

**Python** (`index.py:1040-1050`):

```python
matches = match.merge_matches(matches)  # Called after each phase
```

**Rust**: Only merges at end of `refine_matches()`.

**Location**: `src/license_detection/mod.rs:117-271`

---

### 2.5 Match Grouping Ignores Custom Threshold

**Severity**: HIGH
**Impact**: Detection grouping differs from Python

**Python**: `group_matches(license_matches, lines_threshold=custom)` respects parameter
**Rust**: `_proximity_threshold` parameter is ignored (prefixed with `_`)

**Location**: `src/license_detection/detection.rs:163-166`

---

### 2.6 Expression Normalization Missing

**Severity**: HIGH
**Impact**: ~30+ tests with complex expressions

**Issue**: Python normalizes `lgpl-2.1 WITH exception OR cpl-1.0 WITH exception` to `lzma-sdk-2006`

**Location**: `src/license_detection/expression.rs`, `src/license_detection/spdx_mapping.rs`

---

## 3. MEDIUM SEVERITY DIFFERENCES

### 3.1 Sort Order Missing `matcher_order`

**Python**: Uses `(identifier, start, -hilen, -len, matcher_order)` as sort key
**Rust**: Missing `matcher_order` in sort key

**Location**: `src/license_detection/match_refine.rs:169-175`

---

### 3.2 `filter_contained_matches()` More Aggressive

**Python**: Only uses `qcontains()` for token containment
**Rust**: Adds `licensing_contains_match()` for expression subsumption

**Impact**: Rust removes more matches (intentional enhancement but differs from Python)

**Location**: `src/license_detection/match_refine.rs:323-377`

---

### 3.3 License Key Case Normalization

**Python**: Preserves case (`"MIT"` stays `"MIT"`)
**Rust**: Lowercases all keys (`"MIT"` becomes `"mit"`)

**Location**: `src/license_detection/expression.rs:719-727`

---

### 3.4 `GPL-2.0+` Not Supported

**Issue**: The `+` character is not allowed in license keys

**Location**: `src/license_detection/expression.rs:692-701`
**Test**: `test_parse_gpl_plus_license` is ignored

---

### 3.5 Previous/Next Overlap Check Differs

**Python**: Uses `overlap()` method
**Rust**: Manual calculation with different logic

**Location**: `src/license_detection/match_refine.rs:653-678`

---

### 3.6 Rule Length Uses `length_unique` vs `length`

**Python**: Uses `rule.length`
**Rust**: Uses `rule.length_unique`

**Location**: `src/license_detection/match_refine.rs:1357-1358`

---

### 3.7 License Intro/Clue Not Set for Some Rules

**Issue**: `unknown-license-reference` rule lacks `is_license_intro` or `is_license_clue` flags

**Impact**: Extra detections in output

**Location**: Rule indexing, `src/license_detection/detection.rs:466-471`

---

## 4. LOW SEVERITY / ARCHITECTURAL DIFFERENCES

### 4.1 `qcontains()` Fallback to Bounds

**Python**: Always uses position sets
**Rust**: Falls back to bounds-based check when `qspan_positions` is `None`

**Location**: `src/license_detection/models.rs:457-472`

---

### 4.2 Matcher Combination During Merge

**Python**: Combines matcher names if different (`"aho seq"`)
**Rust**: Keeps first matcher name only

**Location**: `src/license_detection/match_refine.rs:113-153`

---

### 4.3 `discard_reason` Not Tracked

**Python**: Preserves `discard_reason` with complex logic
**Rust**: Doesn't track this field

---

### 4.4 Empty Expression Handling

**Python**: `combine_expressions([])` returns `None`
**Rust**: Returns `Ok("")`

**Location**: `src/license_detection/expression.rs:633-635`

---

## 5. PIPELINE STAGE ANALYSIS

### 5.1 Tokenization

| Aspect | Status | Location |
|--------|--------|----------|
| BOM handling | Missing | `tokenize.rs` |
| Encoding edge cases | Differs | `tokenize.rs`, `query.rs` |
| Stopwords | Equivalent | `tokenize.rs` |

### 5.2 Matching

| Aspect | Status | Location |
|--------|--------|----------|
| Hash match | Equivalent | `hash_match.rs` |
| Aho-Corasick | Equivalent | `aho_match.rs` |
| Sequence matching | Differs (scoring) | `seq_match.rs` |
| Post-phase merging | Missing | `mod.rs` |

### 5.3 Filtering

| Aspect | Status | Location |
|--------|--------|----------|
| `is_false_positive()` | Missing copyright check | `detection.rs` |
| `qdensity()` | Wrong metric | `models.rs` |
| `filter_contained_matches()` | More aggressive | `match_refine.rs` |
| `filter_overlapping_matches()` | Minor differences | `match_refine.rs` |
| `restore_non_overlapping()` | CRITICAL: wrong span type | `match_refine.rs` |

### 5.4 Merging

| Aspect | Status | Location |
|--------|--------|----------|
| Sort order | Missing `matcher_order` | `match_refine.rs` |
| Distance calculation | Bounds vs sets | `models.rs` |
| Score formula | CRITICAL: simplified | `match_refine.rs` |
| Expression combination | Differs | `expression.rs` |

### 5.5 Grouping

| Aspect | Status | Location |
|--------|--------|----------|
| Line threshold | Equivalent | `detection.rs` |
| Token threshold | Tests expect but not implemented | `detection.rs` |
| Custom threshold param | Ignored | `detection.rs` |
| License intro handling | Equivalent | `detection.rs` |
| License clue handling | Minor diff | `detection.rs` |

### 5.6 Expression Handling

| Aspect | Status | Location |
|--------|--------|----------|
| Parsing | Custom vs library | `expression.rs` |
| Case normalization | Differs | `expression.rs` |
| `+` character support | Missing | `expression.rs` |
| Deduplication | String-exact vs symbol-aware | `expression.rs` |
| Normalization/simplification | Missing | `expression.rs` |

---

## 6. GOLDEN TEST FAILURE PATTERNS

### lic1 (57 failures)

| Pattern | Count | Root Cause |
|---------|-------|------------|
| Rule selection differs | ~20 | Scoring/sorting differences |
| Match count mismatch | ~15 | Merging/grouping differences |
| Expression differs | ~10 | Expression combination |
| Missing detection | ~7 | Filtering too aggressive |
| Extra detection | ~5 | Filtering not aggressive enough |

### lic2 (48 failures)

| Pattern | Count | Root Cause |
|---------|-------|------------|
| Test extraction semantics | ~20 | Comparing matches vs detections |
| Extra detections | ~15 | `restore_non_overlapping()` issues |
| Missing detections | ~7 | Encoding/tokenization |
| Wrong rule selected | ~6 | Scoring differences |

### lic3 (35 failures)

| Pattern | Count | Root Cause |
|---------|-------|------------|
| Too few detections | 11 | Missing matches, grouping |
| Too many matches | 10 | Low-coverage matches not filtered |
| Wrong version | 4 | Rule selection |
| Missing detection | 4 | Over-filtering |
| Expression issues | 8 | Normalization, OR handling |

### lic4 (47 failures)

| Pattern | Count | Root Cause |
|---------|-------|------------|
| Match grouping too aggressive | ~15 | LINES_THRESHOLD grouping |
| BOM not handled | ~5 | Missing BOM stripping |
| Expression combination | ~10 | AND combination vs separate |
| Unknown/intro filtering | ~8 | Missing flag on rules |
| Duplicate deduplication | ~9 | Expression simplification |

### external (412 failures)

| Pattern | Count | Root Cause |
|---------|-------|------------|
| SPDX key mapping | ~50 | Missing mappings |
| Duplicate detections | ~200 | Missing post-phase merge |
| Expression combination | ~50 | Detection grouping |
| GPL version detection | ~30 | URL-based references |
| License text subtraction | ~50 | Timing differences |
| Detection grouping | ~20 | Proximity calculation |
| False positive filtering | ~20 | Missing comprehensive FP |

---

## 7. PRIORITY FIX ORDER

### Phase 1: Critical Fixes (Target: +150 tests)

1. **`restore_non_overlapping()` span type** - Use token positions
2. **Score formula** - Implement full Python formula
3. **SPDX key mapping** - Add missing mappings
4. **BOM handling** - Strip UTF-8 BOM

### Phase 2: High Priority (Target: +100 tests)

1. **Copyright word check** - Add to `is_false_positive()`
2. **`qdensity()` metric** - Include unknown tokens
3. **Equal ispan selection** - Use `qspan.magnitude()`
4. **Post-phase merge** - Add `merge_matches()` after each phase
5. **Custom threshold** - Respect `_proximity_threshold` parameter

### Phase 3: Medium Priority (Target: +50 tests)

1. **Sort order** - Add `matcher_order`
2. **GPL-2.0+ support** - Allow `+` in license keys
3. **Expression normalization** - Add simplification layer
4. **Rule flags** - Set `is_license_intro`/`is_license_clue` for appropriate rules

### Phase 4: Low Priority / Enhancements

1. **Case preservation** - Preserve case in expressions
2. **Position set consistency** - Always populate `qspan_positions`
3. **Matcher combination** - Combine matcher names
4. **`discard_reason` tracking** - Add if needed

---

## 8. FILES REQUIRING CHANGES

| File | Number of Issues |
|------|------------------|
| `src/license_detection/match_refine.rs` | 8 |
| `src/license_detection/detection.rs` | 5 |
| `src/license_detection/models.rs` | 4 |
| `src/license_detection/expression.rs` | 4 |
| `src/license_detection/spdx_lid.rs` | 1 |
| `src/license_detection/tokenize.rs` | 1 |
| `src/license_detection/query.rs` | 1 |
| `src/license_detection/mod.rs` | 1 |

---

## 9. NEXT STEPS

1. Create individual plan files for Phase 1 critical fixes
2. Implement fixes in priority order
3. Run golden tests after each fix to measure impact
4. Adjust priorities based on actual impact

---

## Appendix: Agent Reports

This plan consolidates findings from these investigation agents:

1. **Deduplication logic** - `match_refine.rs` analysis
2. **lic1 failures** - Rule selection, match count issues
3. **lic2 failures** - Test extraction semantics, extra detections
4. **lic3 failures** - Grouping, filtering, expression issues
5. **lic4 failures** - BOM, grouping, expression combination
6. **external failures** - SPDX mapping, post-phase merging
7. **Matching stage** - Tokenization, Aho-Corasick, scoring
8. **Filtering stage** - False positives, containment, restoration
9. **Merging stage** - Distance, score, expression combination
10. **Expression handling** - Parsing, normalization, WITH handling
11. **Match grouping** - Thresholds, intro/clue handling
