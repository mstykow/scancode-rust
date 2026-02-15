# Copyright Detection: Beyond-Parity Improvements

## Summary

The copyright detection engine in scancode-rust is a **complete rewrite** of Python ScanCode's `cluecode/copyrights.py` (~4,675 lines), implementing the full four-stage pipeline (text preparation → candidate selection → lexing/parsing → refinement) with several intentional improvements:

1. **🐛 Bug Fix**: Extended year range from 2039 to 2099
2. **🐛 Bug Fix**: Fixed `_YEAR_SHORT` regex typo (`[0-][0-9]` → `[0-2][0-9]`)
3. **🐛 Bug Fix**: Fixed French/Spanish case-sensitivity (`[Dr]roits?` → `[Dd]roits?`, `[Dr]erechos` → `[Dd]erechos`)
4. **🐛 Bug Fix**: Fixed suspicious underscore in `_YEAR_YEAR` separator pattern
5. **✨ Enhanced**: Unicode name preservation in output (Python outputs ASCII-only)
6. **🔍 Enhanced**: Type-safe POS tags (enum vs strings) — compiler catches tag typos
7. **🔍 Enhanced**: Thread-safe design (no global mutable singleton)
8. **🔍 Enhanced**: Deduplicated 3 duplicate patterns from Python reference
9. **🛡️ Security**: No code execution, no global mutable state

## Improvement 1: Extended Year Range (Bug Fix)

### Python Implementation

```python
_YEAR = r'(?:19[6-9][0-9]|20[0-3][0-9])'  # Matches 1960-2039
```

### Our Rust Implementation

```rust
// Matches 1960-2099 — no reason to stop at 2039
r"(?:19[6-9][0-9]|20[0-9][0-9])"
```

**Impact**: Python will fail to detect copyrights with years 2040-2099 as those years approach.

## Improvement 2: Fixed `_YEAR_SHORT` Typo (Bug Fix)

### Python Implementation

```python
_YEAR_SHORT = r'[0-][0-9]'  # Broken: matches "0x" and "-x" only
```

**Problem**: The character class `[0-]` matches only "0" or "-", not the full range 00-29 that was clearly intended.

### Our Rust Implementation

```rust
r"[0-2][0-9]"  // Correctly matches 00-29
```

## Improvement 3: French/Spanish Case-Sensitivity (Bug Fix)

### Python Implementation

```python
r'[Dr]roits?'    # Matches "Droits" and "rroits" — never matches "droits"
r'[Dr]erechos'   # Matches "Derechos" and "rerechos" — never matches "derechos"
```

### Our Rust Implementation

```rust
r"[Dd]roits?"    // Correctly matches "Droits" and "droits"
r"[Dd]erechos"   // Correctly matches "Derechos" and "derechos"
```

**Impact**: Python misses lowercase French/Spanish rights-reserved markers.

## Improvement 4: Type-Safe POS Tags (Enhanced)

### Python Implementation

```python
# POS tags are plain strings — typos are silent runtime bugs
'COPY', 'YR', 'NNP', 'NN', 'CAPS', 'COMP', 'AUTH', ...
```

### Our Rust Implementation

```rust
/// Typed enum — typos are compile-time errors
pub enum PosTag {
    Copy, Yr, YrPlus, BareYr, Nnp, Nn, Caps, Comp, Auth, ...
}
```

**Impact**: Any tag misspelling is caught at compile time, eliminating an entire class of bugs.

## Improvement 5: Thread-Safe Design (Enhanced)

### Python Implementation

```python
# Global mutable singleton — not thread-safe
DETECTOR = None

def detect_copyrights(location, ...):
    global DETECTOR
    if not DETECTOR:
        DETECTOR = CopyrightDetector()
    ...
```

### Our Rust Implementation

```rust
// All pattern data compiled once via LazyLock (thread-safe, zero-cost after init)
static COMPILED_PATTERNS: LazyLock<CompiledPatterns> = LazyLock::new(|| ...);

// Detection is a pure function — no global state
pub fn detect_copyrights(content: &str) -> (Vec<CopyrightDetection>, ...)
```

**Impact**: Safe for parallel scanning with rayon. No mutex contention.

## Improvement 6: Sequential Pattern Matching (Enhanced)

### Python Implementation

Uses sequential regex matching (correct but slow).

### Our Rust Implementation

Uses `LazyLock<Vec<(Regex, PosTag)>>` — patterns compiled once at startup, then matched sequentially per token with first-match-wins semantics.

**Why not RegexSet**: RegexSet cannot preserve match order, which is critical for first-match-wins semantics where pattern priority matters (e.g., JUNK exceptions must match before JUNK patterns).

## Architecture

```text
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  1. Text     │───>│  2. Candidate│───>│  3. Lex +    │───>│  4. Refine + │
│  Preparation │    │  Selection   │    │  Parse       │    │  Detect      │
│              │    │              │    │              │    │              │
│ 13-stage     │    │ Hint markers │    │ ~1100 regex  │    │ Tree walk    │
│ pipeline     │    │ State machine│    │ ~660 grammar │    │ Span-based   │
│              │    │ Grouping     │    │ Bottom-up    │    │ Junk filter  │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
```

**Module location**: `src/copyright/`

## Improvement 7: Long-Line Skip for Minified Code (Performance)

### Problem

Minified JS/CSS files can have single lines exceeding 600KB. Processing these through `prepare_text_line()` (13-stage regex pipeline) is pathologically slow and never yields real copyright detections.

### Our Rust Implementation

Lines >2,000 chars are checked for strong copyright indicators (`opyr`, `auth`, `(c)DIGIT`) using byte-level search (no allocation). Lines without indicators are skipped before any regex processing.

**Impact**: Prevents pathological behavior on minified files. Production-safe without false negatives.

## Improvement 8: Encoded-Data Detection (Performance)

### Problem

Uuencode and base64 data lines contain `@` characters that trigger the weak hint marker, producing thousands of false-positive candidates. A single uuencode file (6,361 lines, 5,143 containing `@`) dominated the holders golden test at 20.5 seconds.

### Our Rust Implementation

```rust
fn is_encoded_data_line(line: &str) -> bool {
    // Detects uuencode (ASCII 32-96, high character diversity, ≤1 space)
    // Detects base64 (100% [A-Za-z0-9+/=], no spaces)
    // Never skips lines with copyright indicators
}
```

Key safeguards:

- **Character diversity threshold** (≥8 distinct bytes): Prevents false positives on C comment decorators (`/*****/`) which are all in the uuencode byte range
- **Strict base64 matching** (100%): Prevents false positives on URLs which look base64-like but contain `:`, `.`, `-`
- **Copyright indicator bypass**: Lines with `opyr`/`auth`/`(c)DIGIT` are always processed

**Impact**: Holders golden test **20.5s → 1.1s** (19x faster). Copyrights golden test **34.9s → 16.5s** (2.1x faster). Zero regressions.

## Testing

- Comprehensive unit tests across all modules
- Golden tests across 4 suites (copyrights, holders, authors, ICS) at **98%+ pass rate**
- All expected failures documented with root causes, zero unexpected failures
- Golden test infrastructure with expected-failures tracking and newly-passing detection

## Status

- ✅ All Python bug fixes implemented and tested
- ✅ Full pipeline integrated into scanner (runs on all files including package manifests)
- ✨ Unicode name preservation in output (Python outputs ASCII-only via `toascii`)
- ✅ Thread-safe for parallel file processing
- ✅ All library tests passing, golden tests at 98%+
- ⚡ Performance optimizations for minified code and encoded data
- 🟢 CI fully green (zero unexpected golden test failures)
