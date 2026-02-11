# Phase 2.3: Rule Threshold Computation - Implementation Report

## Summary

Successfully implemented rule threshold computation for license detection rules in the scancode-rust codebase. This implementation achieves feature parity with the Python ScanCode Toolkit's threshold computation logic.

## Implementation Details

### Files Created

1. **`src/license_detection/rules/thresholds.rs`** (210 lines)
   - Constants matching Python implementation (MIN_MATCH_LENGTH, MIN_MATCH_HIGH_LENGTH, SMALL_RULE, TINY_RULE)
   - `compute_thresholds_occurrences()` - computes occurrence-based thresholds
   - `compute_thresholds_unique()` - computes unique token-based thresholds
   - Comprehensive unit tests (13 functions) + integration tests (6 functions)

2. **`src/license_detection/index/token_sets.rs`** (145 lines)
   - `build_set_and_mset()` - builds token ID sets and multisets
   - `tids_set_counter()` - counts unique tokens in a set
   - `multiset_counter()` - counts total token occurrences in a multiset
   - `high_tids_set_subset()` - filters to high-value (legalese) tokens
   - `high_multiset_subset()` - filters multiset to high-value tokens
   - Unit tests for each function

### Files Modified

1. **`src/license_detection/models.rs`**
   - Added threshold fields to `Rule` struct:
     - `length_unique: usize` - count of unique token IDs
     - `high_length_unique: usize` - count of unique legalese tokens
     - `high_length: usize` - total legalese token occurrences
     - `min_matched_length: usize` - occurrence-based threshold
     - `min_high_matched_length: usize` - high-value occurrence threshold
     - `min_matched_length_unique: usize` - unique token threshold
     - `min_high_matched_length_unique: usize` - unique high-value threshold
     - `is_small: bool` - rule length < SMALL_RULE (15)
     - `is_tiny: bool` - rule length < TINY_RULE (6)
   - Updated tests to initialize new fields

2. **`src/license_detection/rules/loader.rs`**
   - Updated `parse_rule_file()` to initialize threshold fields with default values
   - Thresholds computed later during indexing phase (as in Python)

3. **`src/license_detection/rules/mod.rs`**
   - Added `pub mod thresholds;` module declaration

4. **`src/license_detection/index/mod.rs`**
   - Added `pub mod token_sets;` module declaration

## Threshold Computation Logic

### Key Concepts

1. **Token Classification**
   - Legalese tokens: IDs 0 to len_legalese-1 (high-value, rare words)
   - Junk tokens: IDs len_legalese and above (common words)

2. **Token Counting**
   - `length_unique`: count of distinct token IDs in rule
   - `high_length_unique`: count of distinct legalese token IDs
   - `high_length`: total count of legalese token occurrences (with duplicates)

3. **Threshold Determination**

   **Occurrence-based thresholds** (from `compute_thresholds_occurrences()`):
   - `min_matched_length`: minimum required matched token count
   - `min_high_matched_length`: minimum required matched legalese token count

   **Unique token thresholds** (from `compute_thresholds_unique()`):
   - `min_matched_length_unique`: minimum required distinct matched tokens
   - `min_high_matched_length_unique`: minimum required distinct matched legalese tokens

4. **Rule Classification**
   - `is_tiny`: length < 6 tokens (special handling from Python)
   - `is_small`: length < 15 tokens (exact match only in approx matching)

### Threshold Computation Rules (from Python Reference)

**Occurrence-based thresholds**:

- `minimum_coverage == 100`: exact match required (`min_matched_length = length`)
- `length < 3`: all tokens required (coverage = 100%)
- `3 <= length < 10`: high coverage (80%, all tokens)
- `10 <= length < 30`: medium coverage (50%, half tokens)
- `10 <= length < 200`: standard thresholds (MIN_MATCH_LENGTH=4, MIN_MATCH_HIGH_LENGTH=3)
- `length >= 200`: proportional (10% of tokens)

**Unique token thresholds**:

- `minimum_coverage == 100`: exact match required
- `length > 200`: proportional (10% of unique tokens)
- `length < 5`: all unique tokens required
- `5 <= length < 10`: all but 1 unique token
- `10 <= length < 20`: high unique threshold (equal to high_length_unique)
- `length >= 20`: standard (MIN_MATCH_LENGTH, half of high_length_unique)

## Python Reference Mapping

| Python Constant/File | Rust Constant/Function | Location |
|---------------------|------------------------|----------|
| `MIN_MATCH_LENGTH = 4` | `MIN_MATCH_LENGTH: usize = 4` | thresholds.rs:19 |
| `MIN_MATCH_HIGH_LENGTH = 3` | `MIN_MATCH_HIGH_LENGTH: usize = 3` | thresholds.rs:22 |
| `SMALL_RULE = 15` | `SMALL_RULE: usize = 15` | thresholds.rs:25 |
| `TINY_RULE = 6` | `TINY_RULE: usize = 6` | thresholds.rs:28 |
| `tids_set_counter = len` | `tids_set_counter()` | token_sets.rs:60 |
| `multiset_counter()` | `multiset_counter()` | token_sets.rs:70 |
| `build_set_and_tids_mset()` | `build_set_and_mset()` | token_sets.rs:26 |
| `high_tids_set_subset()` | `high_tids_set_subset()` | token_sets.rs:83 |
| `high_tids_multiset_subset()` | `high_multiset_subset()` | token_sets.rs:99 |
| `compute_thresholds_occurences()` | `compute_thresholds_occurrences()` | thresholds.rs:31 |
| `compute_thresholds_unique()` | `compute_thresholds_unique()` | thresholds.rs:79 |

## Test Coverage

### Unit Tests (19 tests)

- 13 individual threshold computation tests for different rule sizes and coverage levels
- 6 integration tests validating the complete pipeline (token sets → counting → thresholds)
- All edge cases covered:
  - 100% coverage
  - Tiny rules (length < 3)
  - Small rules (length < 10)
  - Medium rules (length < 30)
  - Large rules (length < 200)
  - Very large rules (length >= 200)
  - Rules with no legalese tokens
  - Rules with repeated tokens

### Verification Results

```text
✓ cargo build: No errors
✓ cargo clippy --lib: No warnings
✓ cargo test --lib: 1335 passed, 0 failed
✓ license_detection tests: 86 passed, 0 failed
✓ All threshold computation tests: 19 passed, 0 failed
```

## Future Phases

The threshold computation is part of the indexing phase. The threshold fields are now:

1. **Computed**: Token sets and multisets, token counts, thresholds, rule classification
2. **Not yet computed**: Integration into index construction (Phase 2.2 completion)
3. **Not yet used**: Thresholds will be used in match validation (Phase 3+)

Next implementation steps should integrate these threshold computations into the index construction process in `src/license_detection/index/mod.rs` when Phase 2.2 (Index Implementation) is completed.

## Parity with Python Implementation

✅ All algorithms match Python reference exactly:

- Threshold computation for all rule sizes
- Coverage percentage handling
- Legalese vs non-legalese token distinction
- Rule classification (tiny, small)
- Token counting in sets and multisets

✅ Constants match:

- MIN_MATCH_LENGTH = 4
- MIN_MATCH_HIGH_LENGTH = 3
- SMALL_RULE = 15
- TINY_RULE = 6

✅ Data structures match:

- Token sets (HashSet equivalent to Python intbitset)
- Token multisets (HashMap equivalent to Python defaultdict)
- High-value token filtering

🔲 Documentation improvements possible (see docs/license-detection/improvements/)
