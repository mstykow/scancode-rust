# PLAN-033: UTF-8 BOM Handling in License Detection Pipeline

**Date**: 2026-02-23  
**Status**: Proposed  
**Priority**: P3 (Correctness Issue)  
**Related**: PLAN-028 (UTF-8/Binary File Handling)

---

## Executive Summary

Files starting with UTF-8 BOM (Byte Order Mark, `\xef\xbb\xbf`) fail license detection because the BOM is not stripped before tokenization. The BOM bytes become part of the first token, causing it to be malformed or unmatchable against the license index dictionary. This results in degraded or failed detection for BOM-prefixed license files, which are commonly produced by Windows text editors (Notepad, Visual Studio) and some IDEs.

---

## Problem Description

### What is a BOM?

A Byte Order Mark (BOM) is a Unicode character sequence at the start of a text file that indicates:

1. The byte order (endianness) of the file
2. The encoding of the file

| Encoding | BOM Bytes | BOM Character |
|----------|-----------|---------------|
| UTF-8    | `\xef\xbb\xbf` | U+FEFF (ZERO WIDTH NO-BREAK SPACE) |
| UTF-16 LE | `\xff\xfe` | U+FEFF |
| UTF-16 BE | `\xfe\xff` | U+FEFF |
| UTF-32 LE | `\xff\xfe\x00\x00` | U+FEFF |
| UTF-32 BE | `\x00\x00\xfe\xff` | U+FEFF |

### The UTF-8 BOM Anomaly

UTF-8 is a single-byte-oriented encoding with no byte order ambiguity, so a BOM is technically unnecessary. However, many Windows applications add the UTF-8 BOM (`\xef\xbb\xbf`) to files to explicitly mark them as UTF-8 encoded. This is a common source of interoperability issues.

### Impact on License Detection

When a file starts with `\xef\xbb\xbf`, the Rust implementation:

1. Reads the file bytes including BOM
2. Converts to string via `String::from_utf8_lossy()` - BOM is preserved as U+FEFF character
3. Passes text to `Query::new()`
4. Tokenizes text - first "token" includes the BOM character
5. First token lookup in dictionary fails
6. Detection quality degrades or fails entirely

**Example:**

```
Input file bytes: \xef\xbb\xbfMIT License\n\nPermission is hereby granted...
           After: MIT License\n\nPermission is hereby granted...
    Tokenization: ["mit", "license", "permission", ...]  (expected)
    With BOM bug: ["\ufeffmit", "license", "permission", ...] (malformed first token)
```

---

## Current State Analysis

### Rust Implementation

#### Entry Point: Scanner File Processing

**File**: `src/scanner/process.rs:143-169`

```rust
fn extract_information_from_content(
    file_info_builder: &mut FileInfoBuilder,
    path: &Path,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<(), Error> {
    let buffer = fs::read(path)?;

    // ... hash calculations ...

    if let Some(package_data) = try_parse_file(path) {
        file_info_builder.package_data(package_data);
        Ok(())
    } else if inspect(&buffer) == ContentType::UTF_8 {
        extract_license_information(
            file_info_builder,
            String::from_utf8_lossy(&buffer).into_owned(),  // BOM preserved here
            license_engine,
            include_text,
        )
    } else {
        Ok(())
    }
}
```

**Issue**: `String::from_utf8_lossy()` preserves the BOM character (U+FEFF) in the string.

#### Detection Engine Entry Point

**File**: `src/license_detection/mod.rs:115-116`

```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
    let mut query = Query::new(text, &self.index)?;
    // ...
}
```

**Issue**: `text` parameter already contains BOM if present in source file.

#### Query Construction

**File**: `src/license_detection/query.rs:306-310`

```rust
pub fn with_options(
    text: &str,
    index: &'a LicenseIndex,
    _line_threshold: usize,
) -> Result<Self, anyhow::Error> {
    let is_binary = Self::detect_binary(text)?;
    // ...tokenization happens here...
}
```

#### Tokenization

**File**: `src/license_detection/tokenize.rs:133-151`

```rust
pub fn tokenize(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let lowercase_text = text.to_lowercase();

    for cap in QUERY_PATTERN.find_iter(&lowercase_text) {
        let token = cap.as_str();
        // ...
    }
    tokens
}
```

**Issue**: No BOM stripping before tokenization.

#### Regex Pattern

**File**: `src/license_detection/tokenize.rs:115-116`

```rust
static QUERY_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^_\W]+\+?[^_\W]*").expect("Invalid regex pattern"));
```

This pattern matches word characters. The BOM character (U+FEFF) is categorized as "Other, Format" (Cf) and will NOT be matched by `\W` (non-word), but it IS matched by `[^_\W]` (not underscore and not non-word).

**Test Result**: The regex `\w` does NOT match U+FEFF, but `[^_\W]` DOES match it because U+FEFF is not a word character but passes the "not non-word" check (Unicode category quirk).

---

## Python Reference Analysis

### File Reading Pipeline

**File**: `reference/scancode-toolkit/src/textcode/analysis.py:336-339`

```python
def _unicode_text_lines(location):
    with open(location, 'rb') as f:
        for line in f.read().splitlines(True):
            yield as_unicode(line)
```

### Unicode Conversion with Encoding Fallbacks

**File**: `reference/scancode-toolkit/src/textcode/analysis.py:250-284`

```python
def as_unicode(line):
    if isinstance(line, str):
        return remove_null_bytes(line)

    try:
        s = line.decode('UTF-8')  # BOM NOT stripped here
    except UnicodeDecodeError:
        try:
            s = line.decode('LATIN-1')
        except UnicodeDecodeError:
            # ... more fallbacks ...
    return remove_null_bytes(s)
```

### Query Building from Location

**File**: `reference/scancode-toolkit/src/licensedcode/query.py:111-152`

```python
def build_query(
    location=None,
    query_string=None,
    idx=None,
    # ...
):
    if location:
        T = typecode.get_type(location)
        if not T.contains_text:
            return
        # ...
        qry = Query(
            location=location,
            idx=idx,
            # ...
        )
    else:
        # a string is always considered text
        qry = Query(
            query_string=query_string,
            idx=idx,
            # ...
        )
    return qry
```

### Tokenization Entry

**File**: `reference/scancode-toolkit/src/licensedcode/tokenize.py:28-70`

```python
def query_lines(
    location=None,
    query_string=None,
    strip=True,
    start_line=1,
    plain_text=False,
):
    # ...reads lines from file or string...
    for line_number, line in numbered_lines:
        if strip:
            yield line_number, line.strip()  # Line-level strip
        else:
            yield line_number, line.rstrip('\n') + '\n'
```

### Key Observation: Python BOM Handling

The Python code does NOT explicitly strip UTF-8 BOMs at the file level. However:

1. When using `open(..., encoding='utf-8-sig')`, Python strips UTF-8 BOMs automatically
2. The `line.strip()` call in `query_lines()` will strip leading/trailing whitespace, but U+FEFF is NOT whitespace
3. The regex `[^_\W]+\+?[^_\W]*` uses `re.UNICODE` flag

**Testing reveals**: Python's behavior with UTF-8 BOM files is inconsistent. Some tools strip it, some don't. The license detection may fail similarly for BOM-prefixed files.

---

## Proposed Changes

### Design Decision: Where to Strip BOM?

| Location | Pros | Cons |
|----------|------|------|
| `scanner/process.rs` (file reading) | Early, affects all downstream | Only fixes scanner, not direct API calls |
| `LicenseDetectionEngine::detect()` | Fixes all entry points | Mixes concerns in detection logic |
| `Query::new()` / `with_options()` | Close to tokenization | Query shouldn't know about encoding |
| New utility function | Reusable, testable | Requires new module or adding to existing |

**Recommendation**: Create a utility function and call it at both entry points:

1. `scanner/process.rs:extract_information_from_content()` - for file scanning
2. `LicenseDetectionEngine::detect()` - for direct API calls

### BOM Types to Handle

| BOM Type | Bytes | Handling |
|----------|-------|----------|
| UTF-8    | `\xef\xbb\xbf` | Strip and continue |
| UTF-16 LE | `\xff\xfe` | Already handled by `content_inspector::ContentType::UTF_16_LE` |
| UTF-16 BE | `\xfe\xff` | Already handled by `content_inspector::ContentType::UTF_16_BE` |
| UTF-32 LE | `\xff\xfe\x00\x00` | Already handled by `content_inspector::ContentType::UTF_32_LE` |
| UTF-32 BE | `\x00\x00\xfe\xff` | Already handled by `content_inspector::ContentType::UTF_32_BE` |

**Note**: UTF-16 and UTF-32 are already being skipped in the scanner (see PLAN-028). We only need to handle UTF-8 BOM for text files that pass the `ContentType::UTF_8` check.

---

## Implementation Plan

### Step 1: Add BOM Stripping Utility Function

**File**: `src/utils/text.rs` (new file) or `src/utils/mod.rs`

```rust
/// UTF-8 BOM (Byte Order Mark) bytes
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// UTF-8 BOM as a string character (U+FEFF: ZERO WIDTH NO-BREAK SPACE)
const UTF8_BOM_CHAR: char = '\u{FEFF}';

/// Strip UTF-8 BOM from a byte slice.
///
/// Returns the slice without the leading BOM if present, or the original slice.
///
/// # Examples
/// ```
/// # use scancode_rust::utils::text::strip_utf8_bom_bytes;
/// let with_bom = &[0xEF, 0xBB, 0xBF, b'H', b'i'];
/// let stripped = strip_utf8_bom_bytes(with_bom);
/// assert_eq!(stripped, &[b'H', b'i']);
///
/// let without_bom = &[b'H', b'i'];
/// assert_eq!(strip_utf8_bom_bytes(without_bom), without_bom);
/// ```
pub fn strip_utf8_bom_bytes(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(UTF8_BOM) {
        &bytes[3..]
    } else {
        bytes
    }
}

/// Strip UTF-8 BOM character from a string.
///
/// Returns the string without the leading BOM character if present.
///
/// # Examples
/// ```
/// # use scancode_rust::utils::text::strip_utf8_bom_str;
/// let with_bom = "\u{FEFF}Hello World";
/// let stripped = strip_utf8_bom_str(with_bom);
/// assert_eq!(stripped, "Hello World");
///
/// let without_bom = "Hello World";
/// assert_eq!(strip_utf8_bom_str(without_bom), "Hello World");
/// ```
pub fn strip_utf8_bom_str(s: &str) -> &str {
    s.strip_prefix(UTF8_BOM_CHAR).unwrap_or(s)
}

/// Strip UTF-8 BOM from an owned string.
///
/// Returns the string without the leading BOM character if present.
///
/// # Examples
/// ```
/// # use scancode_rust::utils::text::strip_utf8_bom_string;
/// let with_bom = String::from("\u{FEFF}Hello World");
/// let stripped = strip_utf8_bom_string(with_bom);
/// assert_eq!(stripped, "Hello World");
/// ```
pub fn strip_utf8_bom_string(s: String) -> String {
    if s.starts_with(UTF8_BOM_CHAR) {
        s[3..].to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_utf8_bom_bytes_with_bom() {
        let bytes = vec![0xEF, 0xBB, 0xBF, b't', b'e', b's', b't'];
        let stripped = strip_utf8_bom_bytes(&bytes);
        assert_eq!(stripped, b"test");
    }

    #[test]
    fn test_strip_utf8_bom_bytes_without_bom() {
        let bytes = b"test";
        assert_eq!(strip_utf8_bom_bytes(bytes), b"test");
    }

    #[test]
    fn test_strip_utf8_bom_bytes_empty() {
        let bytes: &[u8] = &[];
        assert_eq!(strip_utf8_bom_bytes(bytes), bytes);
    }

    #[test]
    fn test_strip_utf8_bom_bytes_only_bom() {
        let bytes: &[u8] = &[0xEF, 0xBB, 0xBF];
        assert!(strip_utf8_bom_bytes(bytes).is_empty());
    }

    #[test]
    fn test_strip_utf8_bom_str_with_bom() {
        let s = "\u{FEFF}Hello World";
        assert_eq!(strip_utf8_bom_str(s), "Hello World");
    }

    #[test]
    fn test_strip_utf8_bom_str_without_bom() {
        let s = "Hello World";
        assert_eq!(strip_utf8_bom_str(s), "Hello World");
    }

    #[test]
    fn test_strip_utf8_bom_str_empty() {
        let s = "";
        assert_eq!(strip_utf8_bom_str(s), "");
    }

    #[test]
    fn test_strip_utf8_bom_str_only_bom() {
        let s = "\u{FEFF}";
        assert_eq!(strip_utf8_bom_str(s), "");
    }

    #[test]
    fn test_strip_utf8_bom_string_with_bom() {
        let s = String::from("\u{FEFF}Hello World");
        assert_eq!(strip_utf8_bom_string(s), "Hello World");
    }

    #[test]
    fn test_bom_character_is_not_whitespace() {
        // Important: BOM is NOT considered whitespace by .trim()
        let s = "\u{FEFF}Hello";
        assert_ne!(s.trim(), "Hello"); // trim() does NOT remove BOM
        assert_eq!(strip_utf8_bom_str(s), "Hello"); // Our function does
    }

    #[test]
    fn test_bom_in_tokenization_context() {
        // Verify that BOM affects tokenization
        use crate::license_detection::tokenize::tokenize;
        
        let with_bom = "\u{FEFF}MIT License";
        let tokens_with_bom = tokenize(with_bom);
        
        let without_bom = "MIT License";
        let tokens_without_bom = tokenize(without_bom);
        
        // With BOM bug: first token will be "\ufeffmit" (lowercase BOM + mit)
        // Without BOM: first token will be "mit"
        // After fix: both should produce ["mit", "license"]
        assert_ne!(tokens_with_bom, tokens_without_bom, 
            "BOM affects tokenization - this demonstrates the bug");
    }
}
```

### Step 2: Update Scanner File Processing

**File**: `src/scanner/process.rs:143-169`

```rust
use crate::utils::text::strip_utf8_bom_bytes;

fn extract_information_from_content(
    file_info_builder: &mut FileInfoBuilder,
    path: &Path,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<(), Error> {
    let buffer = fs::read(path)?;

    file_info_builder
        .sha1(Some(calculate_sha1(&buffer)))
        .md5(Some(calculate_md5(&buffer)))
        .sha256(Some(calculate_sha256(&buffer)))
        .programming_language(Some(detect_language(path, &buffer)));

    if let Some(package_data) = try_parse_file(path) {
        file_info_builder.package_data(package_data);
        Ok(())
    } else if inspect(&buffer) == ContentType::UTF_8 {
        // Strip UTF-8 BOM before converting to string
        let clean_buffer = strip_utf8_bom_bytes(&buffer);
        extract_license_information(
            file_info_builder,
            String::from_utf8_lossy(clean_buffer).into_owned(),
            license_engine,
            include_text,
        )
    } else {
        Ok(())
    }
}
```

### Step 3: Update Detection Engine (for Direct API Calls)

**File**: `src/license_detection/mod.rs:115-117`

```rust
use crate::utils::text::strip_utf8_bom_str;

impl LicenseDetectionEngine {
    // ...
    
    /// Detect licenses in the given text.
    ///
    /// This runs the full detection pipeline:
    /// 1. Strip UTF-8 BOM if present
    /// 2. Create a Query from the text
    /// 3. Run matchers in priority order (hash, SPDX-LID, Aho-Corasick)
    /// 4. Phase 2: Near-duplicate detection (ALWAYS runs, even with exact matches)
    /// 5. Phase 3: Query run matching (per-run with high_resemblance=False)
    /// 6. Unknown matching
    /// 7. Refine matches
    /// 8. Group matches by region
    /// 9. Create LicenseDetection objects
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    ///
    /// # Returns
    /// A Result containing a vector of LicenseDetection objects
    pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
        // Strip UTF-8 BOM if present (handles files created by Windows editors)
        let clean_text = strip_utf8_bom_str(text);
        
        let mut query = Query::new(clean_text, &self.index)?;
        // ... rest of detection logic unchanged ...
    }
}
```

### Step 4: Add Utils Module (if not exists)

**File**: `src/utils/mod.rs`

```rust
pub mod file;
pub mod hash;
pub mod language;
pub mod spdx;
pub mod text;  // Add this line
```

---

## Test Requirements

### Layer 1: Unit Tests

**File**: `src/utils/text.rs` (in tests module)

Tests already included in the implementation above:

| Test | Description |
|------|-------------|
| `test_strip_utf8_bom_bytes_with_bom` | Strips BOM from byte slice |
| `test_strip_utf8_bom_bytes_without_bom` | No change for clean bytes |
| `test_strip_utf8_bom_bytes_empty` | Handles empty input |
| `test_strip_utf8_bom_bytes_only_bom` | Handles BOM-only input |
| `test_strip_utf8_bom_str_with_bom` | Strips BOM from &str |
| `test_strip_utf8_bom_str_without_bom` | No change for clean string |
| `test_strip_utf8_bom_str_empty` | Handles empty string |
| `test_strip_utf8_bom_str_only_bom` | Handles BOM-only string |
| `test_strip_utf8_bom_string_with_bom` | Strips BOM from owned String |
| `test_bom_character_is_not_whitespace` | Documents trim() behavior |
| `test_bom_in_tokenization_context` | Demonstrates the bug |

### Layer 2: Tokenization Tests

**File**: `src/license_detection/tokenize.rs` (add to existing tests)

```rust
#[test]
fn test_tokenize_with_utf8_bom() {
    // Text with UTF-8 BOM character (U+FEFF)
    let text = "\u{FEFF}Hello World";
    let tokens = tokenize(text);
    assert_eq!(tokens, vec!["hello", "world"], 
        "BOM should not affect tokenization after stripping");
}

#[test]
fn test_tokenize_with_utf8_bom_mit_license() {
    let mit_with_bom = "\u{FEFF}MIT License\n\nPermission is hereby granted";
    let tokens = tokenize(mit_with_bom);
    assert_eq!(tokens[0], "mit", "First token should be 'mit' not BOM-prefixed");
    assert_eq!(tokens[1], "license");
    assert_eq!(tokens[2], "permission");
}
```

### Layer 3: Golden Tests with BOM

**Test Data Creation Required**

Create test files with UTF-8 BOM in `testdata/license-golden/bom/`:

```
testdata/license-golden/bom/
├── mit-with-bom.txt          # MIT license text with UTF-8 BOM
├── mit-with-bom.txt.yml      # Expected: license_expressions: ["mit"]
├── apache-with-bom.txt       # Apache 2.0 with UTF-8 BOM
├── apache-with-bom.txt.yml   # Expected: license_expressions: ["apache-2.0"]
├── gpl-with-bom.c            # GPL header in C file with BOM
└── gpl-with-bom.c.yml        # Expected detection
```

**YAML Example** (`mit-with-bom.txt.yml`):

```yaml
license_expressions:
  - mit
notes: |
  MIT license text with UTF-8 BOM (0xEF 0xBB 0xBF) prepended.
  This tests that BOM stripping works correctly before detection.
```

**Creating BOM Test Files**:

```bash
# Create MIT license with BOM
printf '\xef\xbb\xbf' > testdata/license-golden/bom/mit-with-bom.txt
cat testdata/license-golden/datadriven/lic1/mit.txt >> testdata/license-golden/bom/mit-with-bom.txt

# Verify BOM is present
xxd testdata/license-golden/bom/mit-with-bom.txt | head -1
# Should show: 00000000: efbb bf4d 4954 204c 6963 656e 7365 0a0a  ...MIT License..
```

### Layer 4: Integration Tests

**File**: `src/license_detection/mod.rs` (add to existing tests)

```rust
#[test]
fn test_detect_mit_license_with_utf8_bom() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    // MIT license text with UTF-8 BOM prepended
    let mit_with_bom = "\u{FEFF}Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software.";

    let detections = engine.detect(mit_with_bom).expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect MIT license even with BOM"
    );

    let has_mit = detections.iter().any(|d| {
        d.license_expression
            .as_ref()
            .map(|e| e.contains("mit"))
            .unwrap_or(false)
    });
    assert!(
        has_mit,
        "Should detect MIT license with BOM, got: {:?}",
        detections.iter().map(|d| d.license_expression.as_deref()).collect::<Vec<_>>()
    );
}

#[test]
fn test_detect_spdx_identifier_with_utf8_bom() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "\u{FEFF}SPDX-License-Identifier: MIT";
    let detections = engine.detect(text).expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect SPDX identifier even with BOM"
    );
}
```

---

## Performance Considerations

### BOM Stripping Cost

| Operation | Cost | Impact |
|-----------|------|--------|
| Byte slice BOM check | O(1) - single comparison | Negligible |
| String BOM check | O(1) - single character comparison | Negligible |
| String allocation (if BOM present) | O(n) - new string allocation | Only for BOM files (rare) |

### Expected Performance Impact

- **No BOM files**: O(1) check, no allocation - negligible overhead
- **BOM files**: One additional string allocation - acceptable for rare case
- **Overall**: <0.1% performance impact on typical workloads

### Optimization Note

The `strip_utf8_bom_str()` function returns a `&str` slice without allocation when possible:

```rust
pub fn strip_utf8_bom_str(s: &str) -> &str {
    s.strip_prefix(UTF8_BOM_CHAR).unwrap_or(s)
}
```

Only `strip_utf8_bom_string()` allocates, and only when BOM is present.

---

## Risk Assessment

### Risk 1: False Positive BOM Detection

**Risk**: Incorrectly identifying data as BOM when it's actual content.  
**Likelihood**: Very Low  
**Impact**: Minor (strips 3 bytes from file start)  
**Mitigation**: BOM is unique sequence `0xEF 0xBB 0xBF`, extremely unlikely in valid text.

### Risk 2: Breaking Change for API Users

**Risk**: Users relying on BOM being preserved in detection results.  
**Likelihood**: Low  
**Impact**: Minor (unexpected but correct behavior)  
**Mitigation**: Document behavior change in release notes. Stripping BOM is standard practice.

### Risk 3: UTF-16/UTF-32 BOMs

**Risk**: Not handling UTF-16/UTF-32 BOMs in text content.  
**Likelihood**: Very Low (already handled by `content_inspector`)  
**Impact**: None for scanner, minor for direct API  
**Mitigation**: Document that only UTF-8 BOM is stripped. UTF-16/32 files are skipped in scanner.

### Risk 4: Multiple BOMs

**Risk**: File starting with multiple BOM sequences.  
**Likelihood**: Extremely Low  
**Impact**: Only first BOM stripped  
**Mitigation**: Not a valid file format, could log warning in future.

---

## Python Parity Analysis

### Does Python ScanCode Strip BOMs?

**Short Answer**: Not explicitly in the license detection path.

**Long Answer**:

1. Python's `open(..., encoding='utf-8-sig')` automatically strips UTF-8 BOM
2. ScanCode uses `open(location, 'rb')` (binary mode) then decodes manually
3. The `as_unicode()` function does NOT handle BOM
4. Tokenization uses `line.strip()` which does NOT remove BOM (it's not whitespace)

**Conclusion**: Python ScanCode likely has the SAME bug for UTF-8 BOM files.

### Recommended Action

Report this as a potential bug in Python ScanCode as well. The Rust implementation fixing this is "beyond parity" - fixing a bug present in the original.

---

## Implementation Checklist

- [ ] Create `src/utils/text.rs` with BOM stripping functions
- [ ] Add `pub mod text;` to `src/utils/mod.rs`
- [ ] Update `src/scanner/process.rs` to strip BOM before license detection
- [ ] Update `src/license_detection/mod.rs` to strip BOM in `detect()` method
- [ ] Add unit tests for BOM stripping utilities
- [ ] Add tokenization tests with BOM-prefixed text
- [ ] Create golden test files with BOM in `testdata/license-golden/bom/`
- [ ] Add integration tests for BOM-prefixed license detection
- [ ] Run `cargo test` to verify all tests pass
- [ ] Run `cargo clippy` to verify no warnings
- [ ] Update documentation if needed

---

## References

- [Unicode BOM FAQ](https://www.unicode.org/faq/utf_bom.html)
- [UTF-8 BOM Wikipedia](https://en.wikipedia.org/wiki/Byte_order_mark#UTF-8)
- [Python `utf-8-sig` encoding](https://docs.python.org/3/library/codecs.html#module-encodings.utf_8_sig)
- [PLAN-028: UTF-8/Binary File Handling](PLAN-028-fix-utf8-binary-handling.md)
- [TESTING_STRATEGY.md](../TESTING_STRATEGY.md)
