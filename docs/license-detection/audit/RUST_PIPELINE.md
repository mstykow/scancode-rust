# Rust License Detection Pipeline Architecture

This document provides a comprehensive overview of the Rust license detection pipeline implemented in `src/license_detection/`.

## Directory Structure Overview

```
src/license_detection/
├── mod.rs                    # Module root, LicenseDetectionEngine entry point
├── aho_match.rs              # Aho-Corasick exact matching (Phase 1c)
├── hash_match.rs             # Hash-based exact matching (Phase 1a)
├── unknown_match.rs          # Unknown license detection (Phase 5)
├── spdx_lid/
│   └── mod.rs                # SPDX-License-Identifier matching (Phase 1b)
├── seq_match/
│   ├── mod.rs                # Sequence matching orchestration
│   ├── candidates.rs         # Candidate selection via set similarity
│   └── matching.rs           # Sequence alignment matching (Phase 2-4)
├── match_refine/
│   ├── mod.rs                # Match refinement orchestration
│   ├── false_positive.rs     # False positive filtering
│   ├── filter_low_quality.rs # Quality threshold filtering
│   ├── handle_overlaps.rs    # Overlap resolution
│   └── merge.rs              # Match merging logic
├── detection/
│   ├── mod.rs                # Detection grouping and assembly
│   ├── analysis.rs           # Detection classification
│   ├── grouping.rs           # Region-based grouping
│   ├── identifier.rs         # Detection identifier generation
│   └── types.rs              # LicenseDetection, DetectionGroup structs
├── query/
│   └── mod.rs                # Query tokenization and management
├── index/
│   ├── mod.rs                # LicenseIndex definition
│   ├── builder/
│   │   └── mod.rs            # Index construction
│   ├── dictionary.rs         # Token dictionary
│   └── token_sets.rs         # Set/mset computation
├── models/
│   ├── mod.rs                # Model exports
│   ├── license_match.rs      # LicenseMatch struct
│   ├── license.rs            # License metadata
│   └── rule.rs               # Rule metadata
├── rules/
│   ├── mod.rs                # Rule loading orchestration
│   ├── loader.rs             # .RULE and .LICENSE file parsing
│   ├── legalese.rs           # Legalese token definitions
│   └── thresholds.rs         # Rule thresholds
├── expression/
│   ├── mod.rs                # Expression AST and utilities
│   ├── parse.rs              # Expression parser
│   └── simplify.rs           # Expression simplification
├── tokenize.rs               # Text tokenization
├── spans.rs                  # Span utilities for coverage
├── spdx_mapping/
│   └── mod.rs                # ScanCode-to-SPDX key mapping
└── investigation/            # Test cases for debugging
```

## Main Entry Points

### Primary Entry Point: `LicenseDetectionEngine`

**File:** `src/license_detection/mod.rs:76-113`

```rust
pub struct LicenseDetectionEngine {
    index: Arc<index::LicenseIndex>,
    spdx_mapping: SpdxMapping,
}

impl LicenseDetectionEngine {
    pub fn new(rules_path: &Path) -> Result<Self>
    pub fn detect(&self, text: &str, unknown_licenses: bool) -> Result<Vec<LicenseDetection>>
    pub fn detect_matches(&self, text: &str, unknown_licenses: bool) -> Result<Vec<LicenseMatch>>
}
```

The `LicenseDetectionEngine` is the main entry point for license detection. It:
1. Loads rules and builds the index at construction
2. Provides `detect()` for full detection with grouping
3. Provides `detect_matches()` for raw match output

### Alternative Entry Points

For direct matcher access:

| Function | File | Purpose |
|----------|------|---------|
| `hash_match()` | `hash_match.rs:72` | Exact hash matching |
| `spdx_lid_match()` | `spdx_lid/mod.rs:265` | SPDX-License-Identifier matching |
| `aho_match()` | `aho_match.rs:76` | Aho-Corasick pattern matching |
| `seq_match_with_candidates()` | `seq_match/matching.rs` | Sequence alignment matching |
| `unknown_match()` | `unknown_match.rs:20` | Unknown license detection |

## Pipeline Flow Diagram

```
                            ┌──────────────────────────────────────┐
                            │         Input Text                   │
                            └─────────────────┬────────────────────┘
                                              │
                                              ▼
                            ┌──────────────────────────────────────┐
                            │     1. Create Query                   │
                            │   query/mod.rs:163                    │
                            │   - Tokenize text                     │
                            │   - Build token ID sequence           │
                            │   - Track line positions              │
                            │   - Detect SPDX-License-Identifier    │
                            └─────────────────┬────────────────────┘
                                              │
                                              ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           PHASE 1: EXACT MATCHING                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐                                                            │
│  │  1a. Hash Match │  hash_match.rs:72                                          │
│  │  (Priority 1)   │  - Compute SHA1 of query tokens                            │
│  └────────┬────────┘  - Lookup in rid_by_hash                                   │
│           │            - Return immediately if match found                       │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │ 1b. SPDX-LID    │  spdx_lid/mod.rs:265                                       │
│  │  (Priority 1)   │  - Parse SPDX-License-Identifier lines                     │
│  └────────┬────────┘  - Resolve license expressions                             │
│           │            - Create tag matches                                     │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │  1c. Aho Match  │  aho_match.rs:76                                           │
│  │  (Priority 2)   │  - Multi-pattern Aho-Corasick automaton                    │
│  └────────┬────────┘  - Find all rule token sequences                           │
│           │            - Verify matchable positions                             │
│           ▼                                                                     │
└─────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
                            ┌──────────────────────────────────────┐
                            │   Check: Is matchable region left?    │
                            │   query/mod.rs:826                    │
                            └─────────────────┬────────────────────┘
                                              │
                          ┌───────────────────┴───────────────────┐
                          │                                       │
                          ▼                                       ▼
              ┌───────────────────────┐           ┌───────────────────────┐
              │   Skip Phases 2-4     │           │  Continue to Phase 2  │
              │   (all covered)       │           │                       │
              └───────────┬───────────┘           └───────────┬───────────┘
                          │                                   │
                          └───────────────┬───────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                      PHASE 2-4: SEQUENCE MATCHING                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  Phase 2: Near-Duplicate Detection                                       │    │
│  │  seq_match/mod.rs:221                                                    │    │
│  │  - HIGH_RESEMBLANCE_THRESHOLD = 0.8                                      │    │
│  │  - compute_candidates_with_msets(near_dupe=true)                         │    │
│  │  - Find rules with >= 80% token overlap                                  │    │
│  │  - seq_match_with_candidates() on each                                   │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                     │                                           │
│                                     ▼                                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  Phase 3: Regular Sequence Matching                                      │    │
│  │  seq_match/mod.rs:247                                                    │    │
│  │  - compute_candidates_with_msets(near_dupe=false)                        │    │
│  │  - Top 70 candidates by set similarity                                   │    │
│  │  - seq_match_with_candidates() for alignment                             │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                     │                                           │
│                                     ▼                                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  Phase 4: Query Run Matching                                             │    │
│  │  seq_match/mod.rs:264                                                    │    │
│  │  - For each query run (sub-region of text)                              │    │
│  │  - compute_candidates_with_msets() on that region                        │    │
│  │  - seq_match_with_candidates() on each query run                         │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                     │                                           │
│                                     ▼                                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  Merge all sequence matches                                              │    │
│  │  match_refine/mod.rs:291                                                 │    │
│  │  - merge_overlapping_matches()                                           │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         PHASE 5: MATCH REFINEMENT                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  match_refine/mod.rs:136                                                        │
│                                                                                 │
│  1. Initial refinement (without false positive filter)                         │
│     - merge_overlapping_matches()                                              │
│     - filter_matches_missing_required_phrases()                                │
│     - filter_spurious_matches()                                                │
│     - filter_below_rule_minimum_coverage()                                     │
│     - filter_too_short_matches()                                               │
│     - filter_short_matches_scattered_on_too_many_lines()                       │
│     - filter_invalid_matches_to_single_word_gibberish()                        │
│     - filter_contained_matches()                                               │
│     - filter_overlapping_matches()                                             │
│     - restore_non_overlapping()                                                │
│                                                                                 │
│  2. Unknown detection (if enabled)                                             │
│     unknown_match.rs:20                                                         │
│     - Split weak matches                                                       │
│     - Find uncovered regions                                                   │
│     - Match ngrams from unknown_automaton                                      │
│     - Filter contained unknown matches                                         │
│                                                                                 │
│  3. Final refinement (with false positive filter)                              │
│     - filter_false_positive_matches()                                          │
│     - filter_false_positive_license_lists_matches()                            │
│     - update_match_scores()                                                    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                       PHASE 6: DETECTION ASSEMBLY                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  detection/mod.rs                                                               │
│                                                                                 │
│  1. sort_matches_by_line() - group_matches_by_region/mod.rs:12                 │
│     - Sort by start_line ascending                                             │
│                                                                                 │
│  2. group_matches_by_region() - grouping.rs                                     │
│     - Group matches with line_gap <= 4 (LINES_THRESHOLD)                       │
│     - Create DetectionGroup for each region                                    │
│                                                                                 │
│  3. create_detection_from_group() - mod.rs:177                                  │
│     - analyze_detection() to classify match quality                            │
│     - determine_license_expression() from matches                              │
│     - determine_spdx_expression() for SPDX form                                │
│     - compute_detection_identifier() for unique ID                             │
│                                                                                 │
│  4. post_process_detections() - mod.rs:348                                      │
│     - filter_detections_by_score()                                             │
│     - rank_detections() by score and coverage                                  │
│     - sort_detections_by_line() for output order                               │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
                            ┌──────────────────────────────────────┐
                            │       Vec<LicenseDetection>          │
                            │   - license_expression               │
                            │   - license_expression_spdx          │
                            │   - matches: Vec<LicenseMatch>       │
                            │   - detection_log: Vec<String>       │
                            │   - identifier                       │
                            │   - file_region                      │
                            └──────────────────────────────────────┘
```

## Key Data Structures

### LicenseMatch

**File:** `models/license_match.rs:12-154`

The fundamental match result from any matching strategy.

```rust
pub struct LicenseMatch {
    pub rid: usize,                          // Rule ID (index into rules_by_rid)
    pub license_expression: String,          // ScanCode expression (e.g., "mit")
    pub license_expression_spdx: String,     // SPDX expression (e.g., "MIT")
    pub start_line: usize,                   // 1-indexed start line
    pub end_line: usize,                     // 1-indexed end line
    pub start_token: usize,                  // Token position in query
    pub end_token: usize,                    // Token position in query
    pub matcher: String,                     // Strategy: "1-hash", "2-aho", "3-seq", etc.
    pub score: f32,                          // Match score 0.0-1.0
    pub matched_length: usize,               // Tokens matched
    pub rule_length: usize,                  // Total tokens in rule
    pub match_coverage: f32,                 // Percentage 0-100
    pub rule_relevance: u8,                  // Rule relevance 0-100
    pub rule_identifier: String,             // e.g., "mit.LICENSE"
    
    // Position tracking for overlap detection
    pub qspan_positions: Option<Vec<usize>>, // Query token positions
    pub ispan_positions: Option<Vec<usize>>, // Rule token positions
    pub hispan_positions: Option<Vec<usize>>, // High-value token positions
    
    // Rule flags
    pub is_license_text: bool,
    pub is_license_intro: bool,
    pub is_license_reference: bool,
    pub is_license_tag: bool,
    // ... more flags
}
```

### LicenseIndex

**File:** `index/mod.rs:41-194`

The central index containing all matching data structures.

```rust
pub struct LicenseIndex {
    // Token management
    pub dictionary: TokenDictionary,         // String -> u16 token mapping
    pub len_legalese: usize,                 // Count of high-value legalese tokens
    
    // Rule storage
    pub rules_by_rid: Vec<Rule>,             // Rules indexed by rid
    pub tids_by_rid: Vec<Vec<u16>>,          // Token IDs for each rule
    
    // Matching indices
    pub rid_by_hash: HashMap<[u8; 20], usize>,  // SHA1 hash -> rid
    pub rules_automaton: Automaton,             // Aho-Corasick automaton
    pub unknown_automaton: Automaton,           // Ngram automaton for unknowns
    
    // Candidate selection
    pub sets_by_rid: HashMap<usize, HashSet<u16>>,      // Unique tokens per rule
    pub msets_by_rid: HashMap<usize, HashMap<u16, usize>>, // Token frequencies
    pub high_postings_by_rid: HashMap<usize, HashMap<u16, Vec<usize>>>, // Positions
    
    // Rule classification
    pub regular_rids: HashSet<usize>,
    pub false_positive_rids: HashSet<usize>,
    pub approx_matchable_rids: HashSet<usize>,
    
    // License metadata
    pub licenses_by_key: HashMap<String, License>,
    pub rid_by_spdx_key: HashMap<String, usize>,
}
```

### Rule

**File:** `models/rule.rs:8-135`

Metadata for a loaded rule from .RULE or .LICENSE files.

```rust
pub struct Rule {
    pub identifier: String,                  // e.g., "mit.LICENSE"
    pub license_expression: String,          // e.g., "mit"
    pub text: String,                        // Pattern text
    pub tokens: Vec<u16>,                    // Token IDs
    
    // Rule type flags
    pub is_license_text: bool,               // Full license text
    pub is_license_notice: bool,             // "Licensed under..."
    pub is_license_reference: bool,          // Name/URL reference
    pub is_license_tag: bool,                // SPDX identifier
    pub is_license_intro: bool,              // Intro text
    pub is_license_clue: bool,               // Clue but not detection
    pub is_false_positive: bool,             // Marked as false positive
    
    // Thresholds
    pub relevance: u8,                       // 0-100 relevance score
    pub minimum_coverage: Option<u8>,        // Optional min coverage
    pub required_phrase_spans: Vec<Range<usize>>, // {{...}} phrases
    
    // Token statistics
    pub length_unique: usize,                // Unique token count
    pub high_length_unique: usize,           // Unique legalese tokens
    pub high_length: usize,                  // Total legalese tokens
    pub min_matched_length: usize,           // Threshold for matching
    pub is_small: bool,                      // < 15 tokens
    pub is_tiny: bool,                       // < 6 tokens
}
```

### Query

**File:** `query/mod.rs:60-145`

Tokenized input text ready for matching.

```rust
pub struct Query<'a> {
    pub text: String,                        // Original input
    pub tokens: Vec<u16>,                    // Token IDs (known tokens only)
    pub line_by_pos: Vec<usize>,             // Line number for each token
    pub unknowns_by_pos: HashMap<Option<i32>, usize>, // Unknown token counts
    pub stopwords_by_pos: HashMap<Option<i32>, usize>, // Stopword counts
    
    // Matchable positions (not yet matched)
    pub high_matchables: HashSet<usize>,     // Legalese positions
    pub low_matchables: HashSet<usize>,      // Non-legalese positions
    
    // Detection flags
    pub has_long_lines: bool,                // Minified JS/CSS detection
    pub is_binary: bool,                     // Binary content flag
    
    // Pre-detected SPDX lines
    pub spdx_lines: Vec<(String, usize, usize)>, // (text, start_token, end_token)
    
    pub index: &'a LicenseIndex,             // Reference to index
}
```

### LicenseDetection

**File:** `detection/types.rs:37-58`

Final grouped detection result.

```rust
pub struct LicenseDetection {
    pub license_expression: Option<String>,      // Combined expression
    pub license_expression_spdx: Option<String>, // SPDX form
    pub matches: Vec<LicenseMatch>,              // All matches in group
    pub detection_log: Vec<String>,              // Quality notes
    pub identifier: Option<String>,              // Unique ID
    pub file_region: Option<FileRegion>,         // Location
}
```

### DetectionGroup

**File:** `detection/types.rs:5-33`

Intermediate grouping of nearby matches.

```rust
pub struct DetectionGroup {
    pub matches: Vec<LicenseMatch>,
    pub start_line: usize,
    pub end_line: usize,
}
```

## Matching Strategies

### 1. Hash Matching (`hash_match.rs`)

**Fastest, highest priority.** Exact match using SHA1 hash of token sequence.

```
Input: query_run.tokens()
Process:
  1. Compute SHA1 hash of token sequence
  2. Lookup in index.rid_by_hash
  3. If found, create LicenseMatch with 100% coverage
Output: 0 or 1 match
```

### 2. SPDX-License-Identifier Matching (`spdx_lid/mod.rs`)

**Detects explicit license tags.** Parses `SPDX-License-Identifier: MIT` style lines.

```
Input: query.spdx_lines (pre-detected during tokenization)
Process:
  1. Parse license expression after identifier
  2. Clean and normalize expression
  3. Resolve to ScanCode license keys
  4. Handle deprecated SPDX identifiers
  5. Create is_license_tag matches
Output: Vec<LicenseMatch> with matcher="1-spdx-id"
```

### 3. Aho-Corasick Matching (`aho_match.rs`)

**Multi-pattern exact matching.** Finds all occurrences of rule token sequences.

```
Input: query_run.tokens(), index.rules_automaton
Process:
  1. Encode tokens as little-endian bytes
  2. Run Aho-Corasick automaton.find_overlapping_iter()
  3. Verify all positions are matchable
  4. Filter by pattern_id_to_rid mapping
  5. Compute coverage and score
Output: Vec<LicenseMatch> with matcher="2-aho"
```

### 4. Sequence Matching (`seq_match/mod.rs`)

**Approximate matching for modified text.** Uses set similarity for candidate selection, then sequence alignment.

```
Input: query_run, index (sets_by_rid, msets_by_rid, high_postings_by_rid)

Process (Phase 2 - Near-duplicate):
  1. Compute token set overlap with all rules
  2. Filter candidates with resemblance >= 0.8
  3. Top 10 candidates
  4. Sequence alignment for each

Process (Phase 3 - Regular):
  1. Compute token set overlap
  2. Top 70 candidates by Jaccard similarity
  3. Sequence alignment for each

Process (Phase 4 - Query runs):
  1. For each text sub-region (query run)
  2. Repeat Phase 3 process
  3. Align candidates to region

Sequence Alignment (matching.rs):
  1. Use high_postings_by_rid to find anchor positions
  2. Build matching blocks using token positions
  3. Merge adjacent/overlapping blocks
  4. Compute coverage from matched tokens

Output: Vec<LicenseMatch> with matcher="3-seq" or "4-seq"
```

### 5. Unknown Matching (`unknown_match.rs`)

**Detects license-like text without known rule.** Uses ngram matching.

```
Input: query, known_matches, index.unknown_automaton
Process:
  1. Find uncovered regions (not in known_matches)
  2. For each region >= MIN_REGION_LENGTH:
     a. Match 6-token ngrams against unknown_automaton
     b. Require >= MIN_NGRAM_MATCHES
     c. Merge overlapping ngram positions
     d. Require >= 24 total matched tokens
     e. Require >= 5 high-value legalese tokens
  3. Create "unknown" license matches
Output: Vec<LicenseMatch> with matcher="5-undetected"
```

## Important Algorithms

### Candidate Selection

**File:** `seq_match/candidates.rs`

Computes set similarity between query and rules for candidate ranking:

```rust
pub fn compute_candidates_with_msets(
    index: &LicenseIndex,
    query_run: &QueryRun,
    near_dupe: bool,        // true for Phase 2, false for Phase 3-4
    max_candidates: usize,  // 10 for near-dupe, 70 for regular
) -> Vec<Candidate>
```

Uses:
- Jaccard similarity (intersection / union)
- Containment (intersection / rule_size)
- Resemblance threshold (0.8 for near-duplicate)

### Sequence Alignment

**File:** `seq_match/matching.rs`

Aligns candidate rules to query text:

1. **High-value token anchoring**: Use `high_postings_by_rid` to find initial positions
2. **Block building**: Create matching blocks from aligned positions
3. **Block merging**: Combine adjacent blocks with small gaps
4. **Coverage computation**: Calculate matched_length / rule_length

### Match Refinement Pipeline

**File:** `match_refine/mod.rs:223-290`

Applied in order:

1. **Required phrase check**: Verify `{{...}}` marked phrases are matched
2. **Spurious match filter**: Remove matches with low token density
3. **Minimum coverage filter**: Remove matches below rule's threshold
4. **Single token filter**: Remove spurious single-token matches
5. **Short match filter**: Remove very short matches
6. **Scattered match filter**: Remove matches spread across too many lines
7. **Gibberish filter**: Remove matches to single-word content in binary files
8. **Overlap resolution**: Keep highest quality match at each position
9. **False positive filter**: Remove matches to false_positive rules
10. **Score update**: Final score computation

### Detection Grouping

**File:** `detection/grouping.rs`

Groups matches by proximity:

```rust
const LINES_THRESHOLD: usize = 4;

// Matches with line_gap <= 4 are grouped together
pub fn group_matches_by_region(matches: &[LicenseMatch]) -> Vec<DetectionGroup>
```

### Expression Determination

**File:** `detection/analysis.rs`

Computes combined license expression from multiple matches:

```rust
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String>
```

Handles:
- Single match: return match expression
- Multiple matches: combine with AND/OR based on positions
- License intros/references: special handling

## Key Module Responsibilities

| Module | Primary Responsibility |
|--------|----------------------|
| `mod.rs` | Engine orchestration, pipeline coordination |
| `hash_match` | Exact SHA1 hash matching |
| `spdx_lid` | SPDX-License-Identifier tag parsing |
| `aho_match` | Aho-Corasick multi-pattern matching |
| `seq_match` | Approximate sequence alignment |
| `match_refine` | Quality filtering, overlap resolution |
| `detection` | Match grouping, expression computation |
| `query` | Text tokenization, position tracking |
| `index` | Index construction and storage |
| `models` | Core data structures |
| `rules` | Rule/license file loading |
| `expression` | License expression parsing/simplification |
| `tokenize` | Text-to-tokens conversion |
| `unknown_match` | Unknown license detection |

## Performance Considerations

1. **Hash matching is fastest**: O(n) for hash computation, O(1) lookup
2. **Aho-Corasick is O(n+m)**: Linear in query length and total pattern length
3. **Sequence matching is expensive**: O(rules × query_length) for candidate selection
4. **Index is Arc-shared**: Enables parallel processing across files
5. **Query runs enable parallelization**: Different regions can be matched independently

## Thread Safety

The pipeline is designed for parallel processing:

- `LicenseIndex` is wrapped in `Arc` for shared ownership
- `Query` is created per-file (no shared mutable state)
- Matchers are pure functions (no side effects)
- `LicenseDetectionEngine::detect()` can be called concurrently
