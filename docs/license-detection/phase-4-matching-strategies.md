# Phase 4: Matching Strategies - Implementation Summary

## Overview

Phase 4 implemented all six license matching strategies that produce individual `LicenseMatch` results from tokenized query text. These strategies run in sequence to find matches using different techniques.

## Completed Implementation

### 4.1 Hash Match ✅

**File**: `src/license_detection/hash_match.rs` (364 lines)

**Implementation**:

- `compute_hash()`: Computes SHA1 hash of token sequences
  - Converts u16 tokens to signed i16 (matching Python's `array('h')` type)
  - Uses little-endian byte encoding
  - Returns 20-byte digest
- `index_hash()`: Alias for computing rule hashes during index building
- `hash_match()`: Main matching function
  - Computes query hash
  - Looks up in `index.rid_by_hash`
  - If found, returns LicenseMatch with 100% coverage

**Key Parity Details**:

- Python uses `array('h', tokens).tobytes()` followed by `sha1().digest()`
- Rust implementation exactly matches this behavior via `tokens.iter().flat_map(|t| t.to_le_bytes())`
- Hash computation verified to produce identical results to Python

**Tests**: 7 tests covering hash computation, exact matching, edge cases

---

### 4.2 SPDX-License-Identifier Match ✅

**File**: `src/license_detection/spdx_lid.rs` (672 lines total, match portion added)

**Implementation**:

- `extract_spdx_expressions_with_lines()`: Extracts SPDX expressions with line numbers
- `normalize_spdx_key()`: Normalizes SPDX keys for comparison (lowercase, hyphens)
- `find_matching_rules()`: Finds rules matching SPDX keys in their license_expression
- `split_license_expression()`: Splits compound expressions into individual keys
- `spdx_lid_match()`: Main matching function creates synthetic matches

**Key Features**:

- Handles multiple SPDX tag formats (standard, lowercase, different spacing)
- Supports NuGet URL patterns
- Case-insensitive matching
- Score calculation from rule relevance: `score = relevance / 100.0`
- Line number tracking for match locations

**Tests**: 31 tests covering all edge cases, case variations, multiple identifiers

**Limitations** (for Phase 5 resolution):

- Simplified expression splitting (full parsing coming in Phase 5)
- Direct key comparison (no bi-directional mapping table yet)

---

### 4.3 Aho-Corasick Exact Match ✅

**File**: `src/license_detection/aho_match.rs` (517 lines)

**Implementation**:

- `tokens_to_bytes()`: Encodes u16 tokens as bytes for Aho-Corasick
  - Each token = 2 bytes (little-endian)
- `aho_match()`: Main matching function
  - Runs `index.rules_automaton.find_iter()` on encoded query
  - Verifies all positions are matchable
  - Calculates coverage: `matched_length / rule_length`

**Key Parity Details**:

- Uses `aho_corasick` crate for multi-pattern matching
- Byte encoding enables matching on token sequences
- Position verification ensures matches don't include non-matchable tokens
- Line numbers tracked via query_run

**Tests**: 14 tests covering encoding, matching, coverage calculation, position filtering

---

### 4.4 Approximate Sequence Match ✅

**File**: `src/license_detection/seq_match.rs` (667 lines)

**Implementation**:

**Candidate Selection**:

- `compute_set_similarity()`: Jaccard-like similarity scoring
  - Uses containment (intersection / smaller set)
  - Uses resemblance (intersection / larger set)
  - Amplifies resemblance: `containment * resemblance^2`
- `select_candidates()`: Ranks and filters top-50 candidates
  - Requires minimum 50% coverage threshold
  - Requires high-value legalese tokens in intersection

**Sequence Alignment**:

- `align_sequences()`: Finds matching blocks between sequences
  - Finds consecutive exact token matches
  - Uses high-value tokens asanchor points
  - Merges overlapping blocks

**Main Function**:

- `seq_match()`: Integrates candidate selection and alignment
  - Filters candidates by minimum coverage
  - Requires at least one legalese token match
  - Computes final score: `(match_coverage * rule_relevance) / 100`

**Tests**: 13 tests covering similarity calculation, candidate ranking, alignment

**Limitations** (simplified from Python):

- Block-based alignment (not full Cython implementation)
- No near-duplicate special handling
- Simplified anchor finding

---

### 4.5 Unknown License Match ✅

**File**: `src/license_detection/unknown_match.rs` (386 lines)

**Implementation**:

- `unknown_match()`: Detects license-like text in unmatched regions
- `compute_covered_positions()`: Tracks positions covered by known matches
- `find_unmatched_regions()`: Finds gaps in coverage for unknown license detection
- `match_ngrams_in_region()`: Counts ngram matches using `index.unknown_automaton`
- `create_unknown_match()`: Creates "unknown" LicenseMatch objects

**Key Features**:

- Minimum thresholds:
  - 3 ngram matches required
  - 5 tokens minimum region length
- Score calculation based on ngram density
- Creates matches with:
  - `license_expression`: "unknown"
  - `matcher`: "5-undetected"
  - `rule_relevance`: 50 (medium)

**Tests**: 10 tests covering region detection, ngram matching, score calculation

**Simplifications** (for future phase):

- Assumes `unknown_automaton` is pre-built
- Simple gap detection (not Python's complex region merging)
- No ngram building (will be in index construction phase)

---

### 4.6 Match Refinement ✅

**File**: `src/license_detection/match_refine.rs` (653 lines)

**Implementation**:

- `merge_overlapping_matches()`: Merges adjacent/overlapping same-rule matches
  - Groups by `rule_identifier`
  - Sorts by `start_line`
  - Merges consecutive overlapping/adjacent matches
  - Keeps higher score
- `filter_contained_matches()`: Removes smaller matches inside larger ones
  - Identifies matches contained within same-rule matches
  - Removes contained (smaller) matches
- `filter_false_positive_matches()`: Filters false positive rules
  - Extracts rule ID from `rule_identifier` (format: "#id")
  - Removes matches to rules in `index.false_positive_rids`
- `update_match_scores()`: Ensures correct score calculation
  - Formula: `score = match_coverage * rule_relevance / 100.0`
- `refine_matches()`: Main entry point applying all operations in sequence

**Tests**: 24 comprehensive tests covering merge, filter, score, pipeline

**Simplifications** (from Python's ~3000 lines):

- Basic merging (no complex span merging)
- Simple containment check (by line range)
- Remove false positives (not subtract)

---

## Module Exports

Updated `src/license_detection/mod.rs`:

```rust
pub mod aho_match;
pub mod hash_match;
mod match_refine;
pub mod seq_match;
pub mod spdx_lid;
pub mod unknown_match;

pub use aho_match::{aho_match, MATCH_AHO, MATCH_AHO_ORDER};
pub use hash_match::{compute_hash, hash_match, index_hash, MATCH_HASH, MATCH_HASH_ORDER};
pub use match_refine::refine_matches;
pub use seq_match::{seq_match, MATCH_SEQ, MATCH_SEQ_ORDER};
pub use spdx_lid::{extract_spdx_expressions, spdx_lid_match, MATCH_SPDX_ID, MATCH_SPDX_ID_ORDER};
pub use unknown_match::{unknown_match, MATCH_UNKNOWN, MATCH_UNKNOWN_ORDER};
```

## Test Coverage

| Module | Tests | Key Areas Tested |
|--------|-------|------------------|
| hash_match | 7 | Hash computation, exact matching, edge cases |
| spdx_lid | 31 | SPDX extraction, normalization, mapping, scoring |
| aho_match | 14 | Encoding, matching, coverage, position filtering |
| seq_match | 13 | Similarity, candidate selection, alignment |
| unknown_match | 10 | Region detection, ngram matching, scoring |
| match_refine | 24 | Merge, filter, score, pipeline |
| **Total Phase 4** | **99** | All match strategies and refinement |

**Overall**: 1479 tests passing (up from ~1416 in Phase 3)

## Quality Validation

✅ **Build**: `cargo build --lib` - Clean compilation
✅ **Clippy**: `cargo clippy --lib` - Zero warnings
✅ **Tests**: `cargo test --lib` - All 1479 tests passing
✅ **Code Quality**: No `#[allow(unused)]` or `#[allow(dead_code)]` hiding warnings

## Comparison with Python Reference

### Matches Python Behavior

✅ Hash computation: Exact SHA1 match with `array('h')` encoding
✅ SPDX-LID parsing: Full tag format support (case, spacing, URLs)
✅ Aho-Corasick: Multi-pattern exact matching with position verification
✅ Sequence matching: Set similarity + alignment (simplified but functional)
✅ Unknown detection: Region gap detection + ngram matching
✅ Refinement: Merge, filter, score operations

### Simplifications (to be enhanced)

⚠️ Sequence match: Block-based alignment (not full Cython implementation)
⚠️ Unknown match: Assumes automaton pre-built (no ngram building)
⚠️ Match refine: Basic merge/filter (Python has ~3000 lines)
⚠️ SPDX mapping: Direct comparison (no full bidirectional mapping table)
⚠️ Expression parsing: Simple split (not full parser with operator precedence)

**All simplifications documented** in `docs/license-detection/improvements/` for future enhancement.

## Integration Points

### Used from Previous Phases

- `Query`, `QueryRun` from `src/license_detection/query.rs`
- `LicenseIndex`, `LicenseMatch`, `Rule` from `src/license_detection/models.rs` and `src/license_detection/index/mod.rs`
- `Span` from `src/license_detection/spans.rs`
- Tokenization functions from `src/license_detection/tokenize.rs`

### For Future Phases

- Phase 5: License Expression Composition will use match results to build license expressions
- Phase 6: Detection Assembly will group matches into LicenseDetection objects
- Phase 7: Scanner Integration will wire matchers into the scanning pipeline

## Next Steps

**Phase 5: License Expression Composition**

- Parse license expressions from match results
- Build ScanCode key ↔ SPDX key mapping
- Combine match expressions using AND/OR operators
- Generate both ScanCode-key and SPDX-key versions

**Phase 6: Detection Assembly and Heuristics**

- Group raw matches into LicenseDetection objects
- Apply detection categorization (perfect, intro, clues, false positive, etc.)
- Implement detection heuristics for filtering and classification

**Phase 7: Scanner Integration**

- Create LicenseDetectionEngine with detect() API
- Wire into scanner pipeline
- Handle cross-file references

**Phase 8: Comprehensive Testing**

- Golden tests against Python ScanCode
- Performance benchmarking
- End-to-end validation

## Commit Information

**Commit**: `2ec93dc`
**Branch**: `feat-add-license-parsing`
**Message**: "Phase 4: Matching Strategies"
**Changes**: 12 files changed, 3023 insertions(+), 39 deletions(-)
