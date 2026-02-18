# PLAN-017: Remaining License Detection Fixes

## Status: In Progress

### Current Test Results (Stable after PLAN-018 flakiness fix)

| Test Suite | Baseline | Current | Change |
|------------|----------|---------|--------|
| lic1 | 213/78 | 228/63 | +15 |
| lic2 | 759/94 | 776/77 | +17 |
| lic3 | 242/50 | 251/41 | +9 |
| lic4 | 265/85 | 281/69 | +16 |
| external | 1935/632 | 1882/685 | -53 |
| unknown | 2/8 | 2/8 | 0 |

**Note**: PLAN-018 fixed golden test flakiness - all results are now deterministic.

---

## Completed Fixes

### Issue 2: Golden Test Comparison ✅ DONE

**Fix**: Changed test to flatten `detection.matches` instead of comparing detection expressions.

```rust
// BEFORE:
let actual: Vec<&str> = detections
    .iter()
    .map(|d| d.license_expression.as_deref().unwrap_or(""))
    .collect();

// AFTER:
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

**Impact**: +49 tests passed on lic1-4, but -56 regression on external.

### Detection.matches Storage Fix ✅ DONE

**Fix**: Store raw matches in `detection.matches`, use filtered matches only for expression computation.

**Impact**: Minimal change (within noise margin).

### Issue 3: Remove filter_short_gpl_matches ✅ DONE

**File**: `src/license_detection/match_refine.rs`

**Fix**: Deleted `filter_short_gpl_matches()` function and its call. Python does NOT have this filter - it was incorrectly added.

**Impact**: +8 tests passed on lic1, +2 on lic2, +4 on external.

### Issue 5: Add Unknown License Filter ✅ DONE

**File**: `src/license_detection/match_refine.rs`, `src/license_detection/mod.rs`

**Fix**: Added `filter_invalid_contained_unknown_matches()` function that filters unknown matches contained within good matches' qregion (token span).

**Impact**: +2 tests passed on lic1, +2 on lic2, +13 on external.

### Issue 4: Query Run Lazy Evaluation ✅ DONE

**File**: `src/license_detection/query.rs`

**Fix**: Simplified `QueryRun` struct from 9 fields to 3 fields. Changed from storing individual references to Query fields, to storing a single `&Query` reference and computing `high_matchables()`/`low_matchables()` on-demand.

**Before**:

```rust
pub struct QueryRun<'a> {
    index: &'a LicenseIndex,
    tokens: &'a [u16],
    line_by_pos: &'a [usize],
    text: &'a str,
    high_matchables: &'a HashSet<usize>,
    low_matchables: &'a HashSet<usize>,
    digit_only_tids: &'a HashSet<u16>,
    pub start: usize,
    pub end: Option<usize>,
    len_legalese: usize,
}
```

**After**:

```rust
pub struct QueryRun<'a> {
    query: &'a Query<'a>,
    pub start: usize,
    pub end: Option<usize>,
}
```

**Impact**: No change to golden tests (within noise margin). Architectural cleanup for maintainability.

---

## Remaining Issues

Change `QueryRun` to store `&Query` reference and compute `high_matchables()`/`low_matchables()` on-demand.

### Issue 6: Add filter_matches_missing_required_phrases ⚠️ COMPLEX

**File**: `src/license_detection/match_refine.rs`

Add filter that removes matches where required phrases weren't matched. This requires:

1. Parse `{{...}}` markers from rule text
2. Track required phrase spans during matching
3. Filter matches missing required phrases
4. Handle `is_continuous` rules (3317 rules)
5. Handle `is_required_phrase` rules (1927 rules)

**Python implementation**: `reference/scancode-toolkit/src/licensedcode/match.py:2154-2328`

**This filter is called FIRST in Python's refine pipeline.**

---

## Issue 6: Detailed Plan

### Investigation Summary: Python vs Rust Comparison

#### Python Implementation (reference/scancode-toolkit/src/licensedcode/match.py:2154-2328)

The `filter_matches_missing_required_phrases()` function:

1. **Inputs**: List of `LicenseMatch` objects
2. **Outputs**: Tuple of `(kept_matches, discarded_matches)`
3. **Purpose**: Remove matches that don't contain required phrases defined in their matched rule

**Key Python LicenseMatch fields needed:**

```python
# From match.py:152-198
class LicenseMatch:
    qspan: Span          # Query token positions matched
    ispan: Span          # Rule token positions matched
    hispan: Span         # High-value token positions (subset of ispan)
    rule: Rule           # Reference to the matched rule
    query: Query         # Reference to the query object
```

**Key Python Rule fields needed:**

```python
# From models.py:1676-1683
required_phrase_spans: list[Span]  # List of spans from {{...}} markers
is_continuous: bool                 # True if whole rule must match continuously
is_required_phrase: bool            # True if this rule IS a required phrase
stopwords_by_pos: dict[int, int]    # Stopword count by rule position
```

**Key Python Query fields needed:**

```python
# From query.py:183, 244
stopwords_by_pos: dict[int, int]    # Stopword count by query position
unknowns_by_pos: dict[int, int]     # Unknown token count by query position
```

**Python `is_continuous()` method on LicenseMatch** (match.py:529-536):

```python
def is_continuous(self):
    """Return True if all matched tokens are continuous without gaps."""
    return (
        self.len() == self.qregion_len() == self.qmagnitude()
    )
```

#### Current Rust Implementation Status

| Component | Field/Method | Rust Status | Gap |
|-----------|--------------|-------------|-----|
| **Rule** | `is_continuous` | ✅ Present | None |
| **Rule** | `is_required_phrase` | ✅ Present | None |
| **Rule** | `required_phrase_spans` | ❌ Missing | Need to parse `{{...}}` |
| **Rule** | `stopwords_by_pos` | ❌ Missing | Need to add |
| **LicenseMatch** | `qspan`/`ispan` | ⚠️ Partial | Have start/end_token, not full spans |
| **LicenseMatch** | `matched_token_positions` | ✅ Present | Used for non-contiguous matches |
| **LicenseMatch** | `is_continuous()` method | ❌ Missing | Need to implement |
| **Query** | `stopwords_by_pos` | ✅ Present | None |
| **Query** | `unknowns_by_pos` | ✅ Present | None |

### Filter Logic Analysis

The Python filter (`match.py:2154-2328`) has these discard conditions:

1. **Solo match exception** (lines 2172-2175):
   - If only 1 match AND NOT (`is_continuous` OR `is_required_phrase`), KEEP it

2. **No required phrases check** (lines 2192-2196):
   - If rule has no `required_phrase_spans` AND NOT `is_continuous`, KEEP

3. **is_continuous validation** (lines 2198-2201):
   - If rule is continuous, but match isn't continuous, DISCARD

4. **Required phrase containment** (lines 2204-2215):
   - For non-continuous rules: check if each `ikey_span` (required phrase span) is contained in `match.ispan`
   - If any required phrase span is missing, DISCARD

5. **Query-side continuity check** (lines 2246-2262):
   - For each required phrase, check if query-side positions are continuous
   - If `qkey_span.magnitude() != len(qkey_span)`, DISCARD (has gaps)

6. **Unknown word check** (lines 2270-2284):
   - Check if required phrase positions contain unknown words
   - If unknowns found in required phrase positions, DISCARD

7. **Stopword consistency check** (lines 2289-2307):
   - Verify stopword counts match between rule and query at aligned positions
   - If mismatch, DISCARD

### Complete Gap List

| # | Gap | File | Lines | Priority |
|---|-----|------|-------|----------|
| 1 | Parse `{{...}}` required phrase markers | `rules/loader.rs` | ~300-350 | HIGH |
| 2 | Add `required_phrase_spans` to Rule struct | `models.rs` | ~63-175 | HIGH |
| 3 | Compute `required_phrase_spans` during rule loading | `rules/loader.rs` | ~300-350 | HIGH |
| 4 | Add `stopwords_by_pos` to Rule struct | `models.rs` | ~63-175 | MEDIUM |
| 5 | Compute `stopwords_by_pos` during rule indexing | `index/builder.rs` | ~50-200 | MEDIUM |
| 6 | Add `is_continuous()` method to LicenseMatch | `models.rs` | ~298-408 | HIGH |
| 7 | Implement `filter_matches_missing_required_phrases()` | `match_refine.rs` | NEW | HIGH |
| 8 | Call filter FIRST in refine pipeline | `match_refine.rs` | ~1008-1048 | HIGH |
| 9 | Store rule reference in LicenseMatch (or access via index) | `models.rs` | ~178-266 | MEDIUM |

### Step-by-Step Implementation Plan

#### Phase 1: Rule Struct Changes (HIGH PRIORITY)

**Step 1.1: Add `required_phrase_spans` field to Rule**

File: `src/license_detection/models.rs` (~line 113, after `is_continuous`)

```rust
/// Token position spans for required phrases parsed from {{...}} markers.
/// Each span represents positions in the rule text that MUST be matched.
#[serde(skip)]
pub required_phrase_spans: Vec<Range<usize>>,
```

**Step 1.2: Add `stopwords_by_pos` field to Rule**

File: `src/license_detection/models.rs` (~line 114, after `required_phrase_spans`)

```rust
/// Mapping from token position to count of stopwords at that position.
/// Used for required phrase validation.
#[serde(skip)]
pub stopwords_by_pos: HashMap<usize, usize>,
```

#### Phase 2: Required Phrase Parsing (HIGH PRIORITY)

**Step 2.1: Create required phrase parser**

File: `src/license_detection/rules/loader.rs` (new function after `parse_rule_file`)

Based on Python's `get_existing_required_phrase_spans()` in `tokenize.py:122-174`:

```rust
/// Parse {{...}} required phrase markers from rule text.
/// Returns list of token position ranges for required phrases.
fn parse_required_phrase_spans(text: &str) -> Result<Vec<Range<usize>>> {
    // Implementation:
    // 1. Tokenize text, tracking positions
    // 2. Find {{ and }} markers
    // 3. Return positions between markers (excluding the markers themselves)
    // 4. Handle errors: nested braces, unclosed braces, empty phrases
}
```

**Step 2.2: Integrate into rule loading**

File: `src/license_detection/rules/loader.rs` (~line 304, in `parse_rule_file`)

```rust
let required_phrase_spans = parse_required_phrase_spans(trimmed_text)?;
```

#### Phase 3: Stopwords by Position (MEDIUM PRIORITY)

**Step 3.1: Create stopwords tokenizer**

File: `src/license_detection/rules/loader.rs` (new function)

Based on Python's `index_tokenizer_with_stopwords()` in `tokenize.py:249-306`:

```rust
/// Tokenize text and track stopwords by position.
/// Returns (tokens, stopwords_by_pos).
fn tokenize_with_stopwords(text: &str, stopwords: &HashSet<&str>) -> (Vec<String>, HashMap<usize, usize>) {
    // Implementation similar to Python
}
```

**Step 3.2: Compute during indexing**

File: `src/license_detection/index/builder.rs` (~line 200, in rule indexing)

```rust
// Compute stopwords_by_pos when building rule
rule.stopwords_by_pos = compute_stopwords_by_pos(&rule.text);
```

#### Phase 4: LicenseMatch Methods (HIGH PRIORITY)

**Step 4.1: Add `is_continuous()` method to LicenseMatch**

File: `src/license_detection/models.rs` (~line 408, in `impl LicenseMatch`)

Based on Python's `is_continuous()` (match.py:529-536):

```rust
/// Return true if all matched tokens are continuous without gaps,
/// unknown words, or stopwords.
pub fn is_continuous(&self, query: &Query) -> bool {
    // A match is continuous if:
    // 1. matched_length == qregion_len (no gaps in matched positions)
    // 2. No unknown tokens in the matched range
    // 3. No extra stopwords between matched tokens
    
    let len = self.len();
    let qregion_len = self.end_token.saturating_sub(self.start_token);
    
    // Check for unknown tokens in range
    let has_unknowns = (self.start_token..self.end_token)
        .any(|pos| query.unknowns_by_pos.contains_key(&Some(pos as i32)));
    
    len == qregion_len && !has_unknowns
}
```

**Step 4.2: Add helper methods for span operations**

File: `src/license_detection/models.rs` (in `impl LicenseMatch`)

```rust
/// Get the ispan (rule-side positions) for this match.
/// Returns a range from 0 to matched_length for contiguous matches,
/// or the actual positions for non-contiguous matches.
pub fn ispan(&self) -> Vec<usize> {
    if let Some(positions) = &self.matched_token_positions {
        positions.clone()
    } else {
        (0..self.matched_length).collect()
    }
}

/// Get the qspan (query-side positions) for this match.
pub fn qspan(&self) -> Vec<usize> {
    if let Some(positions) = &self.matched_token_positions {
        positions.clone()
    } else {
        (self.start_token..self.end_token).collect()
    }
}
```

#### Phase 5: Filter Implementation (HIGH PRIORITY)

**Step 5.1: Implement `filter_matches_missing_required_phrases()`**

File: `src/license_detection/match_refine.rs` (new function, ~line 60)

```rust
/// Filter matches missing required phrases defined in their rules.
///
/// A match is discarded if:
/// 1. Rule has required phrases that weren't matched
/// 2. Rule is continuous but match has gaps/unknowns/stopwords
/// 3. Required phrase spans contain unknown words
/// 4. Stopword counts don't match between rule and query
///
/// Based on Python: `filter_matches_missing_required_phrases()` (match.py:2154-2328)
pub fn filter_matches_missing_required_phrases(
    matches: &[LicenseMatch],
    index: &LicenseIndex,
    query: &Query,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // Implementation follows Python logic:
    // 1. Handle solo match exception
    // 2. Check is_continuous/is_required_phrase rules
    // 3. Validate required phrase containment
    // 4. Check unknown words and stopwords in required phrases
}
```

**Step 5.2: Update refine pipeline**

File: `src/license_detection/match_refine.rs` (~line 1008, in `refine_matches`)

Move this call to be FIRST (after merge):

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,
) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    // FIRST: Filter matches missing required phrases (was missing!)
    let (kept, discarded) = filter_matches_missing_required_phrases(&matches, index, query);
    // Note: Python has special reinstatement logic if all matches discarded
    
    let non_spurious = filter_spurious_matches(&kept);
    // ... rest of pipeline
}
```

#### Phase 6: Rule Reference Access (MEDIUM PRIORITY)

The filter needs access to rule data (`required_phrase_spans`, `is_continuous`, etc.).

**Option A (Current approach)**: Access via `index.rules_by_rid`

- Already used for `filter_below_rule_minimum_coverage()`
- Requires parsing `rule_identifier` to get `rid`

**Option B (Add rule reference to LicenseMatch)**:

- More direct but increases memory
- Not currently implemented

Recommend: **Option A** - consistent with existing pattern.

### Testing Strategy

1. **Unit tests** for `parse_required_phrase_spans()`:
   - Single required phrase
   - Multiple required phrases
   - Nested braces (error)
   - Unclosed braces (error)
   - Empty phrases (error)

2. **Unit tests** for `is_continuous()`:
   - Contiguous match
   - Match with gaps
   - Match with unknowns
   - Match with stopwords

3. **Integration tests** for `filter_matches_missing_required_phrases()`:
   - Match with all required phrases present
   - Match missing required phrases
   - Continuous rule with continuous match
   - Continuous rule with non-continuous match

4. **Golden test comparison**:
   - Run before/after comparison on lic1-4 and external test suites

### Estimated Effort

| Phase | Effort | Complexity |
|-------|--------|------------|
| Phase 1: Rule struct changes | 1 hour | Low |
| Phase 2: Required phrase parsing | 3 hours | Medium |
| Phase 3: Stopwords by position | 2 hours | Medium |
| Phase 4: LicenseMatch methods | 2 hours | Medium |
| Phase 5: Filter implementation | 4 hours | High |
| Phase 6: Rule reference access | 1 hour | Low |
| **Total** | **13 hours** | |

### Risk Assessment

1. **Performance**: The filter adds overhead to every match. Python calls it FIRST to reduce downstream work. Rust should do the same.

2. **Rule data loading**: Required phrase spans must be computed at load time, not at match time. This increases memory but is necessary for correctness.

3. **Span representation**: Python uses a custom `Span` class that supports non-contiguous positions. Rust uses `matched_token_positions: Option<Vec<usize>>`. Ensure these are equivalent for the filter logic.

4. **Rule identifier parsing**: Current code parses `rule_identifier` like `"#42"` to get `rid`. This must be reliable for all match types.

---

## Issue 4: Detailed Plan

### Investigation Summary

#### Current Rust QueryRun Implementation

**File**: `src/license_detection/query.rs` (lines 834-871)

```rust
pub struct QueryRun<'a> {
    index: &'a LicenseIndex,
    tokens: &'a [u16],
    line_by_pos: &'a [usize],
    text: &'a str,
    high_matchables: &'a HashSet<usize>,  // Individual reference to Query field
    low_matchables: &'a HashSet<usize>,   // Individual reference to Query field
    digit_only_tids: &'a HashSet<u16>,
    pub start: usize,
    pub end: Option<usize>,
    len_legalese: usize,
}
```

**Current `high_matchables()` and `low_matchables()`** (lines 1015-1034):

```rust
pub fn high_matchables(&self) -> HashSet<usize> {
    self.high_matchables  // Reference to Query's field
        .iter()
        .filter(|&&pos| pos >= self.start && pos <= self.end.unwrap_or(usize::MAX))
        .copied()
        .collect()
}
```

#### Python QueryRun Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/query.py` (lines 720-861)

```python
class QueryRun(object):
    __slots__ = (
        'query',           # <-- Stores reference to parent Query!
        'start',
        'end',
        'len_legalese',
        'digit_only_tids',
        '_low_matchables',   # <-- Cached, computed on-demand
        '_high_matchables',  # <-- Cached, computed on-demand
    )

    def __init__(self, query, start, end=None):
        self.query = query  # Single Query reference
        self.start = start
        self.end = end
        self._low_matchables = None
        self._high_matchables = None

    @property
    def low_matchables(self):
        """Compute on-demand with lazy caching."""
        if not self._low_matchables:
            self._low_matchables = intbitset(
                [pos for pos in self.query.low_matchables
                 if self.start <= pos <= self.end])
        return self._low_matchables

    @property
    def high_matchables(self):
        """Compute on-demand with lazy caching."""
        if not self._high_matchables:
            self._high_matchables = intbitset(
                [pos for pos in self.query.high_matchables
                 if self.start <= pos <= self.end])
        return self._high_matchables
```

#### Key Differences

| Aspect | Python | Rust (Current) |
|--------|--------|----------------|
| Storage | Single `query` reference | 7+ individual field references |
| Lazy evaluation | Yes, with `_high_matchables`/`_low_matchables` caching | No, computes every call |
| Mutation handling | Automatic (accesses via `self.query`) | Works but via stored references |

#### The Problem: Why This Change Is Needed

1. **Architectural Simplification**: Storing 7+ individual references is harder to maintain than one Query reference.

2. **Consistency with Python**: Python stores `query` reference and computes `high_matchables`/`low_matchables` on-demand with caching.

3. **Correctness with Mutation**: When `Query.subtract()` is called, it modifies `query.high_matchables` and `query.low_matchables`. The current Rust implementation works because the stored references point to Query's fields, but this is fragile.

4. **Future-proofing**: If we ever need to access other Query fields from QueryRun, the current design requires adding more reference fields.

#### What Works Currently

The current implementation is **correct** because:

- `QueryRun` stores `&'a HashSet<usize>` references to `Query`'s fields
- When `Query.subtract()` modifies these sets, the `QueryRun` sees the changes
- Rust's lifetime system ensures `QueryRun` cannot outlive `Query`

#### What Needs to Change

**Goal**: Store `&Query` reference and compute `high_matchables()`/`low_matchables()` on-demand.

**Benefits**:

1. Cleaner, simpler struct (3 fields instead of 9)
2. Consistent with Python design
3. Optional caching for performance (like Python's `_high_matchables`)

### Impact Analysis

#### Callers of QueryRun

From grep analysis, these functions receive `&QueryRun`:

| File | Function | Usage |
|------|----------|-------|
| `aho_match.rs:76` | `aho_match(index, query_run)` | `tokens()`, `start`, `matchables()`, `line_for_pos()`, `matched_text()` |
| `hash_match.rs:72` | `hash_match(index, query_run)` | `tokens()`, `start`, `end`, `line_for_pos()`, `matched_text()` |
| `seq_match.rs:222` | `seq_match_with_candidates(index, query_run, candidates)` | `tokens()`, `tokens_with_pos()`, `matchables()`, `matchable_tokens()` |
| `mod.rs:198-217` | Phase 4 loop | `is_matchable()`, `start`, `end` |

#### Methods Used

All these methods would continue to work with `&Query` reference:

- `tokens()` → `self.query.tokens[start..=end]`
- `tokens_with_pos()` → iterate with position offset
- `matchables()` → compute from `self.query.high_matchables`
- `high_matchables()` → filter `self.query.high_matchables`
- `low_matchables()` → filter `self.query.low_matchables`
- `is_matchable()` → uses `matchables()`
- `matchable_tokens()` → uses `matchables()`
- `line_for_pos()` → `self.query.line_by_pos[pos]`
- `matched_text()` → `self.query.matched_text()`
- `start_line()` → `self.query.line_by_pos[self.start]`
- `end_line()` → `self.query.line_by_pos[self.end]`
- `is_digits_only()` → check tokens against `self.query.index.digit_only_tids`

### Step-by-Step Implementation Plan

#### Step 1: Simplify QueryRun Struct

**File**: `src/license_detection/query.rs` (line 834)

**Before**:

```rust
pub struct QueryRun<'a> {
    index: &'a LicenseIndex,
    tokens: &'a [u16],
    line_by_pos: &'a [usize],
    text: &'a str,
    high_matchables: &'a HashSet<usize>,
    low_matchables: &'a HashSet<usize>,
    digit_only_tids: &'a HashSet<u16>,
    pub start: usize,
    pub end: Option<usize>,
    len_legalese: usize,
}
```

**After**:

```rust
pub struct QueryRun<'a> {
    query: &'a Query<'a>,
    pub start: usize,
    pub end: Option<usize>,
}
```

#### Step 2: Update QueryRun::new()

**File**: `src/license_detection/query.rs` (line 859)

**Before**:

```rust
pub fn new(query: &'a Query<'a>, start: usize, end: Option<usize>) -> Self {
    Self {
        index: query.index,
        tokens: &query.tokens,
        line_by_pos: &query.line_by_pos,
        text: &query.text,
        high_matchables: &query.high_matchables,
        low_matchables: &query.low_matchables,
        digit_only_tids: &query.index.digit_only_tids,
        start,
        end,
        len_legalese: query.index.len_legalese,
    }
}
```

**After**:

```rust
pub fn new(query: &'a Query<'a>, start: usize, end: Option<usize>) -> Self {
    Self { query, start, end }
}
```

#### Step 3: Update QueryRun Methods

**File**: `src/license_detection/query.rs` (lines 874-1065)

**3.1: Update `get_index()`**:

```rust
pub fn get_index(&self) -> &LicenseIndex {
    self.query.index
}
```

**3.2: Update `start_line()`**:

```rust
pub fn start_line(&self) -> Option<usize> {
    self.query.line_by_pos.get(self.start).copied()
}
```

**3.3: Update `end_line()`**:

```rust
pub fn end_line(&self) -> Option<usize> {
    self.end.and_then(|e| self.query.line_by_pos.get(e).copied())
}
```

**3.4: Update `line_for_pos()`**:

```rust
pub fn line_for_pos(&self, pos: usize) -> Option<usize> {
    self.query.line_by_pos.get(pos).copied()
}
```

**3.5: Update `tokens()`**:

```rust
pub fn tokens(&self) -> &[u16] {
    match self.end {
        Some(end) => &self.query.tokens[self.start..=end],
        None => &[],
    }
}
```

**3.6: Update `is_digits_only()`**:

```rust
pub fn is_digits_only(&self) -> bool {
    self.tokens()
        .iter()
        .all(|tid| self.query.index.digit_only_tids.contains(tid))
}
```

**3.7: Update `high_matchables()`** (compute on-demand):

```rust
pub fn high_matchables(&self) -> HashSet<usize> {
    let end_pos = self.end.unwrap_or(usize::MAX);
    self.query.high_matchables
        .iter()
        .filter(|&&pos| pos >= self.start && pos <= end_pos)
        .copied()
        .collect()
}
```

**3.8: Update `low_matchables()`** (compute on-demand):

```rust
pub fn low_matchables(&self) -> HashSet<usize> {
    let end_pos = self.end.unwrap_or(usize::MAX);
    self.query.low_matchables
        .iter()
        .filter(|&&pos| pos >= self.start && pos <= end_pos)
        .copied()
        .collect()
}
```

**3.9: Update `matched_text()`**:

```rust
pub fn matched_text(&self, start_line: usize, end_line: usize) -> String {
    self.query.matched_text(start_line, end_line)
}
```

#### Step 4: Optional - Add Lazy Caching (Performance Optimization)

If profiling shows `high_matchables()`/`low_matchables()` is called frequently, add caching:

**File**: `src/license_detection/query.rs`

```rust
use std::cell::OnceCell;

pub struct QueryRun<'a> {
    query: &'a Query<'a>,
    pub start: usize,
    pub end: Option<usize>,
    // Lazy caches
    _high_matchables: OnceCell<HashSet<usize>>,
    _low_matchables: OnceCell<HashSet<usize>>,
}

impl<'a> QueryRun<'a> {
    pub fn high_matchables(&self) -> &HashSet<usize> {
        self._high_matchables.get_or_init(|| {
            let end_pos = self.end.unwrap_or(usize::MAX);
            self.query.high_matchables
                .iter()
                .filter(|&&pos| pos >= self.start && pos <= end_pos)
                .copied()
                .collect()
        })
    }
}
```

**Note**: This adds interior mutability via `OnceCell`, making `high_matchables()` return `&HashSet` instead of owned `HashSet`. All callers would need to be updated to use `&HashSet` or `.clone()`.

**Recommendation**: Skip caching in Phase 1. Add only if profiling shows it's needed.

#### Step 5: Update Tests

**File**: `src/license_detection/query.rs` (tests at lines 1067-1838)

Most tests use `QueryRun::new()` and access `.start`, `.end`, `.tokens()`, etc. These should continue to work.

Specific tests to verify:

- `test_query_run_new` (line 1242)
- `test_query_whole_query_run` (line 1252)
- `test_query_run_tokens` (line 1264)
- `test_query_run_matchables` (line 1330)
- `test_query_run_high_low_matchables_slice` (line 1666)

#### Step 6: Verify All Callers Compile

Run after making changes:

```bash
cargo check --all
cargo test --lib license_detection::query
```

### Testing Strategy

1. **Unit tests**: Run existing `query.rs` tests to verify no regressions
2. **Integration tests**: Run `golden_test` to verify detection results unchanged
3. **Performance**: Compare before/after on large files (should be negligible)

### Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Lifetime issues | Low | Lifetime `'a` ties QueryRun to Query |
| Performance regression | Low | Computation is simple filtering |
| Caller breakage | Low | All methods maintain same signatures |
| Test failures | Low | Existing tests should pass |

### Estimated Effort

| Task | Time | Complexity |
|------|------|------------|
| Step 1-2: Struct simplification | 30 min | Low |
| Step 3: Method updates | 1 hour | Medium |
| Step 4: Optional caching | 1 hour | Medium |
| Step 5: Test verification | 30 min | Low |
| Step 6: Integration testing | 30 min | Low |
| **Total** | **3.5 hours** | |

---

## Implementation Order

1. ~~**Issue 3** (Remove filter_short_gpl_matches)~~ ✅ DONE
2. ~~**Issue 5** (Add unknown filter)~~ ✅ DONE
3. ~~**Issue 4** (Query Run lazy eval)~~ ✅ DONE - Architectural cleanup
4. **Issue 6** (Add required phrases filter) - Complex, requires rule parsing changes

---

## Verification Commands

```bash
cargo test --release -q --lib license_detection::golden_test
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```
