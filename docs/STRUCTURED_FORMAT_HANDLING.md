# Structured Format Handling - Implementation Plan

## Overview

This plan addresses feature parity with Python ScanCode for structured format handling. The Rust implementation currently lacks several format-specific text extraction handlers that Python has, causing test failures like `ar-ER.js.map`.

## Current State Summary

| Format | Python ScanCode | Rust ScanCode | Gap |
|--------|-----------------|---------------|-----|
| UTF-8/16/32 text | ✅ | ✅ | None |
| Source maps | ✅ | ❌ | **High** |
| PDF | ✅ | ❌ | Medium |
| HTML/XML markup | ✅ | ❌ | Low |
| Spline Font Database | ✅ | ❌ | Low |
| Binary string extraction | ✅ | ⚠️ | Medium |

## Phase 1: Source Map Support (High Priority)

**Goal**: Fix `ar-ER.js.map` and similar source map file license detection.

**Effort**: Small

### Background

Source map files (`.js.map`, `.css.map`) are JSON files used in web development to map compiled/minified code back to original sources. The `sourcesContent` field contains the actual source code with licenses.

### Python Implementation

**File**: `reference/scancode-toolkit/src/textcode/analysis.py`

```python
# Lines 131-139: Detection and routing
if T.is_js_map:
    numbered_lines = list(enumerate(js_map_sources_lines(location), start_line))
    return numbered_lines

# Lines 223-247: Extraction function
def js_map_sources_lines(location):
    with io.open(location, encoding='utf-8') as jsm:
        content = json.load(jsm)
        sources = content.get('sourcesContent', [])
        for entry in sources:
            entry = replace_verbatim_cr_lf_chars(entry)
            for line in entry.splitlines():
                l = remove_verbatim_cr_lf_tab_chars(line)
                yield l
```

### Rust Implementation Plan

**File**: `src/utils/sourcemap.rs` (new file)

```rust
/// Extract source content from a source map file.
pub fn extract_sourcemap_content(bytes: &[u8]) -> Option<String> {
    // 1. Parse as JSON using serde_json
    // 2. Validate source map structure (has "version" and "sourcesContent")
    // 3. Extract and concatenate sourcesContent entries
    // 4. Return combined text
}
```

**File**: `src/utils/file_text.rs`

Modify `extract_text_for_detection()`:
```rust
// Before UTF-8 handling, check for source map
if is_sourcemap_file(path) {
    if let Some(text) = extract_sourcemap_content(bytes) {
        return Some(FileText {
            text,
            source: TextSource::SourceMap,
        });
    }
}
```

### Test Coverage

1. Unit tests in `src/utils/sourcemap.rs`:
   - Valid source map with single source
   - Valid source map with multiple sources
   - Source map with null sourcesContent
   - Invalid JSON handling
   - Non-source-map JSON handling

2. Integration test:
   - `ar-ER.js.map` - should detect 1 MIT match (mit_129.RULE)

### Dependencies

- `serde_json` - already in Cargo.toml

### Success Criteria

- `ar-ER.js.map` test passes with 1 MIT detection
- Source map files with licenses in `sourcesContent` are correctly processed

---

## Phase 2: PDF Text Extraction (Medium Priority)

**Goal**: Extract text from PDF files for license detection.

**Effort**: Medium

### Background

PDFs often contain license information in their text content. Python uses `pdfminer.six` to extract text.

### Python Implementation

**File**: `reference/scancode-toolkit/src/textcode/analysis.py`

```python
# Lines 101-104
if T.is_pdf and T.is_pdf_with_text:
    return enumerate(unicode_text_lines_from_pdf(location), start_line)

# Lines 190-196
def unicode_text_lines_from_pdf(location):
    for line in pdf.get_text_lines(location):
        yield as_unicode(line)
```

**File**: `reference/scancode-toolkit/src/textcode/pdf.py`

Uses `pdfminer.six` library for PDF parsing.

### Rust Implementation Plan

**File**: `src/utils/pdf.rs` (new file)

Options for PDF extraction:
1. **pdf-extract** crate - Pure Rust, simple API
2. **lopdf** crate - Lower level, more control
3. **poppler** bindings - Requires system library

Recommended: Start with `pdf-extract` for simplicity.

```rust
/// Extract text from a PDF file.
pub fn extract_pdf_text(bytes: &[u8]) -> Option<String> {
    // Use pdf-extract or lopdf to extract text
}
```

**File**: `src/utils/file_text.rs`

```rust
fn handle_binary_file(bytes: &[u8], path: &Path) -> Option<FileText> {
    if is_pdf(bytes) {
        if let Some(text) = extract_pdf_text(bytes) {
            return Some(FileText {
                text,
                source: TextSource::PdfText,
            });
        }
        return None;  // PDF without extractable text
    }
    // ... rest of binary handling
}
```

### Test Coverage

1. Unit tests:
   - PDF with extractable text
   - PDF without text (scanned image)
   - Corrupted PDF handling
   - Password-protected PDF (if supported)

2. Integration test:
   - Find or create test PDF with license text

### Dependencies

Add to `Cargo.toml`:
```toml
pdf-extract = "0.7"  # or lopdf = "0.34"
```

### Success Criteria

- PDF files with embedded licenses are detected
- Binary PDFs (scanned images) are properly skipped

---

## Phase 3: HTML/XML Markup Stripping (Low Priority)

**Goal**: Strip HTML/XML tags for cleaner license detection.

**Effort**: Small

### Background

HTML and XML files contain markup that can interfere with license detection. Python strips these tags when `demarkup=True`.

### Python Implementation

**File**: `reference/scancode-toolkit/src/textcode/analysis.py`

```python
# Lines 115-129
if demarkup and markup.is_markup(location):
    numbered_lines = list(enumerate(markup.demarkup(location), start_line))
    return numbered_lines
```

**File**: `reference/scancode-toolkit/src/textcode/markup.py`

Full implementation with HTML/XML tag stripping, entity decoding, etc.

### Rust Implementation Plan

**File**: `src/utils/markup.rs` (new file)

```rust
/// Check if file is HTML/XML markup.
pub fn is_markup(path: &Path, bytes: &[u8]) -> bool {
    // Check extension (.html, .xml, .xhtml, etc.)
    // Check content for markup signatures
}

/// Strip markup tags and decode entities.
pub fn strip_markup(text: &str) -> String {
    // Use a crate like `htmlescape` or `scraper`
    // Strip tags, decode entities, return plain text
}
```

### Test Coverage

1. Unit tests:
   - HTML with license in comments
   - XML with license text
   - Entity decoding (&amp;, &lt;, etc.)
   - Nested tags handling

### Dependencies

Add to `Cargo.toml`:
```toml
scraper = "0.18"  # HTML parsing
htmlescape = "0.3"  # Entity decoding
```

### Success Criteria

- HTML/XML files with licenses are properly detected
- Entity-encoded license text is decoded

---

## Phase 4: Binary String Extraction (Medium Priority)

**Goal**: Extract printable strings from binary files for license detection.

**Effort**: Medium

### Background

Binary files (executables, object files, etc.) can contain embedded license strings. Python uses a `strings`-like approach.

### Python Implementation

**File**: `reference/scancode-toolkit/src/textcode/analysis.py`

```python
# Lines 169-174
if T.is_binary:
    return enumerate(unicode_text_lines_from_binary(location), start_line)

# Lines 179-187
def unicode_text_lines_from_binary(location):
    T = typecode.get_type(location)
    if T.contains_text:
        for line in strings.strings_from_file(location):
            yield remove_verbatim_cr_lf_tab_chars(line)
```

**File**: `reference/scancode-toolkit/src/textcode/strings.py`

Full implementation with configurable minimum string length, encoding detection, etc.

### Rust Implementation Plan

**File**: `src/utils/strings.rs` (new file)

```rust
/// Minimum string length to extract (matches Python default)
const MIN_STRING_LENGTH: usize = 4;

/// Extract printable strings from binary data.
pub fn extract_strings(bytes: &[u8]) -> Vec<String> {
    // Find runs of printable ASCII characters
    // Filter by minimum length
    // Handle UTF-8 sequences within binary
}
```

**File**: `src/utils/file_text.rs`

```rust
fn handle_binary_file(bytes: &[u8], path: &Path) -> Option<FileText> {
    // ... PDF handling first ...
    
    if should_skip_binary_extension(&ext) {
        return None;
    }
    
    // Try string extraction
    let strings = extract_strings(bytes);
    if !strings.is_empty() {
        return Some(FileText {
            text: strings.join("\n"),
            source: TextSource::BinaryStrings,
        });
    }
    
    // Fallback to current behavior
    // ...
}
```

### Test Coverage

1. Unit tests:
   - Binary with embedded ASCII strings
   - Binary with embedded UTF-8 strings
   - Minimum length filtering
   - Empty binary handling

2. Integration test:
   - Binary file with embedded license string

### Dependencies

None (pure Rust implementation)

### Success Criteria

- Binary files with embedded license strings are detected
- Non-printable binaries are properly skipped

---

## Phase 5: Spline Font Database (Low Priority)

**Goal**: Extract text from Spline Font Database files.

**Effort**: Small

### Background

`.sfdb` files are font source files that may contain license information.

### Python Implementation

**File**: `reference/scancode-toolkit/src/textcode/analysis.py`

```python
# Lines 106-112
if T.filetype_file.startswith('Spline Font Database'):
    return enumerate(
        (as_unicode(l) for l in sfdb.get_text_lines(location)),
        start_line,
    )
```

**File**: `reference/scancode-toolkit/src/textcode/sfdb.py`

Parses Spline Font Database format and extracts text lines.

### Rust Implementation Plan

**File**: `src/utils/sfdb.rs` (new file)

```rust
/// Extract text from Spline Font Database.
pub fn extract_sfdb_text(bytes: &[u8]) -> Option<String> {
    // Parse SFDB format
    // Extract text fields that may contain license info
}
```

### Test Coverage

1. Unit tests:
   - Valid SFDB file
   - SFDB with license text
   - Invalid SFDB handling

### Dependencies

None (custom format parser)

### Success Criteria

- SFDB files with licenses are detected

---

## Implementation Order

| Phase | Format | Effort | Priority | Blocker For |
|-------|--------|--------|----------|-------------|
| 1 | Source maps | Small | High | ar-ER.js.map test |
| 2 | PDF | Medium | Medium | Enterprise scans |
| 3 | Binary strings | Medium | Medium | Binary license detection |
| 4 | HTML/XML | Small | Low | Web file scans |
| 5 | SFDB | Small | Low | Rare format |

## File Changes Summary

### New Files

- `src/utils/sourcemap.rs` - Source map extraction
- `src/utils/pdf.rs` - PDF text extraction
- `src/utils/strings.rs` - Binary string extraction
- `src/utils/markup.rs` - HTML/XML stripping
- `src/utils/sfdb.rs` - Spline Font Database

### Modified Files

- `src/utils/file_text.rs` - Add format detection and routing
- `src/utils/mod.rs` - Export new modules
- `Cargo.toml` - Add dependencies

## Testing Strategy

Each phase should include:

1. **Unit tests** in the new module file
2. **Integration test** using actual test files
3. **Golden test** comparison with Python output (where applicable)

## References

- Python implementation: `reference/scancode-toolkit/src/textcode/analysis.py`
- Python PDF: `reference/scancode-toolkit/src/textcode/pdf.py`
- Python markup: `reference/scancode-toolkit/src/textcode/markup.py`
- Python strings: `reference/scancode-toolkit/src/textcode/strings.py`
- Python SFDB: `reference/scancode-toolkit/src/textcode/sfdb.py`
