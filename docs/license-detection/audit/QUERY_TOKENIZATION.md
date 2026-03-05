# Query Construction and Tokenization Audit

## Overview

This document compares the query construction and tokenization algorithms between Python ScanCode Toolkit and the Rust implementation.

**Reference Files:**
- Python: `reference/scancode-toolkit/src/licensedcode/tokenize.py`, `query.py`, `stopwords.py`
- Rust: `src/license_detection/tokenize.rs`, `src/license_detection/query/mod.rs`

---

## 1. Tokenization Algorithm

### Python Implementation

**File:** `tokenize.py:72-79`

```python
query_pattern = '[^_\W]+\+?[^_\W]*'
word_splitter = re.compile(query_pattern, re.UNICODE).findall
```

The pattern breakdown:
- `[^_\W]+` - One or more characters that are NOT underscore and NOT non-word (i.e., alphanumeric including Unicode)
- `\+?` - Optional plus sign (important for license names like "GPL2+")
- `[^_\W]*` - Zero or more alphanumeric characters (including Unicode)

**Key functions:**

| Function | Location | Purpose |
|----------|----------|---------|
| `query_tokenizer()` | tokenize.py:309-329 | Tokenizes query text without filtering stopwords |
| `index_tokenizer()` | tokenize.py:217-244 | Tokenizes rule/query text with stopword filtering |
| `index_tokenizer_with_stopwords()` | tokenize.py:247-306 | Tokenizes and tracks stopwords by position |

### Rust Implementation

**File:** `tokenize.rs:104-115`

```rust
static QUERY_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^_\W]+\+?[^_\W]*").expect("Invalid regex pattern"));
```

**Key functions:**

| Function | Location | Purpose |
|----------|----------|---------|
| `tokenize()` | tokenize.rs:132-150 | Tokenizes with stopword filtering |
| `tokenize_without_stopwords()` | tokenize.rs:166-184 | Tokenizes without stopword filtering |
| `tokenize_with_stopwords()` | tokenize.rs:346-374 | Tokenizes and tracks stopwords by position |

### Algorithm Comparison

| Aspect | Python | Rust | Difference? |
|--------|--------|------|-------------|
| Regex pattern | `[^_\W]+\+?[^_\W]*` | `[^_\W]+\+?[^_\W]*` | **Identical** |
| Unicode support | `re.UNICODE` flag | Unicode-aware by default | **Equivalent** |
| Lowercase conversion | In tokenizer functions | In tokenizer functions | **Identical** |
| Empty text handling | Returns empty generator | Returns empty Vec | **Equivalent** |

### Edge Case: Consecutive Plus Signs

**Python behavior** (tokenize.py:449-452 test):
```python
>>> tokenize("C++ and GPL+")
['c+', 'and', 'gpl+']
```

**Rust behavior** (tokenize.rs:569-572 test):
```rust
assert_eq!(tokenize("C++ and GPL+"), vec!["c+", "and", "gpl+"]);
```

**Status:** ✅ Identical

---

## 2. Stopword Handling

### Python Implementation

**File:** `stopwords.py:16-130`

Python uses a `frozenset` containing 71 stopwords organized by category:

1. **XML character references** (6): `amp`, `apos`, `gt`, `lt`, `nbsp`, `quot`
2. **HTML tags** (26): `a`, `abbr`, `alt`, `blockquote`, `body`, `br`, `class`, `div`, `em`, `h1-h5`, `hr`, `href`, `img`, `li`, `ol`, `p`, `pre`, `rel`, `script`, `span`, `src`, `td`, `th`, `tr`, `ul`
3. **Comment markers** (2): `rem`, `dnl`
4. **DocBook tags** (2): `para`, `ulink`
5. **HTML punctuation/entities** (28): `bdquo`, `bull`, `bullet`, `colon`, `comma`, `emdash`, `emsp`, `ensp`, `ge`, `hairsp`, `ldquo`, `ldquor`, `le`, `lpar`, `lsaquo`, `lsquo`, `lsquor`, `mdash`, `ndash`, `numsp`, `period`, `puncsp`, `raquo`, `rdquo`, `rdquor`, `rpar`, `rsaquo`, `rsquo`, `rsquor`, `sbquo`, `semi`, `thinsp`, `tilde`
6. **XML char entities** (2): `x3c`, `x3e`
7. **CSS** (8): `lists`, `side`, `nav`, `height`, `auto`, `border`, `padding`, `width`
8. **Perl PODs** (3): `head1`, `head2`, `head3`
9. **C literals** (1): `printf`
10. **Shell** (1): `echo`

**Total:** 71 stopwords

### Rust Implementation

**File:** `tokenize.rs:18-102`

Rust uses a `HashSet<&'static str>` with the exact same stopwords.

### Comparison

| Category | Python Count | Rust Count | Match? |
|----------|--------------|------------|--------|
| XML character references | 6 | 6 | ✅ |
| HTML tags | 26 | 26 | ✅ |
| Comment markers | 2 | 2 | ✅ |
| DocBook tags | 2 | 2 | ✅ |
| HTML punctuation/entities | 28 | 28 | ✅ |
| XML char entities | 2 | 2 | ✅ |
| CSS | 8 | 8 | ✅ |
| Perl PODs | 3 | 3 | ✅ |
| C literals | 1 | 1 | ✅ |
| Shell | 1 | 1 | ✅ |
| **Total** | **71** | **71** | ✅ |

**Status:** ✅ Identical stopword set

---

## 3. Unknown Token Tracking

### Python Implementation

**File:** `query.py:233-236, 461-479`

```python
# Data structure
self.unknowns_by_pos = {}  # line 236

# Tracking logic (lines 461-479)
if not started:
    # Unknown tokens before first known token -> magic -1 position
    unknowns_by_pos[-1] += 1
else:
    # Unknown tokens after known position
    unknowns_by_pos[known_pos] += 1
```

Key aspects:
- Unknowns tracked by **known token position**
- Count of unknowns AFTER each known position
- Magic `-1` key for unknowns before the first known token
- Unknowns_span created as Span of positions with unknowns (line 517)

### Rust Implementation

**File:** `query/mod.rs:80-88, 241-244`

```rust
pub unknowns_by_pos: HashMap<Option<i32>, usize>,  // line 88

// Tracking logic (lines 241-244)
if !started {
    *unknowns_by_pos.entry(None).or_insert(0) += 1;
} else {
    *unknowns_by_pos.entry(Some(known_pos)).or_insert(0) += 1;
}
```

### Comparison

| Aspect | Python | Rust | Difference? |
|--------|--------|------|-------------|
| Data structure | `dict` | `HashMap<Option<i32>, usize>` | Equivalent |
| Magic key for leading unknowns | `-1` | `None` | **Different representation** |
| Count semantics | Count AFTER position | Count AFTER position | **Identical** |
| Unknown span tracking | `unknowns_span: Span` | Not implemented | **Missing in Rust** |

**Potential Issue:** Rust uses `None` instead of `-1`. This is functionally equivalent but may cause issues if code expects the integer value.

**Missing Feature:** Rust does not implement `unknowns_span` which Python uses for intersection with query spans during scoring.

---

## 4. Query Building

### Python Implementation

**File:** `query.py:196-295`

```python
class Query:
    __slots__ = (
        'location', 'query_string', 'idx', 'line_threshold',
        'tokens', 'line_by_pos', 'unknowns_by_pos', 'unknowns_span',
        'stopwords_by_pos', 'shorts_and_digits_pos', 'query_runs',
        '_whole_query_run', 'high_matchables', 'low_matchables',
        'spdx_lid_token_ids', 'spdx_lines', 'has_long_lines',
        'is_binary', 'start_line',
    )
```

Construction flow:
1. `tokens_by_line()` yields lines of token IDs (lines 352-525)
2. `tokenize_and_build_runs()` processes lines (lines 527-556)
3. `_tokenize_and_build_runs()` builds query runs (lines 568-652)
4. `high_matchables` and `low_matchables` computed (lines 293-294)

### Rust Implementation

**File:** `query/mod.rs:60-145`

```rust
pub struct Query<'a> {
    pub text: String,
    pub tokens: Vec<u16>,
    pub line_by_pos: Vec<usize>,
    pub unknowns_by_pos: HashMap<Option<i32>, usize>,
    pub stopwords_by_pos: HashMap<Option<i32>, usize>,
    pub shorts_and_digits_pos: HashSet<usize>,
    pub high_matchables: HashSet<usize>,
    pub low_matchables: HashSet<usize>,
    pub has_long_lines: bool,
    pub is_binary: bool,
    pub(crate) query_run_ranges: Vec<(usize, Option<usize>)>,
    pub spdx_lines: Vec<(String, usize, usize)>,
    pub index: &'a LicenseIndex,
}
```

### Comparison

| Field | Python | Rust | Notes |
|-------|--------|------|-------|
| Input text | `location` or `query_string` | `text: String` | Rust unified into single field |
| Token IDs | `tokens: list` | `tokens: Vec<u16>` | Equivalent |
| Line tracking | `line_by_pos: list` | `line_by_pos: Vec<usize>` | Equivalent |
| Unknown tracking | `unknowns_by_pos: dict` | `unknowns_by_pos: HashMap` | Equivalent |
| Unknown span | `unknowns_span: Span` | **Missing** | Not implemented |
| Stopword tracking | `stopwords_by_pos: dict` | `stopwords_by_pos: HashMap` | Equivalent |
| Short/digit positions | `shorts_and_digits_pos: set` | `shorts_and_digits_pos: HashSet` | Equivalent |
| High matchables | `intbitset` | `HashSet<usize>` | Different data structure |
| Low matchables | `intbitset` | `HashSet<usize>` | Different data structure |
| SPDX token IDs | `spdx_lid_token_ids: list` | **Missing** | Computed inline in Rust |
| SPDX lines | `spdx_lines: list` | `spdx_lines: Vec` | Equivalent |
| Long lines flag | `has_long_lines: bool` | `has_long_lines: bool` | Equivalent |
| Binary flag | `is_binary: bool` | `is_binary: bool` | Equivalent |

---

## 5. QueryRun Splitting

### Python Implementation

**File:** `query.py:568-652`

```python
def _tokenize_and_build_runs(self, tokens_by_line, line_threshold=4):
    # ...
    for tokens in tokens_by_line:
        # Break in runs based on threshold of lines that are either:
        # - empty
        # - all unknown
        # - all low id/junk tokens
        # - made only of digits
        
        if len(query_run) > 0 and empty_lines >= line_threshold:
            query_runs_append(query_run)
            query_run = QueryRun(query=self, start=pos)
            empty_lines = 0
```

Breaking conditions:
- `line_threshold` consecutive "junk" lines (default 4)
- "Junk" = empty, all unknown, all digits, or no high-value tokens

Additionally, `break_long_lines()` splits lines with >25 tokens (line 710-717).

### Rust Implementation

**File:** `query/mod.rs:332-343`

```rust
// TODO: Query run splitting is currently disabled because it causes
// double-matching. The is_matchable() check with matched_qspans helps
// but doesn't fully prevent the issue. Further investigation needed.
let query_runs: Vec<(usize, Option<usize>)> = Vec::new();
```

### Comparison

| Aspect | Python | Rust | Difference? |
|--------|--------|------|-------------|
| Run splitting | Active | **DISABLED** | **Major difference** |
| Line threshold | 4 (text), 15 (bin) | N/A | N/A |
| Long line breaking | >25 tokens | Not implemented | **Missing** |
| break_on_boundaries() | Implemented | Not implemented | **Missing** |

**Impact:** This is a **significant behavioral difference**. Query run splitting affects:
1. Match granularity
2. Performance (more/smaller runs vs fewer/larger runs)
3. Match candidate selection

The Rust implementation treats the entire query as a single run, which may cause different matching behavior.

---

## 6. Line Position Tracking

### Python Implementation

**File:** `query.py:369-429`

```python
for line_num, line in qlines:
    # ...
    for token in query_tokenizer(line):
        tid = dic_get(token)
        is_stopword = token in STOPWORDS
        
        if tid is not None and not is_stopword:
            # this is a known token
            known_pos += 1
            started = True
            line_by_pos_append(line_num)  # Track line number
```

Line numbers come from `query_lines()` (tokenize.py:28-70) which:
1. For files: Uses `numbered_text_lines()` from `textcode.analysis`
2. For strings: Uses `enumerate(query_string.splitlines(keepends), start_line)`

### Rust Implementation

**File:** `query/mod.rs:220-235`

```rust
let mut current_line = 1usize;

for line in text.lines() {
    let line_trimmed = line.trim();
    // ...
    for token in tokenize_without_stopwords(line_trimmed) {
        if let Some(tid) = tid_opt {
            known_pos += 1;
            started = true;
            tokens.push(tid);
            line_by_pos.push(current_line);
```

### Comparison

| Aspect | Python | Rust | Difference? |
|--------|--------|------|-------------|
| Line enumeration | Via `enumerate()` | Manual counter | Equivalent |
| Start line | Configurable (default 1) | Fixed at 1 | **Less flexible** |
| Line trimming | Optional `strip` param | Always trimmed | **Different** |
| Empty line handling | Yields empty token list | Skipped naturally | Equivalent |

**Potential Issue:** Rust doesn't support configurable `start_line`. Python allows this for special cases.

**Potential Issue:** Python's `query_lines()` supports `strip=False` to preserve trailing newlines. Rust always trims lines.

---

## 7. Matchable Positions (High/Low)

### Python Implementation

**File:** `query.py:290-294`

```python
len_legalese = idx.len_legalese
tokens = self.tokens

# sets of known token positions initialized after query tokenization:
self.high_matchables = intbitset([p for p, t in enumerate(tokens) if t < len_legalese])
self.low_matchables = intbitset([p for p, t in enumerate(tokens) if t >= len_legalese])
```

**Definition:**
- **High matchables**: Token IDs < `len_legalese` (legalese words like "license", "copyright", etc.)
- **Low matchables**: Token IDs >= `len_legalese` (common English words)

### Rust Implementation

**File:** `query/mod.rs:318-330`

```rust
let high_matchables: HashSet<usize> = tokens
    .iter()
    .enumerate()
    .filter(|(_pos, tid)| (**tid as usize) < len_legalese)
    .map(|(pos, _tid)| pos)
    .collect();

let low_matchables: HashSet<usize> = tokens
    .iter()
    .enumerate()
    .filter(|(_pos, tid)| (**tid as usize) >= len_legalese)
    .map(|(pos, _tid)| pos)
    .collect();
```

### Comparison

| Aspect | Python | Rust | Difference? |
|--------|--------|------|-------------|
| High threshold | `tid < len_legalese` | `tid < len_legalese` | **Identical** |
| Low threshold | `tid >= len_legalese` | `tid >= len_legalese` | **Identical** |
| Data structure | `intbitset` | `HashSet<usize>` | Different implementation |

**Note:** `intbitset` is a memory-efficient integer set. `HashSet<usize>` is functionally equivalent but may have different memory characteristics.

---

## 8. SPDX Line Detection

### Python Implementation

**File:** `query.py:254-268, 486-507`

```python
# SPDX token IDs computed once (lines 254-264)
dic_get = idx.dictionary.get
spdxid = [dic_get(u'spdx'), dic_get(u'license'), dic_get(u'identifier')]
nuget_spdx_id = [dic_get(u'licenses'), dic_get(u'nuget'), dic_get(u'org')]

# Detection during tokenization (lines 491-507)
spdx_start_offset = None
if line_tokens[:3] in spdx_lid_token_ids:
    spdx_start_offset = 0
elif line_tokens[1:4] in spdx_lid_token_ids:
    spdx_start_offset = 1
elif line_tokens[2:5] in spdx_lid_token_ids:
    spdx_start_offset = 2

if spdx_start_offset is not None:
    spdx_prefix, spdx_expression = split_spdx_lid(line)
    spdx_text = ''.join([spdx_prefix or '', spdx_expression])
    spdx_start_known_pos = line_first_known_pos + spdx_start_offset
    if spdx_start_known_pos <= line_last_known_pos:
        self.spdx_lines.append((spdx_text, spdx_start_known_pos, line_last_known_pos))
```

**Key aspects:**
- Detects `["spdx", "license", "identifier"]` or `["spdx", "licence", "identifier"]`
- Also detects NuGet format: `["licenses", "nuget", "org"]`
- Checks at positions 0, 1, or 2 in the line (allows for comment prefix)
- Uses `split_spdx_lid()` from `match_spdx_lid.py` to extract expression

### Rust Implementation

**File:** `query/mod.rs:260-312`

```rust
let spdx_start_offset = if tokens_lower.len() >= 3 {
    let first_three: Vec<&str> = tokens_lower.iter().take(3).map(|s| s.as_str()).collect();
    let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
        || first_three == ["spdx", "licence", "identifier"]
        || first_three == ["licenses", "nuget", "org"];
    if is_spdx_prefix {
        Some(0)
    } else if tokens_lower.len() >= 4 {
        // Check positions 1 and 2 similarly...
    }
}
```

### Comparison

| Aspect | Python | Rust | Difference? |
|--------|--------|------|-------------|
| Token ID lookup | Pre-computed IDs | String comparison | **Different approach** |
| SPDX variants | Both "license"/"licence" | Both "license"/"licence" | ✅ Identical |
| NuGet support | Yes | Yes | ✅ Identical |
| Offset detection | 0, 1, 2 | 0, 1, 2 | ✅ Identical |
| Expression extraction | `split_spdx_lid()` | Line text stored directly | **Different** |

**Potential Issue:** Python uses `split_spdx_lid()` regex to properly extract the SPDX expression from the line text. Rust stores the entire trimmed line. This may cause issues if the line contains additional content before/after the expression.

---

## 9. Additional Python Features Not in Rust

### 9.1 `break_on_boundaries()`

**File:** `query.py:655-704`

```python
def break_on_boundaries(query_run):
    """Yield more query runs broken down on boundaries discovered
    from matched rules and matched rule starts and ends."""
    if len(query_run) < 150:
        yield query_run
    else:
        # Use starts_automaton to find rule boundaries
        matched_starts = get_matched_starts(qr_tokens, qr_start, automaton=idx.starts_automaton)
        # ...
```

**Status:** Not implemented in Rust. This is called via `refine_runs()`.

### 9.2 `numbered_text_lines()` Integration

**File:** `textcode/analysis.py:51-176`

Python has sophisticated file type detection:
- PDF text extraction
- JavaScript source map extraction
- Font database (SFDB) extraction
- Binary string extraction
- HTML/XML demarkup

**Status:** Rust uses simpler approach - just `text.lines()`.

### 9.3 `required_phrase_tokenizer()`

**File:** `tokenize.py:90-119`

Used for parsing `{{required phrase}}` markers in license rules.

**Status:** Partially implemented in Rust (`parse_required_phrase_spans`).

---

## 10. Summary of Differences

### Critical Differences

| Issue | Python | Rust | Impact |
|-------|--------|------|--------|
| Query run splitting | Active | **Disabled** | May cause different match behavior |
| Unknown span tracking | `unknowns_span` | Missing | May affect scoring |
| Line position start | Configurable | Fixed at 1 | Edge case differences |
| SPDX expression extraction | `split_spdx_lid()` regex | Direct line storage | May include extra content |

### Minor Differences

| Issue | Python | Rust | Impact |
|-------|--------|------|--------|
| Leading unknowns key | `-1` | `None` | Internal representation |
| Matchable set type | `intbitset` | `HashSet` | Memory/performance |
| SPDX token detection | Token ID comparison | String comparison | Performance |
| Long line breaking | >25 tokens per line | Not implemented | Minified files |

### Missing Features

1. **`break_on_boundaries()`** - Rule boundary detection for large query runs
2. **`unknowns_span`** - Span tracking for unknown token positions
3. **`refine_runs()`** - Query run refinement based on matched rules
4. **File type-aware line extraction** - PDF, source maps, etc.

---

## 11. Recommendations

1. **Re-enable query run splitting** with proper testing to ensure no double-matching
2. **Implement `unknowns_span`** for proper scoring calculation
3. **Add SPDX expression extraction** using similar regex to Python's `split_spdx_lid()`
4. **Support configurable `start_line`** for special cases
5. **Consider implementing `break_on_boundaries()`** for better match granularity on large files

---

## 12. Test Coverage Comparison

### Python Tests

Located in:
- `tokenize.py` doctests (lines 97-111, 224-237, etc.)
- `tests/textcode/test_analysis.py`

### Rust Tests

Located in:
- `tokenize.rs:376-754` (unit tests)
- `query/test.rs:1-773` (query tests)

Both implementations have comprehensive test coverage for tokenization. Rust tests are more extensive for query construction.
