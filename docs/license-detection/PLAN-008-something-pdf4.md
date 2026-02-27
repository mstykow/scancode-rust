# PLAN-008: should_detect_something_4.pdf

## Status: ROOT CAUSE IDENTIFIED

## Test File
`testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf`

## Issue ~~Description~~ (CORRECTED)

**Original (incorrect):** Binary PDF file should have no license expressions but Rust detects `generic-cla`.

**Actual Issue:** Python detects `generic-cla` from PDF text, but Rust golden test framework skips PDF files entirely.

| Implementation | Behavior | Result |
|---------------|----------|--------|
| **Python** | Extracts text from PDF via `typecode.is_pdf_with_text`, detects `generic-cla` | `["generic-cla"]` |
| **Rust** | Skips PDF files in golden test framework (line 138-140), never attempts detection | `[]` (skipped) |

## Root Cause

The divergence is in the **golden test framework**, not the detection engine.

**Rust golden_test.rs (lines 136-141):**
```rust
if matches!(
    ext,
    "jar" | "zip" | "gz" | "tar" | "gif" | "png" | "jpg" | "jpeg" | "class" | "pdf"
) {
    return Ok(None);  // Skips the file entirely
}
```

**Python behavior:**
1. `typecode.get_type(location)` checks if file `contains_text`
2. For PDFs, Python uses `pdftotext` or similar to extract embedded text
3. Extracted text goes through normal license detection pipeline
4. Result: `generic-cla` detected from Sun Microsystems Contributor Agreement

## PDF Content

The PDF contains a **Sun Microsystems Contributor Agreement** (verified via `pdftotext`):
- ~67KB PDF file, version 1.3, 1 page
- Contains extractable text including "Sun Microsystems, Inc. Contributor Agreement"
- Text mentions: contribution, copyright, patents, license grants

## Investigation Tests Created

**File:** `src/license_detection/investigation/something_pdf4_test.rs`

| Test | Status | Purpose |
|------|--------|---------|
| `test_pdf_exists_and_is_valid` | PASS | Verify PDF file exists and is valid PDF format |
| `test_pdf_yaml_expects_generic_cla` | PASS | Verify YAML expectation is `generic-cla` |
| `test_pdf_contains_extractable_text_via_pdftotext` | PASS | Verify PDF has extractable text |
| `test_license_detection_on_pdf_text_content` | PASS | **Key test**: Detects `generic-cla` when text is extracted |
| `test_rust_golden_test_skips_pdf_files` | PASS | Confirms divergence point (golden test skips PDFs) |
| `test_python_handles_pdfs_with_text` | PASS | Documents Python behavior |

## Fix Required

**Option 1: Implement PDF text extraction (feature parity)**
- Add `pdf-extract` or `lopdf` crate dependency
- Integrate with `textcode` module for text extraction
- Update golden test to extract PDF text before detection

**Option 2: Document as known limitation**
- Update `docs/SUPPORTED_FORMATS.md` to note PDF text extraction not supported
- Skip PDF-related golden tests explicitly (mark as `expected_failure`)

## Recommendation

**Option 1** is preferred for full feature parity with Python ScanCode. The Rust detection engine correctly identifies `generic-cla` when given extracted text - only the extraction step is missing.

## Files to Modify

1. `src/license_detection/golden_test.rs` - Add PDF text extraction
2. `Cargo.toml` - Add PDF extraction crate
3. `src/textcode/` or new `src/pdf/` - Implement PDF text extraction
4. `docs/SUPPORTED_FORMATS.md` - Document PDF support status
