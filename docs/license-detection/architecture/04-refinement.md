# Refinement Stage Architecture Analysis

## Executive Summary

The refinement stage (`src/license_detection/match_refine.rs` and `src/license_detection/spans.rs`) is a critical component of the license detection pipeline that processes raw matches from all matching strategies and produces refined, deduplicated results.

**Overall Assessment**: The implementation is functionally complete and well-tested, but has several architectural issues that impact maintainability and code clarity.

| Category | Issues Found | Severity Distribution |
|----------|--------------|----------------------|
| Test Coverage | 6 | 2 High, 2 Medium, 2 Low |
| Data Structures | 4 | 1 High, 2 Medium, 1 Low |
| Algorithm Structure | 5 | 2 High, 2 Medium, 1 Low |
| Interfaces | 3 | 1 High, 1 Medium, 1 Low |

---

## 1. Test Coverage Analysis

### 1.1 Test Coverage is Extensive but Uneven

**Finding**: The test module contains 1600+ lines of tests (44% of the file), which demonstrates thorough testing. However, coverage is unevenly distributed.

**Issues Identified**:

#### ISSUE-TC-1 (HIGH): No tests for `filter_matches_missing_required_phrases`

**Location**: `match_refine.rs:1019-1211`

The function `filter_matches_missing_required_phrases` is 193 lines long and contains complex logic for validating required phrases, checking continuity, and handling stopwords. It has **zero dedicated unit tests**.

```rust
// match_refine.rs:1019-1211 - 193 lines, no tests
fn filter_matches_missing_required_phrases(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
    query: &Query,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // Complex logic with multiple branches, all untested at unit level
}
```

**Recommendation**: Add unit tests covering:

- Matches with no required phrases (should pass through)
- Matches missing required phrases (should be discarded)
- Continuous rule validation
- Stopword mismatch detection
- Edge cases with empty ispan/qspan

---

#### ISSUE-TC-2 (HIGH): No tests for `filter_invalid_matches_to_single_word_gibberish`

**Location**: `match_refine.rs:1373-1401`

This function handles binary file gibberish detection but lacks unit tests. The helper function `is_valid_short_match` is tested, but the main filter function is not.

```rust
// match_refine.rs:1373-1401 - No unit tests
fn filter_invalid_matches_to_single_word_gibberish(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
    query: &Query,
) -> Vec<LicenseMatch> {
    // Untested logic for binary file handling
}
```

**Recommendation**: Add tests for:

- Binary file detection with single-word matches
- Non-binary files (should pass through)
- Multi-word rules (should pass through)
- High vs low relevance handling

---

#### ISSUE-TC-3 (MEDIUM): Debug tests in production test module

**Location**: `match_refine.rs:2343-2397` and `match_refine.rs:3505-3672`

Two tests are named `debug_*` and contain extensive debugging output:

```rust
// match_refine.rs:2343
#[test]
fn debug_gpl_token_positions_real() { ... }

// match_refine.rs:3505
#[test]
fn debug_gpl_2_0_9_required_phrases_filter() { ... }
```

These tests:

- Load full license rules from disk (slow)
- Print extensive debugging output
- Are better suited as integration tests or development tools

**Recommendation**:

- Move to a separate `debug_tests.rs` or `integration_tests/` directory
- Consider marking as `#[ignore]` by default
- Extract meaningful assertions into proper unit tests

---

#### ISSUE-TC-4 (MEDIUM): Redundant test helper functions

**Location**: `match_refine.rs:1502-1578`

Three nearly identical test helper functions exist:

```rust
// match_refine.rs:1502-1541
fn create_test_match(
    rule_identifier: &str,
    start_line: usize,
    end_line: usize,
    score: f32,
    coverage: f32,
    relevance: u8,
) -> LicenseMatch { ... }

// match_refine.rs:1543-1578
fn create_test_match_with_tokens(
    rule_identifier: &str,
    start_token: usize,
    end_token: usize,
    matched_length: usize,
) -> LicenseMatch { ... }

// match_refine.rs:2828-2872
fn create_test_match_with_flags(
    rule_identifier: &str,
    start_line: usize,
    end_line: usize,
    is_license_reference: bool,
    is_license_tag: bool,
    is_license_intro: bool,
    is_license_clue: bool,
    matcher: &str,
    match_coverage: f32,
    matched_length: usize,
    rule_length: usize,
    license_expression: &str,
) -> LicenseMatch { ... }
```

**Issues**:

1. All three create `LicenseMatch` with mostly overlapping fields
2. `create_test_match_with_flags` has 13 parameters (code smell)
3. Tests using `create_test_match` often manually modify fields afterward

**Recommendation**: Create a builder pattern:

```rust
struct TestMatchBuilder {
    // optional fields with defaults
}

impl TestMatchBuilder {
    fn new() -> Self { ... }
    fn with_rule(self, id: &str) -> Self { ... }
    fn with_lines(self, start: usize, end: usize) -> Self { ... }
    fn with_tokens(self, start: usize, end: usize) -> Self { ... }
    fn with_flags(self, reference: bool, tag: bool, intro: bool, clue: bool) -> Self { ... }
    fn build(self) -> LicenseMatch { ... }
}
```

---

#### ISSUE-TC-5 (LOW): Some filter functions have only integration-level tests

**Location**: Multiple functions

`filter_too_short_matches`, `filter_below_rule_minimum_coverage`, and `filter_short_matches_scattered_on_too_many_lines` are only tested through integration tests that construct full `Rule` objects inline.

**Recommendation**: Add simpler unit tests with mock/stub rule data.

---

#### ISSUE-TC-6 (LOW): No negative test cases for several functions

Functions like `combine_matches` and `is_candidate_false_positive` lack tests for error conditions and boundary cases.

---

### 1.2 Test Helper Simplification Opportunity

**Finding**: Tests could be significantly simplified with better abstractions.

Current test code often follows this pattern:

```rust
// match_refine.rs:1608-1614 (typical pattern)
let mut m1 = create_test_match("#1", 1, 10, 0.9, 100.0, 100);
m1.rule_length = 100;
m1.rule_start_token = 0;
let mut m2 = create_test_match("#1", 5, 15, 0.85, 100.0, 100);
m2.rule_length = 100;
m2.rule_start_token = 4;
```

A builder pattern would reduce this to:

```rust
let m1 = TestMatchBuilder::new()
    .with_rule("#1").with_lines(1, 10)
    .with_rule_length(100).with_rule_start_token(0)
    .build();
```

---

## 2. Data Structure Analysis

### 2.1 `LicenseMatch` Structure Issues

**Location**: `models.rs:206-320`

#### ISSUE-DS-1 (HIGH): Excessive number of fields in `LicenseMatch`

The `LicenseMatch` struct has **32 fields**, making it difficult to work with:

```rust
pub struct LicenseMatch {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub start_token: usize,
    pub end_token: usize,
    pub matcher: String,
    pub score: f32,
    pub matched_length: usize,
    pub rule_length: usize,
    pub match_coverage: f32,
    pub rule_relevance: u8,
    pub rule_identifier: String,
    pub rule_url: String,
    pub matched_text: Option<String>,
    pub referenced_filenames: Option<Vec<String>>,
    pub is_license_intro: bool,
    pub is_license_clue: bool,
    pub is_license_reference: bool,
    pub is_license_tag: bool,
    pub is_license_text: bool,
    pub matched_token_positions: Option<Vec<usize>>,
    pub hilen: usize,
    pub rule_start_token: usize,
    pub qspan_positions: Option<Vec<usize>>,
    pub ispan_positions: Option<Vec<usize>>,
}
```

**Issues**:

1. Mixed concerns: position data, metadata, matching info, and flags all in one struct
2. Some fields are only for output (score, match_coverage), others only for internal processing
3. Many fields have default values that must be manually set in tests

**Recommendation**: Split into focused sub-structures:

```rust
pub struct LicenseMatch {
    // Core identity
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub rule_identifier: String,
    
    // Position info (both query-side and rule-side)
    pub position: MatchPosition,
    
    // Matching metadata
    pub matcher: MatcherInfo,
    
    // Rule classification flags
    pub flags: MatchFlags,
    
    // Optional output data
    pub matched_text: Option<String>,
    pub referenced_filenames: Option<Vec<String>>,
}

pub struct MatchPosition {
    pub query: QueryPosition,   // start_token, end_token, start_line, end_line
    pub rule: RulePosition,     // rule_start_token, qspan_positions, ispan_positions
}

pub struct MatcherInfo {
    pub matcher: String,
    pub score: f32,
    pub matched_length: usize,
    pub rule_length: usize,
    pub match_coverage: f32,
    pub hilen: usize,
    pub rule_relevance: u8,
}

pub struct MatchFlags {
    pub is_license_intro: bool,
    pub is_license_clue: bool,
    pub is_license_reference: bool,
    pub is_license_tag: bool,
    pub is_license_text: bool,
}
```

---

#### ISSUE-DS-2 (MEDIUM): Inconsistent use of `Option<Vec<usize>>` for spans

**Location**: `models.rs:289-319`

Both `qspan_positions` and `ispan_positions` are `Option<Vec<usize>>` where `None` means "contiguous range". This is clever but error-prone:

```rust
// match_refine.rs:543-549 - The pattern used
pub fn qspan(&self) -> Vec<usize> {
    if let Some(positions) = &self.qspan_positions {
        positions.clone()
    } else {
        (self.start_token..self.end_token).collect()
    }
}
```

**Issues**:

1. Every access requires branching
2. Easy to forget to check `is_some()` before using
3. Clone is required even when just reading

**Recommendation**: Consider an enum for clarity:

```rust
pub enum Span {
    Contiguous { start: usize, end: usize },
    Discrete(Vec<usize>),
}
```

---

#### ISSUE-DS-3 (MEDIUM): `#[allow(dead_code)]` on many span methods

**Location**: `spans.rs:14, 37, 68, 92, 98, 104, 110, 116, 121, 136`

The `Span` struct in `spans.rs` has many methods marked `#[allow(dead_code)]`:

```rust
// spans.rs - Multiple unused methods
#[allow(dead_code)]
pub fn from_iterator(...) { ... }

#[allow(dead_code)]
pub fn add(&mut self, ...) { ... }

#[allow(dead_code)]
pub fn is_empty(&self) -> bool { ... }
// ... and more
```

This indicates the `Span` type is underutilized - only `from_range`, `union_span`, and `intersects` are used in production code.

**Recommendation**:

- Remove unused methods
- Or document why they exist for future use
- The `Span` struct seems to duplicate functionality with `models.rs` span handling

---

#### ISSUE-DS-4 (LOW): `match_distance` function marked `#[allow(dead_code)]`

**Location**: `match_refine.rs:802`

```rust
#[allow(dead_code)]
fn match_distance(a: &LicenseMatch, b: &LicenseMatch) -> usize { ... }
```

This function is tested but never used in production. Should either be removed or documented as future functionality.

---

### 2.2 Duplication Between `spans.rs` and `models.rs`

**Finding**: Two different approaches to span handling exist:

1. `spans.rs::Span` - A `Vec<Range<usize>>` based structure
2. `models.rs` - `Option<Vec<usize>>` for qspan/ispan positions

The `Span` struct is only used in `restore_non_overlapping`:

```rust
// match_refine.rs:718-719
fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)
}
```

**Recommendation**: Either:

- Consolidate span handling into one approach
- Document the different use cases for each

---

## 3. Algorithm Structure Analysis

### 3.1 Key Functions and Their Complexity

| Function | Lines | Complexity | Purpose |
|----------|-------|------------|---------|
| `merge_overlapping_matches` | 143 | High | Merge overlapping/adjacent matches |
| `filter_overlapping_matches` | 204 | Very High | Filter based on overlap ratios |
| `filter_matches_missing_required_phrases` | 193 | High | Validate required phrases |
| `filter_false_positive_license_lists_matches` | 85 | Medium | Detect FP license lists |
| `refine_matches` | 63 | Medium | Main pipeline orchestrator |

---

#### ISSUE-AS-1 (HIGH): `filter_overlapping_matches` is too complex

**Location**: `match_refine.rs:513-716`

This 204-line function has deeply nested logic with many conditions:

```rust
// match_refine.rs:513-716 - Simplified view of complexity
pub fn filter_overlapping_matches(...) {
    while i < matches.len().saturating_sub(1) {
        while j < matches.len() {
            // Overlap ratio calculations
            let extra_large_next = overlap_ratio_to_next >= OVERLAP_EXTRA_LARGE;
            let large_next = overlap_ratio_to_next >= OVERLAP_LARGE;
            let medium_next = overlap_ratio_to_next >= OVERLAP_MEDIUM;
            let small_next = overlap_ratio_to_next >= OVERLAP_SMALL;
            
            // 6 different filtering conditions follow...
            if extra_large_next && current_len_val >= next_len_val { ... }
            if large_next && current_len_val >= next_len_val && current_hilen >= next_hilen { ... }
            if medium_next {
                // 3 more nested conditions...
            }
            if small_next { ... }
            
            // Sandwich detection logic
            if i > 0 { ... }
        }
    }
}
```

**Issues**:

1. Multiple levels of nesting
2. Similar but not identical conditions repeated
3. Hard to understand the "why" behind each condition
4. Difficult to test individual conditions in isolation

**Recommendation**: Extract into smaller, well-named functions:

```rust
pub fn filter_overlapping_matches(matches: Vec<LicenseMatch>, index: &LicenseIndex) 
    -> (Vec<LicenseMatch>, Vec<LicenseMatch>) 
{
    // ... setup code ...
    
    while i < matches.len().saturating_sub(1) {
        while j < matches.len() {
            let decision = determine_overlap_action(&matches[i], &matches[j], index);
            match decision {
                OverlapAction::KeepCurrent => { j += 1; }
                OverlapAction::DiscardNext => { discarded.push(matches.remove(j)); }
                OverlapAction::DiscardCurrent => { discarded.push(matches.remove(i)); break; }
            }
        }
    }
}

enum OverlapAction {
    KeepCurrent,
    DiscardNext,
    DiscardCurrent,
}

fn determine_overlap_action(current: &LicenseMatch, next: &LicenseMatch, index: &LicenseIndex) 
    -> OverlapAction 
{
    if should_discard_for_extra_large_overlap(current, next) { return DiscardNext; }
    if should_discard_for_large_overlap(current, next) { return DiscardNext; }
    // ... etc
}
```

---

#### ISSUE-AS-2 (HIGH): Code repetition in overlap threshold checks

**Location**: `match_refine.rs:568-680`

The same pattern appears multiple times with slight variations:

```rust
// Pattern 1 (line 583-586)
if extra_large_next && current_len_val >= next_len_val {
    discarded.push(matches.remove(j));
    continue;
}

// Pattern 2 (line 588-592)
if extra_large_current && current_len_val <= next_len_val {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}

// Pattern 3 (line 594-597)
if large_next && current_len_val >= next_len_val && current_hilen >= next_hilen {
    discarded.push(matches.remove(j));
    continue;
}

// Pattern 4 (line 599-603)
if large_current && current_len_val <= next_len_val && current_hilen <= next_hilen {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

**Recommendation**: Extract into a helper:

```rust
fn apply_overlap_decision(
    matches: &mut Vec<LicenseMatch>,
    discarded: &mut Vec<LicenseMatch>,
    i: &mut usize,
    j: usize,
    should_discard_next: bool,
    should_discard_current: bool,
) -> bool {
    if should_discard_next {
        discarded.push(matches.remove(j));
        true // continue outer loop
    } else if should_discard_current {
        discarded.push(matches.remove(*i));
        *i = i.saturating_sub(1);
        false // break inner loop
    } else {
        false
    }
}
```

---

#### ISSUE-AS-3 (MEDIUM): `combine_matches` modifies many fields

**Location**: `match_refine.rs:113-153`

The `combine_matches` function modifies 11 fields of the merged match:

```rust
fn combine_matches(a: &LicenseMatch, b: &LicenseMatch) -> LicenseMatch {
    let mut merged = a.clone();
    // ... calculations ...
    merged.start_token = *qspan_vec.first().unwrap_or(&a.start_token);
    merged.end_token = qspan_vec.last().map(|&x| x + 1).unwrap_or(a.end_token);
    merged.rule_start_token = *ispan_vec.first().unwrap_or(&a.rule_start_token);
    merged.matched_length = qspan_vec.len();
    merged.hilen = hilen;
    merged.start_line = a.start_line.min(b.start_line);
    merged.end_line = a.end_line.max(b.end_line);
    merged.score = a.score.max(b.score);
    merged.qspan_positions = Some(qspan_vec);
    merged.ispan_positions = Some(ispan_vec);
    merged.match_coverage = /* calculation */;
    merged
}
```

**Recommendation**: Consider creating a `MatchMerger` struct to encapsulate this logic with clearer method names.

---

#### ISSUE-AS-4 (MEDIUM): Tiny functions that could be inlined

**Location**: `match_refine.rs:718-720`, `match_refine.rs:506-511`

```rust
// match_refine.rs:718-720 - Single use
fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)
}

// match_refine.rs:506-511 - Single use  
fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return false;
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}
```

**Recommendation**: These are used only once each. Consider inlining or documenting why they deserve separate functions.

---

#### ISSUE-AS-5 (LOW): Pipeline order in `refine_matches` could be clearer

**Location**: `match_refine.rs:1434-1496`

The main `refine_matches` function has a 14-step pipeline. While well-documented, the logic for the order is not obvious:

```rust
pub fn refine_matches(...) -> Vec<LicenseMatch> {
    let merged = merge_overlapping_matches(&matches);
    let (with_required_phrases, _) = filter_matches_missing_required_phrases(...);
    let non_spurious = filter_spurious_matches(...);
    let above_min_cov = filter_below_rule_minimum_coverage(...);
    let non_single_spurious = filter_matches_to_spurious_single_token(...);
    let non_short = filter_too_short_matches(...);
    let non_scattered = filter_short_matches_scattered_on_too_many_lines(...);
    let non_gibberish = filter_invalid_matches_to_single_word_gibberish(...);
    let merged_again = merge_overlapping_matches(&non_gibberish);
    // ... more steps ...
}
```

**Recommendation**: Add comments explaining why each filter runs in this order, especially why `merge_overlapping_matches` runs twice.

---

## 4. Interface Analysis

### 4.1 Current Interface Structure

The refinement module exports three public functions:

```rust
// mod.rs:50-52
pub use match_refine::{
    filter_invalid_contained_unknown_matches,
    merge_overlapping_matches,
    refine_matches,
};
```

---

#### ISSUE-IF-1 (HIGH): Inconsistent public API design

**Location**: `mod.rs:50-52`, `match_refine.rs:43-61`

`filter_invalid_contained_unknown_matches` is exposed publicly but is only used internally for unknown match handling. Meanwhile, other filter functions that might be useful externally are private.

**Analysis**:

| Function | Public? | External Use | Internal Use |
|----------|---------|--------------|--------------|
| `refine_matches` | Yes | Yes (main entry) | Yes |
| `merge_overlapping_matches` | Yes | Yes (debugging) | Yes |
| `filter_invalid_contained_unknown_matches` | Yes | No | Yes (unknown_match.rs) |
| `filter_overlapping_matches` | No | - | Yes |
| `filter_contained_matches` | No | - | Yes |
| `filter_spurious_matches` | No | - | Yes |

**Recommendation**: Either:

1. Make all filter functions private (use `refine_matches` as only entry point)
2. Or document the intended use of each public function

---

#### ISSUE-IF-2 (MEDIUM): No clear sub-module structure

**Location**: `match_refine.rs` (entire file)

The file contains ~1500 lines mixing:

- 14 filter functions
- 4 merge/combine functions  
- 2 restore functions
- 1 main pipeline function
- Constants
- Helper functions

**Recommendation**: Split into sub-modules:

```
src/license_detection/match_refine/
    mod.rs           # Public API: refine_matches()
    merge.rs         # merge_overlapping_matches, combine_matches
    filter/
        mod.rs       # Filter trait or common patterns
        overlap.rs   # filter_overlapping_matches
        contained.rs # filter_contained_matches
        spurious.rs  # filter_spurious_matches
        phrases.rs   # filter_matches_missing_required_phrases
        coverage.rs  # filter_below_rule_minimum_coverage
        fp_lists.rs  # filter_false_positive_license_lists_matches
    restore.rs       # restore_non_overlapping
    constants.rs     # OVERLAP_*, MIN_* constants
```

---

#### ISSUE-IF-3 (LOW): `Query` dependency creates tight coupling

**Location**: `match_refine.rs:4`, `models.rs:422-439`

Several functions require a `Query` reference for calculations:

```rust
fn filter_spurious_matches(matches: &[LicenseMatch], query: &Query) -> Vec<LicenseMatch>
fn update_match_scores(matches: &mut [LicenseMatch], query: &Query)
fn filter_matches_to_spurious_single_token(matches: &[LicenseMatch], query: &Query, unknown_count: usize)
fn filter_invalid_matches_to_single_word_gibberish(index: &LicenseIndex, matches: &[LicenseMatch], query: &Query)
```

This creates tight coupling between refinement and query processing. The `Query` type provides:

- `unknowns_by_pos` - for density calculations
- `stopwords_by_pos` - for phrase validation
- `shorts_and_digits_pos` - for spurious detection
- `is_binary` - for gibberish filtering

**Recommendation**: Consider extracting these into a simpler interface:

```rust
/// Data needed by refinement from the query
pub struct QueryContext {
    pub unknowns_by_pos: HashMap<Option<i32>, usize>,
    pub stopwords_by_pos: HashMap<Option<i32>, usize>,
    pub shorts_and_digits_pos: HashSet<usize>,
    pub is_binary: bool,
}

impl QueryContext {
    pub fn from_query(query: &Query) -> Self { ... }
}
```

This would:

1. Make dependencies explicit
2. Allow easier testing with mock context
3. Decouple refinement from full Query type

---

## 5. Summary of Recommendations

### High Priority

| ID | Issue | Recommendation |
|----|-------|----------------|
| TC-1 | No tests for `filter_matches_missing_required_phrases` | Add comprehensive unit tests |
| TC-2 | No tests for `filter_invalid_matches_to_single_word_gibberish` | Add unit tests for binary file handling |
| DS-1 | `LicenseMatch` has 32 fields | Split into focused sub-structures |
| AS-1 | `filter_overlapping_matches` is too complex | Extract into smaller, named functions |
| AS-2 | Code repetition in overlap threshold checks | Create helper function |
| IF-1 | Inconsistent public API design | Clarify intended public interface |

### Medium Priority

| ID | Issue | Recommendation |
|----|-------|----------------|
| TC-3 | Debug tests in production module | Move to integration tests |
| TC-4 | Redundant test helper functions | Use builder pattern |
| DS-2 | `Option<Vec<usize>>` for spans | Consider enum-based Span type |
| DS-3 | Unused `#[allow(dead_code)]` methods | Remove or document |
| AS-3 | `combine_matches` modifies many fields | Consider MatchMerger struct |
| AS-5 | Pipeline order not obvious | Add ordering explanation comments |
| IF-2 | No sub-module structure | Split into focused sub-modules |

### Low Priority

| ID | Issue | Recommendation |
|----|-------|----------------|
| TC-5 | Only integration-level tests for some filters | Add simpler unit tests |
| TC-6 | No negative test cases | Add boundary tests |
| DS-4 | `match_distance` marked dead_code | Remove or document |
| AS-4 | Tiny single-use functions | Inline or document |
| IF-3 | `Query` dependency creates tight coupling | Extract QueryContext |

---

## 6. Code Quality Metrics

| Metric | Value | Assessment |
|--------|-------|------------|
| Total lines | 3673 | Large file |
| Production code lines | ~2070 | Moderate |
| Test code lines | ~1600 | Good coverage ratio |
| Functions | 28 | Many, some large |
| Public functions | 3 | Minimal API |
| Average function length | 52 lines | Some outliers |
| Longest function | 204 lines (`filter_overlapping_matches`) | Refactor needed |
| Cyclomatic complexity | High in filter functions | Needs reduction |

---

## 7. Conclusion

The refinement stage is a well-tested, functionally complete implementation. The main areas for improvement are:

1. **Test coverage gaps** - Two major filter functions lack unit tests
2. **Data structure complexity** - The `LicenseMatch` struct has grown too large
3. **Algorithm readability** - `filter_overlapping_matches` needs decomposition
4. **Module organization** - A 3600-line file should be split

Addressing the high-priority issues would significantly improve maintainability without requiring fundamental architectural changes. The builder pattern for tests and sub-module restructuring would make the codebase more approachable for future contributors.
