# Python License Detection Pipeline Architecture

This document describes the high-level architecture and flow of the license detection pipeline in the Python ScanCode Toolkit reference implementation at `reference/scancode-toolkit/src/licensedcode/`.

## Directory Structure Overview

```
licensedcode/
├── __init__.py              # Global constants (MAX_DIST, MIN_MATCH_LENGTH, etc.)
├── cache.py                 # Index caching and loading (LicenseCache)
├── detection.py             # LicenseDetection, detection analysis and combination
├── index.py                 # LicenseIndex - main index and matching orchestration
├── match.py                 # LicenseMatch, match merging and filtering
├── match_aho.py             # Aho-Corasick exact matching strategy
├── match_hash.py            # Hash-based perfect matching strategy
├── match_seq.py             # Sequence alignment matching strategy
├── match_set.py             # Set/multiset candidate ranking
├── match_spdx_lid.py        # SPDX-License-Identifier expression matching
├── match_unknown.py         # Unknown license detection
├── models.py                # License, Rule data models
├── query.py                 # Query, QueryRun - input text processing
├── spans.py                 # Span - integer range/bitset for positions
├── tokenize.py              # Text tokenization utilities
├── seq.py                   # Sequence alignment algorithms (Myers diff)
├── dmp.py                   # Diff-Match-Patch algorithm
├── stopwords.py             # Stopword definitions
├── legalese.py              # Legal language patterns
├── plugin_license.py        # ScanPlugin entry point for scanning
├── license_db.py            # License database utilities
├── languages.py             # Language detection support
├── required_phrases.py      # Required phrase matching
├── frontmatter.py           # YAML frontmatter parsing
└── data/
    ├── licenses/            # License text files (*.LICENSE)
    └── rules/               # Rule text files (*.RULE)
```

## Main Entry Points

### Primary Entry Point: `LicenseIndex.match()`

**File:** `index.py:898-964`

The main entry point for license detection:

```python
def match(
    self,
    location=None,
    query_string=None,
    min_score=0,
    as_expression=False,
    expression_symbols=None,
    approximate=True,
    unknown_licenses=False,
    deadline=sys.maxsize,
    **kwargs,
) -> List[LicenseMatch]:
```

**Parameters:**
- `location`: File path to scan
- `query_string`: Text string to scan (alternative to location)
- `min_score`: Minimum score threshold (0-100)
- `as_expression`: Treat text as SPDX expression
- `approximate`: Enable approximate/fuzzy matching
- `unknown_licenses`: Enable unknown license detection

### Secondary Entry Point: `LicenseIndex.match_query()`

**File:** `index.py:966-1151`

Processes a pre-built `Query` object through the matching pipeline.

### Plugin Entry Point: `LicenseScanner`

**File:** `plugin_license.py:52-181`

The ScanPlugin that integrates license detection into the scanning pipeline:

```python
@scan_impl
class LicenseScanner(ScanPlugin):
    def get_scanner(self, ...):
        from scancode.api import get_licenses
        return partial(get_licenses, ...)
```

## Core Pipeline Stages

### Pipeline Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           INPUT STAGE                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  File/Text → build_query() → Query object                                   │
│  (query.py:111-152)                                                         │
│                                                                              │
│  Query contains:                                                            │
│  - tokens: List[int] (token IDs)                                            │
│  - line_by_pos: List[int] (line numbers)                                    │
│  - query_runs: List[QueryRun] (text chunks)                                 │
│  - spdx_lines: Special SPDX-License-Identifier lines                        │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        HASH MATCH (Stage 1)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  match_hash.py:59-87                                                        │
│                                                                              │
│  If query hash matches a rule hash exactly → return immediately             │
│  Fast path for perfect matches                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     SPDX-ID MATCH (Stage 2)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  match_spdx_lid.py:65-119                                                   │
│                                                                              │
│  Detects "SPDX-License-Identifier: <expr>" lines                            │
│  Parses expression using license_expression library                         │
│  Creates synthetic SpdxRule for matched expression                          │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    AHO-CORASICK MATCH (Stage 3)                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  match_aho.py:84-138                                                        │
│                                                                              │
│  Exact string matching using Aho-Corasick automaton                         │
│  Matches all rules at once in a single pass                                 │
│  Most common matching strategy for license text                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                 APPROXIMATE SEQUENCE MATCH (Stage 4)                         │
├─────────────────────────────────────────────────────────────────────────────┤
│  match_seq.py:48-156 + match_set.py:244-367                                 │
│                                                                              │
│  1. Candidate Selection (match_set.py):                                     │
│     - Compute token set/multiset intersections                              │
│     - Rank by resemblance and containment                                   │
│     - Select top N candidates                                               │
│                                                                              │
│  2. Sequence Alignment (match_seq.py):                                      │
│     - Myers diff or custom sequence matching                                │
│     - Multiple local alignments per candidate                               │
│     - Find matching blocks between query and rule                           │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MATCH REFINEMENT STAGE                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  match.py:refine_matches()                                                  │
│                                                                              │
│  1. Filter false positives                                                  │
│  2. Filter contained matches (small matches inside larger)                  │
│  3. Filter overlapping matches                                              │
│  4. Filter spurious matches                                                 │
│  5. Merge nearby matches                                                    │
│  6. Filter below min_score                                                  │
│  7. Check required phrases                                                  │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    DETECTION CREATION STAGE                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  detection.py:LicenseDetection.from_matches()                               │
│                                                                              │
│  - Analyze matches for unknown licenses, extra words                        │
│  - Combine matches into detection with license expression                   │
│  - Handle license references to other files                                 │
│  - Create unique identifier                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Data Structures

### LicenseIndex (`index.py:131-1203`)

The central index structure holding all license rules:

```python
class LicenseIndex:
    __slots__ = (
        'dictionary',           # {token_string: token_id}
        'rules_by_id',          # {rule_identifier: Rule}
        'rules_by_rid',         # [Rule] indexed by numeric rid
        'tids_by_rid',          # [[token_id]] rule token sequences
        'high_postings_by_rid', # [{token_id: [positions]}] inverted index
        'sets_by_rid',          # [intbitset] token sets per rule
        'msets_by_rid',         # [Counter] token multisets per rule
        'rid_by_hash',          # {hash: rid} for hash matching
        'rules_automaton',      # Aho-Corasick automaton for exact match
        'unknown_automaton',    # Automaton for unknown license detection
        'regular_rids',         # Set of regular rule IDs
        'false_positive_rids',  # Set of false positive rule IDs
        'approx_matchable_rids', # Set of rules for sequence matching
    )
```

### Query (`query.py:155-939`)

Represents input text being scanned:

```python
class Query:
    __slots__ = (
        'location',          # File path
        'query_string',      # Input text
        'tokens',            # [int] token IDs
        'line_by_pos',       # [int] line numbers
        'unknowns_by_pos',   # {pos: count} unknown tokens
        'stopwords_by_pos',  # {pos: count} stopword tokens
        'query_runs',        # [QueryRun] text chunks
        'high_matchables',   # intbitset of high-value token positions
        'low_matchables',    # intbitset of low-value token positions
        'spdx_lines',        # [(text, start, end)] SPDX lines
    )
```

### QueryRun (`query.py`)

A slice/chunk of a Query for matching:

```python
class QueryRun:
    query: Query        # Parent query
    start: int          # Start position in query.tokens
    end: int            # End position in query.tokens
    matchables: intbitset  # Matchable positions
```

### Rule (`models.py:571-1060`)

A license text pattern to match against:

```python
@attr.s
class Rule:
    identifier: str          # Unique ID (e.g., "gpl-2.0_12.RULE")
    license_expression: str  # License expression
    text: str               # Rule text
    length: int             # Token count
    relevance: int          # Relevance score (0-100)
    
    # Flags
    is_license_text: bool
    is_license_notice: bool
    is_license_tag: bool
    is_license_intro: bool
    is_license_reference: bool
    is_false_positive: bool
    
    # Thresholds for matching
    minimum_coverage: float
    _minimum_containment: float
```

### LicenseMatch (`match.py:151-259`)

A match between query text and a rule:

```python
@attr.s
class LicenseMatch:
    rule: Rule              # Matched rule
    qspan: Span             # Query positions matched
    ispan: Span             # Index/rule positions matched
    hispan: Span            # High-value token positions
    matcher: str            # Matching strategy used
    matcher_order: int      # Matcher precedence
    start_line: int         # Match start line (1-based)
    end_line: int           # Match end line (1-based)
    query: Query            # Parent query
    discard_reason: DiscardReason  # If filtered
```

### LicenseDetection (`detection.py:164-511`)

A combined detection from one or more matches:

```python
@attr.s
class LicenseDetection:
    license_expression: str           # Combined expression
    license_expression_spdx: str      # SPDX format
    matches: List[LicenseMatch]       # Underlying matches
    detection_log: List[DetectionRule] # How detection was built
    identifier: str                   # Unique ID
    file_region: FileRegion           # Location in file
```

### Span (`spans.py:42-474`)

Efficient integer set/range for token positions:

```python
class Span(Set):
    _set: intbitset  # Bitmap of integers
    
    # Supports set operations: union, intersection, difference
    # Tracks start, end, and all positions
```

## Matching Strategies

### 1. Hash Matching (`match_hash.py`)

**Purpose:** Instant perfect match detection

**How it works:**
1. Compute SHA1 hash of query token sequence
2. Look up hash in `rid_by_hash` dictionary
3. If found, create match immediately

**Matcher:** `1-hash` (order: 0)

### 2. SPDX-ID Matching (`match_spdx_lid.py`)

**Purpose:** Parse SPDX-License-Identifier expressions

**How it works:**
1. Collect lines starting with "SPDX-License-Identifier:"
2. Parse expression using `license_expression` library
3. Create synthetic `SpdxRule` with parsed expression
4. Return match for entire expression

**Matcher:** `1-spdx-id` (order: 2)

### 3. Aho-Corasick Matching (`match_aho.py`)

**Purpose:** Exact multi-pattern string matching

**How it works:**
1. Build Aho-Corasick automaton from all rule token sequences
2. Stream query tokens through automaton
3. Collect all matches (may overlap)
4. Filter matches not in matchable positions
5. Create `LicenseMatch` for each

**Matcher:** `2-aho` (order: 1)

### 4. Sequence Matching (`match_seq.py`)

**Purpose:** Approximate/fuzzy matching for modified license text

**How it works:**
1. **Candidate Selection** (via `match_set.py`):
   - Compute token set intersection (which tokens in common)
   - Compute token multiset intersection (frequency counts)
   - Calculate resemblance = intersection_size / union_size
   - Calculate containment = intersection_size / rule_size
   - Rank by (resemblance², containment, matched_length)
   - Select top N candidates

2. **Sequence Alignment**:
   - For each candidate rule, run Myers diff-style alignment
   - Use high-postings (positions of "legalese" tokens) for efficiency
   - Find matching blocks (contiguous aligned regions)
   - Create matches for each matching block

**Matcher:** `3-seq` (order: 3)

### 5. Unknown License Matching (`match_unknown.py`)

**Purpose:** Detect unknown/unrecognized license text

**How it works:**
1. Find regions not matched by other matchers
2. Look for license-like patterns using ngrams
3. Create "unknown" matches for detected regions

**Matcher:** `5-undetected` (order: 4)

## Match Refinement Pipeline

**File:** `match.py:1200-1800`

The refinement process filters and merges raw matches:

```
refine_matches(matches, query, min_score, filter_false_positive, merge)
    │
    ├─► filter_matches_below_min_score()     # Score threshold
    ├─► filter_matches_below_min_coverage()  # Coverage threshold
    ├─► filter_false_positive_matches()      # FP rules
    ├─► filter_matches_missing_required_phrases()  # Required content
    ├─► filter_single_token_matches()        # Too short
    ├─► filter_spurious_matches()            # Gibberish
    ├─► filter_contained_matches()           # Contained in larger
    ├─► filter_overlapping_matches()         # Overlap resolution
    └─► merge_matches()                      # Combine nearby matches
```

## Detection Analysis

**File:** `detection.py:1452-2100`

After matches are found, they're analyzed to create detections:

```python
def analyze_detection(license_matches, package_license=False):
    """
    Classify matches into detection categories:
    - perfect-detection
    - unknown-intro-before-detection
    - unknown-file-reference-local
    - license-clues
    - low-quality-matches
    - possible-false-positive
    - undetected-license
    """
```

Key analysis functions:
- `is_correct_detection()` - Perfect 100% coverage matches
- `has_unknown_matches()` - Contains unknown license keys
- `is_false_positive()` - Likely spurious match
- `has_references_to_local_files()` - References other files
- `is_unknown_intro()` - Unknown license introduction text

## Tokenization

**File:** `tokenize.py`

### Query Tokenization

```python
def query_lines(location, query_string, start_line=1):
    """Yield (line_number, text) for each line."""
    
def query_tokenizer(text, stopwords=STOPWORDS):
    """
    Tokenize for matching:
    - Split on whitespace and punctuation
    - Lowercase
    - Remove stopwords
    - Return tokens
    """
```

### Index Tokenization

```python
def index_tokenizer(text, stopwords=STOPWORDS):
    """Tokenize license rule text for indexing."""
```

### Required Phrase Tokenization

```python
def required_phrase_tokenizer(text):
    """
    Handle {{phrase}} markers in rules.
    Used for required content matching.
    """
```

## Key Algorithms

### Candidate Ranking (`match_set.py:244-367`)

Two-stage probabilistic ranking:

1. **Token Set Intersection:**
   - Use intbitset for efficient intersection
   - Calculate Jaccard-like similarity
   - Filter by minimum intersection size

2. **Token Multiset Intersection:**
   - Consider token frequencies
   - Calculate match length approximation
   - Final ranking by (resemblance², containment, length)

### Sequence Alignment (`seq.py`)

Custom Myers diff variant optimized for license matching:

```python
def match_blocks(a, b, a_start, a_end, b2j, len_good, matchables):
    """
    Find matching blocks between sequences a and b.
    Uses b2j (inverted index) for efficiency.
    Returns [(a_pos, b_pos, length), ...]
    """
```

### Match Merging (`match.py`)

Combines nearby matches to the same rule:

```python
def merge_matches(matches, max_dist=50):
    """
    Merge matches that:
    - Are to the same rule
    - Are within max_dist tokens
    - Maintain correct sequence
    """
```

## Index Construction

**File:** `index.py:270-577`

```python
def _add_rules(self, rules, _legalese, _spdx_tokens, _license_tokens):
    """
    Build index from Rule objects:
    
    1. Create token dictionary (string → int ID)
    2. Tokenize each rule → token ID sequence
    3. Build Aho-Corasick automaton from sequences
    4. Compute hashes for perfect match lookup
    5. Build inverted index (high_postings)
    6. Build token sets/multisets for candidate ranking
    7. Compute rule thresholds
    8. Finalize automatons
    """
```

## Caching

**File:** `cache.py:39-200`

```python
class LicenseCache:
    """
    Pickled cache of:
    - db: License database
    - index: LicenseIndex
    - licensing: Licensing object for expression parsing
    - spdx_symbols: SPDX license symbols
    """
    
    @staticmethod
    def load_or_build(only_builtin, licensedcode_cache_dir, force, ...):
        """
        Load from cache if exists and valid.
        Otherwise build and cache.
        Uses file locking for multi-process safety.
        """
```

## Important Configuration Constants

**File:** `__init__.py`

```python
MAX_DIST = 50           # Max distance for merging matches
MIN_MATCH_LENGTH = 4    # Minimum tokens for a valid match
MIN_MATCH_HIGH_LENGTH = 3  # For high-value tokens
SMALL_RULE = 15         # Threshold for "small" rules
TINY_RULE = 6           # Threshold for "tiny" rules
```

**File:** `query.py`

```python
MAX_TOKEN_PER_LINE = 25   # For long line breaking
LINES_THRESHOLD = 4       # Empty lines to break query runs
```

**File:** `detection.py`

```python
IMPERFECT_MATCH_COVERAGE_THR = 100  # Below this = imperfect
CLUES_MATCH_COVERAGE_THR = 60       # Below this = license clue
LOW_RELEVANCE_THRESHOLD = 70        # Below this = low relevance
```

## Summary

The Python license detection pipeline is a sophisticated multi-stage system:

1. **Input Processing**: Tokenize input, build Query with QueryRuns
2. **Fast Path**: Hash matching for exact duplicates
3. **Expression Matching**: SPDX-License-Identifier parsing
4. **Exact Matching**: Aho-Corasick multi-pattern matching
5. **Approximate Matching**: Probabilistic candidate ranking + sequence alignment
6. **Refinement**: Filter, merge, and validate matches
7. **Detection Creation**: Combine matches into meaningful detections

The key insight is the **layered matching strategy** - fast exact matching handles most cases, while expensive approximate matching only runs when necessary on carefully selected candidates.
