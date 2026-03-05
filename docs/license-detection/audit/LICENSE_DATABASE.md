# License Database/Index Structure Comparison: Python vs Rust

This document provides a detailed comparison of the license database and index structures between the Python ScanCode Toolkit reference implementation and the Rust scancode-rust implementation.

## Table of Contents

1. [LicenseIndex Structure](#1-licenseindex-structure)
2. [Rule Storage](#2-rule-storage)
3. [Token Dictionary](#3-token-dictionary)
4. [Inverted Indices](#4-inverted-indices)
5. [Hash Index](#5-hash-index)
6. [Automatons](#6-automatons)
7. [License Metadata](#7-license-metadata)
8. [Summary of Differences](#8-summary-of-differences)

---

## 1. LicenseIndex Structure

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/index.py:131-165`

```python
class LicenseIndex(object):
    __slots__ = (
        'len_legalese',
        'dictionary',
        'digit_only_tids',

        'rules_by_id',
        'rules_by_rid',
        'tids_by_rid',

        'high_postings_by_rid',

        'sets_by_rid',
        'msets_by_rid',

        'rid_by_hash',
        'rules_automaton',
        'fragments_automaton',
        'starts_automaton',
        'unknown_automaton',

        'regular_rids',
        'false_positive_rids',
        'approx_matchable_rids',

        'optimized',
        'all_languages',
    )
```

**Key fields:**
- `len_legalese`: Number of legalese tokens (line 185)
- `dictionary`: `{token_string: token_id}` mapping (line 188)
- `digit_only_tids`: Set of token IDs made entirely of digits (line 191)
- `rules_by_id`: `{identifier: Rule}` mapping (line 198)
- `rules_by_rid`: List of Rule objects indexed by rid (line 201)
- `tids_by_rid`: List of token ID arrays indexed by rid (line 204)
- `high_postings_by_rid`: List of `{token_id: [positions...]}` inverted index (line 209)
- `sets_by_rid`: List of token ID sets per rule (line 212)
- `msets_by_rid`: List of token ID multisets per rule (line 213)
- `rid_by_hash`: `{hash: rid}` mapping (line 216)
- `rules_automaton`: Aho-Corasick automaton for rules (line 219)
- `unknown_automaton`: Aho-Corasick automaton for unknown detection (line 222)
- `regular_rids`: Set of regular rule IDs (line 228)
- `false_positive_rids`: Set of false positive rule IDs (line 230)
- `approx_matchable_rids`: Set of approx-matchable rule IDs (line 234)
- `optimized`: Boolean flag for immutability (line 238)

### Rust Implementation

**File:** `src/license_detection/index/mod.rs:42-194`

```rust
pub struct LicenseIndex {
    pub dictionary: TokenDictionary,
    pub len_legalese: usize,
    pub digit_only_tids: HashSet<u16>,
    pub rid_by_hash: HashMap<[u8; 20], usize>,
    pub rules_by_rid: Vec<Rule>,
    pub tids_by_rid: Vec<Vec<u16>>,
    pub rules_automaton: Automaton,
    pub unknown_automaton: Automaton,
    pub sets_by_rid: HashMap<usize, HashSet<u16>>,
    pub msets_by_rid: HashMap<usize, HashMap<u16, usize>>,
    pub high_postings_by_rid: HashMap<usize, HashMap<u16, Vec<usize>>>,
    pub regular_rids: HashSet<usize>,
    pub false_positive_rids: HashSet<usize>,
    pub approx_matchable_rids: HashSet<usize>,
    pub licenses_by_key: HashMap<String, License>,
    pub pattern_id_to_rid: Vec<usize>,
    pub rid_by_spdx_key: HashMap<String, usize>,
    pub unknown_spdx_rid: Option<usize>,
}
```

### Key Differences

| Aspect | Python | Rust | Notes |
|--------|--------|------|-------|
| Dictionary | `dict[str, int]` | `TokenDictionary` wrapper | Rust encapsulates in separate struct |
| `rules_by_id` | Present | **Not implemented** | Python maps identifier → Rule, Rust only has `rules_by_rid` |
| Automaton storage | Stores `(rid, istart, iend)` tuples | Separate `pattern_id_to_rid` mapping | Different approach to track pattern→rule mapping |
| Hash key type | `bytes` (digest) | `[u8; 20]` fixed array | Rust uses fixed-size array for type safety |
| `optimized` flag | Present | **Not implemented** | Python uses to prevent mutation after optimization |
| SPDX support | In separate `cache.py` | Built into `LicenseIndex` | Rust consolidates SPDX lookups |
| `fragments_automaton` | Present (feature-flagged) | **Not implemented** | Experimental in Python, not ported |
| `starts_automaton` | Present (feature-flagged) | **Not implemented** | Experimental in Python, not ported |

---

## 2. Rule Storage

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/models.py:1308-1470`

```python
@attr.s(slots=True)
class BasicRule:
    rid = attr.ib(default=None)
    identifier = attr.ib(default=None)
    license_expression = attr.ib(default=None)
    license_expression_object = attr.ib(default=None)
    is_builtin = attr.ib(default=True)
    is_license_text = attr.ib(default=False)
    is_license_notice = attr.ib(default=False)
    is_license_reference = attr.ib(default=False)
    is_license_tag = attr.ib(default=False)
    is_license_intro = attr.ib(default=False)
    is_license_clue = attr.ib(default=False)
    is_false_positive = attr.ib(default=False)
    is_required_phrase = attr.ib(default=False)
    # ... many more fields
```

Rule fields include computed values like:
- `length_unique`: Count of unique token IDs
- `high_length_unique`: Count of unique legalese token IDs
- `high_length`: Total count of legalese token occurrences
- `min_matched_length`, `min_high_matched_length`: Thresholds
- `is_small`, `is_tiny`: Size flags
- `starts_with_license`, `ends_with_license`: Position flags

**Storage in index:**
- `rules_by_rid`: List where index = rid (line 201 of index.py)
- `rules_by_id`: Dict mapping identifier → Rule (line 198 of index.py)
- Rules sorted by identifier at index build time (line 323 of index.py)

### Rust Implementation

**File:** `src/license_detection/models/rule.rs:9-135`

```rust
pub struct Rule {
    pub identifier: String,
    pub license_expression: String,
    pub text: String,
    pub tokens: Vec<u16>,
    pub is_license_text: bool,
    pub is_license_notice: bool,
    pub is_license_reference: bool,
    pub is_license_tag: bool,
    pub is_license_intro: bool,
    pub is_license_clue: bool,
    pub is_false_positive: bool,
    pub is_required_phrase: bool,
    pub is_from_license: bool,
    pub relevance: u8,
    pub minimum_coverage: Option<u8>,
    pub is_continuous: bool,
    pub required_phrase_spans: Vec<Range<usize>>,
    pub stopwords_by_pos: HashMap<usize, usize>,
    // ... more fields
}
```

### Key Differences

| Aspect | Python | Rust | Notes |
|--------|--------|------|-------|
| `rid` field | Stored on Rule object | **Not stored** on Rule | Rust tracks rid via index position |
| `license_expression_object` | Cached Expression object | **Not implemented** | Python caches parsed expression |
| Token storage | In separate `tids_by_rid` list | Duplicated in `rule.tokens` field | Rust stores tokens both places |
| Sorting | By identifier (line 323) | By identifier (same) | Consistent behavior |
| Required phrase spans | Parsed at match time | Parsed at index build time | Rust pre-computes for efficiency |
| `length` property | Computed from tokens.len() | Not stored as field | Both compute on demand |

---

## 3. Token Dictionary

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/index.py:295-314`

```python
# Initial dictionary mapping for known legalese tokens
self.dictionary = dictionary = dict(_legalese)
dictionary_get = dictionary.get
self.len_legalese = len_legalese = len(set(dictionary.values()))
highest_tid = len_legalese - 1

# Add SPDX key tokens to the dictionary
for sts in sorted(_spdx_tokens):
    stid = dictionary_get(sts)
    if stid is None:
        highest_tid += 1
        stid = highest_tid
        dictionary[sts] = stid
```

**Legalese source:** `reference/scancode-toolkit/src/licensedcode/legalese.py`
- Contains 4506 words mapping to 4356 unique token IDs
- Multiple words can map to the same ID (spelling variants, typos)

**Token ID assignment:**
- IDs 0 to `len_legalese-1`: Reserved for legalese tokens
- IDs `len_legalese` and above: Assigned dynamically during indexing

### Rust Implementation

**File:** `src/license_detection/index/dictionary.rs:20-29`

```rust
pub struct TokenDictionary {
    tokens_to_ids: HashMap<String, u16>,
    len_legalese: usize,
    next_id: u16,
}
```

**Legalese source:** `src/license_detection/rules/legalese.rs`
- Static `LazyLock<HashMap<String, u16>>` with same 4506 entries
- Directly ported from Python reference

### Key Differences

| Aspect | Python | Rust | Notes |
|--------|--------|------|-------|
| Max tokens | 32767 (2^15 - 1) | 65535 (u16 max) | Rust uses unsigned, effectively 2x capacity |
| Legalese initialization | Dict from `_legalese` param | `LazyLock` static map | Rust uses lazy static initialization |
| Token ID type | `int` | `u16` | Rust uses explicit type |
| Reverse lookup | `tokens_by_tid` property (line 587) | **Not implemented** | Python can reverse-map tid → token |

---

## 4. Inverted Indices

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/index.py:471-480`

```python
# high_postings_by_rid: mapping-like of rule id->(mapping of (token_id->[positions, ...])
postings = defaultdict(list)
for pos, tid in enumerate(rule_token_ids):
    if tid < len_legalese:
        postings[tid].append(pos)
# OPTIMIZED: for speed and memory: convert postings to arrays
postings = {tid: array('h', value) for tid, value in postings.items()}
high_postings_by_rid[rid] = postings
```

**Sets by rid (line 509-512):**
```python
tids_set, mset = match_set.build_set_and_mset(
    rule_token_ids, _use_bigrams=USE_BIGRAM_MULTISETS)
sets_by_rid[rid] = tids_set
msets_by_rid[rid] = mset
```

**Storage format:**
- `high_postings_by_rid[rid]` = `{token_id: array('h', [pos1, pos2, ...])}`
- `sets_by_rid[rid]` = `intbitset` of unique token IDs
- `msets_by_rid[rid]` = Counter-like dict of `{token_id: count}`

### Rust Implementation

**File:** `src/license_detection/index/mod.rs:111-131`

```rust
pub sets_by_rid: HashMap<usize, HashSet<u16>>,
pub msets_by_rid: HashMap<usize, HashMap<u16, usize>>,
pub high_postings_by_rid: HashMap<usize, HashMap<u16, Vec<usize>>>,
```

**File:** `src/license_detection/index/builder/mod.rs:415-424`
```rust
let mut postings: HashMap<u16, Vec<usize>> = HashMap::new();
for (pos, &tid) in rule_token_ids.iter().enumerate() {
    if (tid as usize) < len_legalese {
        postings.entry(tid).or_default().push(pos);
    }
}
if !postings.is_empty() {
    high_postings_by_rid.insert(rid, postings);
}
```

### Key Differences

| Aspect | Python | Rust | Behavioral Impact |
|--------|--------|------|-------------------|
| `sets_by_rid` storage | List indexed by rid | HashMap<rid, HashSet> | Rust uses HashMap, potentially slower but handles sparse rid space |
| `msets_by_rid` storage | List indexed by rid | HashMap<rid, HashMap> | Same as above |
| `high_postings_by_rid` storage | List indexed by rid | HashMap<rid, HashMap> | Same as above |
| Position array type | `array('h')` signed short | `Vec<usize>` | Rust uses usize (8 bytes on 64-bit) vs Python's 2-byte array |
| `intbitset` for sets | Specialized bitset | `HashSet<u16>` | Python's intbitset is more memory-efficient for large sets |
| Sparse handling | List may have `None` entries | HashMap only stores non-empty | Rust approach is more memory-efficient for sparse data |

**Potential performance impact:** Rust's use of HashMap instead of list indexing means O(1) amortized lookup with hash overhead, vs Python's O(1) direct indexing.

---

## 5. Hash Index

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/match_hash.py:44-56`

```python
def tokens_hash(tokens):
    """Return a digest binary string computed from a sequence of numeric token ids."""
    as_bytes = array('h', tokens).tobytes()
    return sha1(as_bytes).digest()

def index_hash(rule_tokens):
    """Return a hash digest string given a sequence of rule tokens."""
    return tokens_hash(rule_tokens)
```

**Indexing (index.py:419-421):**
```python
rule_hash = match_hash_index_hash(rule_token_ids)
dupe_rules_by_hash[rule_hash].append(rule)
# Later: rid_by_hash[rule_hash] = rid (only for regular rules)
```

**Lookup (match_hash.py:59-87):**
```python
def hash_match(idx, query_run, **kwargs):
    matches = []
    query_hash = tokens_hash(query_run.tokens)
    rid = idx.rid_by_hash.get(query_hash)
    if rid is not None:
        # ... create match
```

### Rust Implementation

**File:** `src/license_detection/hash_match.rs:38-47`

```rust
pub fn compute_hash(tokens: &[u16]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    for token in tokens {
        let signed = *token as i16;
        hasher.update(signed.to_le_bytes());
    }
    hasher.finalize().into()
}
```

**Index storage:** `HashMap<[u8; 20], usize>`

### Key Differences

| Aspect | Python | Rust | Notes |
|--------|--------|------|-------|
| Hash algorithm | SHA1 | SHA1 | Same algorithm |
| Token serialization | `array('h').tobytes()` | `i16.to_le_bytes()` per token | Same byte representation (little-endian signed 16-bit) |
| Hash value type | `bytes` (20-byte digest) | `[u8; 20]` fixed array | Rust uses fixed-size array |
| Map key type | `bytes` (hashable) | `[u8; 20]` (implements Hash) | Both work as HashMap keys |
| Duplicate detection | Raises `DuplicateRuleError` | Warns but continues | Python is stricter about duplicates |

**Behavioral note:** Both produce identical hashes for the same token sequence, verified by test at `hash_match.rs:248-251`.

---

## 6. Automatons

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/index.py:219-222`

```python
self.rules_automaton = match_aho.get_automaton()
self.fragments_automaton = USE_AHO_FRAGMENTS and match_aho.get_automaton()
self.starts_automaton = USE_RULE_STARTS and match_aho.get_automaton()
self.unknown_automaton = match_unknown.get_automaton()
```

**Automaton type:** `ahocorasick.Automaton` from pyahocorasick library

**File:** `reference/scancode-toolkit/src/licensedcode/match_aho.py`
```python
def get_automaton():
    return ahocorasick.Automaton(ahocorasick.STORE_ANY, ahocorasick.KEY_SEQUENCE)
```

**Adding sequences (index.py:427):**
```python
rules_automaton_add(tids=rule_token_ids, rid=rid)
# Stores (rid, istart, iend) as value
```

**Finalization (index.py:547):**
```python
self.rules_automaton.make_automaton()
```

**Unknown automaton (match_unknown.py:59-77):**
```python
def add_ngrams(automaton, tids, tokens, rule_length, len_legalese, ...):
    if rule_length >= unknown_ngram_length:
        tids_ngrams = tokenize.ngrams(tids, ngram_length=unknown_ngram_length)
        toks_ngrams = tokenize.ngrams(tokens, ngram_length=unknown_ngram_length)
        for tids_ngram, toks_ngram in zip(tids_ngrams, toks_ngrams):
            if is_good_tokens_ngram(toks_ngram, tids_ngram, len_legalese):
                automaton.add_word(tids_ngram)
```

### Rust Implementation

**File:** `src/license_detection/index/mod.rs:95-103`

```rust
pub rules_automaton: Automaton,
pub unknown_automaton: Automaton,
```

**Type alias:** `pub type Automaton = AhoCorasick;` (uses `aho-corasick` crate)

**File:** `src/license_detection/index/builder/mod.rs:476-491`
```rust
let rules_automaton = AhoCorasickBuilder::new()
    .match_kind(aho_corasick::MatchKind::Standard)
    .build(&rules_automaton_patterns)
    .expect("Failed to build rules automaton");

let unknown_automaton = if unknown_automaton_patterns.is_empty() {
    AhoCorasickBuilder::new()
        .build(std::iter::empty::<&[u8]>())
        .expect("Failed to build empty unknown automaton")
} else {
    let unique_patterns: HashSet<Vec<u8>> = unknown_automaton_patterns.into_iter().collect();
    AhoCorasickBuilder::new()
        .match_kind(aho_corasick::MatchKind::LeftmostFirst)
        .build(&unique_patterns)
        .expect("Failed to build unknown automaton")
};
```

### Key Differences

| Aspect | Python | Rust | Behavioral Impact |
|--------|--------|------|-------------------|
| Library | `pyahocorasick` | `aho-corasick` crate | Different implementations |
| Value storage | `(rid, istart, iend)` tuple | Pattern ID only | Rust uses separate `pattern_id_to_rid` mapping |
| Match kind | STORE_ANY | Standard / LeftmostFirst | Different semantics possible |
| `fragments_automaton` | Present (feature-flagged) | **Not implemented** | Experimental feature not ported |
| `starts_automaton` | Present (feature-flagged) | **Not implemented** | Experimental feature not ported |
| Pattern encoding | Token IDs as bytes | Token IDs → u8 pairs (little-endian) | Same encoding, different types |

**Critical behavioral difference:** Python stores `(rid, istart, iend)` in automaton values, allowing direct rid lookup. Rust stores only pattern_id, requiring the `pattern_id_to_rid` mapping to convert.

---

## 7. License Metadata

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/models.py:111-367`

```python
@attr.s(slots=True)
class License:
    key = attr.ib(repr=True)
    is_deprecated = attr.ib(default=False)
    replaced_by = attr.ib(default=[])
    language = attr.ib(default='en')
    short_name = attr.ib(default=None)
    name = attr.ib(default=None)
    category = attr.ib(default=None)
    owner = attr.ib(default=None)
    homepage_url = attr.ib(default=None)
    notes = attr.ib(default=None)
    is_builtin = attr.ib(default=True)
    is_exception = attr.ib(default=False)
    is_unknown = attr.ib(default=False)
    is_generic = attr.ib(default=False)
    spdx_license_key = attr.ib(default=None)
    other_spdx_license_keys = attr.ib(default=attr.Factory(list))
    # ... many more fields
    text = attr.ib(default=None)
```

**License loading (models.py:800-856):**
- `load_licenses()` loads from `.LICENSE` files with YAML frontmatter
- License objects stored in separate `licenses_db` dict, not in LicenseIndex

### Rust Implementation

**File:** `src/license_detection/models/license.rs:7-55`

```rust
pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_license_key: Option<String>,
    pub other_spdx_license_keys: Vec<String>,
    pub category: Option<String>,
    pub text: String,
    pub reference_urls: Vec<String>,
    pub notes: Option<String>,
    pub is_deprecated: bool,
    pub replaced_by: Vec<String>,
    pub minimum_coverage: Option<u8>,
    pub ignorable_copyrights: Option<Vec<String>>,
    pub ignorable_holders: Option<Vec<String>>,
    pub ignorable_authors: Option<Vec<String>>,
    pub ignorable_urls: Option<Vec<String>>,
    pub ignorable_emails: Option<Vec<String>>,
}
```

**Storage:** `licenses_by_key: HashMap<String, License>` in `LicenseIndex`

### Key Differences

| Aspect | Python | Rust | Notes |
|--------|--------|------|-------|
| Storage location | Separate `licenses_db` | Built into `LicenseIndex` | Rust consolidates all data in one struct |
| `language` field | Present | **Not implemented** | Python tracks license language |
| `short_name` | Present | **Not implemented** | Python has separate short and full name |
| `owner` | Present | **Not implemented** | Python tracks license owner |
| `homepage_url` | Present | **Not implemented** | Python tracks homepage |
| `is_exception` | Present | **Not implemented** | Python marks license exceptions |
| `is_builtin` | Present | **Not implemented** | Python distinguishes builtin vs custom |
| `is_generic` | Present | **Not implemented** | Python marks generic licenses |
| `reference_urls` | Built from multiple URL fields | Single field | Rust consolidates URLs |
| Ignorable fields | Separate fields for each | Same pattern | Both support same ignorable clue types |

---

## 8. Summary of Differences

### Structural Differences

1. **Index Organization**
   - Python: Uses lists indexed by rid for `sets_by_rid`, `msets_by_rid`, `high_postings_by_rid`
   - Rust: Uses HashMaps keyed by rid
   - **Impact**: Rust approach handles sparse rid space better, but may have slight lookup overhead

2. **Automaton Value Storage**
   - Python: Stores `(rid, istart, iend)` tuple directly in automaton
   - Rust: Stores pattern_id, requires separate `pattern_id_to_rid` mapping
   - **Impact**: Extra indirection in Rust, but same functionality

3. **Rule Storage**
   - Python: Stores rid on Rule object
   - Rust: Rid implicit from position in `rules_by_rid` vec
   - **Impact**: Rust doesn't need to store rid on each Rule

### Missing Features in Rust

1. **Not Implemented:**
   - `fragments_automaton` - Experimental ngram fragment matching
   - `starts_automaton` - Experimental rule start detection
   - `optimized` flag - Immutability enforcement
   - `rules_by_id` - Identifier → Rule mapping
   - `tokens_by_tid` - Reverse token lookup (debug only in Python)
   - `license_expression_object` - Cached parsed expression

2. **License Fields Not Implemented:**
   - `language`, `short_name`, `owner`, `homepage_url`
   - `is_exception`, `is_builtin`, `is_generic`

### Behavioral Differences

1. **Duplicate Detection**
   - Python: Raises `DuplicateRuleError` for duplicate rule hashes
   - Rust: Logs warning, continues processing
   - **Impact**: Python is stricter, Rust more lenient

2. **Token ID Range**
   - Python: Max 32767 tokens (signed 16-bit)
   - Rust: Max 65535 tokens (unsigned 16-bit)
   - **Impact**: Rust has 2x capacity

3. **Hash Key Type**
   - Python: Variable-length `bytes`
   - Rust: Fixed `[u8; 20]` array
   - **Impact**: Rust has compile-time size guarantee

### Potential Issues

1. **`sets_by_rid` sparse storage**: Python's list approach may have `None` entries for false-positive rules. Rust's HashMap only stores non-empty entries. Need to verify this doesn't cause lookup misses.

2. **Automaton match kind**: Python uses `STORE_ANY`, Rust uses `Standard` for rules and `LeftmostFirst` for unknown. Need to verify matching behavior is equivalent.

3. **Position array size**: Python uses `array('h')` (2 bytes), Rust uses `Vec<usize>` (8 bytes on 64-bit). Memory usage is 4x higher in Rust for position arrays.

### Recommendations

1. **Verify HashMap approach**: Test that `sets_by_rid.get(rid)` returns same results as Python's `sets_by_rid[rid]` for all rid values.

2. **Consider intbitset equivalent**: Python's `intbitset` is memory-efficient. Rust could benefit from a similar bitset implementation for large token sets.

3. **Implement `rules_by_id`** if needed for identifier-based rule lookup in the matching pipeline.

4. **Add `tokens_by_tid`** if debugging support is needed for token ID to string conversion.

5. **Review automaton match semantics** to ensure `Standard` and `LeftmostFirst` produce equivalent results to Python's `STORE_ANY`.
