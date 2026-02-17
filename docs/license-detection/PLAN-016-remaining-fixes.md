# PLAN-016: Remaining License Detection Fixes

## Status: All Phases Complete - Bug Fix Applied

### Phase A Results (2026-02-17)

- Priority 1: Query run matching - Implemented `matched_qspans` tracking. Query runs remain disabled due to double-matching issues.
- Priority 2: Post-loop logic - Implemented.
- Golden tests: 103 failures (no change from baseline)

### Phase B Results (2026-02-17)

- Fixed `hilen()` implementation - Added `hilen` field to LicenseMatch, populated during matching
- Implemented `qdensity()` and `idensity()` methods
- Golden tests: 103 failures (no change)

### Phase C Results (2026-02-17)

- Implemented `filter_too_short_matches()` with `is_small()` method
- Implemented `filter_spurious_matches()` with density thresholds
- Implemented `filter_below_rule_minimum_coverage()`
- Implemented `filter_short_matches_scattered_on_too_many_lines()`
- Implemented `filter_matches_to_spurious_single_token()`
- Implemented `filter_invalid_matches_to_single_word_gibberish()`
- Golden tests: 103 failures (no change from baseline)

### Bug Fix (2026-02-17)

- Fixed matcher string comparison: Changed `"4-seq"` to `"3-seq"` in all filters
- Fixed `matcher_order()` to use correct matcher strings (`"1-spdx-id"`, `"3-seq"`)
- **Golden tests: 102 failures** (1 test fixed!)

---

## Current State

| Metric | Value |
|--------|-------|
| lic1 passed | 189 |
| lic1 failed | 102 |

---

## Priority 1: Query Run Matching with Matched Position Tracking

### Problem

Query run splitting is disabled because it causes double-matching. Python tracks matched positions across all phases and passes them to `is_matchable()`. Rust doesn't.

### Accuracy Assessment

| Plan Statement | Assessment |
|----------------|------------|
| "Query run splitting is disabled because it causes double-matching" | Correct |
| "Python tracks matched positions across all phases" | Correct |
| "Rust doesn't track matched positions" | Correct |
| Implementation focuses on Phase 4 only | **Incomplete** - Phase 3 also needs awareness |

**Missing from original plan:**

- Rust Phase 3 (`seq_match` on whole query) doesn't exist in Python's approximate matching flow
- Phase 3 causes double-matching because it doesn't check for already-matched positions
- Python only tracks qspans with **100% coverage** for `is_matchable()` checks (index.py:1056-1057)

### Python Reference Details

**File: `reference/scancode-toolkit/src/licensedcode/index.py`**

Tracking matched qspans (lines 1019, 1056-1057):

```python
already_matched_qspans = []
...
already_matched_qspans.extend(
    mtch.qspan for mtch in matched if mtch.coverage() == 100)
```

Using in is_matchable() (lines 1061-1064):

```python
if not whole_query_run.is_matchable(
    include_low=matcher.include_low,
    qspans=already_matched_qspans,
):
```

Inside get_approximate_matches() (lines 739-771):

```python
already_matched_qspans = matched_qspans[:]
...
for match in matched:
    qspan = match.qspan
    query.subtract(qspan)
    already_matched_qspans.append(qspan)
```

**File: `reference/scancode-toolkit/src/licensedcode/query.py:798-818`**

```python
def is_matchable(self, include_low=False, qspans=None):
    if include_low:
        matchables = self.matchables
    else:
        matchables = self.high_matchables
    if self.is_digits_only():
        return False
    if not qspans:
        return matchables
    matched = intbitset.union(*[q._set for q in qspans])
    matchables = intbitset(matchables)
    matchables.difference_update(matched)
    return matchables
```

### Rust Implementation Details

**File: `src/license_detection/mod.rs`**

Phase 2 (lines 149-157) - Does subtract, but doesn't track qspans:

```rust
for m in &near_dupe_matches {
    if m.end_token > m.start_token {
        let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
        query.subtract(&span);  // ✓ Subtracts
        // ✗ Missing: tracking matched_qspans
    }
}
```

Phase 3 (lines 160-165) - Matches whole query WITHOUT awareness of Phase 2 matches:

```rust
{
    let whole_run = query.whole_query_run();
    let seq_matches = seq_match(&self.index, &whole_run);  // ✗ No awareness of matched positions
    all_matches.extend(seq_matches);
}
```

Phase 4 (lines 173-198) - Passes empty `&[]` to is_matchable:

```rust
if !query_run.is_matchable(false, &[]) {  // ✗ Should pass matched_qspans
    continue;
}
```

**File: `src/license_detection/query.rs:436-448`** - Query runs disabled:

```rust
// TODO: Re-enable query run splitting once the matching algorithm
// properly tracks matched qspans across phases.
// let query_runs = Self::compute_query_runs(...);
let query_runs: Vec<(usize, Option<usize>)> = Vec::new();
```

**File: `src/license_detection/query.rs:946-964`** - `is_matchable()` already supports exclude positions:

```rust
pub fn is_matchable(&self, include_low: bool, exclude_positions: &[PositionSpan]) -> bool {
    // ✓ Implementation is correct, just needs to be passed the right data
}
```

### Specific Changes Needed

**File: `src/license_detection/mod.rs`**

**Change 1 (line ~115):** Add matched_qspans tracking variable:

```rust
let mut all_matches = Vec::new();
let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();
```

**Change 2 (lines ~149-157):** Track qspans after Phase 2 near-duplicate matching:

```rust
for m in &near_dupe_matches {
    if m.end_token > m.start_token {
        let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
        query.subtract(&span);
        matched_qspans.push(span);
    }
}
```

**Change 3 (lines ~160-165):** Remove Phase 3 OR make it aware of matched positions:

- **Recommended:** Remove Phase 3 since Python doesn't have this phase in approximate matching
- Python only does: (a) Near-duplicate matching on whole query, (b) Query run matching on individual runs

**Change 4 (line ~183):** Pass matched_qspans to is_matchable:

```rust
if !query_run.is_matchable(false, &matched_qspans) {
```

**File: `src/license_detection/query.rs:436-448`** - Re-enable query runs:

```rust
let query_runs = Self::compute_query_runs(
    &tokens,
    &tokens_by_line,
    _line_threshold,
    len_legalese,
    &index.digit_only_tids,
);
```

### Edge Cases

1. **100% coverage matches only**: Python only tracks matches with `coverage() == 100` for `already_matched_qspans` (index.py:1057). Rust should do the same.

2. **Phase 3 doesn't exist in Python**: The Rust Phase 3 (`seq_match` on whole query) is NOT in Python's `get_approximate_matches()`.

3. **Query subtraction vs qspans tracking**: Python does BOTH `query.subtract()` AND `matched_qspans.append()`. The subtraction updates matchables for future checks, while qspans are passed to `is_matchable()` for per-run checks.

### Verification

```bash
# Run specific tests
cargo test --release --lib license_detection::mod::tests

# Run golden tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
```

### Expected Impact

~20 tests where combined rules should match instead of partial rules.

---

## Priority 2: `has_unknown_intro_before_detection()` Post-Loop Logic

### Problem

The `has_unknown_intro_before_detection()` function is missing post-loop logic that Python has.

### Accuracy Assessment

**The plan is accurate but the code snippet was incomplete.** The actual missing logic is more nuanced.

### Python Reference Details

**File: `reference/scancode-toolkit/src/licensedcode/detection.py:1289-1333`**

Complete function:

```python
# Main loop (lines 1311-1321)
for match in license_matches:
    if is_unknown_intro(match):
        has_unknown_intro = True
        continue

    if has_unknown_intro:
        if not is_match_coverage_less_than_threshold(
            [match], IMPERFECT_MATCH_COVERAGE_THR
        ) and not has_unknown_matches([match]):
            has_unknown_intro_before_detection = True
            return has_unknown_intro_before_detection  # Early return

# POST-LOOP LOGIC (lines 1323-1331) - MISSING IN RUST
if has_unknown_intro:
    filtered_matches = filter_license_intros(license_matches)
    if license_matches != filtered_matches:
        if is_match_coverage_less_than_threshold(
            license_matches=filtered_matches,
            threshold=IMPERFECT_MATCH_COVERAGE_THR,
            any_matches=False,  # KEY: "any_matches=False"
        ):
            has_unknown_intro_before_detection = True

return has_unknown_intro_before_detection
```

The `is_match_coverage_less_than_threshold` with `any_matches=False` (lines 1095-1107):

```python
if not any_matches:
    return not any(
        license_match.coverage() > threshold
        for license_match in license_matches
    )
```

### Rust Implementation Details

**File: `src/license_detection/detection.rs:428-457`**

Current implementation returns `false` after the loop without the post-loop check:

```rust
fn has_unknown_intro_before_detection(matches: &[LicenseMatch]) -> bool {
    // ... early returns ...

    let mut has_unknown_intro = false;

    for m in matches {
        if is_unknown_intro(m) {
            has_unknown_intro = true;
            continue;
        }

        if has_unknown_intro {
            let coverage_ok = m.match_coverage >= IMPERFECT_MATCH_COVERAGE_THR - 0.01;
            let not_unknown = !m.rule_identifier.contains("unknown") 
                && !m.license_expression.contains("unknown");
            if coverage_ok && not_unknown {
                return true;
            }
        }
    }

    false  // <-- MISSING POST-LOOP LOGIC HERE
}
```

Helper `filter_license_intros` exists at line 488-500, `is_match_coverage_below_threshold` at line 284-293.

### Specific Changes Needed

**File: `src/license_detection/detection.rs` - Replace lines 454-456:**

```rust
// Replace:
    false
}

// With:
    if has_unknown_intro {
        let filtered_matches = filter_license_intros(matches);
        if filtered_matches.len() != matches.len() {
            if is_match_coverage_below_threshold(&filtered_matches, IMPERFECT_MATCH_COVERAGE_THR, false) {
                return true;
            }
        }
    }

    false
}
```

### Edge Cases

1. **All unknown intros**: If all matches are unknown intros, function returns `false` early (correct in both).
2. **No unknown intro**: If `has_unknown_intro` is false after loop, skip the post-loop check (correct).
3. **Filtered same as original**: If `filter_license_intros` returns same list, skip coverage check.
4. **any_matches=False semantics**: Return true if **NONE** of filtered matches have coverage > threshold.

### Verification

```bash
cargo test --release --lib license_detection::detection::tests::test_has_unknown
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
```

### Expected Impact

~10 tests fixed.

---

## Priority 3: Missing Filters

### Accuracy Assessment

**The plan was partially accurate but incomplete.**

**Correct:**

- All 3 filters are indeed missing from Rust

**Missing from original plan:**

- **6 additional filters** are also missing from Python's pipeline
- The complexity of `filter_matches_missing_required_phrases()` is **vastly understated** - requires deep integration with Query/Rule objects

### Python's Complete `refine_matches()` Pipeline

From `match.py:2744-2817`:

| Order | Filter | Status |
|-------|--------|--------|
| 1 | `filter_matches_missing_required_phrases()` | Missing |
| 2 | `filter_spurious_matches()` | Missing |
| 3 | `filter_below_rule_minimum_coverage()` | Missing |
| 4 | `filter_matches_to_spurious_single_token()` | Missing |
| 5 | `filter_too_short_matches()` | Missing |
| 6 | `filter_short_matches_scattered_on_too_many_lines()` | Missing |
| 7 | `filter_invalid_matches_to_single_word_gibberish()` | Missing |
| 8 | `merge_matches()` | Implemented |
| 9 | `filter_contained_matches()` | Implemented |
| 10 | `filter_overlapping_matches()` | Implemented |
| 11 | `filter_false_positive_matches()` | Implemented |
| 12 | `filter_false_positive_license_lists_matches()` | Implemented |

### Critical Infrastructure Gap

`LicenseMatch` in `models.rs:178-259` is a **flat struct** without:

- Reference to `Query` object (needed for `unknowns_by_pos`, `stopwords_by_pos`)
- Reference to `Rule` object (needed for `min_matched_length`, `required_phrase_spans`)
- Methods: `qdensity()`, `idensity()`, `qmagnitude()`, `is_continuous()`
- Field: `hispan` (high-value token positions) - needed for correct `hilen()`

### 3.1 `filter_too_short_matches()` - Easiest

**Python:** `match.py:1706-1737`

Filters matches where `matcher == "3-seq"` (Rust: `"4-seq"`) and `match.is_small()` returns true.

**Implementation:**

Add method to `LicenseMatch` in `models.rs`:

```rust
pub fn is_small(&self, min_matched_len: usize, min_high_matched_len: usize, rule_is_small: bool) -> bool {
    if self.matched_length < min_matched_len || self.hilen() < min_high_matched_len {
        return true;
    }
    if rule_is_small && self.match_coverage < 80.0 {
        return true;
    }
    false
}
```

**Problem:** `hilen()` is incorrect - see Additional Problem #1 below.

### 3.2 `filter_spurious_matches()` - Medium Complexity

**Python:** `match.py:1768-1836`

Filters low-density sequence matches. Density thresholds:

```python
if (mlen < 10 and (qdens < 0.1 or idens < 0.1)): discard
elif (mlen < 15 and (qdens < 0.2 or idens < 0.2)): discard
elif (mlen < 20 and hilen < 5 and (qdens < 0.3 or idens < 0.3)): discard
elif (mlen < 30 and hilen < 8 and (qdens < 0.4 or idens < 0.4)): discard
elif (qdens < 0.4 or idens < 0.4): discard
```

**Infrastructure needed:**

- `qdensity()` - query-side density (matched tokens vs gaps in qspan)
- `idensity()` - index-side density (matched tokens vs gaps in ispan)
- These require storing `qspan`/`ispan` properly during matching

### 3.3 `filter_matches_missing_required_phrases()` - Most Complex

**Python:** `match.py:2154-2322`

Validates that rules with `{{...}}` required phrases are actually present in matched text.

**Recommended approach:** Defer complex implementation. Implement simplified version that:

1. Checks the solo-match exception (match.py:2171-2175)
2. Validates `is_continuous` rules have no gaps

### Implementation Order for Filters

1. `filter_too_short_matches()` (smallest change)
2. `filter_spurious_matches()` (requires density methods)
3. `filter_matches_missing_required_phrases()` (defer if not critical)

### Expected Impact

~15-20 tests fixed.

---

## Additional Problems Identified

### Problem 1: Incorrect `hilen()` Implementation

**Description:** Rust's `hilen()` returns `matched_length / 2` (character count halved), while Python's returns `len(self.hispan)` - the count of matched **high-value legalese tokens**.

**Why it matters:** `hilen()` is used in `filter_spurious_matches()` and `filter_overlapping_matches()` to prioritize matches with more legalese tokens.

**Python ref:** `match.py:432-436`
**Rust ref:** `models.rs:302-304`

**Fix:** Add `hispan: Vec<usize>` field to `LicenseMatch`, populated during matching based on `tid < len_legalese`. Then `hilen()` returns `hispan.len()`.

---

### Problem 2: Missing `qdensity()` and `idensity()` Methods

**Description:** Python uses query-side and index-side density to filter spurious matches. Rust doesn't implement these.

**Why it matters:** Required for `filter_spurious_matches()`.

**Python ref:** `match.py:1795-1831`
**Rust ref:** None

**Fix:** Implement density calculations based on gaps in `qspan`/`ispan` vs matched positions.

---

### Problem 3: Missing `filter_below_rule_minimum_coverage()`

**Description:** Rules can have a `minimum_coverage` attribute. Matches below this threshold should be discarded.

**Why it matters:** Some rules require high coverage to be valid (e.g., GPL rules).

**Python ref:** `match.py:1551-1587`
**Rust ref:** `match_refine.rs:642-672` - not implemented

**Fix:** Add filter after `filter_spurious_matches()` in pipeline.

---

### Problem 4: Missing `filter_matches_to_spurious_single_token()`

**Description:** Filters single-token matches surrounded by many unknown/short/digit tokens.

**Why it matters:** Prevents false positives from single common words in binary or code-heavy files.

**Python ref:** `match.py:1622-1700`
**Rust ref:** None

**Fix:** Implement using `query.unknowns_by_pos` to check surrounding context.

---

### Problem 5: Missing `filter_short_matches_scattered_on_too_many_lines()`

**Description:** Short matches scattered across too many lines (more lines than tokens) are likely spurious.

**Why it matters:** A 3-token match spanning 50 lines is likely not a valid license reference.

**Python ref:** `match.py:1931-1972`
**Rust ref:** None

**Fix:** Compute line span and compare to matched token count.

---

### Problem 6: Missing `filter_invalid_matches_to_single_word_gibberish()`

**Description:** Filters gibberish matches in binary files - single-word rules with mixed case or punctuation issues.

**Why it matters:** Binary files often contain random text that matches single-word rules incorrectly.

**Python ref:** `match.py:1839-1901`
**Rust ref:** None

**Fix:** Implement for binary file edge cases.

---

## Implementation Order

### Phase A: Core Pipeline Fixes (High Impact)

1. **Priority 1**: Query run matching with matched_qspans tracking
2. **Priority 2**: Post-loop logic for `has_unknown_intro_before_detection()`

### Phase B: Filter Infrastructure (Prerequisite for filters)

1. Fix `hilen()` - add `hispan` field
1. Implement `qdensity()` and `idensity()` methods

### Phase C: Missing Filters

1. `filter_too_short_matches()`
1. `filter_spurious_matches()`
1. `filter_below_rule_minimum_coverage()`
1. `filter_matches_to_spurious_single_token()`
1. `filter_short_matches_scattered_on_too_many_lines()`
1. `filter_invalid_matches_to_single_word_gibberish()`
1. `filter_matches_missing_required_phrases()` (complex, defer if not critical)

---

## Verification Commands

```bash
# Run specific module tests
cargo test --release --lib license_detection::detection
cargo test --release --lib license_detection::match_refine
cargo test --release --lib license_detection::query

# Run golden tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Check code quality
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```

---

## Test Cases to Add

### Priority 1 Tests

```rust
#[test]
fn test_matched_qspans_prevents_double_matching() {
    // Test that Phase 2 matches are excluded from Phase 4 matching
}

#[test]
fn test_query_run_is_matchable_with_exclusions() {
    // Test is_matchable correctly excludes positions
}
```

### Priority 2 Tests

```rust
#[test]
fn test_has_unknown_intro_before_detection_post_loop_low_coverage() {
    // Unknown intro followed by low-coverage match
}

#[test]
fn test_has_unknown_intro_before_detection_post_loop_high_coverage() {
    // Unknown intro followed by high-coverage match
}
```

### Filter Tests

```rust
#[test]
fn test_filter_too_short_matches() {
    // Test that small seq matches are filtered
}

#[test]
fn test_filter_spurious_matches_density() {
    // Test that low-density matches are filtered
}
```
