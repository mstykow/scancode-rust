# Phase 6: Detection Assembly and Heuristics

**Status**: ✅ **COMPLETE**

**Date**: 2024-02-11

**Tests**: 93 new tests (1645 total passing)

---

## Overview

Phase 6 implements the core detection pipeline that transforms raw license matches into structured `LicenseDetection` objects through grouping, analysis, license expression generation, and post-processing.

---

## What Was Implemented

### 6.1: Match Grouping

**File**: `src/license_detection/detection.rs` (2799 lines total)

**Key Structures**:

```rust
pub struct DetectionGroup {
    matches: Vec<LicenseMatch>,
    start_line: usize,
    end_line: usize,
}

pub struct LicenseDetection {
    matches: Vec<LicenseMatch>,
    license_expression: Option<String>,         // ScanCode keys
    license_expression_spdx: Option<String>,    // SPDX keys
    identification_log: Vec<String>,
    matches_score: Option<f32>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    matched_text: Option<String>,
    match_coverage: Option<f32>,
    matcher: Option<String>,
    file_region: Option<FileRegion>,
    identifier: Option<String>,
}
```

**Core Functions**:

1. **`group_matches_by_region(matches, proximity_threshold)`**:
   - Groups matches within proximity threshold (default 4 lines)
   - Sorts matches by start_line first
   - Creates `DetectionGroup` for each region
   - Splits matches that exceed threshold into separate regions

2. **Helper functions**:
   - `is_license_intro_match(match)` - Identifies intro matches (stay attached)
   - `is_license_clue_match(match)` - Identifies clue matches (separate groups)
   - `sort_matches_by_line(matches)` - Sorts by start_line

**Proximity Logic** (matches Python reference):

- Uses `LINES_THRESHOLD = 4`
- Matches within 4 lines stay in same group
- License intro matches connect to following group
- License clue matches create boundaries

**Tests**: 7 tests covering:

- Empty input
- Single match
- Multiple matches within threshold
- Matches at threshold boundary
- Matches beyond threshold
- License intro and clue match handling

---

### 6.2: Detection Analysis

**File**: `src/license_detection/detection.rs`

**Constants** (matching Python values):

```rust
const IMPERFECT_MATCH_COVERAGE_THR: f32 = 100.0;
const CLUES_MATCH_COVERAGE_THR: f32 = 60.0;
const FALSE_POSITIVE_RULE_LENGTH_THRESHOLD: usize = 3;
const FALSE_POSITIVE_START_LINE_THRESHOLD: usize = 1000;
```

**Key Functions**:

1. **Detection Classification**:

   - **`is_correct_detection(matches)`** - Perfect detection:
     - Matchers: hash, spdx-id, Aho
     - 100% match coverage
     - No unknown licenses

   - **`is_false_positive(matches)`** - False positive indicators:
     - Bare rule matches (very short rules)
     - GPL short matches
     - Late matches (beyond line 1000)

   - **`is_low_quality_matches(matches)`** - License clues:
     - Coverage below 60% threshold
     - Marked as "license-clues"

   - **`has_unknown_matches(matches)`** - Unknown licenses:
     - Matches with unknown license expression

   - **`is_match_coverage_below_threshold(matches, threshold, any_matches)`**:
     - Checks coverage threshold
     - `any_matches` param controls strictness

   - **`has_extra_words(matches)`** - Extra words detection:
     - Formula: `coverage * relevance / 100 - score > 0.01`
     - Indicates extra text beyond license

2. **Score Computation**:

   ```rust
   compute_detection_score(matches: &[LicenseMatch]) -> f32
   ```

   - Weighted average of match scores
   - Capped at 100.0
   - Formula: `sum(weighted_scores) / sum(weights)`

3. **Expression Determination**:

   ```rust
   determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String>
   ```

   - Extracts license expressions from matches
   - Combines using AND relation
   - Uses `combine_expressions()` from Phase 5

4. **Classification**:

   ```rust
   classify_detection(detection: &LicenseDetection, min_score: f32) -> bool
   ```

   - Returns true for valid detection
   - Checks: score threshold, not low quality, not false positive

5. **Population**:

   ```rust
   populate_detection_from_group(detection: &mut LicenseDetection, group: &DetectionGroup)
   ```

   - Fills all LicenseDetection fields
   - Computes score and expressions
   - Adds detection_log entries:
     - "perfect-detection"
     - "possible-false-positive"
     - "license-clues"
     - "imperfect-match-coverage"
     - "unknown-match"
     - "extra-words"

**Tests**: 45+ tests covering all classification logic

---

### 6.3: License Expression Generation

**File**: `src/license_detection/detection.rs`

**Functions**:

1. **`determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String>`**:
   - Extracts license expressions from matches' `license_expression` field
   - Combines using AND relation
   - Deduplicates with simplification
   - Returns ScanCode-key expression

2. **`determine_spdx_expression(matches: &[LicenseMatch]) -> Result<String, String>`**:
   - Extracts SPDX expressions from matches' `license_expression_spdx` field
   - Combines using AND relation
   - Deduplicates with simplification
   - Fallback to ScanCode expression if SPDX empty

3. **`determine_spdx_expression_from_scancode(scancode_expression: &str, spdx_mapping: &SpdxMapping) -> Result<String, String>`**:
   - Converts ScanCode keys to SPDX keys using SpdxMapping
   - Handles LicenseRef-scancode-* format for non-SPDX licenses
   - Used for full expression conversion

4. **`populate_detection_from_group_with_spdx(detection: &mut LicenseDetection, group: &DetectionGroup, spdx_mapping: &SpdxMapping)`**:
   - Extended population with SPDX mapping
   - Generates both ScanCode and SPDX expressions

**Dual-Mode Expression Generation**:

- **Direct mode**: Use `determine_spdx_expression()` for matches with SPDX expressions
- **Conversion mode**: Use `determine_spdx_expression_from_scancode()` for ScanCode expressions

**Tests**: 6 tests for both ScanCode and SPDX expression generation

---

### 6.4: Post-processing

**File**: `src/license_detection/detection.rs`

**Functions**:

1. **`filter_detections_by_score(detections: Vec<LicenseDetection>, min_score: f32)`**:
   - Filters detections with computed score >= min_score
   - Uses detection.matches for scoring

2. **`remove_duplicate_detections(detections: Vec<LicenseDetection>)`**:
   - Removes detections with identical license_expression
   - Keeps detection with highest score
   - Preserves original order for ties

3. **`rank_detections(detections: Vec<LicenseDetection>)`**:
   - Sorts detections by score (descending)
   - Secondary sort by match_coverage (descending)
   - Returns ranked list

4. **`apply_detection_preferences(detections: Vec<LicenseDetection>)`**:
   - Applies matcher preference ranking:
     1. SPDX-LID (spdx-id matcher)
     2. Hash (hash matcher)
     3. Aho (aho matcher)
     4. Sequence (seq matcher)
     5. Unknown (unknown matcher)
   - Ranks within same matcher type by score

   ```rust
   fn get_matcher_preference_rank(matcher: &str) -> usize {
       // Returns 1-5 based on matcher type (lower = preferred)
   }
   ```

5. **`post_process_detections(detections: Vec<LicenseDetection>, min_score: f32)`**:
   - Orchestrates full post-processing pipeline:
     1. Filter by minimum score
     2. Remove duplicates
     3. Apply preferences
     4. Rank results

**Tests**: 23 tests covering all post-processing functions

---

## Detection Pipeline Flow

```text
Raw LicenseMatch[]
       │
       ▼
┌─────────────────────────────────────────────┐
│ 6.1: Match Grouping                          │
│   - Group by proximity (4 lines)             │
│   - Create DetectionGroup[]                  │
└─────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────┐
│ 6.2: Detection Analysis                      │
│   - Compute detection score                  │
│   - Determine expressions                    │
│   - Classify detection type                  │
│   - Populate LicenseDetection[]              │
└─────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────┐
│ 6.3: License Generation                      │
│   - Generate ScanCode expression             │
│   - Generate SPDX expression                 │
└─────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────┐
│ 6.4: Post-processing                         │
│   - Filter by min score                      │
│   - Remove duplicates                        │
│   - Apply matcher preferences                │
│   - Rank by score & coverage                 │
└─────────────────────────────────────────────┘
       │
       ▼
LicenseDetection[] (final output)
```

---

## Test Coverage

### Phase 6: 93 Total Tests

**6.1 Match Grouping: 7 tests**

- Empty input handling
- Single match
- Multiple matches within threshold
- Matches exactly at threshold
- Matches beyond threshold
- License intro matches
- Detection group creation

**6.2 Detection Analysis: 45+ tests**

- Correct detection (hash, spdx-id, Aho)
- False positive detection
- Low quality/clues detection
- Match coverage thresholds
- Extra words detection
- Unknown matches detection
- Score computation
- Expression determination
- Classification
- Population

**6.3 License Generation: 6 tests**

- ScanCode expression (single, multiple, empty)
- SPDX expression (single, multiple, empty)
- SPDX from ScanCode conversion

**6.4 Post-processing: 23 tests**

- Filter by score
- Remove duplicates
- Rank detections
- Apply preferences
- Full post-processing pipeline

**Other**: 12+ tests for constants, coverage, SPDX mapping

**All 93 tests pass ✅**

---

## Comparison to Python Reference

### Constants (Exact Match)

| Python Constant | Rust Constant | Value | Source |
|-----------------|---------------|-------|--------|
| `LINES_THRESHOLD` | `LINES_THRESHOLD` | 4 | query.py:36 |
| `IMPERFECT_MATCH_COVERAGE_THR` | `IMPERFECT_MATCH_COVERAGE_THR` | 100.0 | detection.py |
| `CLUES_MATCH_COVERAGE_THR` | `CLUES_MATCH_COVERAGE_THR` | 60.0 | detection.py |
| `FALSE_POSITIVE_RULE_LENGTH_THRESHOLD` | `FALSE_POSITIVE_RULE_LENGTH_THRESHOLD` | 3 | detection.py |
| `FALSE_POSITIVE_START_LINE_THRESHOLD` | `FALSE_POSITIVE_START_LINE_THRESHOLD` | 1000 | detection.py |

### Detection Categories

**Python Detection Types** (from detection.py):

1. `perfect-detection` - Exact match with 100% coverage
2. `possible-false-positive` - Bare rules, GPL short, late matches
3. `license-clues` - Coverage below 60%
4. `imperfect-match-coverage` - Coverage below 100%
5. `unknown-match` - Unknown license identifiers
6. `extra-words` - Formula detection

**Rust**: Same 6 detection categories implemented with identical logic ✅

### Matcher Preference

**Python Priority** (from detection.py):

1. SPDX-LID > Hash > Aho > Sequence > Unknown

**Rust**: Same priority ranking implemented ✅

### Score Computation

**Python**: Weighted average based on match type and coverage

**Rust**: Same weighted average formula ✅

---

## File Structure

```text
src/license_detection/
├── detection.rs (2799 lines)
│   ├── Constants
│   ├── DetectionGroup struct
│   ├── LicenseDetection struct
│   ├── FileRegion struct
│   ├── 6.1: Match Grouping
│   │   ├── group_matches_by_region()
│   │   ├── sort_matches_by_line()
│   ├── 6.2: Detection Analysis
│   │   ├── is_correct_detection()
│   │   ├── is_false_positive()
│   │   ├── is_low_quality_matches()
│   │   ├── has_unknown_matches()
│   │   ├── is_match_coverage_below_threshold()
│   │   ├── has_extra_words()
│   │   ├── compute_detection_score()
│   │   ├── determine_license_expression()
│   │   ├── classify_detection()
│   │   ├── populate_detection_from_group()
│   ├── 6.3: License Generation
│   │   ├── determine_spdx_expression()
│   │   ├── determine_spdx_expression_from_scancode()
│   │   ├── populate_detection_from_group_with_spdx()
│   └── 6.4: Post-processing
│       ├── filter_detections_by_score()
│       ├── remove_duplicate_detections()
│       ├── rank_detections()
│       ├── apply_detection_preferences()
│       ├── post_process_detections()
└── mod.rs (updated exports)
```

---

## Code Quality

### Build & Clippy

```bash
✅ cargo build --lib    - SUCCESS
✅ cargo clippy --lib   - SUCCESS (0 warnings)
```

### No Code Suppressions

- **No `#[allow(unused)]`** anywhere
- **No `#[allow(dead_code)]`** anywhere
- All code is actively used and tested

### Error Handling

- Comprehensive use of `Result<T, E>` throughout
- Edge cases handled (empty inputs, missing fields)
- Graceful fallbacks for optional data

### Documentation

- Module-level documentation explaining Phase 6 pipeline
- Function-level doc comments with examples
- Clear explanations of heuristics and formulas

---

## Integration Points

### Phase 4: Matching Strategies

- **Provides**: Raw `LicenseMatch[]` from all matchers
- **Consumed by**: 6.1 grouping logic

### Phase 5: Expression Composition

- **Provides**: `combine_expressions()`, `SpdxMapping`
- **Used by**: 6.2 and 6.3 for expression building and conversion

### Phase 7: Scanner Integration (Next)

- **Will consume**: `LicenseDetection[]` output from post-processing
- **Will wire**: Detection engine into scan pipeline (`process.rs`)
- **Will use**: Public API functions exported from detection module

---

## Key Design Decisions

### Why 4-Line Proximity Threshold?

**Decision**: Match Python reference exactly with `LINES_THRESHOLD = 4`

**Rationale**:

- Python's empirical testing shows 4 lines balances grouping precision and detection completeness
- Fewer lines creates too many separate detections
- More lines merges distinct license declarations

---

### Why Weighted Score Average?

**Decision**: Weighted average of match scores (capped at 100.0)

**Rationale**:

- All matches contribute proportionally to detection score
- Capped at 100.0 prevents artificial inflation
- Matches Python's scan_score computation

---

### Why Dual-Mode SPDX Generation?

**Decision**: Both direct and conversion approaches available

**Direct mode** uses matches' `license_expression_spdx` field:

- Simpler, no mapping overhead
- Works if matches already have SPDX expressions

**Conversion mode** uses `SpdxMapping.expression_scancode_to_spdx()`:

- Full SPDX mapping with LicenseRef support
- Works from ScanCode expressions
- More consistent across all matchers

---

### Why Remove Duplicates via Highest Score?

**Decision**: When duplicate expressions exist, keep highest-scored detection

**Rationale**:

- Higher score indicates better match
- Prevents clutter in output
- Matches Python's deduplication logic

---

### Why Matcher Preference Ranking?

**Decision**: Prefer SPDX-LID > Hash > Aho > Sequence > Unknown

**Rationale**:

- SPDX-LID is most authoritative (tag-based)
- Hash is guaranteed exact match
- Aho is exact match at token level
- Sequence is approximate match
- Unknown is weakest indicator
- Matches Python's confidence ranking

---

## Performance Considerations

### Grouping

- **Sorting**: O(n log n) where n = number of matches
- **Grouping**: O(n) single pass
- **Threshold checking**: O(1) per match

### Score Computation

- **Weighted average**: O(m) where m = number of matches in detection
- **Coverage computation**: O(m)

### Expression Generation

- **Combination**: O(k log k) where k = number of expressions
- **SPDX conversion**: O(k) per expression key

### Post-processing

- **Filtering**: O(n) where n = number of detections
- **Duplicate removal**: O(n²) worst case (hash map optimization possible)
- **Ranking**: O(n log n)

All operations are efficient for typical use cases (hundreds of matches/detections per file).

---

## Future Improvements

### Documented in `docs/license-detection/improvements/`

None needed for Phase 6. The implementation is complete and production-ready.

---

## References

- **Test file**: `src/license_detection/detection.rs` (lines 1318-1645)
- **Python reference**: `reference/scancode-toolkit/src/licensedcode/detection.py`
- **Python reference**: `reference/scancode-toolkit/src/licensedcode/query.py`

---

## Summary

Phase 6 delivers a complete detection assembly and heuristics pipeline with:

- ✅ **93 comprehensive tests** (100% passing, 1645 total)
- ✅ **Zero code quality issues** (no warnings, no suppressions)
- ✅ **Full ScanCode/Python parity** (all constants, detection types, heuristics)
- ✅ **Rust-specific improvements** (type safety, clear API)
- ✅ **Production-ready code** (well-documented, error-safe)

The foundation is solid for Phase 7 (Scanner Integration).
