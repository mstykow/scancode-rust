# Tokenization Stage Code Quality Report

**Date**: 2026-02-24  
**Scope**: `query.rs`, `tokenize.rs`, `index/dictionary.rs`  
**Total Lines**: 2,959 lines

---

## Executive Summary

The tokenization stage is well-implemented with good documentation and comprehensive test coverage. However, several code quality issues were identified that could improve maintainability and reduce complexity:

| Category | High | Medium | Low |
|----------|------|--------|-----|
| Test Coverage | 1 | 3 | 2 |
| Data Structures | 1 | 2 | 1 |
| Algorithm Structure | 0 | 2 | 2 |
| Interfaces | 1 | 1 | 0 |

**Most Critical Issues**:

1. **STOPWORDS duplication** across `query.rs` and `tokenize.rs` (HIGH)
2. **SPDX detection logic** deeply embedded in `Query::with_options()` (HIGH)
3. **Missing test for `compute_query_runs`** - function is disabled but not tested (MEDIUM)

---

## 1. Test Coverage Analysis

### 1.1 Summary

| File | Lines of Code | Test Lines | Tests Count |
|------|---------------|------------|-------------|
| `query.rs` | 1,805 | ~770 | 75 |
| `tokenize.rs` | 755 | ~280 | 38 |
| `dictionary.rs` | 399 | ~240 | 13 |

**Assessment**: Test coverage is generally good, but there are gaps in coverage for edge cases and disabled features.

### 1.2 Issues Identified

#### Issue T1: STOPWORDS Duplication Between Files (HIGH)

**Location**:

- `query.rs:59-157` - STOPWORDS constant (99 lines)
- `tokenize.rs:19-103` - STOPWORDS static (85 lines)

**Problem**: The same STOPWORDS list is defined twice with identical content but different representations:

- `query.rs`: `const STOPWORDS: &[&str]`
- `tokenize.rs`: `static STOPWORDS: Lazy<HashSet<&'static str>>`

This violates DRY (Don't Repeat Yourself) and creates maintenance burden. Any update to stopwords must be made in two places.

**Recommendation**:

1. Define STOPWORDS once in `tokenize.rs` as the single source of truth
2. Export a function `is_stopword(word: &str) -> bool` from `tokenize.rs`
3. Remove the duplicate from `query.rs`
4. `query.rs:314` already creates a HashSet from the array - this becomes unnecessary

**Priority**: HIGH - Active maintenance risk

---

#### Issue T2: `compute_query_runs` Function Untested (MEDIUM)

**Location**: `query.rs:476-535`

**Problem**: The `compute_query_runs()` function is defined but currently disabled (lines 436-447 show it's not called). However, there are **no tests** for this function. If it's re-enabled in the future, bugs could go undetected.

```rust
// TODO: Query run splitting is currently disabled because it causes
// double-matching. The is_matchable() check with matched_qspans helps
// but doesn't fully prevent the issue. Further investigation needed.
// See: reference/scancode-toolkit/src/licensedcode/index.py:1056
// let query_runs = Self::compute_query_runs(...);
let query_runs: Vec<(usize, Option<usize>)> = Vec::new();
```

**Recommendation**: Add unit tests for `compute_query_runs()` even though it's disabled, to ensure correctness when re-enabled.

**Priority**: MEDIUM - Technical debt

---

#### Issue T3: Test Helper Duplication in `query.rs` (MEDIUM)

**Location**: `query.rs:1040-1042`

**Problem**: A local `create_query_test_index()` function is defined in the test module, but a nearly identical `create_test_index()` already exists in `test_utils.rs`.

```rust
// query.rs test module
fn create_query_test_index() -> LicenseIndex {
    create_test_index(&[("license", 0), ("copyright", 1), ("permission", 2)], 3)
}
```

vs.

```rust
// test_utils.rs
pub fn create_test_index(legalese: &[(&str, u16)], len_legalese: usize) -> LicenseIndex { ... }
```

**Recommendation**: Remove the local helper and use `create_test_index_default()` or create a specialized variant in `test_utils.rs` if the specific token set is commonly needed.

**Priority**: MEDIUM - Minor redundancy

---

#### Issue T4: No Integration Tests for Tokenization Pipeline (MEDIUM)

**Location**: Missing

**Problem**: Tests focus on individual functions but don't test the full tokenization pipeline from raw text to `Query` object. While `test_engine_detect_*` tests exercise this indirectly, there are no dedicated pipeline tests.

**Recommendation**: Add integration tests that:

1. Take sample license text
2. Verify token IDs produced
3. Verify `high_matchables` / `low_matchables` correctness
4. Verify `unknowns_by_pos` and `stopwords_by_pos` tracking

**Priority**: MEDIUM - Coverage gap

---

#### Issue T5: Redundant Test Cases for Simple Getters (LOW)

**Location**: `query.rs` tests

**Problem**: Several tests for trivial getter methods provide minimal value:

- `test_query_run_get_index` - Tests that `get_index()` returns the index
- `test_query_run_line_for_pos` - Tests `line_by_pos.get(pos)` wrapper

These tests are unlikely to catch bugs and add maintenance overhead.

**Recommendation**: Consider removing tests for trivial delegation methods. Focus test effort on complex logic.

**Priority**: LOW - Minor improvement

---

#### Issue T6: Missing Edge Case Tests (LOW)

**Location**: `tokenize.rs` tests

**Problem**: Some edge cases are not tested:

- Empty string followed by content
- Very long tokens (> 1000 chars)
- Tokens with only underscores
- Multiple consecutive `{{` or `}}` in required phrase parsing

**Recommendation**: Add property-based testing for tokenization edge cases using `proptest` crate.

**Priority**: LOW - Nice to have

---

### 1.3 Ignored Tests

**Good news**: Only one ignored test was found in the entire license detection module:

- `expression.rs:1406` - Not in tokenization scope

No ignored tests in the tokenization stage.

---

## 2. Data Structures Analysis

### 2.1 Summary

The data structures are well-designed overall, with clear separation between:

- `Query` - Input text tokenization state
- `QueryRun` - A slice of a query for efficient matching
- `TokenDictionary` - String to ID mapping
- `PositionSpan` - Token position tracking

### 2.2 Issues Identified

#### Issue D1: Many `#[allow(dead_code)]` Attributes (HIGH)

**Location**: Multiple files

**Problem**: Numerous fields and methods are marked with `#[allow(dead_code)]`:

`query.rs`:

- Lines 24, 30, 44, 58: `PositionSpan` methods
- Line 172: Entire `Query` struct
- Line 259: Entire `Query` impl block  
- Line 841: Entire `QueryRun` impl block

`tokenize.rs`:

- Line 18: `STOPWORDS`
- Line 114: `QUERY_PATTERN`
- Lines 132, 166, 197: Functions

`dictionary.rs`:

- Lines 123, 130, 136, 142: Methods

**Analysis**: Some `#[allow(dead_code)]` is intentional for API completeness or future use. However, the extent suggests either:

1. Features implemented but not yet integrated
2. API designed for Python parity but not needed in Rust
3. Test-only code that should be marked `#[cfg(test)]`

**Recommendation**:

1. Audit each `#[allow(dead_code)]` to determine if it's truly needed
2. For test-only utilities, use `#[cfg(test)]` instead
3. For planned features, add `// TODO: Used by <feature>` comments
4. Remove dead code that has no planned use

**Priority**: HIGH - Code hygiene

---

#### Issue D2: `normalize_text()` is a No-Op (MEDIUM)

**Location**: `tokenize.rs:198-200`

```rust
pub fn normalize_text(text: &str) -> String {
    text.to_string()
}
```

**Problem**: This function does nothing - it's a passthrough. The docstring explains it's intentionally empty ("Python implementation doesn't do special normalization") but this creates confusion about its purpose.

**Recommendation**:

1. If this will never do anything, remove it and add a comment where it was called
2. If it's for future extension, mark it with `// TODO: Implement normalization` and document what should be normalized

**Priority**: MEDIUM - Clarity

---

#### Issue D3: `PositionSpan` vs `spans::Span` Confusion (MEDIUM)

**Location**: `query.rs:7-20`

**Problem**: Two different span types exist:

- `PositionSpan` in `query.rs` - tracks token positions
- `Span` in `spans.rs` - tracks byte ranges for coverage

The docstring mentions this but the naming is confusing for newcomers.

**Recommendation**: Consider renaming to clarify purpose:

- `PositionSpan` -> `TokenPositionSpan` or `TokenRange`
- Add a module-level docstring explaining the distinction

**Priority**: MEDIUM - Clarity

---

#### Issue D4: `Query` Fields Could Be Grouped (LOW)

**Location**: `query.rs:171-257`

**Problem**: The `Query` struct has 14 fields that could be logically grouped:

1. Input: `text`, `index`
2. Tokens: `tokens`, `line_by_pos`
3. Tracking: `unknowns_by_pos`, `stopwords_by_pos`, `shorts_and_digits_pos`
4. Matchables: `high_matchables`, `low_matchables`
5. Flags: `has_long_lines`, `is_binary`
6. Runs: `query_run_ranges`, `spdx_lines`

**Recommendation**: Consider grouping related fields into sub-structs:

```rust
struct Query {
    text: String,
    index: &'a LicenseIndex,
    tokens: TokenSequence,     // tokens, line_by_pos
    tracking: TrackingInfo,    // unknowns_by_pos, stopwords_by_pos, shorts_and_digits_pos
    matchables: Matchables,    // high_matchables, low_matchables
    flags: QueryFlags,         // has_long_lines, is_binary
    runs: QueryRunInfo,        // query_run_ranges, spdx_lines
}
```

**Priority**: LOW - Nice to have, may not be worth the refactoring effort

---

## 3. Algorithm Structure Analysis

### 3.1 Summary

The algorithms are well-structured with clear separation of concerns. However, some functions have grown too large and could benefit from decomposition.

### 3.2 Issues Identified

#### Issue A1: `Query::with_options()` is Too Large (MEDIUM)

**Location**: `query.rs:306-464` (158 lines)

**Problem**: This function does too many things:

1. Binary detection
2. Long line detection
3. Tokenization loop (lines 331-420)
4. SPDX line detection (lines 366-417) - deeply nested
5. Matchables computation (lines 422-434)
6. Query run computation (commented out)

The SPDX detection logic (lines 366-417) is particularly problematic - it's 50+ lines of deeply nested conditional logic.

**Recommendation**: Extract helper functions:

```rust
fn detect_spdx_line(tokens_lower: &[String]) -> Option<usize> { ... }
fn compute_matchables(tokens: &[u16], len_legalese: usize) -> (HashSet<usize>, HashSet<usize>) { ... }
fn process_line_tokens(line: &str, index: &LicenseIndex, stopwords: &HashSet<&str>) -> LineResult { ... }
```

**Priority**: MEDIUM - Maintainability

---

#### Issue A2: SPDX Detection Logic is Repetitive (MEDIUM)

**Location**: `query.rs:371-407`

**Problem**: The SPDX prefix detection has three nearly identical code blocks:

```rust
// Check first three tokens
let first_three: Vec<&str> = tokens_lower.iter().take(3).map(|s| s.as_str()).collect();
let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
    || first_three == ["spdx", "licence", "identifier"];

// Check second three tokens (nearly identical)
let second_three: Vec<&str> = tokens_lower.iter().skip(1).take(3)...;

// Check third three tokens (nearly identical)
let third_three: Vec<&str> = tokens_lower.iter().skip(2).take(3)...;
```

**Recommendation**: Extract a helper function:

```rust
fn is_spdx_license_identifier(tokens: &[&str]) -> bool {
    tokens == ["spdx", "license", "identifier"] || tokens == ["spdx", "licence", "identifier"]
}

fn find_spdx_prefix_offset(tokens_lower: &[String]) -> Option<usize> {
    for offset in 0..3 {
        if offset + 3 <= tokens_lower.len() {
            let slice: Vec<&str> = tokens_lower.iter().skip(offset).take(3).map(|s| s.as_str()).collect();
            if is_spdx_license_identifier(&slice) {
                return Some(offset);
            }
        }
    }
    None
}
```

**Priority**: MEDIUM - DRY violation

---

#### Issue A3: `tokenize_with_stopwords()` Re-implements Logic (LOW)

**Location**: `tokenize.rs:347-375`

**Problem**: This function duplicates logic from `tokenize_without_stopwords()` with slight modifications for tracking stopwords. The core tokenization loop is written twice.

**Recommendation**: Refactor to share core logic:

```rust
fn tokenize_impl(text: &str, track_stopwords: bool) -> (Vec<String>, Option<HashMap<usize, usize>>) {
    // Core tokenization logic
}

pub fn tokenize_without_stopwords(text: &str) -> Vec<String> {
    tokenize_impl(text, false).0
}

pub fn tokenize_with_stopwords(text: &str) -> (Vec<String>, HashMap<usize, usize>) {
    let (tokens, stopwords) = tokenize_impl(text, true);
    (tokens, stopwords.unwrap_or_default())
}
```

**Priority**: LOW - Minor redundancy

---

#### Issue A4: `required_phrase_tokenizer` Iterator is Over-Engineered (LOW)

**Location**: `tokenize.rs:285-332`

**Problem**: The `RequiredPhraseTokenIter` struct and `TokenKind` enum are used only once to create an iterator that yields static strings. The iterator abstraction adds complexity without clear benefit.

**Recommendation**: Consider simplifying to a direct collection approach:

```rust
fn required_phrase_tokenizer(text: &str) -> Vec<TokenKind> {
    // Return Vec<TokenKind> directly
}
```

Or even simpler, process in `parse_required_phrase_spans()` without an intermediate iterator.

**Priority**: LOW - Minor simplification

---

## 4. Interfaces Analysis

### 4.1 Summary

The interfaces between tokenization and other stages are generally clear:

- **Input**: Raw text string
- **Output**: `Query` object with tokenized state

However, there are opportunities to clarify boundaries.

### 4.2 Issues Identified

#### Issue I1: `tokenize` Module Should Be Private (HIGH)

**Location**: `mod.rs:27`

```rust
mod tokenize;
```

**Problem**: The `tokenize` module is currently private (good). However, `query.rs` imports from it:

```rust
use crate::license_detection::tokenize::tokenize_without_stopwords;
```

This is fine, but STOPWORDS is defined in both modules, suggesting unclear ownership of the stopword concept.

**Recommendation**:

1. Keep `tokenize` module private
2. Make `tokenize` the single source of truth for stopwords
3. Add a public(ish) API for stopwords checking that `query.rs` can use:

   ```rust
   // In tokenize.rs
   pub(crate) fn is_stopword(word: &str) -> bool {
       STOPWORDS.contains(word)
   }
   ```

**Priority**: HIGH - Architectural clarity

---

#### Issue I2: No Clear Submodule Boundaries (MEDIUM)

**Location**: `query.rs` structure

**Problem**: `query.rs` contains multiple related concepts:

- `Query` struct and its methods
- `QueryRun` struct and its methods
- `PositionSpan` struct and its methods
- STOPWORDS constant
- SPDX detection logic

All in a single 1,805-line file.

**Recommendation**: Consider splitting into submodules:

```
query/
  mod.rs          // Re-exports Query, QueryRun
  query.rs        // Query struct
  query_run.rs    // QueryRun struct
  position_span.rs // PositionSpan struct
  spdx.rs         // SPDX detection logic
```

This would make the codebase more navigable and each file more focused.

**Priority**: MEDIUM - Organization

---

#### Issue I3: `TokenDictionary` Interface is Clean (POSITIVE)

**Location**: `dictionary.rs`

**Good**: The `TokenDictionary` interface is well-designed:

- Clear separation of legalese vs. regular tokens
- Simple `get()` and `get_or_assign()` API
- Reasonable `Default` implementation

No issues identified here.

---

## 5. Recommendations Summary

### High Priority (Do Soon)

1. **Eliminate STOPWORDS duplication** (T1, I1)
   - Define once in `tokenize.rs`
   - Export `is_stopword()` function
   - Remove duplicate from `query.rs`
   - Estimated effort: 2-4 hours

2. **Audit `#[allow(dead_code)]` usage** (D1)
   - Determine if each is intentional
   - Remove or document appropriately
   - Estimated effort: 2-4 hours

### Medium Priority (Do Eventually)

1. **Add tests for `compute_query_runs`** (T2)
   - Even though disabled, tests document expected behavior
   - Estimated effort: 1-2 hours

2. **Decompose `Query::with_options()`** (A1)
   - Extract SPDX detection helper
   - Extract matchables computation
   - Estimated effort: 4-8 hours

3. **Reduce SPDX detection repetition** (A2)
   - Create `is_spdx_license_identifier()` helper
   - Estimated effort: 1-2 hours

4. **Remove test helper duplication** (T3)
   - Consolidate in `test_utils.rs`
   - Estimated effort: 1 hour

5. **Remove or document `normalize_text()`** (D2)
   - Either remove or add TODO with purpose
   - Estimated effort: 30 minutes

### Low Priority (Nice to Have)

1. **Add property-based tests for tokenization** (T6)
   - Use `proptest` for edge case coverage
   - Estimated effort: 4-8 hours

2. **Consider splitting `query.rs` into submodules** (I2)
   - Would improve navigation
   - Estimated effort: 4-8 hours

3. **Refactor tokenization functions to share logic** (A3)
    - DRY improvement
    - Estimated effort: 2-4 hours

---

## 6. Metrics Summary

| Metric | Value |
|--------|-------|
| Total lines analyzed | 2,959 |
| Test-to-code ratio | ~0.7:1 |
| `#[allow(dead_code)]` count | ~15 |
| Duplicate code blocks | 2 (STOPWORDS, SPDX detection) |
| Functions > 50 lines | 2 (`with_options`, `compute_query_runs`) |
| Ignored tests | 0 (in scope) |

---

## Appendix A: File Structure

```
src/license_detection/
  query.rs           (1,805 lines) - Query and QueryRun structs
  tokenize.rs        (755 lines)   - Tokenization functions
  index/
    dictionary.rs    (399 lines)   - TokenDictionary struct
  test_utils.rs      (198 lines)   - Shared test helpers
```

---

## Appendix B: Test Coverage by Function

| Function | Tested | Notes |
|----------|--------|-------|
| `Query::new` | Yes | Multiple tests |
| `Query::with_options` | Partially | Tested via `new` |
| `Query::compute_query_runs` | **No** | Disabled but untested |
| `Query::detect_binary` | Yes | Via Query tests |
| `Query::detect_long_lines` | Yes | Via Query tests |
| `tokenize` | Yes | Comprehensive |
| `tokenize_without_stopwords` | Yes | Comprehensive |
| `tokenize_with_stopwords` | Yes | Basic tests |
| `parse_required_phrase_spans` | Yes | Edge cases covered |
| `TokenDictionary::*` | Yes | Full coverage |
