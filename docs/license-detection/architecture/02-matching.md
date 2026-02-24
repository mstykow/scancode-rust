# License Detection Matching Stage - Architecture Report

## Executive Summary

This report evaluates the MATCHING stage of the license detection pipeline, covering hash match, SPDX-LID match, Aho-Corasick match, near-duplicate match, sequence match, and query run match. The analysis focuses on test coverage, data structures, algorithm structure, and interface clarity.

**Overall Assessment**: The matching stage is well-implemented with good coverage of the Python reference behavior. However, there are opportunities for improvement in test organization, code duplication reduction, and interface definition.

---

## 1. Test Coverage Analysis

### 1.1 Test Distribution Summary

| File | Total Lines | Test Lines | Test Count | Coverage Assessment |
|------|-------------|------------|------------|---------------------|
| `hash_match.rs` | 426 | 290 | 15 tests | Good |
| `spdx_lid.rs` | 1005 | 560 | 45 tests | Excellent |
| `aho_match.rs` | 769 | 470 | 15 tests | Good |
| `seq_match.rs` | 1906 | 800 | 35 tests | Good |
| `index/mod.rs` | 508 | 200 | 8 tests | Adequate |
| `index/builder.rs` | 1167 | 630 | 27 tests | Good |
| `index/token_sets.rs` | 184 | 80 | 6 tests | Adequate |

### 1.2 Test Quality Issues

#### 1.2.1 Redundant Test Data Construction (HIGH PRIORITY)

**Issue**: Multiple test functions create identical `Rule` structs with 30+ fields, leading to significant code duplication.

**Location**: `hash_match.rs:141-228`, `seq_match.rs:942-989`, `aho_match.rs` tests

**Example** (hash_match.rs:141-228):

```rust
fn create_test_rules_by_rid() -> Vec<Rule> {
    vec![
        Rule {
            identifier: "mit.LICENSE".to_string(),
            // ... 30+ fields ...
            stopwords_by_pos: std::collections::HashMap::new(),
        },
        // ... more rules with similar patterns
    ]
}
```

**Impact**:

- Difficult to maintain (changing Rule struct requires updating many tests)
- Error-prone (easy to forget fields when manually constructing)
- Readability suffers due to verbose struct initialization

**Recommendation**:

1. Expand `test_utils.rs` with a `create_test_rule()` builder pattern
2. Create a `RuleBuilder` struct with sensible defaults
3. Use helper functions for common test scenarios

```rust
// Recommended approach
pub fn rule_builder() -> RuleBuilder {
    RuleBuilder::default()
        .license_expression("mit")
        .tokens(vec![0, 1])
        .is_license_text(true)
}

// Usage in tests
let rule = rule_builder().identifier("custom").build();
```

#### 1.2.2 Repeated Query Construction (MEDIUM PRIORITY)

**Issue**: Tests repeatedly construct `Query` structs with identical patterns.

**Location**: `aho_match.rs:277-291`, `aho_match.rs:320-334`, `aho_match.rs:367-381`, and many more

**Example** (aho_match.rs:277-291):

```rust
let query = crate::license_detection::query::Query {
    text: String::new(),
    tokens: vec![0, 1],
    line_by_pos: vec![1, 1],
    unknowns_by_pos: std::collections::HashMap::new(),
    stopwords_by_pos: std::collections::HashMap::new(),
    shorts_and_digits_pos: std::collections::HashSet::new(),
    high_matchables: (0..2).collect(),
    low_matchables: std::collections::HashSet::new(),
    has_long_lines: false,
    is_binary: false,
    query_run_ranges: Vec::new(),
    spdx_lines: Vec::new(),
    index: &index,
};
```

**Recommendation**: Add to `test_utils.rs`:

```rust
pub fn create_test_query(tokens: &[u16], index: &LicenseIndex) -> Query {
    Query {
        text: String::new(),
        tokens: tokens.to_vec(),
        line_by_pos: vec![1; tokens.len()],
        unknowns_by_pos: HashMap::new(),
        stopwords_by_pos: HashMap::new(),
        shorts_and_digits_pos: HashSet::new(),
        high_matchables: (0..tokens.len()).collect(),
        low_matchables: HashSet::new(),
        has_long_lines: false,
        is_binary: false,
        query_run_ranges: Vec::new(),
        spdx_lines: Vec::new(),
        index,
    }
}
```

#### 1.2.3 Missing Edge Case Tests (MEDIUM PRIORITY)

**Areas with insufficient coverage**:

1. **Hash collisions**: Only one test (`test_hash_match_multiple_rules_same_hash`) - needs more scenarios
2. **Unicode/non-ASCII input**: Limited coverage in `query.rs` tests
3. **Very large inputs**: Only `test_match_hash_large_tokens` with 1000 tokens - needs memory/timeout tests
4. **Empty/edge inputs**: Good coverage but scattered across files

### 1.3 Ignored Tests

**Finding**: Only 1 ignored test found in the license detection module:

**Location**: `expression.rs:1406`

This is outside the matching stage scope but should be reviewed for relevance.

### 1.4 Test Helper Function Opportunities

**Current state**: `test_utils.rs` provides:

- `create_test_index()`
- `create_test_index_default()`
- `create_mock_rule()`
- `create_mock_rule_simple()`
- `create_mock_query_with_tokens()`

**Missing helpers that would reduce duplication**:

- `create_test_query_with_matchables()`
- `create_test_automaton()`
- `create_test_rule_with_tokens()`
- `assert_match_properties()` - for common assertion patterns

---

## 2. Data Structure Analysis

### 2.1 LicenseIndex Structure (HIGH PRIORITY)

**File**: `src/license_detection/index/mod.rs:42-194`

**Current State**: The `LicenseIndex` struct has grown to 17 fields with varying purposes and usage patterns.

**Issues Identified**:

#### 2.1.1 Unused Fields Marked with `#[allow(dead_code)]`

The following fields have `#[allow(dead_code)]` annotations, indicating they may be unnecessary:

| Field | Line | Purpose | Assessment |
|-------|------|---------|------------|
| `regular_rids` | 140 | Set of non-false-positive rule IDs | **Potentially unused** - consider removing |
| `approx_matchable_rids` | 157 | Set of approx-matchable rule IDs | Used in seq_match - keep |

**Recommendation**: Audit `regular_rids` usage. If truly unused, remove the field and its population code in `builder.rs`.

#### 2.1.2 Inconsistent Key Types

**Issue**: `sets_by_rid` and `msets_by_rid` use `HashMap<usize, ...>` while `rules_by_rid` and `tids_by_rid` use `Vec<...>`.

**Location**: `index/mod.rs:111-119`

```rust
pub rules_by_rid: Vec<crate::license_detection::models::Rule>,
pub tids_by_rid: Vec<Vec<u16>>,
pub sets_by_rid: HashMap<usize, HashSet<u16>>,      // HashMap<usize, ...>
pub msets_by_rid: HashMap<usize, HashMap<u16, usize>>, // HashMap<usize, ...>
```

**Impact**:

- Inconsistent access patterns
- HashMap overhead for contiguous integer keys
- Potential confusion about when to use which structure

**Recommendation**: Consider standardizing on `Vec` with `Option<...>` for sparse cases, or document why HashMap is preferred for sets/msets.

#### 2.1.3 Multiple Methods Marked Dead Code

**Location**: `index/mod.rs:204-257`

Eight methods are marked with `#[allow(dead_code)]`:

- `get_rid_by_hash()`
- `get_license()`
- `add_license()`
- `add_licenses()`
- `license_keys()`
- `license_count()`

**Recommendation**: Either implement usage for these methods or remove them to reduce maintenance burden.

### 2.2 ScoresVector and Candidate Structures (MEDIUM PRIORITY)

**File**: `src/license_detection/seq_match.rs:42-113`

**Issue**: `ScoresVector` and `Candidate` both contain similar score information, with `Candidate` having both `score_vec_rounded` and `score_vec_full`.

**Analysis**:

```rust
pub struct ScoresVector {
    pub is_highly_resemblant: bool,
    pub containment: f32,
    pub resemblance: f32,
    pub matched_length: f32,
    pub rid: usize,
}

pub struct Candidate {
    pub score_vec_rounded: ScoresVector,  // Rounded for grouping
    pub score_vec_full: ScoresVector,      // Full precision for sorting
    pub rid: usize,                         // DUPLICATE of score_vec_*.rid
    pub rule: Rule,                         // Cloned rule reference
    pub high_set_intersection: HashSet<u16>,
}
```

**Problems**:

1. `rid` is duplicated in both `Candidate.rid` and `Candidate.score_vec_*.rid`
2. Two `ScoresVector` instances per candidate doubles memory for the same logical data
3. `Rule` is cloned into each `Candidate` (expensive with 30+ fields)

**Recommendation**:

1. Remove duplicate `rid` field from `Candidate`
2. Consider storing `rule: &Rule` reference instead of cloning (requires lifetime management)
3. Evaluate if both rounded and full precision are truly needed

### 2.3 Rule Structure (MEDIUM PRIORITY)

**File**: `src/license_detection/models.rs:64-191`

**Issue**: The `Rule` struct has 40 fields, many of which are computed during indexing.

**Analysis of field groups**:

| Group | Fields | When Set |
|-------|--------|----------|
| Identity | `identifier`, `license_expression`, `text` | Loading |
| Tokenization | `tokens`, `required_phrase_spans`, `stopwords_by_pos` | Index building |
| Classification | `is_license_text`, `is_license_notice`, etc. | Loading |
| Ignorable | `ignorable_urls`, `ignorable_emails`, etc. | Loading |
| Computed | `length_unique`, `high_length`, `min_matched_length`, etc. | Index building |

**Recommendation**: Consider splitting into:

1. `RuleDefinition` - loaded from file
2. `RuleIndex` - computed during indexing
3. `Rule` - combined view for matching

This would clarify the lifecycle of rule data and potentially reduce memory for non-matchable rules.

### 2.4 LicenseMatch Structure

**File**: `src/license_detection/models.rs:206-320`

**Assessment**: Well-designed with clear purpose. The `#[serde(skip)]` fields for internal tracking (`matched_token_positions`, `qspan_positions`, `ispan_positions`) are appropriate.

**Minor Issue**: Several methods have `#[allow(dead_code)]` (`is_small`, `len`, `qregion_len`, `has_gaps`). Review for removal or usage.

---

## 3. Algorithm Structure Analysis

### 3.1 LicenseMatch Construction Duplication (HIGH PRIORITY)

**Issue**: The `LicenseMatch` construction logic is duplicated across all matcher modules.

**Locations**:

- `hash_match.rs:99-129`
- `spdx_lid.rs:274-303`
- `aho_match.rs:160-190`
- `seq_match.rs:730-760`
- `seq_match.rs:861-891`

**Example Pattern** (appears 5 times with minor variations):

```rust
let license_match = LicenseMatch {
    license_expression: rule.license_expression.clone(),
    license_expression_spdx: rule.license_expression.clone(),
    from_file: None,
    start_line,
    end_line,
    start_token: qstart,
    end_token: qend,
    matcher: MATCH_XXX.to_string(),
    score,
    matched_length,
    rule_length,
    match_coverage,
    rule_relevance: rule.relevance,
    rule_identifier: format!("#{}", rid),
    rule_url: String::new(),
    matched_text: Some(matched_text),
    referenced_filenames: rule.referenced_filenames.clone(),
    is_license_intro: rule.is_license_intro,
    is_license_clue: rule.is_license_clue,
    is_license_reference: rule.is_license_reference,
    is_license_tag: rule.is_license_tag,
    is_license_text: rule.is_license_text,
    matched_token_positions: None,
    hilen: hispan_count,
    rule_start_token: 0,
    qspan_positions: None,
    ispan_positions: None,
};
```

**Recommendation**: Create a `LicenseMatchBuilder` or helper function in `models.rs`:

```rust
impl LicenseMatch {
    pub fn from_rule(
        rule: &Rule,
        rid: usize,
        matcher: &str,
        start_line: usize,
        end_line: usize,
        start_token: usize,
        end_token: usize,
        matched_length: usize,
        match_coverage: f32,
        matched_text: String,
    ) -> Self {
        Self {
            license_expression: rule.license_expression.clone(),
            license_expression_spdx: rule.license_expression.clone(),
            from_file: None,
            start_line,
            end_line,
            start_token,
            end_token,
            matcher: matcher.to_string(),
            score: match_coverage / 100.0,
            matched_length,
            rule_length: rule.tokens.len(),
            match_coverage,
            rule_relevance: rule.relevance,
            rule_identifier: format!("#{}", rid),
            rule_url: String::new(),
            matched_text: Some(matched_text),
            referenced_filenames: rule.referenced_filenames.clone(),
            is_license_intro: rule.is_license_intro,
            is_license_clue: rule.is_license_clue,
            is_license_reference: rule.is_license_reference,
            is_license_tag: rule.is_license_tag,
            is_license_text: rule.is_license_text,
            matched_token_positions: None,
            hilen: 0,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
        }
    }
    
    pub fn with_hilen(mut self, hilen: usize) -> Self {
        self.hilen = hilen;
        self
    }
    
    pub fn with_rule_start_token(mut self, token: usize) -> Self {
        self.rule_start_token = token;
        self
    }
}
```

### 3.2 Token Byte Encoding Duplication (MEDIUM PRIORITY)

**Issue**: `tokens_to_bytes()` function is duplicated.

**Locations**:

- `aho_match.rs:44-46`
- `index/builder.rs:221-223`

**Recommendation**: Move to a shared location (e.g., `index/mod.rs` or a `utils.rs` module).

### 3.3 Score Computation Duplication (MEDIUM PRIORITY)

**Issue**: Score vector computation logic is repeated in `seq_match.rs`.

**Locations**: `seq_match.rs:297-311` and `seq_match.rs:355-369`

**Pattern**:

```rust
let svr = ScoresVector {
    is_highly_resemblant: (resemblance * 10.0).round() / 10.0 >= HIGH_RESEMBLANCE_THRESHOLD,
    containment: (containment * 10.0).round() / 10.0,
    resemblance: (amplified_resemblance * 10.0).round() / 10.0,
    matched_length: (matched_length as f32 / 20.0).round(),
    rid,
};

let svf = ScoresVector {
    is_highly_resemblant: resemblance >= HIGH_RESEMBLANCE_THRESHOLD,
    containment,
    resemblance: amplified_resemblance,
    matched_length: matched_length as f32,
    rid,
};
```

**Recommendation**: Extract to a helper function:

```rust
fn compute_score_vectors(
    resemblance: f32,
    containment: f32,
    matched_length: usize,
    rid: usize,
) -> (ScoresVector, ScoresVector) {
    let amplified = resemblance.powi(2);
    (
        ScoresVector {
            is_highly_resemblant: (resemblance * 10.0).round() / 10.0 >= HIGH_RESEMBLANCE_THRESHOLD,
            containment: (containment * 10.0).round() / 10.0,
            resemblance: (amplified * 10.0).round() / 10.0,
            matched_length: (matched_length as f32 / 20.0).round(),
            rid,
        },
        ScoresVector {
            is_highly_resemblant: resemblance >= HIGH_RESEMBLANCE_THRESHOLD,
            containment,
            resemblance: amplified,
            matched_length: matched_length as f32,
            rid,
        },
    )
}
```

### 3.4 Tiny Functions That Could Be Inlined (LOW PRIORITY)

**Issue**: Some one-line functions add indirection without clear benefit.

**Examples**:

| Location | Function | Body |
|----------|----------|------|
| `token_sets.rs:42-44` | `tids_set_counter()` | `tids_set.len()` |
| `token_sets.rs:57-59` | `multiset_counter()` | `mset.values().sum()` |
| `aho_match.rs:58-60` | `byte_pos_to_token_pos()` | `byte_pos / 2` |

**Assessment**: These functions match Python's API naming and provide semantic clarity. Keep them but consider adding `#[inline]` hints.

### 3.5 Deprecated SPDX Substitution Duplication (LOW PRIORITY)

**Issue**: `DEPRECATED_SPDX_EXPRESSION_SUBS` is defined in both:

- `spdx_lid.rs:152-183`
- `index/builder.rs:29-63`

**Recommendation**: Move to a shared constant in `spdx_mapping.rs` or a dedicated `constants.rs` file.

---

## 4. Interface Analysis

### 4.1 Current Module Structure

```
license_detection/
  |-- hash_match.rs      (public: hash_match, compute_hash)
  |-- spdx_lid.rs        (public: spdx_lid_match, split_spdx_lid, clean_spdx_text)
  |-- aho_match.rs       (public: aho_match)
  |-- seq_match.rs       (public: seq_match, seq_match_with_candidates, compute_candidates_with_msets)
  |-- unknown_match.rs   (public: unknown_match)
  |-- index/
       |-- mod.rs        (public: LicenseIndex, Automaton, build_index)
       |-- builder.rs    (public: build_index)
       |-- dictionary.rs (public: TokenDictionary)
       |-- token_sets.rs (public: build_set_and_mset, high_tids_set_subset, etc.)
```

### 4.2 Interface Issues

#### 4.2.1 Inconsistent Matcher Function Signatures (MEDIUM PRIORITY)

**Issue**: Matchers have different function signatures despite similar purposes.

| Matcher | Signature |
|---------|-----------|
| `hash_match` | `(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch>` |
| `spdx_lid_match` | `(index: &LicenseIndex, query: &Query) -> Vec<LicenseMatch>` |
| `aho_match` | `(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch>` |
| `seq_match` | `(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch>` |

**Issue**: `spdx_lid_match` takes `&Query` while others take `&QueryRun`. This is because SPDX detection operates on the whole query, not segmented runs.

**Recommendation**: Document this distinction clearly or create a `Matcher` trait:

```rust
pub trait Matcher {
    fn match_query(&self, index: &LicenseIndex, query: &Query) -> Vec<LicenseMatch>;
    fn order(&self) -> u8;
    fn name(&self) -> &'static str;
}
```

#### 4.2.2 Internal Functions Exposed Publicly (LOW PRIORITY)

**Issue**: Several internal helper functions are public without clear external use:

| Module | Function | Should Be |
|--------|----------|-----------|
| `spdx_lid.rs` | `split_spdx_lid` | `pub(crate)` |
| `spdx_lid.rs` | `clean_spdx_text` | `pub(crate)` |
| `spdx_lid.rs` | `extract_spdx_expressions` | `pub(crate)` or remove |
| `seq_match.rs` | `multisets_intersector` | `pub(crate)` |
| `seq_match.rs` | `compute_set_similarity` | `pub(crate)` |

**Recommendation**: Audit and reduce visibility to `pub(crate)` where appropriate.

#### 4.2.3 Missing Submodule Boundaries (MEDIUM PRIORITY)

**Issue**: The matching logic is flat in `license_detection/` without clear submodule organization.

**Current structure**:

```
license_detection/
  |-- [all matchers flat at top level]
```

**Recommended structure**:

```
license_detection/
  |-- matching/
       |-- mod.rs         (re-exports, Matcher trait)
       |-- hash.rs        (1-hash)
       |-- spdx_lid.rs    (1-spdx-id)
       |-- aho.rs         (2-aho)
       |-- sequence.rs    (3-seq, near-duplicate)
       |-- unknown.rs     (5-unknown)
```

**Benefits**:

1. Clear separation of concerns
2. Easier to add new matchers
3. Better encapsulation of matcher internals
4. Natural place for shared matching utilities

### 4.3 Index Module Interface

**File**: `src/license_detection/index/mod.rs`

**Positive aspects**:

- Clear separation between `mod.rs` (types) and `builder.rs` (construction)
- `token_sets.rs` provides focused utilities for candidate selection
- `dictionary.rs` encapsulates token-to-ID mapping

**Improvement opportunity**: Create a `prelude` module for common imports:

```rust
// src/license_detection/index/prelude.rs
pub use super::{LicenseIndex, Automaton};
pub use super::builder::build_index;
pub use super::dictionary::TokenDictionary;
pub use super::token_sets::*;
```

---

## 5. Specific Issues Summary

### 5.1 High Priority Issues

| # | Issue | Location | Impact | Effort |
|---|-------|----------|--------|--------|
| 1 | LicenseMatch construction duplication | 5 files | Maintenance, bugs | Medium |
| 2 | Redundant test Rule construction | hash_match.rs, seq_match.rs, aho_match.rs | Maintenance, readability | Medium |
| 3 | Unused `regular_rids` field | index/mod.rs:140 | Dead code | Low |
| 4 | Potential memory waste with Candidate cloning | seq_match.rs:96 | Performance | Medium |

### 5.2 Medium Priority Issues

| # | Issue | Location | Impact | Effort |
|---|-------|----------|--------|--------|
| 5 | Repeated Query construction in tests | aho_match.rs tests | Readability | Low |
| 6 | Inconsistent key types (HashMap vs Vec) | index/mod.rs | Confusion | Medium |
| 7 | Score computation duplication | seq_match.rs | Maintenance | Low |
| 8 | `tokens_to_bytes` duplication | aho_match.rs, builder.rs | Maintenance | Low |
| 9 | Inconsistent matcher signatures | matcher files | API clarity | Medium |
| 10 | Missing submodule boundaries | license_detection/ | Organization | High |

### 5.3 Low Priority Issues

| # | Issue | Location | Impact | Effort |
|---|-------|----------|--------|--------|
| 11 | Deprecated SPDX substitution duplication | spdx_lid.rs, builder.rs | Maintenance | Low |
| 12 | Over-exposed internal functions | spdx_lid.rs, seq_match.rs | API clarity | Low |
| 13 | Multiple dead_code annotations | Various | Code cleanliness | Low |

---

## 6. Recommendations Summary

### 6.1 Immediate Actions (Sprint 1)

1. **Create `LicenseMatch::from_rule()` helper** - Reduces ~130 lines of duplicated code
2. **Expand test utilities** - Add `RuleBuilder` and `QueryBuilder` patterns
3. **Remove unused `regular_rids`** - Clean up dead code

### 6.2 Short-term Actions (Sprint 2-3)

1. **Create `matching/` submodule** - Better organization for matchers
2. **Unify `tokens_to_bytes()`** - Single location for shared utility
3. **Add `compute_score_vectors()` helper** - Reduce seq_match duplication

### 6.3 Long-term Considerations

1. **Consider Rule split** - Separate definition from computed data
2. **Evaluate Candidate reference pattern** - Reduce cloning overhead
3. **Standardize index data structures** - Consistent HashMap vs Vec usage
4. **Create Matcher trait** - Unify matcher interface

---

## 7. Test Coverage Gaps

### 7.1 Areas Needing Additional Tests

1. **Performance tests**: No tests for large files (>10MB) or many rules (>10,000)
2. **Concurrency tests**: No tests for thread-safety of index structures
3. **Fuzz testing**: No tests for malformed input handling
4. **Property-based tests**: Consider using `proptest` for score calculations

### 7.2 Recommended New Tests

```rust
// Performance boundary test
#[test]
fn test_hash_match_large_query() {
    let tokens: Vec<u16> = (0..100_000).collect();
    // ... verify memory usage and timeout
}

// Concurrency test
#[test]
fn test_index_thread_safety() {
    let index = Arc::new(build_index(rules, licenses));
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let idx = index.clone();
            std::thread::spawn(move || {
                // concurrent matching
            })
        })
        .collect();
}
```

---

## 8. Conclusion

The matching stage implementation is functionally solid with good test coverage of core functionality. The main areas for improvement are:

1. **Code organization**: The flat module structure and duplicated match construction could be refactored for better maintainability.

2. **Test infrastructure**: Test utilities are underutilized, leading to verbose and fragile test code.

3. **Interface clarity**: While functional, the matcher interfaces could benefit from standardization and reduced visibility of internal helpers.

The recommendations in this report are prioritized by impact and effort. Implementing the high-priority items alone would significantly improve code quality and maintainability.
