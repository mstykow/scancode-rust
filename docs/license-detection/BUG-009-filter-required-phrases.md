# BUG-009: filter_matches_missing_required_phrases Regressions

## Status: PARTIALLY FIXED - Stopword filtering implemented, but other issues remain

## Problem

After implementing `filter_matches_missing_required_phrases`, golden tests regressed:

- lic1: 228→213 (-15 passed)
- lic2: 776→746 (-30 passed)
- lic3: 251→243 (-8 passed)
- lic4: 281→276 (-5 passed)
- external: 1882→1863 (-19 passed)

## Fix Applied: Stopword Filtering in Tokenizer

**Commit**: Stopword filtering added to `required_phrase_tokenizer()`

**Results After Fix**:

| Suite | Baseline | After Fix | Delta |
|-------|----------|-----------|-------|
| lic1 | 228/63 | 221/70 | -7 |
| lic2 | 776/77 | 750/103 | -26 |
| lic3 | 251/41 | 245/47 | -6 |
| lic4 | 281/69 | 282/68 | +1 |
| external | 1882/685 | 1867/700 | -15 |

**Improvement**: The `gpl-2.0_9.txt` case now passes. The stopword filtering fix was correct.

## Remaining Issues

The fix improved results but other regressions remain. Investigation found **three separate issues**:

---

### Issue 1: Missing `is_license_notice` and `is_license_text` fields in `LicenseMatch`

**Status**: Root cause identified, fix planned, VERIFIED

**Confidence**: HIGH for correctness, MEDIUM for golden test impact

#### Problem

The `LicenseMatch` struct in Rust is **missing two fields**:

- **Missing**: `is_license_notice`, `is_license_text`
- Has: `is_license_intro`, `is_license_clue`, `is_license_reference`, `is_license_tag`

The `Rule` struct has both fields, but matchers don't copy them to `LicenseMatch`.

#### Findings

**Missing fields comparison:**

| Field | `Rule` struct | `LicenseMatch` struct |
|-------|--------------|----------------------|
| `is_license_text` | ✅ Present (line 81) | ❌ MISSING |
| `is_license_notice` | ✅ Present (line 84) | ❌ MISSING |
| `is_license_reference` | ✅ Present (line 87) | ✅ Present |
| `is_license_tag` | ✅ Present (line 90) | ✅ Present |
| `is_license_intro` | ✅ Present (line 93) | ✅ Present |
| `is_license_clue` | ✅ Present (line 96) | ✅ Present |

**Where `LicenseMatch` is created (verified line numbers):**

| File | Lines | Status |
|------|-------|--------|
| `hash_match.rs` | 117-120 | ✅ Verified |
| `aho_match.rs` | 178-181 | ✅ Verified |
| `seq_match.rs` | 748-751, 880-881 | ✅ Verified |
| `spdx_lid.rs` | 275-278 | ✅ Verified |
| `unknown_match.rs` | 321-324 | ⚠️ Corrected from 303 |
| `detection.rs` | 1174-1177, 1360-1363, 1476-1479 | ✅ Verified |
| `match_refine.rs` | 1329-1332, 1364-1367, 2540-2541 | ✅ Verified |

**Additional creation sites found in test code (~50+):**

- `models.rs`: ~15 test helper creations
- `detection.rs`: ~10 test helper creations
- `unknown_match.rs`: 2 test creations
- `match_refine.rs`: 2 additional creations

#### JSON Output Impact

**Python's `to_dict()` does NOT output these flags** - they are internal fields accessed via `match.rule.is_license_notice`.

**Rust's `#[derive(Serialize)]` with `#[serde(default)]`** WILL output these fields to JSON, creating a **behavioral difference**.

This is a **feature parity gap** but **NOT causing current golden test failures** (tests don't include these fields).

#### Fix Plan

1. **Add BOTH fields to `LicenseMatch` struct** (`models.rs`):

   ```rust
   #[serde(default)]
   pub is_license_text: bool,
   #[serde(default)]
   pub is_license_notice: bool,
   ```

2. **Update `Default` implementation** (`models.rs`):

   ```rust
   is_license_text: false,
   is_license_notice: false,
   ```

3. **Update all creation sites** to copy both fields from `rule`

4. **Consider serde skip** if JSON output should match Python exactly

---

### Issue 2: `qcontains()` uses range containment instead of set containment

**Status**: Root cause identified, fix planned, VERIFIED

**Confidence**: HIGH (85%) that this will fix extra matches

#### Problem

Examples:

- `COPYING.gplv3`: Expected `["gpl-3.0"]`, got multiple extra matches
- `cddl-1.0.txt`: Expected `["cddl-1.0"]`, got `["unknown-license-reference", "cddl-1.0", "cddl-1.0"]`

#### Findings

**Python vs Rust comparison:**

| File | Python | Rust |
|------|--------|------|
| `COPYING.gplv3` | 1 match: `gpl-3.0: lines 1-674` | 9 matches including extras |
| `cddl-1.0.txt` | 1 match: `cddl-1.0: lines 15-18` | 3 matches including extras |

**Root cause in `qcontains()` (`models.rs:444-452`):**

| Aspect | Python | Rust |
|--------|--------|------|
| Method | `other.qspan in self.qspan` | Range comparison |
| Semantics | SET containment (`issuperset`) | RANGE containment |
| Gap handling | Matches in gaps are NOT contained | Matches in gaps ARE contained |

Python (`match.py:444-448` + `spans.py:177-201`):

```python
def qcontains(self, other):
    return other.qspan in self.qspan  # uses intbitset.issuperset()
```

Rust (`models.rs:444-452`):

```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    self.start_token <= other.start_token && self.end_token >= other.end_token  // RANGE containment
}
```

**The problem:** When matches have **discontinuous qspans** (with gaps), range containment incorrectly considers matches as "contained" even when they are in the gaps.

Example:

- GPL-3.0 qspan: positions `0-10, 16-5191, 5194-5335, 5338-5517` (has gaps)
- Another match at positions 11-15:
  - **Python**: NOT contained (positions 11-15 are not in GPL-3.0's qspan SET)
  - **Rust**: CONTAINED (11 >= 0 and 15 <= 5517 - using RANGE)

#### `qspan_positions` Availability

**CONFIRMED**: `qspan_positions` is available and populated after merging:

- Defined at `models.rs:303`: `pub qspan_positions: Option<Vec<usize>>`
- Populated at `match_refine.rs:200` during `merge_overlapping_matches()`
- Only available **after merging**, which is exactly when `filter_contained_matches()` is called

#### Fix Plan

Modify `qcontains()` to use `qspan_positions` when available:

```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    if let (Some(self_positions), Some(other_positions)) = 
        (&self.qspan_positions, &other.qspan_positions) 
    {
        return other_positions.iter().all(|p| self_positions.contains(p));
    }
    
    // Fallback to range containment when positions not available
    if self.start_token == 0 && self.end_token == 0
        && other.start_token == 0 && other.end_token == 0
    {
        return self.start_line <= other.start_line && self.end_line >= other.end_line;
    }
    self.start_token <= other.start_token && self.end_token >= other.end_token
}
```

#### Performance Consideration

**MEDIUM concern**: `Vec::contains()` is O(n) per position check. For GPL-3.0 with ~5500 positions, checking containment could be O(n²) worst case.

**Recommendation**: Consider `HashSet<usize>` or pre-sorting for binary search.

#### Edge Case

`qspan_positions` is only populated after merging. The fallback to range containment for non-merged matches is appropriate since those have contiguous positions.

---

### Issue 3: `match_coverage` not recalculated after merging matches

**Status**: Root cause identified, fix planned, VERIFIED

**Confidence**: HIGH (90%) that this will fix missing matches

#### Problem

- `crapl-0.1.txt`: Expected `["crapl-0.1"]`, got `[]`

#### Findings

**Python**: Finds `crapl-0.1` via `crapl-0.1_3.RULE` using matcher `2-aho` (Aho-Corasick)

**Rust**: Also finds the matches but they are lost during refinement:

- Aho-Corasick matcher finds 4 matches for crapl-0.1 rules (all coverage=100%)
- After `merge_overlapping_matches()`, `matched_length` is updated
- But `match_coverage` is NOT recalculated
- Merged match ends up with coverage=3.1% (should be 100%)
- Low-coverage match is filtered out by `filter_below_rule_minimum_coverage()`

#### Root Cause

**Location:** `src/license_detection/match_refine.rs:126-211` (`merge_overlapping_matches`)

Lines 195-201 update `matched_length` but NOT `match_coverage`:

```rust
accum.matched_length = merged_qspan.len();  // Updated
accum.hilen = merged_hispan.len();           // Updated
// match_coverage NOT updated!
```

**Flow for `crapl-0.1.txt`:**

1. Aho matches found for `#10583` (crapl-0.1_3.RULE): lines 7-9, coverage=100%
2. Seq matches also found for same rule with low coverage
3. After merging by `rule_identifier`, the merged match inherits the stale `match_coverage`
4. Final coverage=3.1% (≈ 1/32, suggesting a 1-token match merged into a 32-token rule match)

#### Fix Plan

**RECOMMENDED: Fix in `merge_overlapping_matches()` (Option A)**

Add after line 201 in `match_refine.rs`:

```rust
accum.matched_length = merged_qspan.len();
accum.hilen = merged_hispan.len();
// Add this:
if accum.rule_length > 0 {
    accum.match_coverage = (accum.matched_length.min(accum.rule_length) as f32 
                            / accum.rule_length as f32) * 100.0;
}
```

**Why Option A is correct:**

1. `filter_below_rule_minimum_coverage()` uses `match_coverage` AFTER the first merge
2. Coverage must be correct before any filters run
3. Option B (in `update_match_scores`) is called too late - filters have already discarded low-coverage matches

#### Edge Cases

1. **Merged `matched_length` > `rule_length`**: Cap coverage at 100% using `.min()`
2. **Zero `rule_length`**: Guard against division by zero
3. **Non-contiguous matches**: `matched_length = merged_qspan.len()` counts unique positions correctly

#### Verification

```bash
cargo test debug_crapl_0_1 --lib -- --nocapture
cargo test license_golden --lib
```

---

## Implementation Checklist

### Stopword Filtering (DONE)

- [x] Modify `required_phrase_tokenizer()` to lowercase and filter stopwords
- [x] Add unit tests for stopword filtering edge cases
- [x] Run golden tests to verify improvement

### Issue 1: Missing `is_license_notice` and `is_license_text` (VERIFIED)

- [ ] Add BOTH fields to `LicenseMatch` struct
- [ ] Update `Default` implementation
- [ ] Update all creation sites (7+ production files, ~50 test helpers)
- [ ] Consider `#[serde(skip)]` to match Python JSON output
- [ ] Add tests

### Issue 2: `qcontains()` set containment (VERIFIED)

- [ ] Modify `qcontains()` to use `qspan_positions` for set containment
- [ ] Add fallback to range containment when positions not available
- [ ] Consider `HashSet` for performance (O(n) → O(1) per lookup)
- [ ] Add tests for discontinuous qspans

### Issue 3: `match_coverage` recalculation (VERIFIED)

- [ ] Add coverage recalculation in `merge_overlapping_matches()` (Option A)
- [ ] Handle edge cases: cap at 100%, guard division by zero
- [ ] Add tests

---

## Priority Order

1. **Issue 3 (match_coverage)** - HIGH impact, fixes missing matches
2. **Issue 2 (qcontains)** - HIGH impact, fixes extra matches  
3. **Issue 1 (is_license_notice)** - MEDIUM impact, feature parity

Recommend implementing Issues 2 and 3 first, then re-run golden tests to measure impact.

---

## Original Root Cause Analysis

### Example: gpl-2.0_9.txt

- **Expected**: `["gpl-2.0"]`
- **Actual**: `["gpl-2.0", "gpl-1.0-plus"]`

### Investigation Summary

**Python only produces 1 final match**: `gpl-2.0_7.RULE` (lines 31-46)

**Rust produces 2 final matches**:

- `gpl-2.0 (#16951)`: lines 31-43
- `gpl-1.0-plus (#20733)`: lines 45-46

### Key Finding: Python Filters via `filter_contained_matches`

Python's `filter_contained_matches()` removes the gpl-1.0-plus match because:

- `gpl-2.0_7.RULE` has qspan [118-243] (closed interval)
- `gpl_66.RULE` (gpl-1.0-plus) has qspan [223-243] (closed interval)
- The smaller match is **contained** in the larger match, so it's filtered out

### Why Rust Doesn't Filter: Required Phrase Spans Bug

Rust's `filter_contained_matches()` does NOT remove the gpl-1.0-plus match because:

- The gpl-2.0_7.RULE match (rid=#17911) **FAILS** the required phrase check
- Therefore, it's not kept as a "containment container"
- The gpl-1.0-plus match survives

---

## Detailed Comparison: Python vs Rust Tokenization

### Python Implementation (`tokenize.py:90-120, 182-213`)

Python's `required_phrase_tokenizer()`:

```python
def required_phrase_tokenizer(text, stopwords=STOPWORDS, preserve_case=False):
    if not text:
        return
    if not preserve_case:
        text = text.lower()

    for token in required_phrase_splitter(text):
        if token and token not in stopwords:  # <-- STOPWORDS ARE FILTERED HERE
            yield token
```

Key characteristics:

1. Tokenizes text using `required_phrase_splitter` regex: `(?:[^_\W]+\+?[^_\W]*|{{|}})`
2. **Filters stopwords during tokenization** - stopwords are never yielded
3. Yields `{{`, `}}`, and non-stopword tokens
4. `ipos` is incremented only for non-marker tokens (which are already stopword-filtered)

### Rust Implementation (`tokenize.rs:224-335`)

Rust's `required_phrase_tokenizer()`:

```rust
fn required_phrase_tokenizer(text: &str) -> RequiredPhraseTokenIter {
    let tokens: Vec<TokenKind> = REQUIRED_PHRASE_PATTERN
        .find_iter(text)
        .map(|m| {
            let token = m.as_str();
            if token == REQUIRED_PHRASE_OPEN {
                TokenKind::Open
            } else if token == REQUIRED_PHRASE_CLOSE {
                TokenKind::Close
            } else {
                TokenKind::Word  // <-- ALL WORDS, INCLUDING STOPWORDS
            }
        })
        .collect();
    RequiredPhraseTokenIter { tokens, pos: 0 }
}
```

Rust's `parse_required_phrase_spans()`:

```rust
for token in required_phrase_tokenizer(text) {
    // ...
    } else {
        // Token is a word - STOPWORDS ARE NOT FILTERED
        if in_required_phrase {
            current_phrase_positions.push(ipos);
        }
        ipos += 1;  // <-- ipos incremented for EVERY word, including stopwords
    }
}
```

Key difference: **Rust does NOT filter stopwords during position counting**.

---

### The Bug: Missing Stopword Filtering

**Python filters stopwords in `required_phrase_tokenizer()`**:

- Stopwords are never yielded as tokens
- `ipos` never increments for stopwords
- Positions are calculated only on non-stopword tokens

**Rust does NOT filter stopwords**:

- All tokens (including stopwords) are counted
- `ipos` increments for stopwords too
- This causes cumulative position drift

### Evidence: Stopwords in gpl-2.0_7.RULE

```text
=== Stopwords found in rule text ===
Total stopwords: 2
  Position 62: 'a' (HTML tag stopword)
  Position 80: 'a' (HTML tag stopword)
```

These 2 stopwords cause a cumulative 2-position offset by the end of the rule text.

### Position Drift Trace

| Span | Python (closed) | Python (half-open) | Rust | Offset |
|------|-----------------|-------------------|------|--------|
| 1    | Span(18, 30)    | 18..31            | 18..31 | 0     |
| 2    | Span(64, 67)    | 64..68            | 65..69 | +1    |
| 3    | Span(78, 81)    | 78..82            | 80..84 | +2    |
| 4    | Span(109, 125)  | 109..126          | 111..128 | +2   |

The offset matches exactly: 2 stopwords = 2 position drift.

### Token Count Verification

- **Python**: 126 non-stopword tokens (positions 0-125)
- **Rust `tokenize()`**: 126 non-stopword tokens ✓
- **Rust `parse_required_phrase_spans()`**: Counts 128 positions (includes 2 stopwords)

---

## Fix Plan

### Overview

The fix requires modifying `required_phrase_tokenizer()` in Rust to filter stopwords, matching Python's behavior exactly.

### Step 1: Modify `required_phrase_tokenizer()` to Filter Stopwords

**File**: `src/license_detection/tokenize.rs`

**Current implementation** (lines 285-300):

```rust
fn required_phrase_tokenizer(text: &str) -> RequiredPhraseTokenIter {
    let tokens: Vec<TokenKind> = REQUIRED_PHRASE_PATTERN
        .find_iter(text)
        .map(|m| {
            let token = m.as_str();
            if token == REQUIRED_PHRASE_OPEN {
                TokenKind::Open
            } else if token == REQUIRED_PHRASE_CLOSE {
                TokenKind::Close
            } else {
                TokenKind::Word
            }
        })
        .collect();
    RequiredPhraseTokenIter { tokens, pos: 0 }
}
```

**New implementation**:

```rust
fn required_phrase_tokenizer(text: &str) -> RequiredPhraseTokenIter {
    let lowercase_text = text.to_lowercase();
    let tokens: Vec<TokenKind> = REQUIRED_PHRASE_PATTERN
        .find_iter(&lowercase_text)
        .filter_map(|m| {
            let token = m.as_str();
            if token == REQUIRED_PHRASE_OPEN {
                Some(TokenKind::Open)
            } else if token == REQUIRED_PHRASE_CLOSE {
                Some(TokenKind::Close)
            } else if !token.is_empty() && !STOPWORDS.contains(token) {
                // Filter stopwords just like Python
                Some(TokenKind::Word)
            } else {
                None  // Skip stopwords
            }
        })
        .collect();
    RequiredPhraseTokenIter { tokens, pos: 0 }
}
```

**Key changes**:

1. Convert text to lowercase before tokenizing (matching Python)
2. Filter out stopwords using `STOPWORDS` set
3. Use `filter_map` to skip stopwords entirely

### Step 2: Update `TokenKind` Enum (Optional Optimization)

The `TokenKind::Word` variant currently doesn't store the actual word. If needed for debugging, we could change:

```rust
enum TokenKind {
    Open,
    Close,
    Word(String),  // Store actual word for debugging
}
```

But this is optional - the current design is sufficient if stopwords are filtered.

### Step 3: Update the Iterator

The `RequiredPhraseTokenIter` can remain mostly the same, but ensure it doesn't yield tokens for filtered stopwords.

### Step 4: Add Comprehensive Tests

Add test cases that verify stopword filtering behavior:

```rust
#[test]
fn test_parse_required_phrase_spans_filters_stopwords() {
    // Text with 'a' (stopword) inside required phrase
    let text = "Hello {{a world}} test";
    let spans = parse_required_phrase_spans(text);
    // Python: tokens are ['hello', '{{', 'world', '}}', 'test']
    // Position: hello=0, world is inside phrase at position 0 (after filtering 'a')
    assert_eq!(spans, vec![1..2]);  // 'world' is at position 1
}

#[test]
fn test_parse_required_phrase_spans_stopword_outside_phrase() {
    // Text with 'a' (stopword) outside required phrase
    let text = "{{Hello}} a {{world}}";
    let spans = parse_required_phrase_spans(text);
    // Python filters 'a', so tokens are: ['{{', 'hello', '}}', '{{', 'world', '}}']
    // Positions: hello=0, world=1
    assert_eq!(spans, vec![0..1, 1..2]);
}

#[test]
fn test_python_parity_gpl_2_0_7() {
    // Full integration test with actual rule file
    // Verify exact match with Python output
}
```

### Step 5: Verify Against Python Reference

Run comparison test:

```bash
# Python
cd scancode-playground && venv/bin/python -c "
from licensedcode.tokenize import get_existing_required_phrase_spans
text = open('../reference/scancode-toolkit/src/licensedcode/data/rules/gpl-2.0_7.RULE').read()
import re; match = re.search(r'---\n.*?---\n(.*)', text, re.DOTALL)
print(get_existing_required_phrase_spans(match.group(1)))
"

# Rust
cargo test test_python_parity_gpl_2_0_7 --lib -- --nocapture
```

### Step 6: Run Golden Tests

After fix, run golden tests to verify regression is resolved:

```bash
cargo test license_golden --lib
```

Expected: All 5 test suites should pass (228, 776, 251, 281, 1882 tests respectively).

---

## Edge Cases to Consider

### 1. Stopwords at Phrase Boundaries

```text
{{a hello}} world
```

- 'a' is stopword, should be filtered
- 'hello' is at position 0 (not 1)

### 2. Stopwords Inside Phrases

```text
{{hello a world}}
```

- 'a' is stopword, should be filtered
- Positions: hello=0, world=1 (span = 0..2)

### 3. Multiple Consecutive Stopwords

```text
{{a p div hello}}
```

- 'a', 'p', 'div' are all stopwords
- Only 'hello' remains at position 0

### 4. Empty Phrase After Stopword Filtering

```text
{{a p div}}
```

- All tokens are stopwords
- Should return empty spans (error case)

### 5. Unicode Stopwords

The stopwords list includes ASCII-only words. Unicode text should work correctly as stopwords won't match.

### 6. Case Sensitivity

Python lowercases text before checking stopwords. Rust must do the same.

### 7. Marker Tokens

`{{` and `}}` should never be filtered as stopwords (they're not words).

---

## Additional Notes

### The gpl-2.0_7.RULE File Content

```yaml
---
license_expression: gpl-2.0
is_license_notice: yes
---

This package is free software; you can redistribute it and/or modify
   it under the terms of the {{GNU General Public License as published by
   the Free Software Foundation; version 2}} dated June, 1991.
 
   This package is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   {{GNU General Public License}} for more details.
 
   You should have received a copy of the {{GNU General Public License}}
   along with this package; if not, write to the Free Software
   Foundation, Inc., 51 Franklin St, Fifth Floor, Boston, MA
   02110-1301, USA.
 
On Debian systems, the {{complete text of the GNU General
Public License can be found in `/usr/share/common-licenses/GPL`}}'.
```

### Investigation Commands

Run Python to see refined matches:

```bash
cd scancode-playground && venv/bin/python -c "
from licensedcode.cache import get_index
from licensedcode.query import Query
from licensedcode import match_aho, match

idx = get_index()
text = open('tests/licensedcode/data/datadriven/lic1/gpl-2.0_9.txt').read()
query = Query(query_string=text, idx=idx)
raw = list(match_aho.exact_match(idx, query.whole_query_run(), idx.rules_automaton))
refined, _ = match.refine_matches(raw, query, filter_false_positive=False, merge=True)
for m in refined:
    print(f'{m.rule.identifier}: lines {m.start_line}-{m.end_line}')
"
```

Run Rust debug test:

```bash
cargo test debug_gpl_2_0_9 --lib -- --nocapture
```

### Related Files

- **Python tokenizer**: `scancode-playground/src/licensedcode/tokenize.py`
- **Rust tokenizer**: `src/license_detection/tokenize.rs`
- **Stopwords**: `src/license_detection/tokenize.rs:STOPWORDS` and `scancode-playground/src/licensedcode/stopwords.py`
- **Index builder**: `src/license_detection/index/builder.rs:220` (where `parse_required_phrase_spans` is called)
- **Match filtering**: `src/license_detection/match_refine.rs:filter_matches_missing_required_phrases`
