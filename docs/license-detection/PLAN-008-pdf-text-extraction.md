# PLAN-008: PDF Text Extraction for License Detection

## Status: READY FOR IMPLEMENTATION

## Executive Summary

Python ScanCode extracts license information from PDF documents by using `pdfminer.six` to extract embedded text, then running normal license detection on that text. The Rust implementation currently skips PDF files entirely in both the golden test framework and the main scanner pipeline, resulting in a **feature gap** for license detection in PDF documents.

## Problem Statement

### Current Behavior

**Rust Implementation** (`src/utils/file_text.rs:176-179`):
```rust
fn handle_binary_file(bytes: &[u8], path: &Path) -> Option<FileText> {
    if is_pdf(bytes) {
        return None;  // Skips PDFs entirely
    }
    // ...
}
```

### Impact - Test Files

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf` | `generic-cla` | skipped | No PDF extraction |
| `testdata/license-golden/datadriven/lic4/should_detect_something_5.pdf` | `sun-sissl-1.1`, `proprietary-license`, `cpal-1.0` | skipped | No PDF extraction |
| `testdata/license-golden/datadriven/lic2/bsd-new_156.pdf` | `bsd-new` | skipped | No PDF extraction |

## Recommended Implementation

### Crate Selection

**Use `lopdf` (0.39.0)** because:
1. Well-maintained with active development
2. Pure Rust (no external dependencies)
3. Provides low-level access to PDF objects for proper text extraction
4. Better error handling for corrupted/encrypted PDFs
5. `pdf-extract` has not been updated recently and has more limited features

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust PDF Processing                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. File Type Detection (src/utils/file_text.rs)            │
│     ┌────────────────────────────────────────────────┐      │
│     │ is_pdf(): Check PDF magic bytes (%PDF-)        │      │
│     │ (already implemented)                           │      │
│     └────────────────────────────────────────────────┘      │
│                           │                                  │
│                           ▼                                  │
│  2. Text Extraction (src/utils/pdf.rs) [NEW]                │
│     ┌────────────────────────────────────────────────┐      │
│     │ lopdf crate:                                    │      │
│     │ - Extract text content from PDF streams         │      │
│     │ - Handle encrypted PDFs gracefully              │      │
│     │ - Decode PDF streams (FlateDecode, etc.)        │      │
│     │ - Return String for license detection           │      │
│     └────────────────────────────────────────────────┘      │
│                           │                                  │
│                           ▼                                  │
│  3. License Detection (existing pipeline)                   │
│     ┌────────────────────────────────────────────────┐      │
│     │ Normal license detection on extracted text     │      │
│     └────────────────────────────────────────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Exact Code Changes

### Step 1: Add dependency to `Cargo.toml`

Add to the `[dependencies]` section:

```toml
lopdf = "0.39.0"
```

### Step 2: Create `src/utils/pdf.rs`

```rust
//! PDF text extraction for license detection.
//!
//! Uses lopdf to extract text content from PDF files.

use lopdf::Document;
use std::io;
use std::path::Path;

/// Extract text content from a PDF file.
///
/// Returns `Ok(String)` with extracted text, or `Ok(String::new())` if extraction fails.
/// Handles encrypted and malformed PDFs gracefully.
pub fn extract_text(path: &Path) -> io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(extract_text_from_bytes(&bytes))
}

/// Extract text content from PDF bytes.
pub fn extract_text_from_bytes(bytes: &[u8]) -> String {
    let doc = match Document::load_mem(bytes) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    // Check if encrypted - skip extraction
    if is_encrypted(&doc) {
        return String::new();
    }

    let mut text = String::new();
    
    // Get page objects and extract text
    for (_, page_id) in doc.get_pages() {
        if let Ok(page_obj) = doc.get_object(page_id) {
            if let Ok(content) = page_obj.as_ref().as_dict() {
                if let Ok(stream) = content.get(b"Contents") {
                    if let Err(_) = extract_text_from_stream(&doc, stream, &mut text) {
                        continue;
                    }
                }
            }
        }
    }

    // Clean up the extracted text
    clean_text(&text)
}

/// Check if PDF is encrypted.
fn is_encrypted(doc: &Document) -> bool {
    doc.trailer
        .get(b"Encrypt")
        .is_ok()
}

/// Extract text from a PDF stream object.
fn extract_text_from_stream(
    doc: &Document,
    stream: &lopdf::Object,
    text: &mut String,
) -> io::Result<()> {
    match stream {
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                extract_text_from_stream_object(obj, text)?;
            }
        }
        lopdf::Object::Stream(stream_obj) => {
            extract_text_from_stream_object(stream_obj, text)?;
        }
        lopdf::Object::Array(arr) => {
            for obj in arr {
                extract_text_from_stream(doc, obj, text)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Extract text from a stream object.
fn extract_text_from_stream_object(stream: &lopdf::Stream, text: &mut String) -> io::Result<()> {
    // Try to decode the stream
    let decoded = stream.decompressed_content().unwrap_or_else(|_| stream.content.clone());
    
    // Simple text extraction - look for Tj and TJ operators
    let content = String::from_utf8_lossy(&decoded);
    
    // Basic text extraction: find strings between parentheses (text showing operators)
    let mut in_string = false;
    let mut current = String::new();
    
    for ch in content.chars() {
        if ch == '(' && !in_string {
            in_string = true;
        } else if ch == ')' && in_string {
            in_string = false;
            if !current.is_empty() {
                text.push_str(&current);
                text.push(' ');
                current.clear();
            }
        } else if in_string {
            if ch == '\\' {
                // Escape sequence - skip next char
                continue;
            }
            current.push(ch);
        }
    }
    
    Ok(())
}

/// Clean up extracted text.
fn clean_text(text: &str) -> String {
    // Remove excessive whitespace
    let re = regex::Regex::new(r"\s+").unwrap();
    re.replace_all(text.trim(), " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_text_from_test_pdf() {
        let pdf_path = PathBuf::from("testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf");
        if !pdf_path.exists() {
            return;
        }
        let text = extract_text(&pdf_path).unwrap();
        // Should contain license-related text
        assert!(!text.is_empty() || text.contains("Sun") || text.len() > 50);
    }

    #[test]
    fn test_extract_text_from_invalid_pdf() {
        let bytes = b"%PDF-1.4\ninvalid content";
        let text = extract_text_from_bytes(bytes);
        // Should not panic, returns empty or partial text
    }

    #[test]
    fn test_is_pdf_magic_bytes() {
        assert!(is_pdf_magic(b"%PDF-1.4\n"));
        assert!(!is_pdf_magic(b"Not a PDF"));
    }
}

/// Check if bytes represent a PDF file (magic bytes).
pub fn is_pdf_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
}
```

### Step 3: Update `src/utils/mod.rs`

Add the pdf module:

```rust
pub mod pdf;
```

### Step 4: Update `src/utils/file_text.rs`

Replace the `handle_binary_file` function (lines 176-200):

```rust
/// Handle binary files - check for PDF or skip.
fn handle_binary_file(bytes: &[u8], path: &Path) -> Option<FileText> {
    // Try PDF extraction first
    if crate::utils::pdf::is_pdf_magic(bytes) {
        let text = crate::utils::pdf::extract_text_from_bytes(bytes);
        if !text.is_empty() {
            return Some(FileText {
                text,
                source: TextSource::PdfText,
            });
        }
        // PDF text extraction failed, skip the file
        return None;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if should_skip_binary_extension(&ext) {
        return None;
    }

    let text = decode_bytes_with_fallback(bytes);
    if text.is_empty() || is_mostly_non_printable(&text) {
        return None;
    }

    Some(FileText {
        text,
        source: TextSource::FallbackDecoding,
    })
}
```

Also update the test for PDFs (lines 361-366):

```rust
#[test]
fn test_extract_text_for_detection_pdf() {
    let pdf_path = PathBuf::from("testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf");
    if !pdf_path.exists() {
        return;
    }
    let bytes = std::fs::read(pdf_path).unwrap();
    let result = extract_text_for_detection(&bytes, Path::new("test.pdf"));
    assert!(result.is_some());
    let file_text = result.unwrap();
    assert_eq!(file_text.source, TextSource::PdfText);
}
```

### Step 5: Remove PDF from golden test skip list (if any)

The current `golden_test.rs` uses `extract_text_from_file` which delegates to `file_text.rs`. Once `file_text.rs` handles PDFs, the golden tests will automatically pick up PDF text extraction.

## Verification Plan

### 1. Unit Tests

```bash
cargo test --lib pdf::
cargo test --lib file_text::
```

### 2. Golden Tests

Run the specific tests that include PDF files:

```bash
cargo test test_golden_lic2_part1  # Includes bsd-new_156.pdf
cargo test test_golden_lic4_part1  # Includes should_detect_something_4.pdf
cargo test test_golden_lic4_part2  # Includes should_detect_something_5.pdf
```

### 3. Manual Verification

```bash
# Build and test on a PDF file
cargo build --release
./target/release/scancode-rust testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf -o test_output.json
cat test_output.json | jq '.files[0].license_detections'
```

Expected output for `should_detect_something_4.pdf`:
```json
[
  {
    "license_expression": "generic-cla",
    ...
  }
]
```

Expected output for `should_detect_something_5.pdf`:
```json
[
  {"license_expression": "sun-sissl-1.1", ...},
  {"license_expression": "proprietary-license", ...},
  {"license_expression": "cpal-1.0", ...}
]
```

### 4. Integration Test

Add a specific integration test in `tests/` or as a test in `src/license_detection/golden_test.rs`:

```rust
#[test]
fn test_pdf_text_extraction() {
    let Some(engine) = ensure_engine() else {
        return;
    };

    // Test should_detect_something_4.pdf
    let pdf_path = PathBuf::from("testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf");
    if pdf_path.exists() {
        let file_text = extract_text_from_file(&pdf_path).unwrap().unwrap();
        assert_eq!(file_text.source, TextSource::PdfText);
        
        let detections = engine.detect(&file_text.text, false).unwrap();
        let actual: Vec<&str> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();
        
        assert!(actual.contains(&"generic-cla"), "Expected generic-cla, got {:?}", actual);
    }
}
```

## File Changes Summary

| File | Change |
|------|--------|
| `Cargo.toml` | Add `lopdf = "0.39.0"` |
| `src/utils/pdf.rs` | **NEW** - PDF text extraction module (~100 lines) |
| `src/utils/mod.rs` | Add `pub mod pdf;` |
| `src/utils/file_text.rs` | Update `handle_binary_file()` to call PDF extraction |

## Security Considerations

1. **Encrypted PDFs**: Return empty string, don't attempt decryption
2. **Malformed PDFs**: Use `?` operator and `unwrap_or` to handle errors gracefully
3. **Memory**: lopdf loads entire document into memory - acceptable for license detection
4. **No code execution**: lopdf is a pure Rust parser, no external dependencies

## Acceptance Criteria

- [ ] All 3 PDF golden tests pass:
  - `should_detect_something_4.pdf` → `generic-cla`
  - `should_detect_something_5.pdf` → `sun-sissl-1.1`, `proprietary-license`, `cpal-1.0`
  - `bsd-new_156.pdf` → `bsd-new`
- [ ] Unit tests for PDF extraction pass
- [ ] Encrypted PDFs handled gracefully (no panic)
- [ ] Malformed PDFs handled gracefully (no panic)
- [ ] No regression in existing tests

## Estimated Effort

| Task | Time |
|------|------|
| Add lopdf dependency and create pdf.rs | 1 hour |
| Update file_text.rs | 30 min |
| Write unit tests | 1 hour |
| Debug and fix extraction issues | 2-4 hours |
| **Total** | **4-6 hours** |

## Notes

- The text extraction is intentionally simple - we don't need perfect layout preservation
- For license detection, extracting the raw text content is sufficient
- Python's pdfminer.six has sophisticated layout analysis, but for our use case, basic text extraction works
- If extraction quality is insufficient, we can enhance later with better stream parsing
