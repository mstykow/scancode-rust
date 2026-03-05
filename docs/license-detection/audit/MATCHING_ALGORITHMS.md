# Matching Algorithms Audit: Python vs Rust

This document provides a detailed comparison of all license detection matching strategies between the Python ScanCode Toolkit implementation and the Rust rewrite.

## Overview

| Strategy | Python Matcher | Rust Matcher | Priority |
|----------|---------------|--------------|----------|
| Hash Matching | `match_hash.py` | `hash_match.rs` | 0 (highest) |
| SPDX-LID | `match_spdx_lid.py` | `spdx_lid/mod.rs` | 2 |
| Aho-Corasick | `match_aho.py` | `aho_match.rs` | 1 |
| Sequence | `match_seq.py`, `match_set.py`, `seq.py` | `seq_match/` | 3 |
| Unknown | `match_unknown.py` | `unknown_match.rs` | 6 (lowest) |

---

## 1. Hash Matching

### Algorithm Overview

Hash matching provides exact, O(1) matching of entire token sequences against a pre-computed hash index.

### Python Implementation (`match_hash.py`)

**Hash Algorithm:**
- **Lines 44-49**: `tokens_hash()` uses SHA1 hash
- Converts token IDs to signed 16-bit integers via `array('h', tokens).tobytes()`
- Python's `array('h')` creates signed 16-bit integers

**Matching Process:**
- **Lines 59-87**: `hash_match()` computes query hash and looks up in `idx.rid_by_hash`
- Returns at most one match (first exact match found)
- Creates `LicenseMatch` with `qspan` and `ispan` spans

**Key Constants:**
- `MATCH_HASH = '1-hash'` (line 40)
- `MATCH_HASH_ORDER = 0` (line 41)

### Rust Implementation (`hash_match.rs`)

**Hash Algorithm:**
- **Lines 38-47**: `compute_hash()` uses SHA1 via `sha1` crate
- Converts `u16` tokens to signed `i16`, then to little-endian bytes
- Produces identical hash values to Python

**Matching Process:**
- **Lines 72-138**: `hash_match()` mirrors Python logic
- Returns `Vec<LicenseMatch>` (0 or 1 match)

**Key Constants:**
- `MATCH_HASH: &str = "1-hash"` (line 16)
- `MATCH_HASH_ORDER: u8 = 0` (line 24)

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Hash computation | `array('h').tobytes()` | `i16.to_le_bytes()` per token | Identical output |
| Return type | List of matches | Vec of matches | Same semantics |
| Span handling | `Span(range(...))` | `Span::from_range(...)` | Equivalent |

### Edge Cases

1. **Empty tokens**: Both produce SHA1 of empty byte sequence
2. **Hash collisions**: Both return only the first match (deterministic)
3. **Token overflow**: Python's `array('h')` handles values -32768 to 32767; Rust's `as i16` cast is equivalent

---

## 2. SPDX-License-Identifier Matching

### Algorithm Overview

Parses SPDX license identifier tags (e.g., `SPDX-License-Identifier: MIT`) and resolves them to license expressions.

### Python Implementation (`match_spdx_lid.py`)

**Regex Patterns:**
- **Lines 394-396**: `_split_spdx_lid` regex with typos built-in:
  ```python
  r'(spd[xz][\-\\s]+lin?[cs]en?[sc]es?[\-\\s]+identifi?er\s*:\s*)'
  ```
- **Lines 398-400**: NuGet pattern for `licenses.nuget.org`

**Expression Parsing:**
- **Lines 154-193**: `get_expression()` parses with fallback recovery
- **Lines 271-340**: `_reparse_invalid_expression()` for malformed expressions
- Uses `license-expression` library for parsing

**Deprecated Identifier Handling:**
- **Lines 202-223**: `get_old_expressions_subs_table()` maps deprecated SPDX IDs
- Example: `GPL-2.0-with-classpath-exception` → `GPL-2.0-only WITH Classpath-exception-2.0`

**Text Cleaning:**
- **Lines 358-391**: `clean_text()` strips punctuation, fixes unbalanced parens

### Rust Implementation (`spdx_lid/mod.rs`)

**Regex Patterns:**
- **Lines 53-59**: Uses `lazy_static!` for compiled regexes
- Same patterns as Python

**Expression Parsing:**
- **Lines 550-601**: `find_matching_rule_for_expression()` resolves expressions
- **Lines 468-512**: `reparse_invalid_expression()` for malformed expressions
- Custom `LicenseExpression` enum (not external library)

**Deprecated Identifier Handling:**
- **Lines 169-200**: `DEPRECATED_SPDX_EXPRESSION_SUBS` constant array
- Same mappings as Python

**Text Cleaning:**
- **Lines 84-107**: `clean_spdx_text()` mirrors Python logic

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Parsing library | `license-expression` crate | Custom enum | Different error handling |
| Matcher order | 2 | 1 | **Different** - see note |
| Regex compilation | Runtime | Compile-time via `lazy_static` | Rust faster |
| Expression AST | External library | Internal `LicenseExpression` enum | Same semantics |

**Important**: Python's `MATCH_SPDX_ID_ORDER = 2`, Rust's `MATCH_SPDX_ID_ORDER = 1`. This is a potential ordering difference.

### Edge Cases

1. **Typo tolerance**: Both handle `spdx`/`spdz`, `license`/`licence`, `identifier`/`identifer`
2. **Bare license lists**: Both infer OR for lists without keywords (u-boot style)
3. **Unknown symbols**: Both return `unknown-spdx` symbol on parse failure

---

## 3. Aho-Corasick Matching

### Algorithm Overview

Uses Aho-Corasick automaton for efficient multi-pattern exact matching of rule token sequences.

### Python Implementation (`match_aho.py`)

**Automaton Construction:**
- **Lines 45-76**: Uses `pyahocorasick` library
- `ahocorasick.STORE_ANY, ahocorasick.KEY_SEQUENCE`
- Stores `(rid, start, end)` tuples as values

**Matching Process:**
- **Lines 84-138**: `exact_match()` iterates over automaton matches
- **Lines 162-176**: `get_matched_positions()` yields `(rid, qstart, qend, istart, iend)`
- **Lines 141-159**: `get_matched_spans()` filters by matchable positions

**Key Constants:**
- `MATCH_AHO_EXACT = '2-aho'` (line 78)
- `MATCH_AHO_EXACT_ORDER = 1` (line 79)

### Rust Implementation (`aho_match.rs`)

**Automaton Construction:**
- **Lines 44-46**: `tokens_to_bytes()` encodes u16 tokens as little-endian bytes
- Uses `aho_corasick` crate
- Pattern ID maps to RID via `pattern_id_to_rid` lookup

**Matching Process:**
- **Lines 76-198**: `aho_match()` uses `find_overlapping_iter()`
- **Lines 96-98**: Critical token boundary check (prevents false matches)
- **Lines 103-107**: Filters by matchable positions

**Key Constants:**
- `MATCH_AHO: &str = "2-aho"` (line 17)
- `MATCH_AHO_ORDER: u8 = 2` (line 25) - **Different from Python!**

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Token encoding | Tuple of u16 | Little-endian bytes | Different internal representation |
| Matcher order | 1 | 2 | **Different ordering!** |
| Boundary check | N/A (tuple keys) | Byte alignment check (line 96) | Rust prevents cross-token matches |
| Overlapping matches | Via `iter()` | Via `find_overlapping_iter()` | Same behavior |

**Critical Difference**: Rust's matcher order is 2 vs Python's 1. This changes matching priority.

### Edge Cases

1. **Token boundary crossing**: Rust explicitly checks `byte_start % 2 != 0` to reject matches that start mid-token
2. **Overlapping patterns**: Both find all overlapping matches
3. **Non-matchable positions**: Both filter matches against `matchables` set

---

## 4. Sequence Matching

### Algorithm Overview

Two-phase approach:
1. **Candidate selection** using set/multiset similarity (match_set.py)
2. **Block matching** using longest common substring (match_seq.py, seq.py)

### Python Implementation

#### Candidate Selection (`match_set.py`)

**Set Operations:**
- **Lines 109-116**: `tids_sets_intersector()` uses `intbitset` for bitmap intersections
- **Lines 119-137**: `multisets_intersector()` for frequency counters

**Similarity Metrics:**
- **Lines 370-449**: `compare_token_sets()` computes:
  - `resemblance = matched_length / union_len`
  - `containment = matched_length / iset_len`
  - `amplified_resemblance = resemblance ** 2`

**Ranking:**
- **Lines 244-367**: `compute_candidates()` two-phase ranking:
  1. Set intersection (line 272-302)
  2. Multiset refinement (line 311-367)
- **Lines 461-498**: `filter_dupes()` groups by `(license_expression, is_highly_resemblant, containment, resemblance, matched_length, rule_length)`

#### Block Matching (`seq.py`)

**Longest Common Substring:**
- **Lines 19-81**: `find_longest_match()` DP algorithm
- Uses `j2len` dictionary for O(n*m) LCS with early termination
- **Lines 84-104**: `extend_match()` extends into low-value tokens

**Divide-and-Conquer:**
- **Lines 107-176**: `match_blocks()` queue-based recursion
- Merges adjacent blocks (lines 156-174)

#### Sequence Matching (`match_seq.py`)

- **Lines 48-156**: `match_sequence()` finds repeated matches
- Loops while `qstart <= qfinish` and high matchables exist
- Delegates to Cython or Python `match_blocks()`

### Rust Implementation (`seq_match/`)

#### Candidate Selection (`candidates.rs`)

**Similarity Metrics:**
- **Lines 215-279**: `compute_set_similarity()` computes same metrics
- Same formula for resemblance, containment, amplified_resemblance

**Ranking:**
- **Lines 286-455**: `compute_candidates_with_msets()` mirrors Python two-phase
- **Lines 144-174**: `filter_dupes()` same grouping logic

**ScoresVector:**
- **Lines 18-66**: Custom struct with `Ord` implementation
- Same comparison order: `is_highly_resemblant` > `containment` > `resemblance` > `matched_length`

#### Block Matching (`matching.rs`)

**Longest Common Substring:**
- **Lines 33-105**: `find_longest_match()` direct port of Python algorithm
- Same `j2len` HashMap approach

**Divide-and-Conquer:**
- **Lines 128-195**: `match_blocks()` same queue-based algorithm
- Same adjacent block merging (lines 172-193)

#### Sequence Matching (`mod.rs`)

- **Lines 211-348**: `seq_match_with_candidates()` matches against candidates

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Bitset library | `intbitset` | `HashSet<u16>` | Rust simpler but potentially slower |
| Threshold check | Inside `compare_token_sets` | In `compute_candidates_with_msets` | Same filtering |
| Match blocks import | Cython or Python | Native Rust | Same algorithm |
| Coverage filter | None (no 50% threshold) | None | **Identical behavior** |

### Key Similarity Details

Both compute:
```
resemblance = matched_length / (query_len + rule_len - matched_length)
containment = matched_length / rule_len
amplified_resemblance = resemblance^2
```

Both filter candidates requiring:
- High token intersection (legalese tokens)
- Minimum matched length thresholds from rule metadata

---

## 5. Unknown Matching

### Algorithm Overview

Detects license-like content that doesn't match any known license using ngram matching.

### Python Implementation (`match_unknown.py`)

**Ngram Construction:**
- **Lines 59-77**: `add_ngrams()` adds 6-grams to automaton
- **Lines 93-129**: `is_good_tokens_ngram()` filters ngrams:
  - Rejects: >2 digits, year patterns, >2 single chars, low diversity, no high tokens, markers

**Markers to Reject:**
- **Lines 80-90**: `copyright`, `rights`, `reserved`, `trademark`, URLs, etc.

**Matching Process:**
- **Lines 132-239**: `match_unknowns()` 
- **Lines 242-260**: `get_matched_ngrams()` yields positions

**Thresholds:**
- **Line 220**: `len(qspan) < unknown_ngram_length * 4` (24 tokens) or `len(hispan) < 5`
- `UNKNOWN_NGRAM_LENGTH = 6` (line 49)

### Rust Implementation (`unknown_match.rs`)

**Ngram Matching:**
- **Lines 144-172**: `get_matched_ngrams()` uses byte-encoded tokens
- **Lines 20-97**: `unknown_match()` main function

**Region Detection:**
- **Lines 99-142**: `find_unmatched_regions()` finds gaps in known matches
- **Lines 174-196**: `compute_qspan_union()` merges overlapping ngram positions

**Thresholds:**
- **Line 76**: `qspan_length < UNKNOWN_NGRAM_LENGTH * 4` (24 tokens)
- **Line 87**: `hispan < 5`
- `UNKNOWN_NGRAM_LENGTH = 6` (line 14)

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Matcher name | `'6-unknown'` | `'5-undetected'` | **Different identifier!** |
| Matcher order | 6 | 5 | **Different priority!** |
| Ngram construction | In index build | In index build | Same logic |
| Threshold check | Combined in one `if` | Separate checks | Same result |

**Important Differences:**
- Python: `MATCH_UNKNOWN = '6-unknown'`, `MATCH_UNKNOWN_ORDER = 6`
- Rust: `MATCH_UNKNOWN = '5-undetected'`, `MATCH_UNKNOWN_ORDER = 5`

This is both a naming and priority difference.

### Edge Cases

1. **Empty query**: Both return empty matches
2. **All positions covered**: Both find no unmatched regions
3. **Weak matches**: Both skip matches below thresholds (qspan < 24, hispan < 5)

---

## Summary of Key Differences

### Matcher Ordering Differences

| Strategy | Python Order | Rust Order | Issue |
|----------|-------------|------------|-------|
| Hash | 0 | 0 | Same |
| Aho-Corasick | 1 | 2 | **Different** |
| SPDX-LID | 2 | 1 | **Different** |
| Sequence | 3 | 3 | Same |
| Unknown | 6 | 5 | **Different** |

**Recommendation**: The Aho-Corasick and SPDX-LID order swap could affect matching priority. The Unknown matcher name and order differences should be aligned.

### Matcher Name Differences

| Strategy | Python Name | Rust Name |
|----------|-------------|-----------|
| Unknown | `6-unknown` | `5-undetected` |

**Recommendation**: Align to Python names for JSON output compatibility.

### Algorithm Equivalence

| Strategy | Algorithms Match | Notes |
|----------|-----------------|-------|
| Hash | Yes | Identical SHA1 computation |
| SPDX-LID | Mostly | Different parsing library, same behavior |
| Aho-Corasick | Yes | Rust adds boundary safety check |
| Sequence | Yes | Direct port of LCS algorithm |
| Unknown | Yes | Same thresholds and filtering |

### Potential Behavioral Differences

1. **SPDX-LID parser**: Uses different libraries; edge case handling may differ
2. **Aho-Corasick byte encoding**: Rust's boundary check prevents false positives
3. **Unknown matcher naming**: Output compatibility issue

### Recommendations

1. **Align matcher orders** to match Python's priority system
2. **Rename unknown matcher** from `'5-undetected'` to `'6-unknown'`
3. **Document Aho-Corasick boundary check** as a correctness improvement
4. **Add tests** for deprecated SPDX identifier substitutions
5. **Verify** edge cases in expression parsing between libraries
