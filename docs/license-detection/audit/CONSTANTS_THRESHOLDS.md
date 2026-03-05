# License Detection Constants and Thresholds Audit

This document compares configuration constants and thresholds between the Python ScanCode Toolkit reference implementation and the Rust reimplementation.

## Summary

| Category | Match Status | Notes |
|----------|--------------|-------|
| Match Length Thresholds | **ALIGNED** | All values match |
| Rule Size Thresholds | **ALIGNED** | All values match |
| Resemblance Thresholds | **ALIGNED** | All values match |
| Query Thresholds | **DIFFERENT** | `MAX_DIST` value differs (50 vs 100) |
| Detection Thresholds | **ALIGNED** | All values match |
| False Positive Thresholds | **ALIGNED** | All values match |
| Matcher Order Constants | **ALIGNED** | All values match |
| Candidate Limits | **ALIGNED** | All values match |

---

## 1. Matching Thresholds

### MAX_DIST (Merge Distance)

Maximum distance between two matches to merge them.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `50` | `reference/scancode-toolkit/src/licensedcode/__init__.py:13` |
| Rust | `100` | `src/license_detection/match_refine/merge.rs:11` |

**Status**: ⚠️ **DIFFERENT**

**Impact**: Rust uses a larger merge distance (100 vs 50), which means matches further apart may be merged in Rust but not in Python. This could result in:
- Fewer but larger matches in Rust
- Different match boundaries for adjacent/overlapping license regions

**Python Code**:
```python
# maximum distance between two matches to merge
MAX_DIST = 50
```

**Rust Code**:
```rust
const MAX_DIST: usize = 100;
```

---

### MIN_MATCH_LENGTH

Minimum number of tokens a match should have to be considered as worthy keeping.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `4` | `reference/scancode-toolkit/src/licensedcode/__init__.py:16` |
| Rust | `4` | `src/license_detection/rules/thresholds.rs:4` |

**Status**: ✅ **ALIGNED**

---

### MIN_MATCH_HIGH_LENGTH

Minimum match length for high-value (legalese) token matching.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `3` | `reference/scancode-toolkit/src/licensedcode/__init__.py:17` |
| Rust | `3` | `src/license_detection/rules/thresholds.rs:7` |

**Status**: ✅ **ALIGNED**

---

### SMALL_RULE

Rules with fewer tokens than this are treated as "small rules" (exact match only).

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `15` | `reference/scancode-toolkit/src/licensedcode/__init__.py:20` |
| Rust | `15` | `src/license_detection/rules/thresholds.rs:10` |

**Status**: ✅ **ALIGNED**

---

### TINY_RULE

Rules with fewer tokens than this are treated as "tiny rules" (special handling).

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `6` | `reference/scancode-toolkit/src/licensedcode/__init__.py:23` |
| Rust | `6` | `src/license_detection/rules/thresholds.rs:13` |

**Status**: ✅ **ALIGNED**

---

## 2. Resemblance Thresholds

### HIGH_RESEMBLANCE_THRESHOLD

Threshold for high resemblance in near-duplicate detection.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `0.8` | `reference/scancode-toolkit/src/licensedcode/match_set.py:245` (as default param) |
| Rust | `0.8` | `src/license_detection/seq_match/mod.rs:30` |

**Status**: ✅ **ALIGNED**

**Python Code**:
```python
def compute_candidates(..., high_resemblance_threshold=0.8, ...):
```

**Rust Code**:
```rust
pub const HIGH_RESEMBLANCE_THRESHOLD: f32 = 0.8;
```

---

### MAX_NEAR_DUPE_CANDIDATES

Maximum number of top near-duplicate candidates to consider.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `10` | `reference/scancode-toolkit/src/licensedcode/index.py:741` (local constant) |
| Rust | `10` | `src/license_detection/seq_match/mod.rs:33` |

**Status**: ✅ **ALIGNED**

---

## 3. Query Thresholds

### MAX_TOKEN_PER_LINE

Maximum tokens per line before breaking into pseudo-lines (for minified files).

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `25` | `reference/scancode-toolkit/src/licensedcode/query.py:103` |
| Rust | Not implemented | - |

**Status**: ⚠️ **NOT IMPLEMENTED IN RUST**

**Impact**: For minified JavaScript/CSS files with very long lines, Python breaks them into pseudo-lines of 25 tokens each. Rust may handle these differently.

---

### LINES_THRESHOLD

Number of empty/junk lines to break query into runs.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `4` | `reference/scancode-toolkit/src/licensedcode/query.py:108` |
| Rust | `4` | `src/license_detection/detection/analysis.rs:10` |

**Status**: ✅ **ALIGNED**

---

### TEXT_LINE_THRESHOLD

Line threshold for text files when building queries.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `15` | `reference/scancode-toolkit/src/licensedcode/query.py:115` (as `text_line_threshold` param default) |
| Rust | `15` | `src/license_detection/query/mod.rs:161` |

**Status**: ✅ **ALIGNED**

---

### MAX_TOKENS

Maximum number of tokens supported in the index.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `32767` ((2^15) - 1) | `reference/scancode-toolkit/src/licensedcode/index.py:124` |
| Rust | Not explicitly defined | - |

**Status**: ⚠️ **NOT EXPLICITLY DEFINED IN RUST**

**Impact**: Rust uses `u16` for token IDs, which would support up to 65535 tokens, double Python's limit. This should not cause issues.

---

## 4. Detection Thresholds

### IMPERFECT_MATCH_COVERAGE_THR

Coverage threshold below which detections are not considered "perfect".

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `100` | `reference/scancode-toolkit/src/licensedcode/detection.py:82` |
| Rust | `100.0` | `src/license_detection/detection/analysis.rs:14` |

**Status**: ✅ **ALIGNED**

---

### CLUES_MATCH_COVERAGE_THR

Coverage values below this are reported as "license clues" rather than detections.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `60` | `reference/scancode-toolkit/src/licensedcode/detection.py:85` |
| Rust | `60.0` | `src/license_detection/detection/analysis.rs:17` |

**Status**: ✅ **ALIGNED**

---

### LOW_RELEVANCE_THRESHOLD

Relevance threshold below which matches are considered "low relevance".

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `70` | `reference/scancode-toolkit/src/licensedcode/detection.py:88` |
| Rust | Not implemented | - |

**Status**: ⚠️ **NOT IMPLEMENTED IN RUST**

**Impact**: Python filters matches with `rule.relevance < LOW_RELEVANCE_THRESHOLD` into a separate "low-relevance" detection category. Rust does not have this constant defined.

**Python Code**:
```python
LOW_RELEVANCE_THRESHOLD = 70
```

---

### FALSE_POSITIVE_START_LINE_THRESHOLD

Start line threshold for identifying false positive matches.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `1000` | `reference/scancode-toolkit/src/licensedcode/detection.py:93` |
| Rust | `1000` | `src/license_detection/detection/analysis.rs:25` |

**Status**: ✅ **ALIGNED**

---

### FALSE_POSITIVE_RULE_LENGTH_THRESHOLD

Rule length threshold for identifying false positive matches.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `3` | `reference/scancode-toolkit/src/licensedcode/detection.py:96` |
| Rust | `3` | `src/license_detection/detection/analysis.rs:21` |

**Status**: ✅ **ALIGNED**

---

## 5. False Positive List Thresholds

### MIN_SHORT_FP_LIST_LENGTH

Minimum length for a short sequence of false positive matches.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `15` | `reference/scancode-toolkit/src/licensedcode/match.py:2399` |
| Rust | `15` | `src/license_detection/match_refine/false_positive.rs:8` |

**Status**: ✅ **ALIGNED**

---

### MIN_LONG_FP_LIST_LENGTH

Minimum length for a long sequence of false positive matches.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `150` | `reference/scancode-toolkit/src/licensedcode/match.py:2405` |
| Rust | `150` | `src/license_detection/match_refine/false_positive.rs:9` |

**Status**: ✅ **ALIGNED**

---

### MIN_UNIQUE_LICENSES_PROPORTION

Minimum proportion of matches with unique license expressions.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `1/3` (≈ 0.333) | `reference/scancode-toolkit/src/licensedcode/match.py:2402` |
| Rust | `1.0/3.0` (≈ 0.333) | `src/license_detection/match_refine/false_positive.rs:11` |

**Status**: ✅ **ALIGNED**

---

### MAX_CANDIDATE_LENGTH

Maximum match length for a false positive candidate.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | Not explicitly defined | - |
| Rust | `20` | `src/license_detection/match_refine/false_positive.rs:12` |

**Status**: ⚠️ **RUST EXTENSION**

**Impact**: Rust adds an additional constraint `MAX_CANDIDATE_LENGTH = 20` for candidate false positives. This is a Rust-specific optimization not present in Python.

---

### MAX_DISTANCE_BETWEEN_CANDIDATES

Maximum distance between false positive candidates.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | Not explicitly defined | - |
| Rust | `10` | `src/license_detection/match_refine/false_positive.rs:13` |

**Status**: ⚠️ **RUST EXTENSION**

**Impact**: Rust uses this to group nearby false positive candidates. This is a Rust-specific optimization.

---

## 6. Matcher Order Constants

These constants define the order/priority of different matchers.

### Hash Matcher

| Implementation | Name | Order | File Location |
|----------------|------|-------|---------------|
| Python | `1-hash` | `0` | `reference/scancode-toolkit/src/licensedcode/match_hash.py:40-41` |
| Rust | `1-hash` | `0` | `src/license_detection/hash_match.rs:16,24` |

**Status**: ✅ **ALIGNED**

---

### SPDX ID Matcher

| Implementation | Name | Order | File Location |
|----------------|------|-------|---------------|
| Python | `1-spdx-id` | `2` | `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py:61-62` |
| Rust | `1-spdx-id` | `1` | `src/license_detection/spdx_lid/mod.rs:42,50` |

**Status**: ⚠️ **DIFFERENT ORDER**

**Impact**: Python uses order 2, Rust uses order 1. This affects which matcher takes priority when both could match.

---

### Aho-Corasick Exact Matcher

| Implementation | Name | Order | File Location |
|----------------|------|-------|---------------|
| Python | `2-aho` | `1` | `reference/scancode-toolkit/src/licensedcode/match_aho.py:78-79` |
| Rust | `2-aho` | `2` | `src/license_detection/aho_match.rs:17,25` |

**Status**: ⚠️ **DIFFERENT ORDER**

**Impact**: Python uses order 1, Rust uses order 2. This swaps the priority of Aho-Corasick and SPDX ID matchers.

---

### Aho-Corasick Fragment Matcher

| Implementation | Name | Order | File Location |
|----------------|------|-------|---------------|
| Python | `5-aho-frag` | `5` | `reference/scancode-toolkit/src/licensedcode/match_aho.py:80-81` |
| Rust | Not implemented | - | - |

**Status**: ⚠️ **NOT IMPLEMENTED IN RUST**

---

### Sequence Matcher

| Implementation | Name | Order | File Location |
|----------------|------|-------|---------------|
| Python | `3-seq` | `3` | `reference/scancode-toolkit/src/licensedcode/match_seq.py:44-45` |
| Rust | `3-seq` | `3` | `src/license_detection/seq_match/mod.rs:25,27` |

**Status**: ✅ **ALIGNED**

---

### Unknown Matcher

| Implementation | Name | Order | File Location |
|----------------|------|-------|---------------|
| Python | `6-unknown` | `6` | `reference/scancode-toolkit/src/licensedcode/match_unknown.py:46-47` |
| Rust | `5-undetected` | `5` | `src/license_detection/unknown_match.rs:9,12` |

**Status**: ⚠️ **DIFFERENT NAME AND ORDER**

**Impact**: 
- Python: `6-unknown` with order `6`
- Rust: `5-undetected` with order `5`

The naming differs slightly ("unknown" vs "undetected"), and the order differs (6 vs 5).

---

### MATCHER_UNDETECTED_ORDER

Python defines an additional constant:

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `4` | `reference/scancode-toolkit/src/licensedcode/detection.py:78` |
| Rust | Uses `5` | `src/license_detection/unknown_match.rs:12` |

**Status**: ⚠️ **DIFFERENT**

---

## 7. Candidate Limits

### MAX_CANDIDATES (per query run)

Maximum number of candidates to consider per query run.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `70` | `reference/scancode-toolkit/src/licensedcode/index.py:786` (local constant) |
| Rust | `70` | `src/license_detection/mod.rs:262,278,455,471` |

**Status**: ✅ **ALIGNED**

---

## 8. N-gram Constants

### UNKNOWN_NGRAM_LENGTH

Length of n-grams used for unknown license detection.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `6` | `reference/scancode-toolkit/src/licensedcode/match_unknown.py:49` |
| Rust | `6` | `src/license_detection/unknown_match.rs:14` |

**Status**: ✅ **ALIGNED**

---

### AHO_FRAGMENTS_NGRAM_LEN

Length of n-grams for Aho-Corasick fragment detection.

| Implementation | Value | File Location |
|----------------|-------|---------------|
| Python | `6` | `reference/scancode-toolkit/src/licensedcode/index.py:105` |
| Rust | Not implemented | - |

**Status**: ⚠️ **FEATURE NOT IMPLEMENTED IN RUST**

Note: This feature is disabled by default in Python (`USE_AHO_FRAGMENTS = False`).

---

## 9. Overlap Thresholds (Rust-specific)

Rust defines overlap percentage thresholds not explicitly present in Python:

| Constant | Value | File Location |
|----------|-------|---------------|
| `OVERLAP_SMALL` | `0.10` (10%) | `src/license_detection/match_refine/handle_overlaps.rs:13` |
| `OVERLAP_MEDIUM` | `0.40` (40%) | `src/license_detection/match_refine/handle_overlaps.rs:14` |
| `OVERLAP_LARGE` | `0.70` (70%) | `src/license_detection/match_refine/handle_overlaps.rs:15` |
| `OVERLAP_EXTRA_LARGE` | `0.90` (90%) | `src/license_detection/match_refine/handle_overlaps.rs:16` |

**Status**: ⚠️ **RUST EXTENSION**

**Impact**: These thresholds are used for handling overlapping matches in Rust's implementation. They appear to be derived from Python's overlap handling logic but are made explicit constants.

---

## 10. Unknown Match Constants (Rust-specific)

| Constant | Value | File Location |
|----------|-------|---------------|
| `MIN_NGRAM_MATCHES` | `3` | `src/license_detection/unknown_match.rs:16` |
| `MIN_REGION_LENGTH` | `5` | `src/license_detection/unknown_match.rs:18` |

**Status**: ⚠️ **RUST EXTENSION**

---

## Summary of Differences

### Critical Differences (May Affect Behavior)

1. **MAX_DIST**: Python `50`, Rust `100` — Rust may merge more matches
2. **Matcher Order Swap**: SPDX ID and Aho-Corasick matchers have swapped order
3. **Unknown Matcher**: Different name (`unknown` vs `undetected`) and order (`6` vs `5`)

### Missing Features in Rust

1. **MAX_TOKEN_PER_LINE**: Not implemented (minified file handling)
2. **LOW_RELEVANCE_THRESHOLD**: Not implemented
3. **Aho-Corasick Fragments**: Feature not implemented (was disabled in Python anyway)

### Rust Extensions

1. **Overlap thresholds**: Made explicit as constants
2. **MAX_CANDIDATE_LENGTH**: Additional constraint for false positives
3. **MAX_DISTANCE_BETWEEN_CANDIDATES**: Additional constraint for grouping

---

## Recommendations

1. **Investigate MAX_DIST difference**: Determine if `100` is intentional or should be `50` to match Python
2. **Review matcher order**: Confirm the order swap for SPDX ID and Aho-Corasick is intentional
3. **Consider implementing LOW_RELEVANCE_THRESHOLD**: This filtering logic may affect detection categorization
4. **Document unknown matcher name difference**: Clarify if `undetected` vs `unknown` naming is intentional
5. **Consider implementing MAX_TOKEN_PER_LINE**: For proper handling of minified files
