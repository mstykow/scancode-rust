# License Detection Audit: Differences Report

## Executive Summary

This report condenses findings from a comprehensive audit comparing the Python ScanCode Toolkit license detection engine with the Rust implementation. **~2% of golden tests are failing** due to the differences documented below.

---

## Critical Differences (High Impact on Results)

### 1. QueryRun Splitting Disabled in Rust
**File**: `QUERY_TOKENIZATION.md`

- **Python**: Actively splits text into QueryRuns when encountering 4+ empty/junk lines (`LINES_THRESHOLD=4`)
- **Rust**: QueryRun splitting is **disabled** 
- **Impact**: Different matching behavior for files with multiple license sections separated by blank lines
- **Location**: Python `query.py:583-652`, Rust `query/mod.rs`

### 2. Required Phrase Handling Not Implemented
**File**: `RULE_ENGINE.md`

- **Python**: Validates `{{phrase}}` markers in rules - match must contain required phrases
- **Rust**: **Not implemented** - `required_phrase_spans` field exists but is never checked
- **Impact**: False positives for rules that require specific phrases to be present
- **Location**: Python `match.py:1045-1122`, Rust missing in `match_refine/`

### 3. Detection Score Formula Mismatch
**File**: `SCORING.md`

- **Python**: `sum(match.score × match.len / total_length)` - length-weighted average
- **Rust**: `sum(match.score × match_coverage × relevance) / sum(match_coverage)` - coverage-weighted
- **Impact**: Different detection scores in multi-match scenarios
- **Example**: Same matches → Python: 89.09, Rust: 71.33
- **Location**: Python `detection.py`, Rust `detection/analysis.rs`

### 4. Matcher Order Swapped
**File**: `MATCHING_ALGORITHMS.md`

| Matcher | Python Order | Rust Order |
|---------|-------------|------------|
| Aho-Corasick | 1 | 2 |
| SPDX-LID | 2 | 1 |

- **Impact**: Different matcher names in output, potentially different tie-breaking
- **Also**: Unknown matcher named `6-unknown` (Python) vs `5-undetected` (Rust)

### 5. Score Rounding Precision
**File**: `SCORING.md`, `MATCH_REFINEMENT.md`

- **Python**: `round(score, 2)` - 2 decimal places
- **Rust**: `score.round()` - integer rounding
- **Impact**: Score values differ in output JSON
- **Location**: Throughout scoring code

### 6. MAX_DIST Threshold Different
**File**: `CONSTANTS_THRESHOLDS.md`

- **Python**: `MAX_DIST = 50` tokens
- **Rust**: `MAX_DIST = 100` tokens
- **Impact**: Different match merging behavior - Rust merges more aggressively
- **Location**: Python `__init__.py:24`, Rust `match_refine/merge.rs`

### 7. Detection Analysis Functions Not Called
**File**: `DETECTION_ASSEMBLY.md`

Three classification functions exist in Rust but are **not integrated**:
- `has_unknown_intro_before_detection()`
- `has_references_to_local_files()`
- `is_low_quality_matches()`

- **Impact**: Detection classification (`perfect-detection`, `unknown-intro-before-detection`, etc.) differs from Python
- **Location**: Rust `detection/analysis.rs`

---

## High Priority Differences

### 8. Expression Rendering - Extra Parentheses
**File**: `EXPRESSION_HANDLING.md`

- **Python**: `mit AND apache AND gpl`
- **Rust**: `(mit AND apache) AND gpl`
- **Impact**: Golden test failures for multi-license expressions
- **Location**: Rust `expression/` module

### 9. Unknowns Span Tracking Missing
**File**: `QUERY_TOKENIZATION.md`

- **Python**: Tracks `unknowns_span` for scoring adjustments
- **Rust**: Not implemented
- **Impact**: Slight scoring differences when unknown tokens present
- **Location**: Python `query.py`, Rust `query/mod.rs`

### 10. License Alias Mapping Not Used
**File**: `SPDX_DATA.md`

- **Python**: Recognizes aliases like `GPL-2.0` → `GPL-2.0-only`
- **Rust**: `other_spdx_license_keys` loaded but **not used** in mapping
- **Impact**: Expressions with deprecated SPDX IDs may fail to resolve
- **Location**: Rust `spdx_mapping/mod.rs`

### 11. Two Inconsistent SPDX Substitution Tables
**File**: `SPDX_DATA.md`

- `spdx_lid/mod.rs` uses `-plus` suffix
- `index/builder/mod.rs` uses `-or-later` suffix
- **Python** uses `-or-later` (SPDX standard)
- **Impact**: Inconsistent SPDX expression generation
- **Location**: Rust inconsistent across modules

### 12. Score Output Conversion Missing
**File**: `CLI_OUTPUT.md`

- **Internal**: Scores stored as 0.0-1.0
- **Python**: Multiplies by 100 before output
- **Rust**: **Bug** - does not multiply by 100
- **Impact**: Output shows scores like 0.89 instead of 89.0

---

## Medium Priority Differences

### 13. Relevance Computation Not Implemented
**File**: `RULE_ENGINE.md`

- **Python**: Computes `relevance` based on rule characteristics
- **Rust**: `compute_thresholds_unique()` exists but **not integrated**
- **Impact**: Rule relevance values may differ
- **Location**: Rust `rules/thresholds.rs`

### 14. Missing CLI Options
**File**: `CLI_OUTPUT.md`

Rust missing several CLI options present in Python:
- `--license` (enable/disable license detection)
- `--license-score` (minimum score threshold)
- `--license-diag` (diagnostic output)
- `--is-license-text` (license text detection flag)

### 15. Missing Detection Output Fields
**File**: `CLI_OUTPUT.md`

Rust output missing:
- Top-level `license_detections` collection
- `license_clues` tracking
- `percentage_of_license_text` metric
- `detected_license_expression` (only SPDX version exposed)
- `detection_log` in serialized output

### 16. Candidate Scores in Overlap Resolution
**File**: `MATCH_REFINEMENT.md`

- **Rust**: Uses `candidate_resemblance`/`candidate_containment` for tie-breaking
- **Python**: Does not have this mechanism
- **Impact**: May produce different results in overlap scenarios

### 17. MAX_TOKEN_PER_LINE Missing
**File**: `CONSTANTS_THRESHOLDS.md`

- **Python**: `MAX_TOKEN_PER_LINE = 25` - handles minified JS/CSS
- **Rust**: Not implemented
- **Impact**: Different behavior for minified files

---

## Low Priority / Implementation Differences

### 18. Token Dictionary Signedness
**File**: `LICENSE_DATABASE.md`

- **Python**: Signed 16-bit (max 32767 tokens)
- **Rust**: Unsigned 16-bit (max 65535 tokens)
- **Impact**: None currently (4506 tokens used) - future extensibility

### 19. Sparse vs Dense Storage
**File**: `LICENSE_DATABASE.md`

- **Python**: List indexed by rid (dense)
- **Rust**: HashMap for sparse data
- **Impact**: Memory and lookup behavior, not functional difference

### 20. Position Array Size
**File**: `LICENSE_DATABASE.md`

- **Python**: `array('h')` - 2 bytes per position
- **Rust**: `Vec<usize>` - 8 bytes per position
- **Impact**: 4x memory usage, not functional difference

### 21. Experimental Automatons Not Ported
**File**: `LICENSE_DATABASE.md`

Python has experimental:
- `fragments_automaton`
- `starts_automaton`

These are **not ported** to Rust. Unknown if they affect behavior.

### 22. Missing License Metadata Fields
**File**: `LICENSE_DATABASE.md`

Rust License struct missing fields:
- `language`
- `short_name`
- `owner`
- `is_exception`
- `is_builtin`
- `is_generic`

Low impact if only used for display.

---

## Summary Table

| Category | Count |
|----------|-------|
| **Critical** (affects results) | 7 |
| **High Priority** (affects golden tests) | 5 |
| **Medium Priority** (affects output/completeness) | 5 |
| **Low Priority** (implementation details) | 5 |
| **Total Differences** | 22 |

---

## Recommendations

### Must Fix for Parity
1. Enable QueryRun splitting with `LINES_THRESHOLD=4`
2. Implement required phrase checking
3. Fix detection score formula to match Python
4. Swap matcher orders (Aho=1, SPDX-LID=2)
5. Fix score rounding to 2 decimal places
6. Align `MAX_DIST=50` with Python
7. Integrate detection analysis functions
8. Fix expression rendering (remove extra parentheses)
9. Use license alias mapping
10. Fix score output (multiply by 100)

### Should Fix for Completeness
11. Implement `unknowns_span` tracking
12. Unify SPDX substitution tables
13. Integrate relevance computation
14. Add missing CLI options
15. Add missing output fields

### Consider
16. Evaluate MAX_TOKEN_PER_LINE necessity
17. Document memory tradeoffs (position arrays)
18. Verify experimental automatons not needed

---

## Files Referenced

| Audit Document | Topic |
|----------------|-------|
| `PYTHON_PIPELINE.md` | Python architecture |
| `RUST_PIPELINE.md` | Rust architecture |
| `LICENSE_DATABASE.md` | Index/storage structures |
| `QUERY_TOKENIZATION.md` | Query/tokenization |
| `MATCHING_ALGORITHMS.md` | Matching strategies |
| `MATCH_REFINEMENT.md` | Match filtering |
| `SCORING.md` | Score calculation |
| `RULE_ENGINE.md` | Rule loading |
| `EXPRESSION_HANDLING.md` | License expressions |
| `DETECTION_ASSEMBLY.md` | Detection grouping |
| `SPDX_DATA.md` | SPDX license data |
| `CONSTANTS_THRESHOLDS.md` | Constants comparison |
| `CLI_OUTPUT.md` | CLI/output format |
