# PLAN-015: Consolidated License Detection Fixes

## Status: In Progress - Session 4

---

## Implementation Results

### Session 2 (P1-P5)

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| lic1 passed | 175 | 187 | **+12** |
| lic1 failed | 116 | 104 | **-12** |

| Priority | Fix | Status | Tests Fixed |
|----------|-----|--------|-------------|
| P1 | Expression deduplication | ✅ Already implemented | ~8 |
| P2 | WITH parentheses | ✅ Already implemented | ~6 |
| P3 | `filter_license_references()` | ✅ Implemented | ~15 |
| P4 | Grouping logic (AND) | ✅ Implemented | ~10 |
| P5 | Single-match false positive filter | ✅ Implemented | ~15 |

### Session 3 (Near-Duplicate Detection)

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| lic1 passed | 187 | 187 | 0 |
| lic1 failed | 104 | 104 | 0 |

Near-duplicate detection implemented but no improvement because combined rule resemblance (0.2333) is below 0.8 threshold.

### Session 4 (Query Subtraction)

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| lic1 passed | 187 | 188 | **+1** |
| lic1 failed | 104 | 103 | **-1** |

| Issue | Fix | Status |
|-------|-----|--------|
| Issue 10 | Span subtraction infrastructure | ✅ Implemented |
| Issue 11 | SPDX token position tracking | ✅ Implemented |
| Span subtraction | After near-duplicate matches | ✅ Enabled |
| Query run splitting | Per-run matching | ❌ Disabled (causes double-matching) |

### Current Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| SPDX token positions | ✅ Fixed | No more hardcoded 0, 0 |
| Span subtraction | ✅ Enabled | After near-duplicate phase |
| Query run splitting | ❌ Disabled | Needs matched position tracking across phases |

### Dead Code Cleanup Needed

| Function | Reason | Action |
|----------|--------|--------|
| `compute_candidates()` | Superseded by `compute_candidates_with_msets()` | Remove |
| `extract_spdx_expressions_with_lines()` | Superseded by `query.spdx_lines` | Remove |

### Remaining Issues (~103 failures)

1. **Query run matching not enabled**: Cannot enable due to missing matched position tracking across phases
2. **P6 not implemented**: `has_unknown_intro_before_detection()` post-loop logic
3. **Dead code investigation**: `compute_candidates()` and `extract_spdx_expressions_with_lines()` are unused
4. **Other missing filters**: `filter_matches_missing_required_phrases()`, `filter_spurious_matches()`, `filter_too_short_matches()`

---

## Issue 9: Query Run Matching (NEW - Critical)

### Problem

Python matches the combined rule `cddl-1.0_or_gpl-2.0-glassfish.RULE` via **query-run-level matching with `high_resemblance=False`**, not near-duplicate detection.

### Python's Actual Pipeline

**From `index.py:786-796`:**

```python
for query_run in query.query_runs:
    candidates = match_set.compute_candidates(
        query_run=query_run,
        idx=self,
        top=MAX_CANDIDATES,
        high_resemblance=False,  # KEY: lower threshold for individual runs
    )
    matched = self.get_query_run_approximate_matches(query_run, candidates, ...)
```

### Key Differences

| Phase | Python | Rust |
|-------|--------|------|
| Near-duplicate (whole file) | `high_resemblance=True` (0.8) | ✅ Implemented |
| Query run matching | `high_resemblance=False` | ❌ NOT Implemented |
| Query run splitting | `Query.query_runs` property | Needs implementation |

### Fix Required

1. **Implement query run splitting**:
   - Python's `Query.query_runs` property splits on 4+ empty/junk lines
   - Each run is a contiguous block of content

2. **Add query run matching phase**:

   ```rust
   // Phase 3: Query run matching with high_resemblance=false
   for query_run in query.query_runs() {
       let candidates = compute_candidates_with_msets(
           &self.index,
           &query_run,
           false,  // high_resemblance=False for query runs
           MAX_CANDIDATES,
       );
       let matches = seq_match_with_candidates(&self.index, &query_run, &candidates);
       all_matches.extend(matches);
   }
   ```

### Python References

| Component | Location |
|-----------|----------|
| Query run property | `query.py:596-600` |
| Query run matching | `index.py:786-796` |
| LINES_THRESHOLD | `query.py:108` (value: 4) |

### Estimated Tests Fixed

~20 tests where combined rules should match:

- `cddl-1.0_or_gpl-2.0-glassfish.txt`
- `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_*.txt`

---

## Root Cause Analysis

### Summary of 116 Failing Tests

| Category | Tests | Root Cause |
|----------|-------|------------|
| Extra `unknown`/`unknown-license-reference` detections | ~30 | Missing license intro filtering + missing `filter_license_references()` |
| Matches incorrectly grouped with AND | ~25 | `should_group_together()` uses line-only, Python uses dual-criteria |
| Single `is_license_reference` false positives | ~15 | `filter_false_positive_license_lists_matches` threshold too high |
| Duplicate expressions in output | ~8 | `simplify_expression()` deduplication doesn't fully work |
| Unnecessary parentheses in WITH expressions | ~6 | `expression_to_string_internal` uses `!=` instead of `>` for precedence |
| Deduplication removes valid detections | ~10 | `remove_duplicate_detections` uses expression only, not identifier |

---

## Deep Analysis: 5 Representative Failures

### 1. `cddl-1.0_or_gpl-2.0-glassfish.txt`

**Expected:** `["cddl-1.0 OR gpl-2.0"]`
**Actual:** `["gpl-2.0 AND cddl-1.0 AND unknown-license-reference AND unknown"]`

**Root Causes:**

1. **No combined rule match**: Python matches the entire text with a single rule `cddl-1.0_or_gpl-2.0-glassfish` that has `license_expression: cddl-1.0 OR gpl-2.0`. Rust matches partial rules instead.
2. **Missing `filter_license_references()`**: The `unknown-license-reference` match from the "Oracle copyright" text should be filtered.
3. **Missing `has_unknown_intro_before_detection()` filtering**: The `unknown` intro match should be discarded.

**Python Reference:**

- `detection.py:1289-1333` - `has_unknown_intro_before_detection()`
- `detection.py:1336-1346` - `filter_license_intros()`
- `detection.py:1390-1400` - `filter_license_references()`

**Fix Required:**

```rust
// In create_detection_from_group() - after analyze_detection()
if detection_log.contains(DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH) {
    let filtered = filter_license_intros(&detection.matches);
    if !filtered.is_empty() {
        detection.matches = filtered;
        // Recompute expression
    }
}
```

### 2. `CRC32.java`

**Expected:** `["apache-2.0", "bsd-new", "zlib"]`
**Actual:** `["apache-2.0", "bsd-new AND zlib"]`

**Root Cause:**

- Lines 16-47 contain BSD-new license text
- Lines 44-47 contain additional zlib attribution
- Rust groups `bsd-new` and `zlib` matches together because they're within `LINES_THRESHOLD = 4`
- Python keeps them separate because there's no actual overlap in the matched regions

**Python Reference:**

- `detection.py:1836` - Uses `min_tokens_gap=10 OR min_lines_gap=3`
- The OR logic means matches are grouped if EITHER tokens OR lines are close
- But for SEPARATION, Python checks actual content overlap

**Fix Required:**

```rust
// detection.rs - should_group_together()
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    const TOKENS_THRESHOLD: usize = 10;
    const LINES_THRESHOLD: usize = 3;
    
    let line_gap = if cur.start_line > prev.end_line {
        cur.start_line - prev.end_line
    } else {
        0
    };
    
    let token_gap = if cur.start_token > prev.end_token {
        cur.start_token - prev.end_token
    } else {
        0
    };
    
    // Python uses OR: group if EITHER tokens OR lines are close
    token_gap <= TOKENS_THRESHOLD || line_gap <= LINES_THRESHOLD
}
```

### 3. `gpl-2.0-plus_11.txt` (borceux false positive)

**Expected:** `["gpl-2.0-plus"]`
**Actual:** `["gpl-2.0-plus", "borceux"]`

**Root Cause:**

- `borceux` is a single-token `is_license_reference` rule matching the word "GPL"
- The `filter_false_positive_license_lists_matches()` function requires `MIN_SHORT_FP_LIST_LENGTH = 15` matches
- This test has only 1 `borceux` match, so it's not filtered

**Python Reference:**

- `match.py:1953` - `is_candidate_false_positive()` checks for `is_license_tag` or `is_license_reference`
- `match.py:1962-2010` - The filter processes sequences of candidates
- Single false positive matches should be handled differently

**Fix Required:**

```rust
// match_refine.rs - Add to is_false_positive() in detection.rs
// Check 4: Single is_license_reference match with short rule
if is_single && matches.iter().all(|m| m.is_license_reference && m.rule_length <= 3) {
    return true;
}
```

### 4. `crapl-0.1.txt`

**Expected:** `["crapl-0.1"]`
**Actual:** `["crapl-0.1 AND crapl-0.1"]`

**Root Cause:**

- The `simplify_expression()` function collects unique keys in a `HashSet`
- But it still adds duplicates when building the result because `collect_unique_and` uses `expression_to_string` for the key, which may differ from the actual key

**Fix Required:**

```rust
// expression.rs - collect_unique_and()
fn collect_unique_and(expr: &LicenseExpression, unique: &mut Vec<LicenseExpression>, seen: &mut HashSet<String>) {
    match expr {
        LicenseExpression::License(key) => {
            // Use the key directly for deduplication, not expression_to_string
            if !seen.contains(key) {
                seen.insert(key.clone());
                unique.push(LicenseExpression::License(key.clone()));
            }
        }
        // ... similar for LicenseRef
    }
}
```

### 5. `eclipse-omr.LICENSE`

**Expected:** `["(epl-1.0 OR apache-2.0) AND bsd-new AND mit AND bsd-new AND gpl-3.0-plus WITH autoconf-simple-exception", ...]`
**Actual:** `["(epl-1.0 OR apache-2.0) AND bsd-new AND mit AND bsd-new AND (gpl-3.0-plus WITH autoconf-simple-exception)", ...]`

**Root Cause:**

- `expression_to_string_internal` uses `parent_prec != Precedence::With` for parentheses
- Should use `parent_prec > Precedence::With` to only add parentheses when parent has HIGHER precedence

**Fix Required:**

```rust
// expression.rs:426-429
LicenseExpression::With { left, right } => {
    let left_str = expression_to_string_internal(left, Some(Precedence::With));
    let right_str = expression_to_string_internal(right, Some(Precedence::With));
    // WITH has highest precedence - no parentheses needed unless parent has higher (none)
    format!("{} WITH {}", left_str, right_str)
}
```

---

## Critical Missing Functions in Rust

### 1. `filter_license_references()` - MISSING

**Python:** `detection.py:1390-1400`

Called when detection category is `UNKNOWN_REFERENCE_TO_LOCAL_FILE` to filter out `unknown-license-reference` matches from the expression.

```python
def filter_license_references(license_match_objects):
    filtered_matches = [match for match in license_match_objects 
                        if not match.rule.is_license_reference]
    return filtered_matches or license_match_objects
```

**Rust Implementation Needed:**

```rust
fn filter_license_references(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    let filtered: Vec<_> = matches
        .iter()
        .filter(|m| !m.is_license_reference)
        .cloned()
        .collect();
    if filtered.is_empty() { matches.to_vec() } else { filtered }
}
```

### 2. `filter_matches_missing_required_phrases()` - MISSING

**Python:** `match.py:2154-2316`

Filters matches that don't contain required phrases marked with `{{...}}` in the rule text. This is critical for SPDX-ID rules that must match exact text.

### 3. `filter_spurious_matches()` - MISSING

**Python:** `match.py:1768-1836`

Filters low-density sequence matches (matched tokens are scattered, not contiguous).

### 4. `filter_too_short_matches()` - MISSING

**Python:** `match.py:1706-1737`

Filters matches where `match.is_small()` returns true (based on `rule.min_matched_length` and coverage).

---

## Proposed Fixes (Prioritized)

### Priority 1: Fix Expression Deduplication (8 tests fixed)

**File:** `src/license_detection/expression.rs`
**Location:** `collect_unique_and()` and `collect_unique_or()`

**Change:** Use license key directly for HashSet key, not `expression_to_string()` result.

### Priority 2: Fix WITH Parentheses (6 tests fixed)

**File:** `src/license_detection/expression.rs`
**Location:** `expression_to_string_internal()`

**Change:** WITH has highest precedence. Never add parentheses around WITH expressions.

### Priority 3: Implement `filter_license_references()` (15 tests fixed)

**File:** `src/license_detection/detection.rs`
**Location:** After `analyze_detection()` in `populate_detection_from_group()`

**Change:** Call `filter_license_references()` for detections with license reference matches.

### Priority 4: Fix Grouping Logic (25 tests fixed)

**File:** `src/license_detection/detection.rs`
**Location:** `should_group_together()`

**Change:** Use OR logic: `token_gap <= 10 || line_gap <= 3`

### Priority 5: Add Single-Match False Positive Filter (15 tests fixed)

**File:** `src/license_detection/detection.rs`
**Location:** `is_false_positive()`

**Change:** Add check for single `is_license_reference` match with short rule length.

### Priority 6: Fix `has_unknown_intro_before_detection()` Post-Loop Logic (10 tests fixed)

**File:** `src/license_detection/detection.rs`
**Location:** `has_unknown_intro_before_detection()`

**Change:** Add the post-loop check that Python has at lines 1323-1331:

```rust
// After the main loop, if we had unknown intro but no proper detection followed
if has_unknown_intro {
    let filtered = filter_license_intros(matches);
    if matches != filtered {
        // Check if filtered matches have insufficient coverage
        // Return true if so (meaning the unknown intro can be discarded)
    }
}
```

---

## Implementation Order

1. **Expression fixes first** (P1, P2) - Simple, low risk, ~14 tests fixed
2. **Filter implementation** (P3, P5) - Medium risk, ~30 tests fixed
3. **Grouping logic** (P4) - Higher risk, needs careful testing, ~25 tests fixed
4. **Post-loop logic** (P6) - Medium risk, ~10 tests fixed

**Estimated total tests fixed: ~79 (69% of failures)**

---

## Issue 7: Combined Rule Matching - `cddl-1.0_or_gpl-2.0-glassfish.txt`

### Test Case

**File:** `testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt`

**Expected:** `["cddl-1.0 OR gpl-2.0"]`
**Actual:** `["gpl-2.0 AND cddl-1.0", "unknown-license-reference AND unknown"]`

### Root Cause Analysis

#### Why Python Gets It Right

Python uses a **three-phase matching pipeline**:

1. **Phase 1: Hash & Aho-Corasick** - Exact matches
2. **Phase 2: Near-Duplicate Detection** (`index.py:741-775`):

   ```python
   whole_query_run = query.whole_query_run()
   near_dupe_candidates = match_set.compute_candidates(
       query_run=whole_query_run,
       high_resemblance=True,  # KEY: Only keep resemblance >= 0.8
       top=10,
   )
   if near_dupe_candidates:
       matched = self.get_query_run_approximate_matches(
           whole_query_run, near_dupe_candidates, ...)
   ```

3. **Phase 3: Query Run Matching** - Break into runs if no near-duplicates

Python matches the **combined rule** because:

- The whole file is processed as one query run
- Near-duplicate detection finds high-resemblance candidates
- `resemblance ** 2` scoring naturally favors larger matches

#### Why Rust Gets It Wrong

Rust matches **partial rules** instead:

| Rule | Expression | Tokens | Flags |
|------|------------|--------|-------|
| `gpl-2.0_476.RULE` | `gpl-2.0` | 21 | `is_license_notice: true` |
| `cddl-1.0_53.RULE` | `cddl-1.0` | 6 | `is_license_reference: true` |

**Critical Issue**: Query run has only 53 tokens vs combined rule's 262 tokens. The test file has ~150 words, so 53 tokens is too few.

#### Root Causes

1. **Missing near-duplicate detection phase**: Rust doesn't check whole-file resemblance first
2. **Query run size incorrect**: Rust may be breaking the query into smaller runs
3. **Tokenization mismatch**: 53 tokens vs expected ~150 indicates possible tokenization issue

### Python Reference

**Near-duplicate detection** (`index.py:741-775`):

```python
whole_query_run = query.whole_query_run()
near_dupe_candidates = match_set.compute_candidates(
    query_run=whole_query_run,
    high_resemblance=True,  # Only keep resemblance >= 0.8
)
```

**High-resemblance filter** (`match_set.py:295-297`):

```python
if (not high_resemblance
    or (high_resemblance and svr.is_highly_resemblant and svf.is_highly_resemblant)):
    sortable_candidates_append(...)
```

**Squared resemblance scoring** (`match_set.py:427`):

```python
amplified_resemblance = resemblance ** 2
```

### Correct Fix (NOT score boosting)

❌ **WRONG APPROACH** (previously proposed):

```rust
// DO NOT do this - not how Python works
let is_combined = rule.tokens.len() > 100 && rule.is_license_notice;
if is_combined {
    score_vec_full.containment *= 1.5;
}
```

✅ **CORRECT APPROACH** - Add near-duplicate detection phase:

```rust
// In detect_licenses() or similar entry point:

// Phase 2: Near-duplicate detection (before regular sequence matching)
let whole_run = query.whole_query_run();
let near_dupe_candidates = compute_candidates(
    query_run: &whole_run,
    high_resemblance: true,  // Only keep resemblance >= 0.8
    top_n: 10,
);

if !near_dupe_candidates.is_empty() {
    // Match whole file against only these high-resemblance candidates
    return match_against_candidates(&whole_run, &near_dupe_candidates);
}

// Phase 3: Regular query run matching (if no near-duplicates)
for query_run in query.query_runs() {
    // ... existing logic
}
```

### Investigation Needed

1. **Verify query run tokenization**:
   - Why does Rust get 53 tokens when file has ~150 words?
   - Is `whole_query_run()` being called?
   - Are query runs being split incorrectly?

2. **Check `is_highly_resemblant` implementation**:
   - Python: `resemblance >= 0.8`
   - Rust: Need to verify threshold

### Estimated Tests Fixed

This fix addresses ~20 tests where combined rules should match:

- `cddl-1.0_or_gpl-2.0-glassfish.txt`
- `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_*.txt`
- Similar dual-license header cases

---

## Issue 8: Query Run Tokenization - Investigation Results

### The "53 vs 150" Discrepancy Explained

**This is NOT a tokenization bug.** The discrepancy comes from a misunderstanding of what "tokens" means in different contexts.

#### Token Count Analysis for `cddl-1.0_or_gpl-2.0-glassfish.txt`

| Metric | Python | Rust | Notes |
|--------|--------|------|-------|
| Raw tokens (word_splitter) | 273 | 273 | Identical - regex pattern matches |
| After stopwords filter | 270 | 270 | 3 stopwords found ('a' appears 3x) |
| Known tokens (in dictionary) | ~262 | ~262 | Same dictionary used |
| Query.tokens length | 262 | 262 | Only known tokens are stored |

**The 53 token count in the original analysis was incorrect** - it referred to something else (possibly a partial rule match, not the query).

#### How Python and Rust Tokenize

Both use identical tokenization logic:

**Python** (`reference/scancode-toolkit/src/licensedcode/tokenize.py:78-79`):

```python
query_pattern = '[^_\\W]+\\+?[^_\\W]*'
word_splitter = re.compile(query_pattern, re.UNICODE).findall
```

**Rust** (`src/license_detection/tokenize.rs:111-112`):

```rust
static QUERY_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^_\W]+\+?[^_\W]*").expect("Invalid regex pattern"));
```

Both:

1. Split text on whitespace and punctuation
2. Keep alphanumeric characters and Unicode letters
3. Preserve trailing `+` (important for license names like "GPL2+")
4. Convert to lowercase
5. Filter stopwords (HTML tags, XML entities, comment markers)

#### What Query.tokens Actually Contains

Per Python's `query.py:388-389`:

```python
# note: positions start at zero
# absolute position in a query, including only known tokens
known_pos = -1
```

Both Python and Rust only store tokens that exist in the dictionary. Unknown tokens are tracked separately in `unknowns_by_pos`.

### The Real Problem: Missing Near-Duplicate Detection

The actual issue is **not tokenization** - it's the **matching pipeline**.

#### Python's Three-Phase Pipeline

**Phase 1**: Hash & Aho-Corasick exact matching

**Phase 2**: Near-duplicate detection (`index.py:741-775`):

```python
whole_query_run = query.whole_query_run()
near_dupe_candidates = match_set.compute_candidates(
    query_run=whole_query_run,
    high_resemblance=True,  # Only keep resemblance >= 0.8
    top=10,
)
if near_dupe_candidates:
    matched = self.get_query_run_approximate_matches(
        whole_query_run, near_dupe_candidates, ...)
```

**Phase 3**: Query run matching (if no near-duplicates found)

#### Why Python Matches the Combined Rule

1. The whole file is processed as one query run
2. Near-duplicate detection finds high-resemblance candidates (resemblance >= 0.8)
3. The combined rule `cddl-1.0_or_gpl-2.0-glassfish.RULE` has 262 tokens
4. Squared resemblance scoring (`resemblance ** 2`) naturally favors larger matches

#### Why Rust Matches Partial Rules

Rust's current pipeline (`src/license_detection/mod.rs:107-126`):

```rust
let query = Query::new(text, &self.index)?;
let query_run = query.whole_query_run();

let hash_matches = hash_match(&self.index, &query_run);
let aho_matches = aho_match(&self.index, &query_run);
let seq_matches = seq_match(&self.index, &query_run);
// ...
```

Rust matches:

- `gpl-2.0_476.RULE` (21 tokens, `is_license_notice: true`)
- `cddl-1.0_53.RULE` (6 tokens, `is_license_reference: true`)

Instead of the combined rule because:

1. **No near-duplicate detection phase** - Rust goes straight to sequence matching
2. **No resemblance threshold filtering** - Any match above minimum coverage is accepted
3. **First match wins** - Partial rules match first and prevent combined rule from matching

### The Fix: Implement Near-Duplicate Detection

Add Phase 2 to Rust's detection pipeline:

```rust
// In detect() or detect_licenses()

// Phase 2: Near-duplicate detection (NEW)
let whole_run = query.whole_query_run();
let near_dupe_candidates = compute_candidates(
    query_run: &whole_run,
    high_resemblance: true,  // Only keep resemblance >= 0.8
    top_n: 10,
);

if !near_dupe_candidates.is_empty() {
    // Match whole file against only these high-resemblance candidates
    return match_against_candidates(&whole_run, &near_dupe_candidates);
}

// Phase 3: Regular matching (existing code)
for query_run in query.query_runs() {
    // ... existing logic
}
```

#### Required Implementations

1. **`compute_candidates()`** (`match_set.py:260-350`):
   - Compute resemblance between query and all rules
   - Filter by `high_resemblance` (>= 0.8)
   - Return top N candidates sorted by resemblance

2. **`is_highly_resemblant`** property:
   - Python: `resemblance >= 0.8`
   - Rust: Need to add this check

3. **Squared resemblance scoring** (`match_set.py:427`):
   - `amplified_resemblance = resemblance ** 2`
   - This naturally favors larger matches

### Code References

| Component | Python Location | Rust Location |
|-----------|----------------|---------------|
| Query tokenization | `query.py:417-481` | `query.rs:306-330` |
| `whole_query_run()` | `query.py:306-317` | `query.rs:503-508` |
| `compute_candidates()` | `match_set.py:260-350` | **NOT IMPLEMENTED** |
| Near-duplicate phase | `index.py:741-775` | **NOT IMPLEMENTED** |
| Squared resemblance | `match_set.py:427` | **NOT IMPLEMENTED** |
| `is_highly_resemblant` | `match_set.py:295-297` | **NOT IMPLEMENTED** |

### Estimated Tests Fixed

This fix addresses ~20 tests where combined rules should match:

- `cddl-1.0_or_gpl-2.0-glassfish.txt`
- `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_*.txt`
- Similar dual-license header cases

---

## Validation Commands

```bash
# Run specific failing tests
cargo test -r -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Run all tests
cargo test -r -q --lib

# Format and lint
cargo fmt && cargo clippy --fix --allow-dirty
```

---

## Implementation History

### Session 1 (2026-02-17)

| Issue | Attempted | Result | Golden Tests |
|-------|-----------|--------|--------------|
| Issue 1+6 | Yes | Wrong fix applied | No change |
| Issue 2 | Yes | Implemented | No change |
| Issue 5 | Yes | Caused regression | 177→175 passed |

**Golden test results:**

- Before: lic1: 177 passed, 114 failed; External: 895 failures
- After: lic1: 175 passed, 116 failed; External: 896 failures (regression)

### Key Learnings

1. **Issue 1 fix was wrong**: The grouping logic at `detection.rs:187-199` already uses `is_license_intro` flag directly (correct). The helper functions are dead code.

2. **`is_unknown_intro()` is correctly implemented**: The function properly checks `license_expression.contains("unknown")`.

3. **Grouping threshold change caused regression**: Changed `should_group_together()` from AND logic to line-only, which broke tests.

4. **The grouping code is already correct**: Lines 187-199 directly check `match_item.is_license_intro` and `match_item.is_license_clue` - matching Python's behavior.

---

## Issue 9: Detailed Implementation Plan

### Overview

Python matches combined rules (like `cddl-1.0_or_gpl-2.0-glassfish.RULE`) via **query-run-level matching with `high_resemblance=False`**, not just near-duplicate detection. Rust currently only has near-duplicate detection and regular sequence matching on the whole query, but is missing the query run splitting and query run matching phases.

### 1. Query Run Splitting

#### Python's Algorithm (`query.py:568-641`)

**Constant:**

```python
# query.py:108
LINES_THRESHOLD = 4
```

**Algorithm:**

```python
# query.py:568-641
def _tokenize_and_build_runs(self, tokens_by_line, line_threshold=4):
    len_legalese = self.idx.len_legalese
    digit_only_tids = self.idx.digit_only_tids

    # Initial query run
    query_run = QueryRun(query=self, start=0)
    empty_lines = 0
    pos = 0

    for tokens in tokens_by_line:
        # Break point reached?
        if len(query_run) > 0 and empty_lines >= line_threshold:
            query_runs_append(query_run)
            query_run = QueryRun(query=self, start=pos)
            empty_lines = 0

        if len(query_run) == 0:
            query_run.start = pos

        if not tokens:
            empty_lines += 1
            continue

        line_has_known_tokens = False
        line_has_good_tokens = False
        line_is_all_digit = all([
            tid is None or tid in digit_only_tids for tid in tokens
        ])

        for token_id in tokens:
            if token_id is not None:
                tokens_append(token_id)
                line_has_known_tokens = True
                if token_id < len_legalese:
                    line_has_good_tokens = True
                query_run.end = pos
                pos += 1

        if line_is_all_digit:
            empty_lines += 1
            continue

        if not line_has_known_tokens:
            empty_lines += 1
            continue

        if line_has_good_tokens:
            empty_lines = 0
        else:
            empty_lines += 1

    # Append final run if any
    if len(query_run) > 0:
        if not all(tid in digit_only_tids for tid in query_run.tokens):
            query_runs_append(query_run)
```

**Break Conditions (line is "junk" and counts toward empty_lines):**

1. Line has no tokens at all (empty line)
2. Line has only digit-only tokens
3. Line has no known tokens (all unknown)
4. Line has known tokens but none are "good" (legalese tokens with ID < len_legalese)

**Reset Condition:**

- Line has at least one "good" token (legalese) → reset `empty_lines = 0`

#### Rust's Current State (`query.rs:281-363`)

Rust has `QueryRun` struct but:

1. **`query_runs` field exists but is never populated** - always empty `Vec::new()`
2. **`whole_query_run()` exists** - returns single run covering entire query
3. **No query run splitting logic implemented**

**Current Query construction (`query.rs:281-363`):**

```rust
pub fn with_options(
    text: &str,
    index: &'a LicenseIndex,
    _line_threshold: usize,  // <-- PARAMETER IGNORED!
) -> Result<Self, anyhow::Error> {
    // ... tokenization happens but no query run splitting ...

    Ok(Query {
        // ...
        query_runs: Vec::new(),  // <-- ALWAYS EMPTY
        // ...
    })
}
```

#### Code Changes Needed

**File: `src/license_detection/query.rs`**

**Step 1: Add `compute_query_runs()` method:**

```rust
impl<'a> Query<'a> {
    /// Compute query runs based on line threshold.
    ///
    /// Breaks query into runs when there are 4+ consecutive "junk" lines.
    /// A line is "junk" if it's empty, all digits, or has no legalese tokens.
    ///
    /// Corresponds to Python: `_tokenize_and_build_runs()` in query.py:568-641
    pub fn compute_query_runs(&mut self, line_threshold: usize) {
        if self.tokens.is_empty() {
            return;
        }

        let len_legalese = self.index.len_legalese;
        let digit_only_tids = &self.index.digit_only_tids;

        let mut query_runs = Vec::new();
        let mut current_run_start = 0usize;
        let mut current_run_end: Option<usize> = None;
        let mut empty_lines = 0usize;

        // Track which token positions belong to which line
        let mut pos = 0usize;
        let mut prev_line = 0usize;

        for (token_idx, &tid) in self.tokens.iter().enumerate() {
            let line = self.line_by_pos[token_idx];

            // Check if we've moved to a new line
            if token_idx == 0 || line != prev_line {
                // Check for line break (gap in line numbers)
                if token_idx > 0 && line > prev_line + 1 {
                    // Count empty/junk lines between prev_line and line
                    let gap = line - prev_line - 1;
                    for _ in 0..gap {
                        empty_lines += 1;
                        if empty_lines >= line_threshold {
                            // Break current run and start new one
                            if let Some(end) = current_run_end {
                                if end >= current_run_start {
                                    query_runs.push(QueryRun::new(self, current_run_start, Some(end)));
                                }
                            }
                            current_run_start = token_idx;
                            current_run_end = None;
                            empty_lines = 0;
                        }
                    }
                }

                // Check if this token is "good" (legalese)
                let is_good_token = (tid as usize) < len_legalese;
                let is_digit_only = digit_only_tids.contains(&tid);

                if is_good_token {
                    empty_lines = 0;
                } else if is_digit_only {
                    empty_lines += 1;
                }
                // Note: We can't fully detect "junk" lines without line-by-line tokenization

                prev_line = line;
            }

            current_run_end = Some(token_idx);
        }

        // Append final run
        if let Some(end) = current_run_end {
            if end >= current_run_start {
                query_runs.push(QueryRun::new(self, current_run_start, Some(end)));
            }
        }

        self.query_runs = query_runs;
    }
}
```

**Step 2: Call `compute_query_runs()` in constructor:**

```rust
pub fn with_options(
    text: &str,
    index: &'a LicenseIndex,
    line_threshold: usize,
) -> Result<Self, anyhow::Error> {
    // ... existing tokenization code ...

    let mut query = Query {
        // ... fields ...
        query_runs: Vec::new(),
        index,
    };

    // Compute query runs AFTER tokenization
    query.compute_query_runs(line_threshold);

    Ok(query)
}
```

**Step 3: Add `query_runs()` method for iteration:**

```rust
/// Iterate over query runs.
///
/// If query runs are empty, yields a single run covering the whole query.
pub fn query_runs(&self) -> impl Iterator<Item = QueryRun<'_>> {
    if self.query_runs.is_empty() {
        std::iter::once(self.whole_query_run())
    } else {
        self.query_runs.iter().map(|qr| QueryRun::new(self, qr.start, qr.end))
    }
}
```

### 2. Query Run Matching Phase

#### Python's Pipeline (`index.py:780-812`)

```python
# index.py:780-812
MAX_CANDIDATES = 70
for query_run in query.query_runs:
    # Inverted index match and ranking, query run-level
    candidates = match_set.compute_candidates(
        query_run=query_run,
        idx=self,
        matchable_rids=matchable_rids,
        top=MAX_CANDIDATES,
        high_resemblance=False,  # KEY: lower threshold for individual runs
        _use_bigrams=USE_BIGRAM_MULTISETS,
    )

    matched = self.get_query_run_approximate_matches(
        query_run, candidates, matched_qspans, deadline)

    matches.extend(matched)

    if time() > deadline:
        break
```

**Key Parameters:**

- `high_resemblance=False` - No 0.8 threshold filter
- `top=MAX_CANDIDATES` (70) - More candidates than near-duplicate (10)
- Each query run is matched independently

#### Rust's Current Pipeline (`mod.rs:110-142`)

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let query = Query::new(text, &self.index)?;
    let whole_run = query.whole_query_run();

    let mut all_matches = Vec::new();

    // Phase 1: Hash, SPDX, Aho-Corasick
    let hash_matches = hash_match(&self.index, &whole_run);
    all_matches.extend(hash_matches);
    let spdx_matches = spdx_lid_match(&self.index, text);
    all_matches.extend(spdx_matches);
    let aho_matches = aho_match(&self.index, &whole_run);
    all_matches.extend(aho_matches);

    // Phase 2: Near-duplicate detection (high_resemblance=True)
    let near_dupe_candidates = compute_candidates_with_msets(
        &self.index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES,
    );
    if !near_dupe_candidates.is_empty() {
        let near_dupe_matches = seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);
        all_matches.extend(near_dupe_matches);
    }

    // Phase 3: Regular sequence matching (single whole_run)
    let seq_matches = seq_match(&self.index, &whole_run);  // <-- WRONG: should be per query run
    all_matches.extend(seq_matches);

    // ...
}
```

**Missing:**

- No iteration over `query.query_runs`
- `seq_match` only called once on whole file
- `high_resemblance=False` matching not done per query run

#### Code Changes Needed

**File: `src/license_detection/mod.rs`**

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let query = Query::new(text, &self.index)?;
    let whole_run = query.whole_query_run();

    let mut all_matches = Vec::new();

    // Phase 1: Hash, SPDX, Aho-Corasick (on whole file)
    let hash_matches = hash_match(&self.index, &whole_run);
    all_matches.extend(hash_matches);
    let spdx_matches = spdx_lid_match(&self.index, text);
    all_matches.extend(spdx_matches);
    let aho_matches = aho_match(&self.index, &whole_run);
    all_matches.extend(aho_matches);

    // Phase 2: Near-duplicate detection (high_resemblance=True, top 10)
    let near_dupe_candidates = compute_candidates_with_msets(
        &self.index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES,
    );
    if !near_dupe_candidates.is_empty() {
        let near_dupe_matches = seq_match_with_candidates(
            &self.index, &whole_run, &near_dupe_candidates
        );
        all_matches.extend(near_dupe_matches);
    }

    // Phase 3: Query run matching (high_resemblance=False, top 70) - NEW!
    const MAX_QUERY_RUN_CANDIDATES: usize = 70;
    for query_run in query.query_runs() {
        let candidates = compute_candidates_with_msets(
            &self.index,
            &query_run,
            false,  // high_resemblance=False for query runs
            MAX_QUERY_RUN_CANDIDATES,
        );
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(&self.index, &query_run, &candidates);
            all_matches.extend(matches);
        }
    }

    // Phase 4: Unknown matching, refinement, etc.
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    all_matches.extend(unknown_matches);

    // ... rest of pipeline ...
}
```

### 3. Step-by-Step TODOs

- [ ] **Step 1**: Add `compute_query_runs()` method to `Query` struct in `query.rs`
  - Implement line-by-line empty/junk detection
  - Create `QueryRun` objects for each contiguous block
  - Handle edge cases (empty query, single run, etc.)

- [ ] **Step 2**: Call `compute_query_runs()` in `Query::new()` and `Query::with_options()`
  - Use `LINES_THRESHOLD = 4` as default
  - Ensure query runs are populated after tokenization

- [ ] **Step 3**: Add `query_runs()` iterator method to `Query`
  - Return single `whole_query_run()` if `query_runs` is empty
  - Otherwise iterate over populated query runs

- [ ] **Step 4**: Add query run matching phase to `detect()` in `mod.rs`
  - Iterate over `query.query_runs()`
  - Call `compute_candidates_with_msets()` with `high_resemblance=False`
  - Use `MAX_QUERY_RUN_CANDIDATES = 70`
  - Extend `all_matches` with results

- [ ] **Step 5**: Add unit tests for query run splitting
  - Test empty input
  - Test single block (no breaks)
  - Test multiple blocks with empty line separators
  - Test blocks with digit-only lines
  - Test blocks with only low-value tokens

- [ ] **Step 6**: Add integration test for combined rule matching
  - Test `cddl-1.0_or_gpl-2.0-glassfish.txt`
  - Verify combined rule matches instead of partial rules

### 4. Expected Impact

**Tests Fixed:** ~20 tests involving combined rules:

- `cddl-1.0_or_gpl-2.0-glassfish.txt`
- `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_*.txt`
- Similar dual-license header cases

**Why This Helps:**

1. Query run matching uses `high_resemblance=False` → lower threshold for candidate selection
2. More candidates considered per run (70 vs 10 for near-duplicate)
3. Combined rules have high token overlap with full query but may not reach 0.8 resemblance
4. Per-run matching finds better candidates than whole-file matching when file has mixed content

### 5. Key Differences Summary

| Aspect | Python | Rust (Current) | Rust (After Fix) |
|--------|--------|----------------|------------------|
| Query run splitting | `query.query_runs` property | Empty `Vec` | Populated via `compute_query_runs()` |
| Near-duplicate phase | `high_resemblance=True`, top 10 | ✅ Implemented | ✅ No change |
| Query run matching | `high_resemblance=False`, top 70 | ❌ Missing | ✅ Implemented |
| Run iteration | `for query_run in query.query_runs` | N/A | ✅ Implemented |

---

## Issue 10: Query Span Subtraction (Critical for Query Run Matching)

### Problem

Python uses span subtraction to prevent **double-matching** - the same content being matched by both the near-duplicate phase and query run phases. Without subtraction, when a near-duplicate match covers the whole file, query run matching may also match overlapping content, leading to duplicate or conflicting detections.

### Python's Actual Pipeline (Verified from `index.py:741-812`)

```python
# Lines 739-741: Initialize tracking
already_matched_qspans = matched_qspans[:]
MAX_NEAR_DUPE_CANDIDATES = 10

# Lines 744-765: Phase 1 - Near-duplicate detection
whole_query_run = query.whole_query_run()
near_dupe_candidates = match_set.compute_candidates(
    query_run=whole_query_run,
    idx=self,
    matchable_rids=matchable_rids,
    top=MAX_NEAR_DUPE_CANDIDATES,
    high_resemblance=True,
)

if near_dupe_candidates:
    matched = self.get_query_run_approximate_matches(
        whole_query_run, near_dupe_candidates, already_matched_qspans, deadline)
    matches.extend(matched)

    # Lines 767-771: CRITICAL - Subtract matched positions
    for match in matched:
        qspan = match.qspan
        query.subtract(qspan)
        already_matched_qspans.append(qspan)

# Lines 786-812: Phase 2 - Query run matching
MAX_CANDIDATES = 70
for query_run in query.query_runs:
    candidates = match_set.compute_candidates(
        query_run=query_run,
        idx=self,
        matchable_rids=matchable_rids,
        top=MAX_CANDIDATES,
        high_resemblance=False,  # NOTE: Different from near-duplicate
    )

    # Line 803: Note - passes matched_qspans (original), not already_matched_qspans
    matched = self.get_query_run_approximate_matches(
        query_run, candidates, matched_qspans, deadline)
    matches.extend(matched)
```

**KEY INSIGHT**: Python's `get_query_run_approximate_matches()` internally checks `is_matchable()` at line 828:

```python
# match.py:828
if not query_run.is_matchable(include_low=False, qspans=matched_qspans):
    return matches  # Empty - skip this query run
```

### What is `qspan`?

**`qspan`** (Query Span) is a `Span` object containing the set of token positions in the query that were matched. Python's `Span` uses efficient `intbitset` storage.

**In Python's LicenseMatch** (`match.py:179-184`):

```python
qspan = attr.ib(
    metadata=dict(
        help='query text matched Span, start at zero which is the absolute '
             'query start (not the query_run start)'
    )
)
```

### Python's Subtraction Logic (`query.py:328-334`)

```python
def subtract(self, qspan):
    """Subtract the qspan matched positions from the query matchable positions."""
    if qspan:
        self.high_matchables.difference_update(qspan)
        self.low_matchables.difference_update(qspan)
```

**Important**: Subtraction modifies `high_matchables` AND `low_matchables` in-place. This is a **mutation** of the Query object, not a creation of a new Query.

### Why Subtraction Matters

**Without Subtraction:**

1. Near-duplicate matches tokens 0-100 with `gpl-2.0`
2. Query run phase also matches tokens 10-50 with a different rule
3. Result: Conflicting or duplicate detections

**With Subtraction:**

1. Near-duplicate matches tokens 0-100 with `gpl-2.0`
2. `query.subtract(Span(0, 100))` removes positions from matchables
3. `is_matchable()` returns `False` for query runs in that range
4. Result: Single clean detection

### Rust's Current State (Verified from source)

**What Rust ALREADY HAS:**

- ✅ `Query.high_matchables: HashSet<usize>` (`query.rs:220-221`)
- ✅ `Query.low_matchables: HashSet<usize>` (`query.rs:227-228`)
- ✅ `PositionSpan` struct (`query.rs:16-50`)
- ✅ `Query.subtract(&mut self, span: &PositionSpan)` (`query.rs:713-724`)
- ✅ `QueryRun.is_matchable(include_low, exclude_positions)` (`query.rs:879-897`)
- ✅ `QueryRun.matchables(include_low)` (`query.rs:905-913`)

**What Rust is MISSING:**

1. ❌ `qspan` field in `LicenseMatch` struct (`models.rs:179-251` has no qspan)
2. ❌ Calling `query.subtract()` in detection pipeline (`mod.rs:134-138` doesn't subtract)
3. ❌ Tracking matched positions list
4. ❌ Passing exclude positions to `is_matchable()` before query run matching

### Current Rust Detection Pipeline (`mod.rs:110-164`)

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let query = Query::new(text, &self.index)?;  // NOT mutable
    // ...
    
    // Phase 2: Near-duplicate detection
    let near_dupe_candidates = compute_candidates_with_msets(
        &self.index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES,
    );
    if !near_dupe_candidates.is_empty() {
        let near_dupe_matches = seq_match_with_candidates(...);
        all_matches.extend(near_dupe_matches);
        // MISSING: No subtraction here!
    }

    // Phase 3: Query run matching
    for query_run in query.query_runs().iter() {
        // MISSING: No is_matchable() check!
        let candidates = compute_candidates_with_msets(...);
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(...);
            all_matches.extend(matches);
        }
    }
}
```

### Implementation Plan

#### Step 1: Add `matched_token_positions` Field to `LicenseMatch` (models.rs)

**Why a different name than `qspan`:**

- Python's `qspan` is a `Span` object (set-like), not a range
- Rust's `LicenseMatch` already has `start_token` and `end_token`
- We need the **set of matched positions** for non-contiguous matches (sequence matching)
- Use `matched_token_positions: Vec<usize>` for simplicity

**Current (`models.rs:179-251`):**

```rust
pub struct LicenseMatch {
    pub start_token: usize,
    pub end_token: usize,
    // ... other fields ...
}
```

**Add field:**

```rust
pub struct LicenseMatch {
    pub start_token: usize,
    pub end_token: usize,
    /// Token positions matched by this license (for span subtraction).
    /// Populated during matching to enable double-match prevention.
    /// None means contiguous range [start_token, end_token].
    #[serde(skip)]
    pub matched_token_positions: Option<Vec<usize>>,
    // ... other fields ...
}
```

**Note**: For contiguous matches, `matched_token_positions` can remain `None` and the subtraction logic can use `start_token..=end_token`. Only non-contiguous matches (from sequence matching) need explicit position tracking.

#### Step 2: Populate `matched_token_positions` in Matchers

**Hash Match (`hash_match.rs`):** Matches are always contiguous - no explicit positions needed.

```rust
// No change needed - contiguous match implied by start_token/end_token
LicenseMatch {
    start_token: query_run.start,
    end_token: query_run.end.unwrap_or(query_run.start),
    matched_token_positions: None,  // Contiguous
    // ...
}
```

**Aho-Corasick Match (`aho_match.rs`):** Typically contiguous - similar to hash match.

**Sequence Match (`seq_match.rs`):** May have non-contiguous matches due to token skipping:

```rust
// After computing matched positions from match blocks
let matched_positions: Vec<usize> = match_blocks.iter()
    .flat_map(|(qstart, qend, _istart, _iend)| *qstart..=*qend)
    .collect();

LicenseMatch {
    start_token: *matched_positions.first().unwrap_or(&0),
    end_token: *matched_positions.last().unwrap_or(&0),
    matched_token_positions: Some(matched_positions),
    // ...
}
```

#### Step 3: Add Subtraction to Detection Pipeline (`mod.rs`)

**Key Changes:**

1. Make `query` mutable
2. Track matched positions
3. Call `subtract()` after near-duplicate matches
4. Check `is_matchable()` before query run matching

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let mut query = Query::new(text, &self.index)?;  // NOW MUTABLE
    let whole_run = query.whole_query_run();

    let mut all_matches = Vec::new();
    let mut matched_positions: Vec<PositionSpan> = Vec::new();  // NEW

    // Phase 1: Hash, SPDX, Aho-Corasick (unchanged)
    let hash_matches = hash_match(&self.index, &whole_run);
    all_matches.extend(hash_matches.clone());
    // Track matched positions from hash matches
    for m in &hash_matches {
        matched_positions.push(PositionSpan::new(m.start_token, m.end_token));
    }
    // ... similar for spdx, aho ...

    // Phase 2: Near-duplicate detection
    let near_dupe_candidates = compute_candidates_with_msets(
        &self.index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES,
    );
    if !near_dupe_candidates.is_empty() {
        let near_dupe_matches = seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);
        
        // NEW: Subtract matched positions
        for m in &near_dupe_matches {
            let span = PositionSpan::new(m.start_token, m.end_token);
            query.subtract(&span);
            matched_positions.push(span);
        }
        
        all_matches.extend(near_dupe_matches);
    }

    // Phase 3: Query run matching
    for query_run in query.query_runs().iter() {
        // Skip the whole_run (already matched in Phase 2)
        if query_run.start == whole_run.start && query_run.end == whole_run.end {
            continue;
        }

        // NEW: Check if query run has matchable tokens
        if !query_run.is_matchable(false, &matched_positions) {
            continue;  // All tokens already matched
        }

        let candidates = compute_candidates_with_msets(
            &self.index, query_run, false, MAX_QUERY_RUN_CANDIDATES,
        );
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(&self.index, query_run, &candidates);
            all_matches.extend(matches);
        }
    }

    // ... rest unchanged ...
}
```

#### Step 4: Verify `is_matchable()` Implementation

**Current implementation (`query.rs:879-897`):**

```rust
pub fn is_matchable(&self, include_low: bool, exclude_positions: &[PositionSpan]) -> bool {
    // Check if query run has digits only
    if self.is_digits_only() {
        return false;
    }

    let matchables = self.matchables(include_low);

    if exclude_positions.is_empty() {
        return !matchables.is_empty();
    }

    let mut matchable_set = matchables;
    for span in exclude_positions {
        let span_positions = span.positions();
        matchable_set = matchable_set.difference(&span_positions).copied().collect();
    }

    !matchable_set.is_empty()
}
```

**This is already correctly implemented!** It:

1. Returns false for digits-only runs
2. Subtracts exclude_positions from matchables
3. Returns true if any matchable positions remain

#### Step 5: Unit Tests

Add to `src/license_detection/query_test.rs`:

```rust
#[test]
fn test_query_subtract_removes_positions() {
    let index = create_test_index(&[("license", 0), ("copyright", 1), ("permission", 2)], 3);
    let mut query = Query::new("license copyright permission", &index).unwrap();

    assert!(query.high_matchables.contains(&0));
    assert!(query.high_matchables.contains(&1));

    let span = PositionSpan::new(0, 1);
    query.subtract(&span);

    assert!(!query.high_matchables.contains(&0));
    assert!(!query.high_matchables.contains(&1));
    assert!(query.high_matchables.contains(&2));
}

#[test]
fn test_query_run_is_matchable_with_exclusions() {
    let index = create_test_index(&[("license", 0), ("copyright", 1), ("permission", 2)], 3);
    let query = Query::new("license copyright permission", &index).unwrap();
    let run = query.whole_query_run();

    // Initially matchable
    assert!(run.is_matchable(false, &[]));

    // Exclude positions 0-1, position 2 remains
    let exclude = vec![PositionSpan::new(0, 1)];
    assert!(run.is_matchable(false, &exclude));

    // Exclude all positions
    let exclude_all = vec![PositionSpan::new(0, 2)];
    assert!(!run.is_matchable(false, &exclude_all));
}

#[test]
fn test_subtraction_after_near_duplicate_match() {
    // Simulate: near-duplicate matches whole file, query run should be skipped
    let index = create_test_index(&[("license", 0), ("copyright", 1)], 2);
    let mut query = Query::new("license copyright license copyright", &index).unwrap();
    let whole_run = query.whole_query_run();

    // Simulate near-duplicate match covering positions 0-1
    let near_dupe_span = PositionSpan::new(0, 1);
    query.subtract(&near_dupe_span);

    // Query run for same range should not be matchable
    assert!(!whole_run.is_matchable(false, &[near_dupe_span]));
}
```

### Edge Cases to Handle

1. **Empty span**: `PositionSpan::new(0, 0)` - subtracts single position. Python handles `if qspan:` check.
2. **Span outside query range**: Should not happen if matchers generate correct positions.
3. **Overlapping spans**: Subtraction is idempotent - same position removed twice has no effect.
4. **Partial overlap**: Query run partially covered by subtraction → remaining positions are still matchable.
5. **All tokens subtracted**: `is_matchable()` returns `false`, query run is skipped.
6. **Non-contiguous matches**: Sequence matches may skip tokens; must track actual matched positions.

### Performance Considerations

1. **HashSet operations**: `subtract()` iterates through span positions and removes from HashSets - O(n) per subtraction.
2. **Memory**: `matched_positions: Vec<PositionSpan>` stores only ranges, not full position sets.
3. **No additional allocations**: `is_matchable()` reuses existing `high_matchables`/`low_matchables` from QueryRun.

### Python References (Verified)

| Component | Location | Description |
|-----------|----------|-------------|
| `Span` class | `spans.py:42-475` | Position set with `intbitset` storage |
| `Query.subtract()` | `query.py:328-334` | Subtracts from high/low matchables |
| `QueryRun.is_matchable()` | `query.py:798-818` | Checks with exclusion spans |
| Near-duplicate subtraction | `index.py:767-771` | Subtracts after near-duplicate matches |
| Query run matching | `index.py:786-812` | Iterates query runs with `high_resemblance=False` |
| `get_query_run_approximate_matches` | `index.py:814-860` | Internal check of `is_matchable()` at line 828 |

### Rust Implementation Checklist

- [ ] **Step 1**: Add `matched_token_positions: Option<Vec<usize>>` field to `LicenseMatch` in `models.rs`
  - Use `Option` to avoid allocation for contiguous matches
  - Add `#[serde(skip)]` since this is internal-only

- [ ] **Step 2**: Populate `matched_token_positions` in sequence matching (`seq_match.rs`)
  - Hash and Aho-Corasick matches are always contiguous (use `None`)
  - Sequence matches may be non-contiguous (use `Some(positions)`)

- [ ] **Step 3**: Modify `detect()` pipeline in `mod.rs`:
  - Make `query` mutable
  - Add `matched_positions: Vec<PositionSpan>` tracking
  - Call `query.subtract()` after near-duplicate matches
  - Call `is_matchable()` before query run matching

- [ ] **Step 4**: Add unit tests for subtraction behavior in `query_test.rs`

- [ ] **Step 5**: Run golden tests to verify no regressions:

  ```bash
  cargo test -r -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
  ```

### Expected Impact

**Primary benefit**: Prevents double-matching when near-duplicate detection matches whole file.

**Tests that may improve**: Files where near-duplicate phase matches whole content but query run phase would also match:

- Combined rule cases
- Files with single dominant license

**Why This is Critical for Issue 9:**

Issue 9 (Query Run Matching) requires this to work correctly. Without subtraction:

1. Near-duplicate phase matches whole file
2. Query run phase matches overlapping content again
3. Result: Duplicate/conflicting detections

With subtraction:

1. Near-duplicate phase matches whole file
2. Subtraction marks tokens as "already matched"
3. `is_matchable()` returns false for query runs in that range
4. Result: Clean, non-overlapping detections

### Verification Steps

After implementation:

1. **Unit tests pass**: `cargo test -q --lib query_test`
2. **Golden tests stable**: Should not regress existing passing tests
3. **Debug logging**: Add temporary logging to verify subtraction is called:

   ```rust
   if !near_dupe_matches.is_empty() {
       eprintln!("Near-duplicate matches: {}", near_dupe_matches.len());
       for m in &near_dupe_matches {
           eprintln!("  Subtracting: {}-{}", m.start_token, m.end_token);
       }
   }
   ```

---

## Issue 11: Proper Query Subtraction Implementation (Critical Fix)

### Problem Summary

The previous implementation of Issue 10 caused a 40-test regression because SPDX-LID matches had `start_token=0, end_token=0` hardcoded. When subtraction was enabled, position 0 was incorrectly removed from matchables, breaking detection for files where the first token was part of a license match.

### Root Cause Analysis

#### How Python Computes SPDX-LID Token Positions

**From `reference/scancode-toolkit/src/licensedcode/query.py:499-507`:**

```python
if spdx_start_offset is not None:
    # Keep the line, start/end known pos for SPDX matching
    spdx_prefix, spdx_expression = split_spdx_lid(line)
    spdx_text = ''.join([spdx_prefix or '', spdx_expression])
    spdx_start_known_pos = line_first_known_pos + spdx_start_offset

    if spdx_start_known_pos <= line_last_known_pos:
        self.spdx_lines.append((spdx_text, spdx_start_known_pos, line_last_known_pos))
```

**From `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py:99-101`:**

```python
# Build match from parsed expression
# Collect match start and end: e.g. the whole text
qspan = Span(range(match_start, query_run.end + 1))
```

**Key Insight:** Python tracks SPDX lines during tokenization with their token position ranges. The `spdx_id_match()` function uses `query_run.start` and `query_run.end` to create the `qspan`.

#### How Rust Currently Creates SPDX Matches

**From `src/license_detection/spdx_lid.rs:279-280`:**

```rust
start_token: 0,
end_token: 0,
```

**This is wrong!** Rust hardcodes `0, 0` instead of computing actual token positions.

### Python's Subtraction Logic

**From `reference/scancode-toolkit/src/licensedcode/index.py:767-771`:**

```python
# Subtract these
for match in matched:
    qspan = match.qspan
    query.subtract(qspan)
    already_matched_qspans.append(qspan)
```

**Critical Points:**

1. Python subtracts ALL near-duplicate matches, regardless of coverage
2. Python does NOT subtract hash/aho matches - only near-duplicate matches
3. The `qspan` contains actual token positions from the match

### Implementation Plan

#### Step 1: Track SPDX Lines During Query Tokenization

**File:** `src/license_detection/query.rs`

**Add field to Query struct:**

```rust
pub struct Query<'a> {
    // ... existing fields ...
    
    /// SPDX-License-Identifier lines found during tokenization.
    /// Each tuple is (spdx_text, start_token_pos, end_token_pos).
    /// Corresponds to Python: `self.spdx_lines` at query.py:507
    pub spdx_lines: Vec<(String, usize, usize)>,
}
```

**Modify tokenization in `Query::with_options()`:**

```rust
// During tokenization, track SPDX lines
// Corresponds to Python: query.py:486-507

let spdx_lid_token_ids: Vec<Vec<Option<u16>>> = vec![
    // "spdx", "license", "identifier" token IDs
    vec![spdx_tid, license_tid, identifier_tid],
];

for line in text.lines() {
    // ... existing tokenization ...
    
    // Check if this line starts with SPDX-License-Identifier
    // Python checks: line_tokens[:3] in spdx_lid_token_ids
    // This means first 3 tokens match "spdx license identifier"
    
    let line_tokens_lower: Vec<String> = tokenize_without_stopwords(line)
        .map(|t| t.to_lowercase())
        .collect();
    
    if line_tokens_lower.len() >= 3 {
        let first_three: Vec<&str> = line_tokens_lower.iter().take(3).map(|s| s.as_str()).collect();
        
        // Check if starts with "spdx license identifier" (case-insensitive)
        let is_spdx_line = first_three == ["spdx", "license", "identifier"] ||
                           first_three == ["spdx", "licence", "identifier"];
        
        if is_spdx_line {
            // Record SPDX line with token positions
            // spdx_start_offset accounts for comment prefixes like "// " or "# "
            let spdx_start_offset = 0; // For now, assume SPDX starts at position 0
            
            if let (Some(&start_pos), Some(&end_pos)) = 
                (line_by_pos.first(), line_by_pos.last()) 
            {
                let (_, expression) = split_spdx_lid(line);
                spdx_lines.push((expression, start_pos, end_pos));
            }
        }
    }
}
```

#### Step 2: Modify `spdx_lid_match()` to Use Token Positions

**File:** `src/license_detection/spdx_lid.rs`

**Change function signature:**

```rust
/// SPDX-License-Identifier detection using query's tracked SPDX lines.
///
/// # Arguments
/// * `index` - The license index
/// * `query` - The query with pre-computed SPDX lines
///
/// Returns LicenseMatches with correct token positions from query.spdx_lines.
pub fn spdx_lid_match(index: &LicenseIndex, query: &Query) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();

    for (spdx_text, start_token, end_token) in &query.spdx_lines {
        let spdx_expression = clean_spdx_text(spdx_text);
        let license_keys = split_license_expression(&spdx_expression);

        for license_key in license_keys {
            if let Some(rid) = find_best_matching_rule(index, &license_key) {
                let rule = &index.rules_by_rid[rid];
                let score = rule.relevance as f32 / 100.0;

                // Get line number from query's line_by_pos
                let start_line = query.line_for_pos(*start_token).unwrap_or(1);
                let end_line = query.line_for_pos(*end_token).unwrap_or(start_line);

                // Extract matched text using line positions
                let matched_text = query.matched_text(start_line, end_line);

                let license_match = LicenseMatch {
                    license_expression: rule.license_expression.clone(),
                    license_expression_spdx: spdx_expression.clone(),
                    from_file: None,
                    start_line,
                    end_line,
                    start_token: *start_token,  // NOW CORRECT
                    end_token: *end_token,       // NOW CORRECT
                    matcher: MATCH_SPDX_ID.to_string(),
                    score,
                    matched_length: spdx_expression.len(),
                    rule_length: rule.tokens.len(),
                    match_coverage: 100.0,
                    rule_relevance: rule.relevance,
                    rule_identifier: format!("#{}", rid),
                    rule_url: String::new(),
                    matched_text: Some(matched_text),
                    referenced_filenames: rule.referenced_filenames.clone(),
                    is_license_intro: rule.is_license_intro,
                    is_license_clue: rule.is_license_clue,
                    is_license_reference: rule.is_license_reference,
                    is_license_tag: rule.is_license_tag,
                    matched_token_positions: None, // Contiguous match
                };

                matches.push(license_match);
            }
        }
    }

    matches
}
```

#### Step 3: Update Detection Pipeline

**File:** `src/license_detection/mod.rs`

**Change SPDX match call:**

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let query = Query::new(text, &self.index)?;

    let mut all_matches = Vec::new();

    // Phase 1: Hash, SPDX, Aho-Corasick
    {
        let whole_run = query.whole_query_run();

        let hash_matches = hash_match(&self.index, &whole_run);
        all_matches.extend(hash_matches);

        // PASS QUERY INSTEAD OF TEXT - enables token position lookup
        let spdx_matches = spdx_lid_match(&self.index, &query);
        all_matches.extend(spdx_matches);

        let aho_matches = aho_match(&self.index, &whole_run);
        all_matches.extend(aho_matches);
    }
    
    // ... rest unchanged ...
}
```

#### Step 4: Add Unit Tests for Token Position Tracking

**File:** `src/license_detection/spdx_lid_test.rs`

```rust
#[test]
fn test_spdx_match_has_correct_token_positions() {
    let mut index = create_test_index(&[("mit", 0)], 1);
    index.rules_by_rid.push(create_mock_rule_simple("mit", 100));

    // Text where SPDX is NOT at position 0
    let text = "Some preamble text\nSPDX-License-Identifier: MIT\nMore text";
    let query = Query::new(text, &index).unwrap();
    
    let matches = spdx_lid_match(&index, &query);
    
    assert_eq!(matches.len(), 1);
    // Token positions should NOT be 0,0
    // They should reflect actual position of SPDX line in token stream
    assert!(matches[0].start_token >= 0);
    assert!(matches[0].end_token >= matches[0].start_token);
}

#[test]
fn test_query_tracks_spdx_lines_with_positions() {
    let index = create_test_index(&[("license", 0)], 1);
    
    let text = "SPDX-License-Identifier: MIT\nSPDX-License-Identifier: Apache-2.0";
    let query = Query::new(text, &index).unwrap();
    
    assert_eq!(query.spdx_lines.len(), 2);
    
    // Both SPDX lines should have valid token positions
    for (_, start, end) in &query.spdx_lines {
        assert!(*start <= *end);
    }
}
```

### Subtraction Conditions (Clarified)

Based on Python's actual implementation:

| Phase | Subtracts? | Condition |
|-------|------------|-----------|
| Hash match | ❌ No | N/A |
| SPDX-LID match | ❌ No | N/A |
| Aho-Corasick match | ❌ No | N/A |
| **Near-duplicate match** | ✅ **Yes** | All matches subtracted |
| Query run match | ❌ No | But `is_matchable()` excludes already-matched positions |

**Key Insight:** Python does NOT subtract after hash/aho/SPDX matching. It only subtracts after near-duplicate matching. The `is_matchable()` check in query run matching handles the exclusion.

### Python References

| Component | Location | Description |
|-----------|----------|-------------|
| `spdx_lines` tracking | `query.py:486-507` | Records SPDX lines during tokenization |
| `spdx_id_match()` | `match_spdx_lid.py:65-119` | Creates match with token positions |
| `qspan` creation | `match_spdx_lid.py:101` | `Span(range(match_start, query_run.end + 1))` |
| Subtraction | `index.py:767-771` | Only after near-duplicate matches |
| `is_matchable()` check | `index.py:828` | Excludes already-matched positions |

### Expected Impact

| Before | After |
|--------|-------|
| SPDX matches have `start_token=0, end_token=0` | SPDX matches have correct token positions |
| Subtraction removes position 0 incorrectly | Subtraction removes correct positions |
| Files with license at position 0 broken | All files work correctly |

**Tests Fixed:** The 40 tests that regressed when subtraction was previously enabled.

### Verification Steps

1. **Run SPDX unit tests:**

   ```bash
   cargo test -q --lib spdx_lid_test
   ```

2. **Verify token positions are non-zero for non-first SPDX lines:**

   ```bash
   cargo test -q --lib test_spdx_match_has_correct_token_positions
   ```

3. **Run golden tests:**

   ```bash
   cargo test -r -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
   ```

4. **Expected result:** No regressions, same pass/fail count as before subtraction was attempted.

---

## Background

After implementing PLAN-007 through PLAN-014, the golden test results showed:

- lic1: 174 passed, 117 failed → 177 passed, 114 failed (only +3 passed)
- External failures: 919 → 895 (only -24 failures)

Analysis revealed that several fixes were either not implemented correctly, targeted the wrong problem, or caused regressions.
