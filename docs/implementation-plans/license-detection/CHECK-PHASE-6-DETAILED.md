# Phase 6 Dead Code Analysis - Detailed Report

**Date**: 2026-02-13
**Phase**: 6 (Detection Assembly and Heuristics)
**Status**: Phase 7 Complete - Scanner Integration Done

## Summary

Analysis of all `#[allow(dead_code)]` annotations in Phase 6 files reveals:

| Category | Count | Action |
|----------|-------|--------|
| JUSTIFIED (Public API / Future Use) | 7 | Keep with annotation |
| SHOULD REMOVE ANNOTATION (Actually Used) | 3 | Remove annotation |
| SHOULD REMOVE ITEM (Unused) | 0 | N/A |
| SHOULD INTEGRATE (Missing Integration) | 4 | Document for future work |

**Critical Finding**: Most `#[allow(dead_code)]` annotations are on items that ARE actually used (tests, internal helpers) but the Rust compiler considers "dead" because they're not called from production code paths.

---

## Phase 6 Core Files Analysis

### detection.rs

## Item: `FileRegion`

- **File**: `src/license_detection/detection.rs:123`
- **Type**: struct
- **Current Usage**: Used in `LicenseDetection.file_region: Option<FileRegion>` (line 118), populated at lines 631 and 711. Never read back.
- **Python Reference**: `FileRegion` class at `detection.py:150` is used for serialization in output JSON.
- **Verdict**: **JUSTIFIED**
- **Recommendation**: Keep. This struct is part of the public API and is populated for JSON output/serialization. Python uses it similarly - the values are written but not read within the detection logic.

---

### expression.rs

## Item: `ParseError`

- **File**: `src/license_detection/expression.rs:17`
- **Type**: enum
- **Current Usage**: Used extensively as return type in `parse_expression()`, `tokenize()`, `parse_tokens()`, etc. The annotation is INCORRECT - this IS used.
- **Python Reference**: No direct equivalent - Python uses `license_expression` library which has its own error types.
- **Verdict**: **SHOULD REMOVE ANNOTATION**
- **Recommendation**: Remove `#[allow(dead_code)]`. The enum is the error type for the entire expression parsing module.

## Item: `ValidationResult`

- **File**: `src/license_detection/expression.rs:78`
- **Type**: enum
- **Current Usage**: Only used in `validate_expression()` and tests (lines 801-813).
- **Python Reference**: No direct equivalent in the Python codebase - validation is done differently.
- **Verdict**: **JUSTIFIED** (Public API for validation)
- **Recommendation**: Keep. This is part of the public API for validating license expressions against known keys. Useful for external consumers.

## Item: `license_keys()`

- **File**: `src/license_detection/expression.rs:121`
- **Type**: method on `LicenseExpression`
- **Current Usage**: Called by `validate_expression()` (line 264) and extensively in tests.
- **Python Reference**: `Licensing.license_keys()` in the `license-expression` Python library.
- **Verdict**: **JUSTIFIED** (Public API)
- **Recommendation**: Keep. Part of public API for introspecting expressions. Used by validation and useful for callers.

## Item: `collect_keys()`

- **File**: `src/license_detection/expression.rs:130`
- **Type**: private method
- **Current Usage**: Called by `license_keys()` only.
- **Python Reference**: Internal implementation detail.
- **Verdict**: **JUSTIFIED** (Internal helper)
- **Recommendation**: Keep. This is a recursive helper for `license_keys()`. The annotation should remain.

## Item: `validate_expression()`

- **File**: `src/license_detection/expression.rs:257`
- **Type**: public function
- **Current Usage**: Only used in tests (lines 795-811).
- **Python Reference**: No direct equivalent - Python validates differently.
- **Verdict**: **JUSTIFIED** (Public API)
- **Recommendation**: Keep. This is a public API function for validating license expressions. It's part of the module's contract even if not used internally.

## Item: `CombineRelation`

- **File**: `src/license_detection/expression.rs:342`
- **Type**: enum
- **Current Usage**: Both variants used in `combine_expressions()`. `And` used in production code, `Or` used in tests.
- **Python Reference**: `combine_expressions()` in Python's `license-expression` library supports both AND and OR.
- **Verdict**: **SHOULD REMOVE ANNOTATION** (enum is used)
- **Recommendation**: Remove `#[allow(dead_code)]` from the enum. Both variants are used (And in production, Or in tests). The enum itself IS used.

## Item: `CombineRelation::Or`

- **File**: `src/license_detection/expression.rs:347`
- **Type**: enum variant
- **Current Usage**: Used in tests only (lines 399, 903, 924, 935).
- **Python Reference**: Python's `combine_expressions` supports OR relation.
- **Verdict**: **JUSTIFIED** (API completeness)
- **Recommendation**: Keep the annotation on this variant only. The `Or` variant is part of the API for completeness and is tested.

---

### spdx_mapping.rs

## Item: `spdx_to_scancode` field

- **File**: `src/license_detection/spdx_mapping.rs:34`
- **Type**: struct field
- **Current Usage**: Built in `build_from_licenses()` but never read outside of tests.
- **Python Reference**: Python has bidirectional mapping and uses `build_spdx_license_expression()` for ScanCode->SPDX. The reverse mapping is less commonly used.
- **Verdict**: **JUSTIFIED** (Bidirectional API)
- **Recommendation**: Keep. This enables the `spdx_to_scancode()` method which is part of the public API. Users may need to convert SPDX keys back to ScanCode keys.

## Item: `spdx_to_scancode()` method

- **File**: `src/license_detection/spdx_mapping.rs:133`
- **Type**: public method
- **Current Usage**: Used in tests (lines 338-370, 489, 521, 529).
- **Python Reference**: No direct Python equivalent in the main codebase.
- **Verdict**: **JUSTIFIED** (Public API)
- **Recommendation**: Keep. Part of the bidirectional mapping API. Useful for external consumers.

## Item: `scancode_count()` method

- **File**: `src/license_detection/spdx_mapping.rs:199`
- **Type**: public method
- **Current Usage**: Used in tests only (lines 480, 518).
- **Python Reference**: No equivalent.
- **Verdict**: **JUSTIFIED** (Public API for diagnostics)
- **Recommendation**: Keep. Useful for diagnostics and testing.

## Item: `spdx_count()` method

- **File**: `src/license_detection/spdx_mapping.rs:205`
- **Type**: public method
- **Current Usage**: Used in tests only (line 519).
- **Python Reference**: No equivalent.
- **Verdict**: **JUSTIFIED** (Public API for diagnostics)
- **Recommendation**: Keep. Useful for diagnostics and testing.

## Item: Convenience functions `scancode_to_spdx()`, `spdx_to_scancode()`, `expression_scancode_to_spdx()`

- **File**: `src/license_detection/spdx_mapping.rs:234, 249, 264`
- **Type**: public functions
- **Current Usage**: Used in tests only (lines 488-492).
- **Python Reference**: Similar convenience patterns exist in Python.
- **Verdict**: **JUSTIFIED** (Public API convenience)
- **Recommendation**: Keep. These are convenience functions for users who prefer free functions over methods.

---

## Other License Detection Files (Supporting Infrastructure)

### unknown_match.rs

## Item: `MATCH_UNKNOWN_ORDER`

- **File**: `src/license_detection/unknown_match.rs:67`
- **Type**: constant
- **Current Usage**: Used in tests only (line 340).
- **Python Reference**: `MATCH_UNKNOWN_ORDER = 6` in Python.
- **Verdict**: **JUSTIFIED** (API consistency)
- **Recommendation**: Keep. Matcher order constants are part of the API for consistency with Python.

### spdx_lid.rs

## Item: `MATCH_SPDX_ID_ORDER`

- **File**: `src/license_detection/spdx_lid.rs:40`
- **Type**: constant
- **Current Usage**: Used in tests only.
- **Python Reference**: `MATCH_SPDX_ID_ORDER = 2` in Python.
- **Verdict**: **JUSTIFIED** (API consistency)

## Item: `extract_spdx_expressions()`

- **File**: `src/license_detection/spdx_lid.rs:100`
- **Type**: public function
- **Current Usage**: Used in tests only.
- **Python Reference**: Similar function exists.
- **Verdict**: **JUSTIFIED** (Public API)

### seq_match.rs

## Item: `MATCH_SEQ_ORDER`

- **File**: `src/license_detection/seq_match.rs:18`
- **Type**: constant
- **Current Usage**: Used in tests only (line 662).
- **Verdict**: **JUSTIFIED** (API consistency)

### spans.rs

## Item: `Span.ranges` field

- **File**: `src/license_detection/spans.rs:14`
- **Type**: struct field
- **Current Usage**: Stored but never read directly.
- **Verdict**: **SHOULD INTEGRATE**
- **Recommendation**: The Span struct is incomplete. Either complete the implementation or remove if not needed for Phase 6.

## Item: `Span.add()`, `ranges_overlap()`, `merge_ranges()`, `is_empty()`, `len()`, `total_length()`

- **File**: `src/license_detection/spans.rs:67-116`
- **Type**: methods
- **Current Usage**: Used in tests only.
- **Verdict**: **SHOULD INTEGRATE** or **SHOULD REMOVE ITEM**
- **Recommendation**: The Span type appears to be partially implemented infrastructure for tracking matched positions. If needed for Phase 6, integrate. If not, consider removal.

### hash_match.rs

## Item: `MATCH_HASH_ORDER`

- **File**: `src/license_detection/hash_match.rs:23`
- **Type**: constant
- **Verdict**: **JUSTIFIED** (API consistency)

## Item: `index_hash()`

- **File**: `src/license_detection/hash_match.rs:54`
- **Type**: public function
- **Current Usage**: Used in tests only (line 234).
- **Verdict**: **JUSTIFIED** (Public API)

### aho_match.rs

## Item: `MATCH_AHO_ORDER`

- **File**: `src/license_detection/aho_match.rs:25`
- **Type**: constant
- **Verdict**: **JUSTIFIED** (API consistency)

### dictionary.rs

## Item: `is_legalese_token()`, `is_legalese()`, `len()`, `is_empty()`

- **File**: `src/license_detection/index/dictionary.rs:123-145`
- **Type**: methods
- **Current Usage**: Used in tests only.
- **Verdict**: **JUSTIFIED** (Public API for TokenDictionary)

### query.rs

## Item: `PositionSpan` struct and methods

- **File**: `src/license_detection/query.rs:17-50`
- **Type**: struct and methods
- **Current Usage**: Used in tests and `Query.subtract()`.
- **Verdict**: **JUSTIFIED** (Internal helper used in production)

## Item: `STOPWORDS` constant

- **File**: `src/license_detection/query.rs:58`
- **Type**: constant
- **Current Usage**: Used in `Query::with_options()`.
- **Verdict**: **SHOULD REMOVE ANNOTATION** (Actually used)

## Item: `Query` struct and many methods

- **File**: `src/license_detection/query.rs:172-615`
- **Type**: struct and methods
- **Current Usage**: Heavily used in production and tests.
- **Verdict**: **SHOULD REMOVE ANNOTATION** (Actually used)

## Item: `QueryRun` struct and methods

- **File**: `src/license_detection/query.rs:625-806`
- **Type**: struct and methods
- **Current Usage**: Heavily used in production and tests. Some fields (`len_legalese`, `digit_only_tids`) are stored but not used.
- **Verdict**: **PARTIAL** - Remove struct-level annotation, keep field-level annotations on unused fields

### index/mod.rs

## Item: `regular_rids`, `approx_matchable_rids` fields

- **File**: `src/license_detection/index/mod.rs:139, 156`
- **Type**: struct fields
- **Verdict**: **SHOULD INTEGRATE**
- **Recommendation**: These are populated during index building but not yet used in matching logic. They're needed for proper rule filtering.

## Item: Various LicenseIndex methods

- **File**: `src/license_detection/index/mod.rs:176-229`
- **Type**: methods
- **Verdict**: **JUSTIFIED** (Public API)

### loader.rs

## Item: `LicenseFrontmatter`, `RuleFrontmatter` structs

- **File**: `src/license_detection/rules/loader.rs:64, 146`
- **Type**: structs
- **Current Usage**: Used internally for parsing, not exposed.
- **Verdict**: **JUSTIFIED** (Internal parsing implementation)

### tokenize.rs

## Item: `STOPWORDS`, `QUERY_PATTERN`, `tokenize()`, `tokenize_without_stopwords()`, `normalize_text()`

- **File**: `src/license_detection/tokenize.rs:14, 109, 127, 161, 192`
- **Type**: static and functions
- **Current Usage**: `tokenize_without_stopwords()` is used in `Query::with_options()`. Others used in tests.
- **Verdict**: **SHOULD REMOVE ANNOTATION on `tokenize_without_stopwords`**, **JUSTIFIED** on others

---

## Actions Taken

### 1. Annotations Removed (Items that ARE used)

The following `#[allow(dead_code)]` annotations were removed because the items ARE actually used:

1. `ParseError` enum (expression.rs:17) - Used as return type throughout module
2. `CombineRelation` enum (expression.rs:342) - Used in `combine_expressions()`
3. `STOPWORDS` in query.rs (line 58) - Used in `Query::with_options()`
4. `Query` struct annotation (query.rs:172) - Heavily used
5. `QueryRun` struct-level annotation (query.rs:625) - Struct is used; added field-level annotations for unused fields `len_legalese` and `digit_only_tids`
6. `tokenize_without_stopwords()` (tokenize.rs:161) - Used in `Query::with_options()`

### 2. Items Kept (JUSTIFIED)

The following annotations remain because they're on:

- Public API items intended for external use (validation, convenience functions)
- API consistency items (matcher order constants)
- Bidirectional mapping fields
- Internal helpers used only by other annotated items

### 3. Items Flagged for Integration (SHOULD INTEGRATE)

The following items need future integration work:

1. **`Span` type (spans.rs)** - Incomplete implementation for tracking matched positions
2. **`regular_rids`, `approx_matchable_rids` (index/mod.rs)** - Populated but not used in matching logic

---

## Files Modified

| File | Changes |
|------|---------|
| `src/license_detection/expression.rs` | Removed `#[allow(dead_code)]` from `ParseError` and `CombineRelation` enums |
| `src/license_detection/query.rs` | Removed `#[allow(dead_code)]` from `STOPWORDS` and `Query` struct; changed `QueryRun` from struct-level to field-level annotations for `len_legalese` and `digit_only_tids` |
| `src/license_detection/tokenize.rs` | Removed `#[allow(dead_code)]` from `tokenize_without_stopwords()` |

---

## Verification

After changes:

- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test --lib` passes all tests

---

## Recommendations for Future Work

### 1. Complete Span Implementation (spans.rs)

The `Span` type is partially implemented but not integrated into the matching pipeline. Either:

- Complete the implementation and use it for tracking matched positions
- Remove it if the functionality is covered elsewhere

### 2. Use Rule Classification Sets (index/mod.rs)

The `regular_rids` and `approx_matchable_rids` sets are populated during index building but not used in matching logic. They should be used to:

- Filter rules during candidate selection
- Optimize matching by excluding false positive rules from certain phases

### 3. Consider Removing Unused Tokenize Functions

If `tokenize()` and `normalize_text()` are not needed for the current implementation, consider removing them to reduce maintenance burden.
