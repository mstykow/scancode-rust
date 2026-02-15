# Copyright Detection Implementation Plan

> **Status**: ✅ Implementation Complete — Golden Tests at 98%+
> **Priority**: P1 - High Priority Core Feature
> **Actual Effort**: ~4 weeks
> **Dependencies**: None (independent of license detection)

## Table of Contents

- [Overview](#overview)
- [Python Reference Analysis](#python-reference-analysis)
- [Rust Architecture](#rust-architecture)
- [Implementation Summary](#implementation-summary)
- [Beyond-Parity Improvements](#beyond-parity-improvements)
- [Golden Test Results](#golden-test-results)
- [Remaining Expected Failures](#remaining-expected-failures)
- [Future Enhancements](#future-enhancements)

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

### Current State

**All features implemented:**

- ✅ Copyright pattern matching engine (~1,100 POS tag patterns)
- ✅ Grammar parser (~660 rules)
- ✅ Holder name extraction
- ✅ Year and year-range parsing (1960-2099)
- ✅ Multi-line statement handling
- ✅ Author detection
- ✅ Scanner integration (runs on all files including package manifests)
- ✅ Unicode name preservation (no transliteration — names like "François Müller" kept intact)
- ✅ Linux CREDITS file parsing
- ✅ Junk/false-positive filtering (~130+ junk patterns)
- ✅ Thread-safe design via `LazyLock`
- ✅ Performance optimizations (long-line skip, encoded-data detection)
- ✅ Golden test infrastructure with expected-failures tracking

---

## Python Reference Analysis

### Architecture Overview

The Python implementation (`reference/scancode-toolkit/src/cluecode/copyrights.py`, ~4,675 lines) uses a **four-stage pipeline**:

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
- **ASCII normalization**: `toascii()` with transliteration (e.g., `ñ` → `n`) — **we intentionally skip this step** to preserve Unicode names in output
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

1. ✅ **FIXED — Suspicious regex in `_YEAR_YEAR`**: Underscore `_` in year-year separator removed
2. ✅ **FIXED — Duplicate patterns**: 3 duplicates removed
3. ✅ **FIXED — Global mutable state**: Replaced with `LazyLock` (thread-safe)
4. ✅ **FIXED — Year range `_YEAR`**: Extended from 2039 to 2099
5. ✅ **FIXED — `_YEAR_SHORT` typo**: `[0-][0-9]` → `[0-2][0-9]`
6. ✅ **FIXED — French/Spanish case bugs**: `[Dr]roits?` → `[Dd]roits?`, `[Dr]erechos` → `[Dd]erechos`
7. **Duplicate grammar rules**: Some rules have identical numbers — preserved for compatibility
8. **Order-dependent PATTERNS**: First-match-wins semantics preserved (same as Python)
9. **Excessive post-processing**: Grammar is permissive by design; refiners compensate
10. **Hardcoded names**: NNP exceptions preserved from Python (e.g., `suzuki`, `karsten`, `wiese`)

### Python Dependencies

The Python implementation depends on:

- **pygmars**: Custom lexer/parser library (regex-based POS tagger + CFG parser)
- **commoncode.text**: `toascii()` (Unicode→ASCII transliteration), `unixlinesep()` (line ending normalization)
- **textcode.gibberish**: `Gibberish` detector (filters binary/garbled text)
- **textcode.markup**: `strip_known_markup_from_text()` (HTML/XML tag removal)
- **textcode.analysis**: `numbered_text_lines()` (file reading with line numbers + optional demarkup)

---

## Rust Architecture

### Module Structure

```text
src/copyright/
├── mod.rs            # Public API: detect_copyrights(), re-exports
├── types.rs          # Core types: CopyrightDetection, HolderDetection, AuthorDetection, PosTag enum, Token, ParseNode, TreeLabel
├── prepare.rs        # 13-stage text normalization pipeline (Unicode preserved, no transliteration)
├── hints.rs          # Hint markers, year detection, gibberish filter
├── candidates.rs     # Candidate line selection, multi-line grouping, long-line + encoded-data skip
├── lexer.rs          # POS tagger — tokenize + classify via patterns
├── patterns.rs       # ~1,100 ordered regex patterns for POS tagging
├── grammar.rs        # ~660 grammar rules as Rust data structures
├── parser.rs         # Bottom-up grammar parser with max_iterations=50
├── detector.rs       # Full pipeline orchestrator + span-based extraction
├── refiner.rs        # Post-processing cleanup incl. ~130+ junk patterns
├── credits.rs        # Linux CREDITS file parser
└── golden_test.rs    # Golden test harness with expected-failures mechanism
```

### Key Design Decisions

#### 1. Enum-Based POS Tags (vs Python's String Tags)

**Python**: Tags are strings like `'COPY'`, `'NNP'`, `'JUNK'` — typos compile fine, no exhaustive matching.

**Rust**: `PosTag` enum — compiler enforces correctness, `match` is exhaustive, zero-cost abstraction.

#### 2. Sequential Pattern Matching with LazyLock (not RegexSet)

The original plan proposed `RegexSet` for parallel matching. In practice, we use `LazyLock<Vec<(Regex, PosTag)>>` — patterns compiled once at startup, matched sequentially per token with first-match-wins semantics.

**Why not RegexSet**: RegexSet cannot preserve match order, which is critical for first-match-wins semantics where pattern priority matters (e.g., JUNK exceptions must match before JUNK patterns).

#### 3. Thread-Safe via LazyLock (vs Python's Global Singleton)

**Python**: `DETECTOR = None` global singleton, not thread-safe.

**Rust**: All pattern data compiled once via `LazyLock` (thread-safe, zero-cost after init). Detection is a pure function — no global mutable state.

#### 4. Grammar as Rust Data Structures (vs Python's String Grammar)

**Python**: Grammar is a multi-line string parsed at runtime by pygmars.

**Rust**: Grammar rules encoded as `Vec<GrammarRule>` with `Matcher` enum variants (`Tag`, `Label`, `AnyTag`, `TagOrLabel`). Single-pass bottom-up parser with `max_iterations = 50` safety limit.

#### 5. Unicode Name Preservation (Beyond Parity)

**Python**: Calls `toascii(line, translit=True)` using `text_unidecode`, destroying Unicode characters (François→Francois, Müller→Muller).

**Rust**: Preserves original Unicode in output. The `deunicode` transliteration step was removed because it's unnecessary — the POS tagger uses Unicode-aware regex patterns (`\p{Lu}`, `\p{Ll}`) that correctly tag accented names as proper nouns. This is a beyond-parity improvement: names like "François Müller" appear in output with original accents intact.

#### 6. Performance Optimizations (Beyond Parity)

Two early-skip optimizations in candidate selection prevent pathological performance on non-copyright content:

- **Long-line skip**: Lines >2,000 chars without strong copyright indicators (`opyr`/`auth`/`(c)DIGIT`) are skipped before expensive regex processing. Protects against minified JS/CSS (e.g., 624KB single-line files).
- **Encoded-data detection**: Uuencode and base64 data lines are identified and skipped, preventing thousands of false-positive candidates from weak hint markers like `@`. This reduced the holders golden test from **20.5s → 1.1s** (19x faster).

#### 7. Scanner Integration — All Files

Copyright and license detection run on ALL files, including package manifests. Package parsing and text-based detection are independent (matching Python's plugin architecture where plugins run independently on every file).

---

## Implementation Summary

All 6 phases are complete:

| Phase | Deliverables | Status |
|-------|-------------|--------|
| **1. Core Types & Text Prep** | `types.rs`, `hints.rs`, `prepare.rs`, `candidates.rs`, `credits.rs` | ✅ Complete |
| **2. Lexer (POS Tagger)** | `patterns.rs` (~1,100 patterns), `lexer.rs` | ✅ Complete |
| **3. Grammar Parser** | `grammar.rs` (~660 rules), `parser.rs` | ✅ Complete |
| **4. Detection & Refinement** | `detector.rs`, `refiner.rs` (~130+ junk patterns) | ✅ Complete |
| **5. Scanner Integration** | `mod.rs` public API, `process.rs` integration, output format | ✅ Complete |
| **6. Testing & Golden Tests** | Comprehensive unit tests, golden tests at 98%+ pass rate | ✅ Complete |

---

## Beyond-Parity Improvements

Documented in detail in [`docs/improvements/copyright-detection.md`](../../improvements/copyright-detection.md).

| # | Type | Improvement |
|---|------|-------------|
| 1 | 🐛 Bug Fix | Extended year range from 2039 to 2099 |
| 2 | 🐛 Bug Fix | Fixed `_YEAR_SHORT` regex typo (`[0-][0-9]` → `[0-2][0-9]`) |
| 3 | 🐛 Bug Fix | Fixed French/Spanish case-sensitivity (`[Dr]roits?` → `[Dd]roits?`) |
| 4 | 🐛 Bug Fix | Fixed suspicious underscore in `_YEAR_YEAR` separator |
| 5 | ✨ Enhanced | Unicode name preservation in output (Python outputs ASCII-only) |
| 6 | 🔍 Enhanced | Type-safe POS tags (enum vs strings) |
| 7 | 🔍 Enhanced | Thread-safe design (`LazyLock` vs global mutable singleton) |
| 8 | 🔍 Enhanced | Deduplicated 3 duplicate patterns from Python reference |
| 9 | ⚡ Performance | Long-line skip for minified JS/CSS (avoids pathological regex) |
| 10 | ⚡ Performance | Encoded-data detection (uuencode/base64 skip — 19x faster on encoded files) |
| 11 | 🔍 Enhanced | Golden test expected-failures infrastructure with pass-rate tracking |
| 12 | 🛡️ Security | No code execution, no global mutable state |

---

## Golden Test Results

### Current Baseline

Four golden test suites (copyrights, holders, authors, ICS) validate output against the Python reference at **98%+ overall pass rate**. All expected failures are documented with known root causes. There are **zero unexpected failures** — CI is fully green.

Run golden tests with: `cargo test --features golden-tests copyright::golden_test -- --nocapture`

---

## Remaining Expected Failures

Some tests across the 4 suites produce slightly different output than the Python reference. These fall into several categories:

### Category 1: Complex Multi-line Copyrights with URLs

Long copyright statements spanning many lines with inline URLs and multiple holders. The span collection collects slightly more or less text than Python.

**Example**: `partial_detection.txt` — multi-line file with Debian markup, inline emails, and multi-holder copyrights. Python detects duplicate `(c)` variants; our refiner deduplicates more aggressively.

**Difficulty**: Medium. Requires tuning span collection boundaries and suffix stripping.

### Category 2: ICS False `(c)` Code Patterns

ICS source files containing `(c)` in C code contexts (type casts, ternary operators) falsely detected as copyright symbols. Our ~130+ junk patterns catch most cases but some remain. This is the largest category of expected failures.

**Example**: `iptables-extensions/libxt_LED.c` — `(c)` appears in bitwise/ternary code expressions.

**Difficulty**: Low-medium. More junk patterns in `refiner.rs`.

### Category 3: HTML/Markup-Heavy Files

Files with heavy HTML markup (credits pages, documentation) where tag stripping produces slightly different whitespace or token boundaries than Python.

**Example**: `bzip2/manual.html`, `sonivox-docs/JET_*.html` — complex HTML with copyright notices embedded in markup.

**Difficulty**: Medium. May require deeper demarkup preprocessing.

### Category 4: Edge-Case Copyright Phrasings

Unusual copyright formats that the grammar doesn't fully cover.

**Example**: "copyrighted by [holder]" phrasing, "Copyright or Copr." variants, copyright statements split across comment decorators.

**Difficulty**: Medium-high. Grammar/parser changes needed.

### Category 5: Remaining Author/Holder Gaps

Specific holder/author detections that differ from Python.

**Example**: "Originally by [Name]" not detected as author (tagged as Junk), address continuation after "Inc." truncated.

**Difficulty**: Low. Specific PosTag fixes in `patterns.rs`.

---

## Testing

### Test Coverage

Every module has comprehensive unit tests covering its core functionality. Golden test suites (copyrights, holders, authors, ICS) validate end-to-end output against the Python reference.

Key coverage areas:

- **Text preparation**: All normalization paths, Unicode preservation, HTML entity decoding
- **Candidate selection**: Line grouping, multi-line handling, long-line skip, encoded-data detection
- **Refinement**: String cleanup, junk filtering, edge cases
- **Hints**: Hint markers, year detection, gibberish filtering
- **Patterns**: POS tag categories, regex correctness
- **Detection**: End-to-end pipeline, Unicode holders, span extraction
- **Grammar/Parser**: Rule existence, pattern matching, tree building
- **Credits**: CREDITS file parsing, structured format

### Testing Workflow

```bash
# Unit tests (fast, ~3s):
cargo test --all

# Golden tests (comprehensive, ~30s with warm cache):
cargo test --features golden-tests copyright::golden_test -- --nocapture

# Specific golden suite:
cargo test --features golden-tests copyright::golden_test::tests::test_golden_copyrights -- --nocapture
```

---

## Success Criteria

- [x] Detects all standard copyright formats (`©`, `(c)`, `Copyright`, `Copr.`, `SPDX-FileCopyrightText`)
- [x] Extracts holder names accurately (companies, persons, organizations)
- [x] Parses year ranges correctly (single years, ranges, comma-separated)
- [x] Handles multi-line copyright statements
- [x] Detects authors (`@author`, `Written by`, `Developed by`, etc.)
- [x] Parses Linux CREDITS files
- [x] Handles "All Rights Reserved" in English, German, French, Spanish, Dutch
- [x] Filters junk/false positives effectively (~130+ patterns)
- [x] Golden tests at 98%+ against Python reference (expected failures documented)
- [x] Thread-safe (no global mutable state)
- [x] All known Python bugs are fixed
- [x] `cargo clippy` clean, `cargo fmt` clean
- [x] Comprehensive test coverage (unit tests across all modules + golden tests)

---

## Future Enhancements

### Priority 1: Reduce Remaining Expected Failures → 0

The remaining expected failures are documented above. Fixing them in priority order:

1. **ICS false `(c)` patterns** (largest category) — Add junk patterns in `refiner.rs`. Low effort, high impact.
2. **HTML-heavy files** — Improve demarkup preprocessing. Medium effort.
3. **Multi-line URL copyrights** — Tune span collection boundaries. Medium effort.
4. **Edge-case phrasings** — Grammar/parser additions. Medium-high effort.
5. **Author/holder gaps** (smallest category) — Specific PosTag fixes. Low effort.

### Priority 2: Performance Optimizations

- **RegexSet pre-filter**: ~1,100 sequential regex patterns per token could use a `RegexSet` to identify candidate matches, then check candidates in order for first-match-wins semantics. Would reduce per-token matching from O(n) to O(1) average case.
- **Per-file deadline/timeout**: Python supports a `deadline` parameter for aborting long-running detection. We have `max_iterations = 50` in the parser but no wall-clock timeout. Add `std::time::Instant`-based deadline check.

### Priority 3: Feature Enhancements

- **`include_*` filtering parameters**: Python's API supports `include_copyrights`, `include_holders`, `include_authors` flags. We always detect everything. Adding these would be trivial — filter after detection.
- **Full `demarkup` preprocessing**: Python calls `strip_known_markup_from_text` which handles HTML, RST, roff, and other markup formats more aggressively. Our `prepare_text_line` does basic normalization but doesn't strip full document markup. Would improve detection on heavily marked-up files.
- **Grammar top-level rules**: The grammar's COPYRIGHT/AUTHOR top-level rules don't always fire, so detection relies on span-based extraction as fallback. Investigating why these rules don't match could improve structural accuracy.

---

## Related Documents

- **Architecture**: [`docs/ARCHITECTURE.md`](../../ARCHITECTURE.md) — Scanner pipeline, copyright detection section
- **Improvements**: [`docs/improvements/copyright-detection.md`](../../improvements/copyright-detection.md) — Beyond-parity improvements over Python
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
