# Copyright Detection Implementation Plan

> **Status**: 🟡 Planning Complete — Ready for Implementation
> **Priority**: P1 - High Priority Core Feature
> **Estimated Effort**: 3-4 weeks
> **Dependencies**: None (independent of license detection)

## Table of Contents

- [Overview](#overview)
- [Python Reference Analysis](#python-reference-analysis)
- [Rust Architecture Design](#rust-architecture-design)
- [Implementation Phases](#implementation-phases)
- [Beyond-Parity Improvements](#beyond-parity-improvements)
- [Testing Strategy](#testing-strategy)
- [Success Criteria](#success-criteria)

---

## Overview

Copyright detection extracts copyright statements, holder names, author information, and year ranges from source files. It is the second most important text detection feature after license detection, and is completely independent — it can be implemented in parallel with license detection.

### Scope

**In Scope:**

- Copyright statement detection (`© 2024 Company Name`, `Copyright (c) 2024`, etc.)
- Copyright holder extraction (company names, person names)
- Author detection (`@author`, `Written by`, `Developed by`, etc.)
- Year and year-range parsing (`2020`, `2020-2024`, `2020, 2021, 2022`)
- Multi-line copyright statement handling
- Linux CREDITS file parsing (structured `N:/E:/W:` format)
- SPDX-FileCopyrightText and SPDX-FileContributor support
- Email and URL extraction within copyright context
- "All Rights Reserved" handling (multiple languages)
- Junk/false-positive filtering
- Scanner pipeline integration

**Out of Scope:**

- General email/URL extraction from source code (see `EMAIL_URL_DETECTION_PLAN.md`)
- Copyright policy evaluation
- License-copyright correlation (post-processing)

### Current State in Rust

**Implemented:**

- ✅ Copyright field structures in file data (`copyrights`, `holders`, `authors`)
- ✅ Output format placeholders

**Missing:**

- ❌ Copyright pattern matching engine
- ❌ Holder name extraction
- ❌ Year parsing logic
- ❌ Multi-line statement handling
- ❌ Author detection
- ❌ Scanner integration

---

## Python Reference Analysis

### Architecture Overview

The Python implementation (`reference/scancode-toolkit/src/cluecode/copyrights.py`, ~4675 lines) uses a **four-stage pipeline**:

```text
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  1. Text     │───>│  2. Candidate│───>│  3. Lex +    │───>│  4. Tree     │
│  Preparation │    │  Selection   │    │  Parse       │    │  Walk +      │
│              │    │              │    │  (pygmars)   │    │  Refinement  │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
```

#### Stage 1: Text Preparation (`prepare_text_line`)

Normalizes raw text lines before detection:

- **Copyright symbol normalization**: `©`, `(C)`, `(c)`, `&#169;`, `&#xa9;`, `\251`, `&copy;`, `u00a9` → all become `(c)`
- **HTML entity decoding**: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#13;`, `&#10;`, `&ensp;`, `&emsp;`, etc.
- **Comment marker removal**: `/*`, `*/`, `*`, `#`, `%`, `rem`, `dnl`, `."` (man pages)
- **Markup stripping**: Debian `<s></s>` tags, HTML tags, RST `|copy|`
- **Quote normalization**: backticks and double quotes → single quotes
- **Escape handling**: `\t`, `\n`, `\r`, `\0` → spaces
- **Punctuation cleanup**: Remove `*#"%[]{}` backtick, fold consecutive quotes
- **ASCII normalization**: `toascii()` with transliteration (e.g., `ñ` → `n`)
- **Emdash normalization**: `–` → `-`
- **Placeholder removal**: `<insert`, `year>`, `<year>`, `<name>`

#### Stage 2: Candidate Line Selection (`collect_candidate_lines`)

Filters lines to only those likely containing copyright information:

- **Hint-based filtering** (`copyrights_hint.py`): Lines containing any of ~25 markers:
  - `©`, `(c)`, `|copy|`, `&#169;`, `opyr`, `opyl`, `copr`, `right`, `reserv`, `auth`, `devel`, `<s>`, `</s>`, `by`, `@`, etc.
- **Year detection**: Lines containing years 1960-present
- **Gibberish detection**: Filters out binary/garbled text
- **Digit-only filtering**: Lines with only digits and punctuation are skipped
- **Multi-line grouping**: Groups consecutive candidate lines, with special handling for:
  - Lines ending with `copyright`, `and`, `by`, `,`, or a year → continue to next line
  - "All rights reserved" → end of statement marker
  - Empty lines → group boundary (unless inside a copyright statement)

#### Stage 3: Lexing + Parsing (pygmars)

Uses a **two-pass NLP-inspired approach**:

**Pass 1 — Lexing (POS Tagging):**

Tokenizes text on `[\t =;]+` and assigns Part-of-Speech (POS) tags via ~500+ ordered regex patterns. Key tag categories:

| Tag | Meaning | Examples |
|-----|---------|---------|
| `COPY` | Copyright keyword | `Copyright`, `(c)`, `Copr.`, `SPDX-FileCopyrightText` |
| `YR` | Year | `2024`, `1999,` |
| `YR-RANGE` | Year range (grammar-built) | `2020-2024` |
| `NNP` | Proper noun | `John`, `Smith`, `California` |
| `CAPS` | All-caps word | `MIT`, `IBM`, `GOOGLE` |
| `COMP` | Company suffix | `Inc.`, `Ltd.`, `GmbH`, `Foundation` |
| `UNI` | University | `University`, `College`, `Academy` |
| `NAME` | Name (grammar-built) | `John Smith` |
| `COMPANY` | Company (grammar-built) | `Google Inc.` |
| `AUTH` / `AUTH2` | Author keyword | `Author`, `Written`, `Developed` |
| `EMAIL` | Email address | `foo@bar.com` |
| `URL` / `URL2` | URL | `http://example.com`, `example.com` |
| `CC` | Conjunction | `and`, `&`, `,` |
| `VAN` | Name particle | `van`, `von`, `de`, `du` |
| `NN` | Common noun | (catch-all for unrecognized words) |
| `JUNK` | Junk to ignore | Programming keywords, HTML tags, etc. |

The PATTERNS list is **order-dependent** — first match wins. This is critical for correctness.

**Pass 2 — Grammar Parsing:**

A context-free grammar (~200+ rules) builds a parse tree from tagged tokens. Key grammar productions:

- `YR-RANGE`: Combines `YR`, `DASH`, `TO`, `CC` tokens into year ranges
- `NAME`: Combines `NNP`, `VAN`, `PN`, `CAPS` into person names
- `COMPANY`: Combines `NNP`, `COMP`, `UNI`, `CAPS` into organization names
- `COPYRIGHT`: Combines `COPY`, `YR-RANGE`, `NAME`, `COMPANY` into full statements
- `AUTHOR`: Combines `AUTH`, `NAME`, `COMPANY` into author attributions
- `ALLRIGHTRESERVED`: Matches "All Rights Reserved" patterns (multiple languages)

#### Stage 4: Tree Walk + Refinement

Walks the parse tree and yields `Detection` objects:

- **CopyrightDetection**: Full copyright statement with start/end lines
- **HolderDetection**: Extracted holder name (strips years, emails, URLs)
- **AuthorDetection**: Extracted author name

**Refinement functions** clean up detected strings:

- `refine_copyright()`: Strips punctuation, unbalanced parens, duplicate "Copyright" words, junk prefixes/suffixes
- `refine_holder()`: Strips dates, "All Rights Reserved", junk holders, normalizes names
- `refine_author()`: Strips author keywords, junk authors, normalizes names
- `is_junk_copyright()`: ~40 regex patterns for false positives (e.g., "copyright holder or simply", "full copyright statement")

#### Special Cases

**Linux CREDITS files** (`linux_credits.py`):

- Detects structured `N:/E:/W:` format used by Linux kernel, LLVM, Botan, etc.
- Yields `AuthorDetection` objects directly (bypasses main pipeline)
- Checks first 50 lines for structured format; bails out if none found

**"All Rights Reserved" in multiple languages:**

- English: "All Rights Reserved"
- German: "Alle Rechte vorbehalten"
- French: "Tous droits réservés"
- Spanish: "Reservados todos los derechos"
- Dutch: "Alle rechten voorbehouden"

### Known Bugs and Issues in Python

1. **Suspicious regex in `_YEAR_YEAR`**: Comment says `# fixme v ....the underscore below is suspicious` for pattern `(19[6-9][0-9][\\.,\\-]_)+[6-9][0-9]` — the underscore `_` in the year-year separator is likely a bug
2. **Duplicate patterns**: Several regex patterns appear multiple times (e.g., `^[Cc]opyrighted[\.,\)]$` at lines 658-659, `^[Cc]opyrights[\.,\)]$` at lines 659-660)
3. **Duplicate grammar rules**: Some rules have identical numbers (e.g., multiple `#2840`, `#2274`, `#970`)
4. **Global mutable state**: `DETECTOR = None` singleton pattern is not thread-safe
5. **Order-dependent PATTERNS**: First-match-wins semantics make the pattern list fragile — reordering breaks detection
6. **Excessive post-processing**: Many `FIXME: the grammar should not allow this to happen` comments indicate the grammar is too permissive
7. **Hardcoded names**: Specific person names hardcoded as NNP exceptions (e.g., `suzuki`, `karsten`, `wiese`)
8. **`is_private_ip` bug**: Line 493 in `finder.py` has `private(` instead of `private = (` for IPv6 — missing assignment
9. **Year range `_YEAR`**: Only covers 1960-2039 (`20[0-3][0-9]`), will break in 2040
10. **Missing `$` anchors**: Some patterns have inconsistent anchoring

### Python Dependencies

The Python implementation depends on:

- **pygmars**: Custom lexer/parser library (regex-based POS tagger + CFG parser)
- **commoncode.text**: `toascii()` (Unicode→ASCII transliteration), `unixlinesep()` (line ending normalization)
- **textcode.gibberish**: `Gibberish` detector (filters binary/garbled text)
- **textcode.markup**: `strip_known_markup_from_text()` (HTML/XML tag removal)
- **textcode.analysis**: `numbered_text_lines()` (file reading with line numbers + optional demarkup)

---

## Rust Architecture Design

### Design Philosophy

The Rust implementation will **not** replicate the Python architecture line-by-line. Instead, it will:

1. **Achieve identical outcomes** through a cleaner, more maintainable design
2. **Eliminate the pygmars dependency** — replace with a purpose-built Rust lexer/parser
3. **Fix all known bugs** from the Python implementation
4. **Leverage Rust's type system** for safety (enums for POS tags, no stringly-typed dispatch)
5. **Be thread-safe by design** (no global mutable state)
6. **Use `regex` crate** for high-performance pattern matching (compiled once, reused)

### High-Level Architecture

```text
┌─────────────────────────────────────────────────────────────────────┐
│                    Copyright Detection Pipeline                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐  │
│  │ TextPreparer │───>│ Candidate    │───>│ CopyrightDetector    │  │
│  │              │    │ Selector     │    │                      │  │
│  │ • normalize  │    │              │    │ ┌──────────────────┐ │  │
│  │ • demarkup   │    │ • hint match │    │ │ Lexer            │ │  │
│  │ • to_ascii   │    │ • year match │    │ │ (POS tagger)     │ │  │
│  │ • clean      │    │ • gibberish  │    │ └────────┬─────────┘ │  │
│  └──────────────┘    │   filter     │    │          │           │  │
│                      │ • grouping   │    │ ┌────────▼─────────┐ │  │
│                      └──────────────┘    │ │ Parser           │ │  │
│                                          │ │ (grammar rules)  │ │  │
│  ┌──────────────┐                        │ └────────┬─────────┘ │  │
│  │ Credits      │                        │          │           │  │
│  │ Detector     │                        │ ┌────────▼─────────┐ │  │
│  │ (CREDITS     │                        │ │ Refiner          │ │  │
│  │  files)      │                        │ │ (post-process)   │ │  │
│  └──────────────┘                        │ └──────────────────┘ │  │
│                                          └──────────────────────┘  │
│                                                                      │
│  Output: Vec<Detection>                                             │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ CopyrightDetection { statement, start_line, end_line }       │  │
│  │ HolderDetection    { holder,    start_line, end_line }       │  │
│  │ AuthorDetection    { author,    start_line, end_line }       │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### Core Data Types

```rust
/// A detected copyright-related item
#[derive(Debug, Clone, PartialEq)]
pub enum Detection {
    Copyright(CopyrightDetection),
    Holder(HolderDetection),
    Author(AuthorDetection),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CopyrightDetection {
    pub copyright: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HolderDetection {
    pub holder: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuthorDetection {
    pub author: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Part-of-Speech tag for a token (type-safe, not stringly-typed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PosTag {
    // Copyright keywords
    Copy,           // "Copyright", "(c)", "Copr.", etc.
    SpdxContrib,    // "SPDX-FileContributor"

    // Year-related
    Yr,             // A year like "2024"
    YrPlus,         // Year with plus: "2024+"
    BareYr,         // Short year: "99"

    // Names and entities
    Nnp,            // Proper noun: "John", "Smith"
    Nn,             // Common noun (catch-all)
    Caps,           // All-caps word: "MIT", "IBM"
    Pn,             // Dotted name: "P.", "DMTF."
    MixedCap,       // Mixed case: "LeGrande"
    Name,           // Compound name (built by grammar)
    Company,        // Full company name (built by grammar)

    // Organization suffixes
    Comp,           // Company suffix: "Inc.", "Ltd.", "GmbH"
    Uni,            // University: "University", "College"

    // Author keywords
    Auth,           // "Author", "@author"
    Auth2,          // "Written", "Developed", "Created"
    Auths,          // "Authors", "author's"
    AuthDot,        // "Author.", "Authors."
    Maint,          // "Maintainer", "Developer"
    Contributors,   // "Contributors"
    Commit,         // "Committers"

    // Rights reserved
    Right,          // "Rights", "Rechte", "Droits"
    Reserved,       // "Reserved", "Vorbehalten", "Réservés"

    // Conjunctions and prepositions
    Cc,             // "and", "&", ","
    Of,             // "of", "De", "Di"
    By,             // "by"
    In,             // "in", "en"
    Van,            // "van", "von", "de", "du"
    To,             // "to"
    Dash,           // "-", "--", "/"

    // Special
    Email,          // Email address
    Url,            // URL with scheme
    Url2,           // URL without scheme (domain.com)
    Holder,         // "Holder", "Holders"
    Is,             // "is", "are"
    Held,           // "held"
    Notice,         // "NOTICE"
    Portions,       // "Portions", "Parts"
    Oth,            // "Others", "et al."
    Following,      // "following"
    Mit,            // "MIT" (special handling)
    Linux,          // "Linux"
    Parens,         // "(" or ")"
    At,             // "AT" (obfuscated email)
    Dot,            // "DOT" (obfuscated email)
    Ou,             // "OU" (org unit in certs)

    // Structural
    EmptyLine,      // Empty line marker
    Junk,           // Junk to ignore

    // Cardinals
    Cd,             // Cardinal number
    Cds,            // Small cardinal (0-39)
    Month,          // Month abbreviation
    Day,            // Day of week
}

/// A token with its POS tag and source location
#[derive(Debug, Clone)]
pub struct Token {
    pub value: String,
    pub tag: PosTag,
    pub start_line: usize,
    pub pos: usize,  // position within line
}

/// A node in the parse tree
#[derive(Debug, Clone)]
pub enum ParseNode {
    Token(Token),
    Tree {
        label: TreeLabel,
        children: Vec<ParseNode>,
    },
}

/// Labels for parse tree nodes (grammar non-terminals)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeLabel {
    YrRange,
    YrAnd,
    AllRightReserved,
    Name,
    NameEmail,
    NameYear,
    NameCopy,
    NameCaps,
    Company,
    AndCo,
    Copyright,
    Copyright2,
    Author,
    AndAuth,
    InitialDev,
    DashCaps,
}
```

### Key Design Decisions

#### 1. Enum-Based POS Tags (vs Python's String Tags)

**Python**: Tags are strings like `'COPY'`, `'NNP'`, `'JUNK'` — typos compile fine, no exhaustive matching.

**Rust**: Tags are enum variants — compiler enforces correctness, `match` is exhaustive, zero-cost abstraction.

#### 2. Compiled Regex Set (vs Python's Linear Scan)

**Python**: ~500+ regex patterns checked sequentially per token (first match wins). Each pattern is compiled individually.

**Rust**: Use `regex::RegexSet` for the initial match, then only compile the matched pattern for capture groups. This gives O(n) matching where n is the text length, not O(p×n) where p is the number of patterns.

**Alternative**: For the lexer specifically, since patterns are anchored and we need first-match-wins semantics, we can use a single large `regex::RegexSet::new(patterns)` call to find all matches, then pick the first (lowest index) match. This is significantly faster than Python's sequential approach.

#### 3. Thread-Safe Detector (vs Python's Global Singleton)

**Python**: `DETECTOR = None` global singleton, not thread-safe.

**Rust**: `CopyrightDetector` will be `Send + Sync`, created per-thread or wrapped in `Arc`. The compiled regex patterns will be in a `lazy_static!` or `OnceLock` for one-time initialization.

#### 4. Grammar as Data (vs Python's String Grammar)

**Python**: Grammar is a multi-line string parsed at runtime by pygmars.

**Rust**: Grammar rules will be encoded as Rust data structures (arrays of `GrammarRule` structs), checked at compile time. The parser will be a simple bottom-up chart parser operating on the token stream.

#### 5. Year Range: 1960-2099 (vs Python's 1960-2039)

**Python**: `20[0-3][0-9]` stops at 2039.

**Rust**: `20[0-9][0-9]` covers through 2099. We'll also add a compile-time or runtime check to extend this if needed.

### Module Structure

```text
src/
├── copyright/
│   ├── mod.rs              # Public API: detect_copyrights()
│   ├── detector.rs         # CopyrightDetector (lexer + parser + refiner)
│   ├── lexer.rs            # POS tagger (regex-based token classification)
│   ├── parser.rs           # Grammar-based parse tree builder
│   ├── patterns.rs         # Compiled regex patterns for lexer (PATTERNS equivalent)
│   ├── grammar.rs          # Grammar rules (GRAMMAR equivalent)
│   ├── refiner.rs          # Post-detection cleanup (refine_copyright/holder/author)
│   ├── candidate.rs        # Candidate line selection and grouping
│   ├── prepare.rs          # Text preparation and normalization
│   ├── credits.rs          # Linux CREDITS file detection
│   ├── hints.rs            # Copyright hint markers and year patterns
│   ├── junk.rs             # Junk/false-positive filtering
│   └── types.rs            # Detection, Token, PosTag, ParseNode types
├── copyright_test.rs       # Unit tests
└── copyright_golden_test.rs # Golden tests against Python reference
```

---

## Implementation Phases

### Phase 1: Core Types and Text Preparation (3-4 days)

**Goal**: Establish the foundation — types, text normalization, and candidate selection.

**Deliverables:**

1. `types.rs`: All core types (`Detection`, `PosTag`, `Token`, `ParseNode`, `TreeLabel`)
2. `hints.rs`: Copyright hint markers and year detection regex
3. `prepare.rs`: `prepare_text_line()` — full text normalization pipeline
4. `candidate.rs`: `collect_candidate_lines()` — candidate line selection and grouping
5. `credits.rs`: Linux CREDITS file detection (`is_credits_file()`, `detect_credits_authors()`)

**Testing**: Unit tests for each function, especially:

- Copyright symbol normalization (all variants)
- HTML entity decoding
- Candidate line grouping (multi-line statements)
- CREDITS file parsing

### Phase 2: Lexer (POS Tagger) (4-5 days)

**Goal**: Implement the regex-based POS tagger that classifies tokens.

**Deliverables:**

1. `patterns.rs`: All ~500+ regex patterns compiled into a `RegexSet` + individual `Regex` objects
2. `lexer.rs`: `Lexer` struct with `lex_tokens()` method
3. Tokenizer: Split text on `[\t =;]+`, strip quotes/colons, filter empty tokens

**Key Implementation Details:**

- Patterns must maintain **order-dependent first-match-wins** semantics
- Use `RegexSet` for fast multi-pattern matching, then select lowest-index match
- Each pattern maps to a `PosTag` enum variant
- Compile all patterns once at startup (lazy_static or OnceLock)

**Bug Fixes from Python:**

- Fix the suspicious underscore in `_YEAR_YEAR` pattern
- Remove duplicate patterns
- Extend year range to 2099
- Add missing `$` anchors where needed

**Testing**: Test each POS tag category with representative inputs.

### Phase 3: Grammar Parser (4-5 days)

**Goal**: Implement the grammar-based parser that builds parse trees from tagged tokens.

**Deliverables:**

1. `grammar.rs`: All ~200+ grammar rules as data structures
2. `parser.rs`: `Parser` struct with `parse()` method

**Parser Algorithm:**

The Python pygmars parser uses a **regex-over-tags** approach: each grammar rule is a regex pattern over POS tag sequences. The parser repeatedly applies rules bottom-up until no more rules match.

The Rust implementation will use the same approach:

- Grammar rules are patterns over tag sequences (e.g., `<COPY> <COPY> <YR-RANGE> <NAME>`)
- Parser iterates over the token list, trying to match rules
- When a rule matches, the matched tokens are replaced with a new `Tree` node
- Process repeats until no more rules match (single loop, matching Python's `loop=1`)

**Key Implementation Details:**

- Rules are tried in order (first match wins within each pass)
- The parser must handle nested trees (rules can match tree nodes, not just tokens)
- Grammar rule matching uses tag comparison, not string comparison

**Testing**: Test grammar rules with pre-tagged token sequences.

### Phase 4: Detection and Refinement (3-4 days)

**Goal**: Walk parse trees, extract detections, and refine results.

**Deliverables:**

1. `detector.rs`: `CopyrightDetector` — orchestrates lexer → parser → tree walk → refinement
2. `refiner.rs`: `refine_copyright()`, `refine_holder()`, `refine_author()`
3. `junk.rs`: `is_junk_copyright()` — false positive filtering

**Refinement Operations:**

- Strip unbalanced parentheses (all types: `()`, `<>`, `[]`, `{}`)
- Remove duplicate "Copyright" words
- Strip junk prefixes and suffixes
- Normalize "SPDX-FileCopyrightText" → "Copyright"
- Strip trailing periods (with exceptions for abbreviations like "Inc.", "Ltd.")
- Remove "All Rights Reserved" from holder names
- Filter junk holders and authors

**Testing**: Test refinement functions with edge cases from Python test suite.

### Phase 5: Scanner Integration (2-3 days)

**Goal**: Wire copyright detection into the scanner pipeline.

**Deliverables:**

1. `mod.rs`: Public API `detect_copyrights(path: &Path) -> Vec<Detection>`
2. Scanner integration: Call copyright detection for each file during scanning
3. Output format: Populate `copyrights`, `holders`, `authors` arrays in `FileInfo`
4. CLI flag: `--copyright` to enable copyright scanning

**Integration Points:**

- `src/scanner/process.rs`: Add copyright detection call after package parsing
- `src/models/file_info.rs`: Ensure copyright/holder/author fields are populated
- Output JSON: Match Python ScanCode's output format exactly

**Testing**: Integration tests with real files, golden tests against Python output.

### Phase 6: Golden Tests and Polish (2-3 days)

**Goal**: Validate against Python reference, fix discrepancies, document improvements.

**Deliverables:**

1. Golden test infrastructure for copyright detection
2. Run against Python ScanCode's test corpus
3. Document any intentional behavioral differences
4. Performance benchmarks

---

## Beyond-Parity Improvements

### 1. Thread Safety (Bug Fix)

**Python**: Global `DETECTOR` singleton is not thread-safe.
**Rust**: `CopyrightDetector` is `Send + Sync` by design. No global mutable state.

### 2. Extended Year Range (Bug Fix)

**Python**: Year patterns stop at 2039 (`20[0-3][0-9]`).
**Rust**: Year patterns cover 1960-2099 (`20[0-9][0-9]`), future-proofed.

### 3. Fixed `_YEAR_YEAR` Pattern (Bug Fix)

**Python**: Suspicious underscore in year-year separator pattern.
**Rust**: Corrected to use proper separator characters only.

### 4. Deduplicated Patterns (Bug Fix)

**Python**: Several duplicate regex patterns and grammar rules.
**Rust**: All patterns and rules are unique, verified at compile time.

### 5. Type-Safe POS Tags (Enhancement)

**Python**: String-based tags (`'COPY'`, `'NNP'`) — typos are silent bugs.
**Rust**: Enum-based tags — compiler catches all errors, exhaustive matching.

### 6. Performance (Enhancement)

**Python**: Sequential regex matching (~500 patterns per token).
**Rust**: `RegexSet` for parallel multi-pattern matching, compiled once.

### 7. Fixed `is_private_ip` Bug (Bug Fix)

**Python**: Missing assignment in IPv6 branch (`private(` instead of `private = (`).
**Rust**: Correct implementation with proper return value.

### 8. Gibberish Detection Improvement (Enhancement)

**Python**: Uses a separate `Gibberish` class.
**Rust**: Consider using entropy-based detection or a simpler heuristic that doesn't require a trained model, while maintaining the same filtering quality.

---

## Testing Strategy

### Unit Tests (`copyright_test.rs`)

Test each component in isolation:

1. **Text preparation**: All normalization paths (copyright symbols, HTML entities, comment markers)
2. **Candidate selection**: Line grouping, multi-line handling, hint matching
3. **Lexer**: POS tag assignment for each tag category
4. **Parser**: Grammar rule matching for each production
5. **Refinement**: String cleanup for copyrights, holders, authors
6. **Junk filtering**: False positive detection
7. **CREDITS files**: Structured format parsing

### Golden Tests (`copyright_golden_test.rs`)

Compare output against Python ScanCode reference:

1. Run Python ScanCode on test corpus, capture JSON output
2. Run Rust implementation on same corpus
3. Compare `copyrights`, `holders`, `authors` arrays
4. Document any intentional differences

### Test Data

- Use existing test files from `reference/scancode-toolkit/tests/cluecode/`
- Add new test files for:
  - Edge cases found during implementation
  - Bug fixes (demonstrate Python bugs are fixed)
  - Beyond-parity features
  - Multi-language "All Rights Reserved"
  - SPDX-FileCopyrightText variants
  - Complex multi-line statements

### Performance Tests

- Benchmark against Python ScanCode on large codebases
- Measure per-file detection time
- Profile regex compilation and matching

---

## Success Criteria

- [ ] Detects all standard copyright formats (`©`, `(c)`, `Copyright`, `Copr.`, `SPDX-FileCopyrightText`)
- [ ] Extracts holder names accurately (companies, persons, organizations)
- [ ] Parses year ranges correctly (single years, ranges, comma-separated, with "present")
- [ ] Handles multi-line copyright statements
- [ ] Detects authors (`@author`, `Written by`, `Developed by`, etc.)
- [ ] Parses Linux CREDITS files
- [ ] Handles "All Rights Reserved" in English, German, French, Spanish, Dutch
- [ ] Filters junk/false positives effectively
- [ ] Golden tests pass against Python reference (with documented intentional differences)
- [ ] Thread-safe (no global mutable state)
- [ ] Performance: ≥ Python ScanCode speed (expected 5-10x faster)
- [ ] All known Python bugs are fixed
- [ ] `cargo clippy` clean, `cargo fmt` clean
- [ ] Comprehensive test coverage

---

## Related Documents

- **Architecture**: [`docs/ARCHITECTURE.md`](../../ARCHITECTURE.md) — Scanner pipeline, copyright detection section
- **Email/URL Detection**: [`EMAIL_URL_DETECTION_PLAN.md`](EMAIL_URL_DETECTION_PLAN.md) — Related text extraction
- **License Detection**: [`LICENSE_DETECTION_PLAN.md`](LICENSE_DETECTION_PLAN.md) — Similar pattern matching approach
- **Testing Strategy**: [`docs/TESTING_STRATEGY.md`](../../TESTING_STRATEGY.md) — Testing approach
- **Python Reference**: `reference/scancode-toolkit/src/cluecode/copyrights.py` — Original implementation

---

## Appendix: Python File Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `copyrights.py` | 4675 | Main detection: lexer patterns, grammar, detector, refiners, candidate selection, text prep |
| `copyrights_hint.py` | 163 | Hint markers for candidate line selection, year regex, copyright symbol variants |
| `plugin_copyright.py` | 49 | Scanner plugin integration (thin wrapper) |
| `linux_credits.py` | 155 | Linux CREDITS file parser (structured N:/E:/W: format) |
| `finder.py` | 597 | Email/URL finding (shared with email/URL detection — out of scope for this plan) |
| `finder_data.py` | ~500 | Junk email/URL/host classification data (shared — out of scope) |
