# PLAN-009: should_detect_something_5.pdf

## Status: INVESTIGATION COMPLETE - FIX IDENTIFIED

## Test File
`testdata/license-golden/datadriven/lic4/should_detect_something_5.pdf`

## Issue
Binary PDF file with extractable text should have licenses detected, but Rust golden test skips PDFs entirely.

**Expected:** `["sun-sissl-1.1", "proprietary-license", "cpal-1.0"]`
**Actual:** Golden test fails because PDFs are skipped but YAML has non-empty expectations

## Root Cause Analysis

### Python Behavior (verified via `reference/scancode-playground`)
```
typecode.get_type(path):
  is_binary: True
  is_pdf: True
  is_pdf_with_text: True  <-- KEY: Python knows PDF has extractable text
  contains_text: True

textcode.analysis.numbered_text_lines():
  Number of lines extracted: 83
  First line: "SUN INDUSTRY STANDARDS SOURCE LICENSE"

License detection result:
  detected_license_expression: sun-sissl-1.1
```

### Rust Behavior (current)
```
content_inspector::inspect(&bytes):
  content_type: BINARY
  is_binary: true

golden_test.rs:read_test_file_content():
  Line 138: PDF files explicitly skipped
  Returns: None

Result: Test failure (YAML has non-empty expectations but file was skipped)
```

### Divergence Point
**File:** `src/license_detection/golden_test.rs:138`

```rust
if matches!(
    ext,
    "jar" | "zip" | "gz" | "tar" | "gif" | "png" | "jpg" | "jpeg" | "class" | "pdf"
    //                                                                    ^^^^^^^^
    // PDFs are skipped unconditionally, even if they have extractable text
) {
    return Ok(None);
}
```

## Investigation Test File
Created: `src/license_detection/investigation/something_pdf5_test.rs`

### Test Results
| Test | Result | Purpose |
|------|--------|---------|
| `test_python_extracts_pdf_text` | PASS | Verified Python extracts 83 lines of text |
| `test_python_detects_licenses_from_pdf` | PASS | Verified Python detects `sun-sissl-1.1` |
| `test_rust_golden_test_skips_pdfs` | PASS | Confirmed Rust skips PDFs |
| `test_yaml_matches_python_expectations` | PASS | YAML expects `sun-sissl-1.1` |
| `test_pdf_license_detection_with_text_extraction` | FAIL | Will pass once PDF extraction implemented |
| `test_document_python_vs_rust_divergence` | FAIL | Documents the divergence |

## Required Fix

### Option A: Implement PDF Text Extraction (Recommended)
1. Add PDF text extraction capability using a Rust crate:
   - `pdf-extract` - Simple text extraction
   - `lopdf` - Low-level PDF manipulation
   - `pdf` - Another option

2. Update `src/license_detection/golden_test.rs`:
   - Before line 138, check if PDF has extractable text
   - If yes, extract text and run detection
   - If no, skip as before

3. Implementation pattern (following Python):
   ```rust
   if ext == "pdf" {
       if let Some(text) = extract_pdf_text(&bytes) {
           return Ok(Some(text));
       }
       // PDF has no extractable text, skip
       return Ok(None);
   }
   ```

### Option B: Mark PDF Tests as Expected-Skip (Temporary)
1. Add marker in YAML for tests requiring PDF extraction
2. Skip these tests with clear message until PDF extraction is implemented

## Files to Modify

| File | Change |
|------|--------|
| `src/license_detection/golden_test.rs` | Remove `"pdf"` from skip list, add PDF text extraction |
| New: `src/utils/pdf.rs` | PDF text extraction module |
| `Cargo.toml` | Add PDF extraction crate dependency |

## Python Reference Code

**File:** `reference/scancode-toolkit/src/textcode/analysis.py:101-104`
```python
if T.is_pdf and T.is_pdf_with_text:
    if TRACE:
        logger_debug('numbered_text_lines:', 'is_pdf')
    return enumerate(unicode_text_lines_from_pdf(location), start_line)
```

**File:** `reference/scancode-toolkit/src/textcode/pdf.py`
- Uses `pdfminer.six` library for text extraction

## Next Steps
1. Evaluate PDF extraction crates (`pdf-extract`, `lopdf`)
2. Implement `extract_pdf_text()` function
3. Update golden test to use PDF text extraction
4. Run all PDF golden tests to verify fix
