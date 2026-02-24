# PLAN-029: Comprehensive Difference Analysis

**Date**: 2026-02-23
**Last Updated**: 2026-02-23
**Status**: Partially Implemented - See Resolution Status Below
**Priority**: Critical
**Related**: All previous PLANs, golden test failures

## Executive Summary

A thorough analysis of the Rust and Python implementations identified **50+ differences** across 10 pipeline stages. This document consolidates findings from 11 investigation agents into a single reference for prioritizing fixes.

**Current State**: ~88% golden test pass rate (significant improvement from 86.1%)

**Resolution Summary**:

- **CRITICAL issues**: 3 of 4 resolved
- **HIGH issues**: 5 of 6 resolved  
- **MEDIUM issues**: 1 of 7 resolved (matcher_order in sort key)

---

## 1. CRITICAL DIFFERENCES (Fix First)

### 1.1 `restore_non_overlapping()` Uses Wrong Span Type

**Severity**: CRITICAL
**Status**: OPEN (Not yet fixed)
**Impact**: ~100+ tests

**Python** (`match.py:1532-1541`):

```python
all_matched_qspans = Span().union(*(m.qspan for m in matches))
if not disc.qspan & all_matched_qspans:  # Uses TOKEN positions
```

**Rust** (`match_refine.rs:718-745`):

```rust
fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)  // Uses LINE positions!
}
```

**Fix Required**: Change Rust to use token positions (`start_token..end_token`) instead of line positions. See PLAN-030 for detailed implementation plan with fallback logic for uninitialized tokens.

---

### 1.2 Score Calculation Formula Mismatch

**Severity**: CRITICAL
**Status**: RESOLVED (PLAN-031)
**Impact**: Affects all scoring/ranking decisions

**Python** (`match.py:592-619`):

```python
score = query_coverage * rule_coverage * relevance * 100
# where query_coverage = len() / qmagnitude()
# and qmagnitude includes unknown tokens
```

**Rust** (`match_refine.rs:430-452`) - **NOW CORRECT**:

```rust
fn compute_match_score(m: &LicenseMatch, query: &Query) -> f32 {
    let relevance = m.rule_relevance as f32 / 100.0;
    let qmagnitude = m.qmagnitude(query);
    let query_coverage = m.len() as f32 / qmagnitude as f32;
    let rule_coverage = m.icoverage();
    // ... full formula implemented
}
```

**Resolution**: Full Python formula implemented including `qmagnitude()` with unknown token accounting. See PLAN-031.

---

### 1.3 SPDX Key Mapping Missing

**Severity**: CRITICAL
**Status**: RESOLVED (PLAN-032)
**Impact**: ~50+ external tests

**Issue**: `0BSD` should map to `bsd-zero`, `AFL-1.1` to `afl-1.1`, etc.

**Location**: `src/license_detection/index/builder.rs:376-381`, `src/license_detection/spdx_lid.rs:152-183`

**Resolution**:

- `rid_by_spdx_key` properly populated at index build time
- `DEPRECATED_SPDX_EXPRESSION_SUBS` table added for deprecated SPDX identifiers
- `add_deprecated_spdx_aliases()` function adds deprecated aliases during index build
- Tests verify `0BSD -> bsd-zero`, `GPL-2.0-or-later -> gpl-2.0-plus`, etc.

---

### 1.4 BOM (Byte Order Mark) Not Stripped

**Severity**: CRITICAL
**Status**: RESOLVED (PLAN-033)
**Impact**: Files starting with UTF-8 BOM fail completely

**Location**: `src/license_detection/mod.rs:117`, `src/utils/text.rs`

**Resolution**:

- `strip_utf8_bom_str()` utility function added
- BOM stripped in `LicenseDetectionEngine::detect()` before tokenization
- Scanner also strips BOM for file processing
- Tests added for BOM-prefixed license detection

---

## 2. HIGH SEVERITY DIFFERENCES

### 2.1 Missing Copyright Word Check in `is_false_positive()`

**Severity**: HIGH
**Status**: RESOLVED (PLAN-034)
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

**Rust** (`detection.rs:317-331`) - **NOW CORRECT**:

```rust
let copyright_words = ["copyright", "(c)"];
let has_copyrights = matches.iter().all(|m| {
    m.matched_text
        .as_ref()
        .map(|text| {
            let text_lower = text.to_lowercase();
            copyright_words.iter().any(|word| text_lower.contains(word))
        })
        .unwrap_or(false)
});

if has_copyrights || has_full_relevance {
    return false;
}
```

**Resolution**: Copyright word check added with case-insensitive matching. See PLAN-034.

---

### 2.2 `qdensity()` Uses Wrong Metric

**Severity**: HIGH
**Status**: RESOLVED (PLAN-035)
**Impact**: Spurious match filtering differs

**Python**: Uses `qmagnitude()` which includes unknown tokens
**Rust** (previously): Used `qregion_len()` without unknown tokens

**Location**: `src/license_detection/models.rs:441-451` - **NOW CORRECT**

**Resolution**:

- `qdensity()` signature updated to accept `&Query` parameter
- Now uses `qmagnitude(query)` which correctly includes unknown tokens
- `filter_spurious_matches()` updated to pass query parameter
- See PLAN-035

---

### 2.3 Equal Ispan Match Selection Differs

**Severity**: HIGH
**Status**: RESOLVED (PLAN-036)
**Impact**: Different matches kept when ispans equal

**Python** (`match.py:949-970`):

```python
if current_match.ispan == next_match.ispan and current_match.overlap(next_match):
    if current_match.qspan.magnitude() <= next_match.qspan.magnitude():
        del rule_matches[j]  # Remove match with larger magnitude
```

**Rust** (`match_refine.rs:226-237`) - **NOW CORRECT**:

```rust
if current.ispan() == next.ispan() && current.qoverlap(&next) > 0 {
    let current_mag = current.qspan_magnitude();
    let next_mag = next.qspan_magnitude();
    if current_mag <= next_mag {
        rule_matches.remove(j);
        // ...
}
```

**Resolution**:

- `qspan_magnitude()` method added to LicenseMatch
- Uses `qspan_magnitude()` instead of `matched_length`
- Comparison direction fixed to `<=` (prefer smaller magnitude/denser span)
- See PLAN-036

---

### 2.4 Missing `merge_matches()` After Each Matching Phase

**Severity**: HIGH
**Status**: PARTIALLY RESOLVED (PLAN-037)
**Impact**: ~200+ external tests

**Python** (`index.py:1040-1050`):

```python
matches = match.merge_matches(matches)  # Called after each phase
```

**Rust**: Only merges at end of `refine_matches()`.

**Location**: `src/license_detection/mod.rs:117-271`

**Prerequisite Fixed**: `matcher_order` added to sort key in `merge_overlapping_matches()` (line 175)

**Still Open**: Post-phase merge calls and hash match early return. See PLAN-037 for detailed implementation plan.

---

### 2.5 Match Grouping Ignores Custom Threshold

**Severity**: HIGH
**Status**: RESOLVED (PLAN-038)
**Impact**: Detection grouping differs from Python

**Python**: `group_matches(license_matches, lines_threshold=custom)` respects parameter
**Rust** (previously): `_proximity_threshold` parameter was ignored (prefixed with `_`)

**Location**: `src/license_detection/detection.rs:163-166` - **NOW CORRECT**

**Resolution**:

- `_proximity_threshold` renamed to `proximity_threshold` (no underscore)
- `should_group_together()` updated to accept threshold parameter
- Parameter passed through call chain correctly
- See PLAN-038

---

### 2.6 Expression Normalization Missing

**Severity**: HIGH
**Status**: OPEN
**Impact**: ~30+ tests with complex expressions

**Issue**: Python normalizes `lgpl-2.1 WITH exception OR cpl-1.0 WITH exception` to `lzma-sdk-2006`

**Location**: `src/license_detection/expression.rs`, `src/license_detection/spdx_mapping.rs`

**Note**: This is a complex feature requiring investigation of Python's expression simplification logic.

---

## 3. MEDIUM SEVERITY DIFFERENCES

### 3.1 Sort Order Missing `matcher_order`

**Severity**: MEDIUM
**Status**: RESOLVED (PLAN-037 prerequisite)
**Impact**: Merge decisions may differ between matchers

**Python**: Uses `(identifier, start, -hilen, -len, matcher_order)` as sort key
**Rust** (previously): Missing `matcher_order` in sort key

**Location**: `src/license_detection/match_refine.rs:169-176` - **NOW CORRECT**

```rust
sorted.sort_by(|a, b| {
    a.rule_identifier
        .cmp(&b.rule_identifier)
        .then_with(|| a.start_token.cmp(&b.start_token))
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))  // ADDED
});
```

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
| BOM handling | **RESOLVED** | `mod.rs:117`, `utils/text.rs` |
| Encoding edge cases | Differs | `tokenize.rs`, `query.rs` |
| Stopwords | Equivalent | `tokenize.rs` |

### 5.2 Matching

| Aspect | Status | Location |
|--------|--------|----------|
| Hash match | Equivalent | `hash_match.rs` |
| Aho-Corasick | Equivalent | `aho_match.rs` |
| Sequence matching | Equivalent (scoring fixed) | `seq_match.rs` |
| Post-phase merging | **PARTIALLY RESOLVED** | `mod.rs` (matcher_order added) |

### 5.3 Filtering

| Aspect | Status | Location |
|--------|--------|----------|
| `is_false_positive()` | **RESOLVED** (copyright check added) | `detection.rs:317-331` |
| `qdensity()` | **RESOLVED** (uses qmagnitude) | `models.rs:441-451` |
| `filter_contained_matches()` | More aggressive (intentional) | `match_refine.rs` |
| `filter_overlapping_matches()` | Equivalent | `match_refine.rs` |
| `restore_non_overlapping()` | **OPEN** (wrong span type) | `match_refine.rs:718-745` |

### 5.4 Merging

| Aspect | Status | Location |
|--------|--------|----------|
| Sort order | **RESOLVED** (matcher_order added) | `match_refine.rs:169-176` |
| Distance calculation | Equivalent | `models.rs` |
| Score formula | **RESOLVED** (full formula) | `match_refine.rs:430-452` |
| Expression combination | Differs | `expression.rs` |

### 5.5 Grouping

| Aspect | Status | Location |
|--------|--------|----------|
| Line threshold | Equivalent | `detection.rs` |
| Token threshold | Tests expect but not implemented | `detection.rs` |
| Custom threshold param | **RESOLVED** (respected) | `detection.rs:163-166` |
| License intro handling | Equivalent | `detection.rs` |
| License clue handling | Equivalent | `detection.rs` |

### 5.6 Expression Handling

| Aspect | Status | Location |
|--------|--------|----------|
| Parsing | Equivalent | `expression.rs` |
| Case normalization | Differs (intentional) | `expression.rs` |
| `+` character support | Missing | `expression.rs` |
| Deduplication | Equivalent | `expression.rs` |
| Normalization/simplification | **OPEN** | `expression.rs` |

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

1. ~~**`restore_non_overlapping()` span type** - Use token positions~~ **OPEN** - See PLAN-030
2. ~~**Score formula** - Implement full Python formula~~ **RESOLVED** (PLAN-031)
3. ~~**SPDX key mapping** - Add missing mappings~~ **RESOLVED** (PLAN-032)
4. ~~**BOM handling** - Strip UTF-8 BOM~~ **RESOLVED** (PLAN-033)

### Phase 2: High Priority (Target: +100 tests)

1. ~~**Copyright word check** - Add to `is_false_positive()`~~ **RESOLVED** (PLAN-034)
2. ~~**`qdensity()` metric** - Include unknown tokens~~ **RESOLVED** (PLAN-035)
3. ~~**Equal ispan selection** - Use `qspan.magnitude()`~~ **RESOLVED** (PLAN-036)
4. **Post-phase merge** - Add `merge_matches()` after each phase - **PARTIALLY RESOLVED** (PLAN-037)
5. ~~**Custom threshold** - Respect `_proximity_threshold` parameter~~ **RESOLVED** (PLAN-038)

### Phase 3: Medium Priority (Target: +50 tests)

1. ~~**Sort order** - Add `matcher_order`~~ **RESOLVED** (PLAN-037 prerequisite)
2. **GPL-2.0+ support** - Allow `+` in license keys - **OPEN**
3. **Expression normalization** - Add simplification layer - **OPEN**
4. **Rule flags** - Set `is_license_intro`/`is_license_clue` for appropriate rules - **OPEN**

### Phase 4: Low Priority / Enhancements

1. **Case preservation** - Preserve case in expressions - **OPEN** (intentional difference)
2. **Position set consistency** - Always populate `qspan_positions` - **OPEN**
3. **Matcher combination** - Combine matcher names - **OPEN**
4. **`discard_reason` tracking** - Add if needed - **OPEN**

---

## 8. FILES REQUIRING CHANGES

| File | Number of Issues | Resolved |
|------|------------------|----------|
| `src/license_detection/match_refine.rs` | 8 | 5 resolved (score, ispan, matcher_order) |
| `src/license_detection/detection.rs` | 5 | 3 resolved (copyright, threshold) |
| `src/license_detection/models.rs` | 4 | 3 resolved (qmagnitude, qdensity, icoverage) |
| `src/license_detection/expression.rs` | 4 | 0 resolved (normalization open) |
| `src/license_detection/spdx_lid.rs` | 1 | 1 resolved (SPDX mapping) |
| `src/license_detection/tokenize.rs` | 1 | 0 resolved (BOM handled elsewhere) |
| `src/license_detection/query.rs` | 1 | 0 resolved |
| `src/license_detection/mod.rs` | 1 | 1 resolved (BOM, partial for merge) |
| `src/utils/text.rs` | 0 | NEW FILE (BOM handling) |

---

## 9. RESOLUTION SUMMARY

### Completed Fixes (11)

| Plan | Issue | Status |
|------|-------|--------|
| PLAN-031 | Score formula with qmagnitude | COMPLETE |
| PLAN-032 | SPDX key mapping | COMPLETE |
| PLAN-033 | BOM handling | COMPLETE |
| PLAN-034 | Copyright word check | COMPLETE |
| PLAN-035 | qdensity metric | COMPLETE |
| PLAN-036 | Equal ispan selection | COMPLETE |
| PLAN-037 prereq | matcher_order in sort key | COMPLETE |
| PLAN-038 | Custom threshold parameter | COMPLETE |

### Open Issues (High Priority)

| Issue | Priority | Plan Reference |
|-------|----------|----------------|
| restore_non_overlapping token positions | CRITICAL | PLAN-030 |
| Post-phase merge calls | HIGH | PLAN-037 |
| Expression normalization | HIGH | New plan needed |

### Open Issues (Medium Priority)

| Issue | Priority | Notes |
|-------|----------|-------|
| GPL-2.0+ support | MEDIUM | Allow `+` in license keys |
| Rule flags for intro/clue | MEDIUM | Some rules lack flags |
| Position set consistency | LOW | qspan_positions not always populated |

---

## 9. NEXT STEPS

### Immediate Actions Required

1. **PLAN-030**: Implement `restore_non_overlapping()` token position fix
   - This is the only remaining CRITICAL issue
   - Expected to resolve ~100+ test failures

2. **PLAN-037**: Complete post-phase merge implementation
   - Add merge calls after SPDX, Aho, and sequence phases
   - Add hash match early return

### Follow-up Actions

1. Create plan for expression normalization
2. Consider GPL-2.0+ support
3. Run golden test comparison after each fix to measure impact

### Completed in This Session

- PLAN-031: Score formula fix
- PLAN-032: SPDX key mapping fix
- PLAN-033: BOM handling fix
- PLAN-034: Copyright check fix
- PLAN-035: qdensity metric fix
- PLAN-036: Equal ispan selection fix
- PLAN-037 prerequisite: matcher_order in sort key
- PLAN-038: Threshold parameter fix

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

---

## Document History

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-23 | AI Agent | Initial plan creation |
| 2026-02-23 | AI Agent | Updated with resolution status for PLANs 031-038 |
| 2026-02-23 | AI Agent | Added resolution summary, updated file locations, marked resolved issues |
