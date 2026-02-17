# PLAN-015: Consolidated License Detection Fixes

## Status: In Progress - Session 3

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

**Near-duplicate detection implemented but no improvement because:**

The combined rule's resemblance (0.2333) is below the 0.8 threshold. Python uses a **different approach**:

| Aspect | Python | Rust |
|--------|--------|------|
| Query runs | Splits query into runs | Not fully implemented |
| Near-duplicate | `high_resemblance=True` on whole file | Same |
| **Query run matching** | `high_resemblance=False` on each run | **Missing** |

### Remaining Issues (~104 failures)

1. **Query run matching not implemented**: Python splits queries into runs and matches each with `high_resemblance=False`
2. **P6 not implemented**: `has_unknown_intro_before_detection()` post-loop logic
3. **Other missing filters**: `filter_matches_missing_required_phrases()`, `filter_spurious_matches()`, `filter_too_short_matches()`

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

## Background

After implementing PLAN-007 through PLAN-014, the golden test results showed:
- lic1: 174 passed, 117 failed → 177 passed, 114 failed (only +3 passed)
- External failures: 919 → 895 (only -24 failures)

Analysis revealed that several fixes were either not implemented correctly, targeted the wrong problem, or caused regressions.
