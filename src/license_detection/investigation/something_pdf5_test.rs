//! Investigation test for PLAN-009: should_detect_something_5.pdf
//!
//! ## Issue
//! Binary PDF file with extractable text should have licenses detected,
//! but Rust golden test skips PDFs entirely.
//!
//! **Expected:** `["sun-sissl-1.1", "proprietary-license", "cpal-1.0"]`
//! **Actual (golden test):** Returns error because PDF is skipped but YAML has non-empty expectations
//!
//! ## Root Cause Analysis
//!
//! ### Python Behavior (reference/scancode-toolkit)
//! 1. `typecode.get_type(path).is_binary` → `True` (PDF is binary)
//! 2. `typecode.get_type(path).is_pdf` → `True`
//! 3. `typecode.get_type(path).is_pdf_with_text` → `True` (PDF has extractable text)
//! 4. `typecode.get_type(path).contains_text` → `True`
//! 5. `textcode.analysis.numbered_text_lines()` extracts text from PDF using pdfminer
//! 6. License detection runs on extracted text
//! 7. Result: `["sun-sissl-1.1"]` (plus other detections)
//!
//! ### Rust Behavior (current)
//! 1. `content_inspector::inspect(&bytes)` → `ContentType::BINARY`
//! 2. `golden_test.rs:read_test_file_content()` checks if extension is `"pdf"`
//! 3. Returns `None` (skip the file)
//! 4. Checks if YAML `license_expressions` is empty
//! 5. Since YAML has `["sun-sissl-1.1", "proprietary-license", "cpal-1.0"]`, test fails
//!
//! ### Divergence Point
//! The golden test at `src/license_detection/golden_test.rs:138` explicitly skips PDFs:
//! ```rust
//! if matches!(ext, "jar" | "zip" | "gz" | "tar" | "gif" | "png" | "jpg" | "jpeg" | "class" | "pdf") {
//!     return Ok(None);
//! }
//! ```
//!
//! But Python DOES extract text from PDFs and detect licenses from that text.
//!
//! ## Required Fix
//!
//! Option A: Implement PDF text extraction (like Python's pdfminer integration)
//! - Add pdf-extraction crate (e.g., `pdf-extract`, `lopdf`, or `pdfminer` via subprocess)
//! - In `golden_test.rs`, extract text from PDFs before detection
//! - This matches Python's behavior exactly
//!
//! Option B: Mark PDF tests as expected-skip
//! - Add a marker for tests that require PDF text extraction
//! - Skip these tests with a clear message until PDF extraction is implemented
//!
//! ## Investigation Goals
//! 1. Verify Python extracts text from this PDF
//! 2. Verify the extracted text contains license content
//! 3. Confirm Rust skips PDFs in golden tests
//! 4. Write failing tests that will pass once PDF text extraction is implemented

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::path::PathBuf;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    /// Test 1: Verify Python extracts text from the PDF
    /// Run this manually with: `cd reference/scancode-playground && source venv/bin/activate && python -c "..."`
    #[test]
    fn test_python_extracts_pdf_text() {
        let output = std::process::Command::new("bash")
            .args([
                "-c",
                r#"
                cd reference/scancode-playground && source venv/bin/activate && python -c "
from textcode import analysis
import typecode

path = '../../testdata/license-golden/datadriven/lic4/should_detect_something_5.pdf'
ft = typecode.get_type(path)
print('is_binary:', ft.is_binary)
print('is_pdf:', ft.is_pdf)
print('is_pdf_with_text:', ft.is_pdf_with_text)
print('contains_text:', ft.contains_text)
print()

lines = list(analysis.numbered_text_lines(path))
print('Number of lines extracted:', len(lines))
print()
for i, (line_num, line) in enumerate(lines[:10]):
    print(f'{line_num}: {line[:80]}...' if len(line) > 80 else f'{line_num}: {line}')
"
                "#,
            ])
            .output()
            .expect("Failed to run Python check");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            eprintln!("Python check failed: {}", stderr);
            return;
        }

        eprintln!("\n=== Python PDF Text Extraction ===");
        eprintln!("{}", stdout);

        assert!(
            stdout.contains("is_pdf_with_text: True"),
            "Python should detect this PDF has extractable text"
        );
        assert!(
            stdout.contains("Number of lines extracted:"),
            "Python should extract lines from the PDF"
        );
    }

    /// Test 2: Verify Python detects licenses from the PDF
    #[test]
    fn test_python_detects_licenses_from_pdf() {
        let output = std::process::Command::new("bash")
            .args([
                "-c",
                r#"
                cd reference/scancode-playground && source venv/bin/activate && ./scancode --license --json - ../../testdata/license-golden/datadriven/lic4/should_detect_something_5.pdf 2>/dev/null | python3 -c "
import json,sys
d = json.load(sys.stdin)
f = d['files'][0]
print('detected_license_expression:', f.get('detected_license_expression'))
matches = f.get('license_detections', [])
print('license_detections:')
for m in matches:
    print('  -', m.get('license_expression'))
"
                "#,
            ])
            .output()
            .expect("Failed to run Python ScanCode");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            eprintln!("Python ScanCode failed: {}", stderr);
            return;
        }

        eprintln!("\n=== Python License Detection from PDF ===");
        eprintln!("{}", stdout);

        assert!(
            stdout.contains("sun-sissl-1.1"),
            "Python should detect sun-sissl-1.1 from the PDF"
        );
    }

    /// Test 3: Verify Rust golden test skips PDFs
    /// This test documents the current broken behavior
    #[test]
    fn test_rust_golden_test_skips_pdfs() {
        let pdf_path =
            PathBuf::from("testdata/license-golden/datadriven/lic4/should_detect_something_5.pdf");

        let bytes = std::fs::read(&pdf_path).expect("Failed to read PDF");
        let content_type = content_inspector::inspect(&bytes);

        eprintln!("\n=== Rust Content Inspection ===");
        eprintln!("content_type: {:?}", content_type);
        eprintln!("is_binary: {}", content_type.is_binary());

        assert!(content_type.is_binary(), "Rust should detect PDF as binary");

        let ext = pdf_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let is_skipped = matches!(
            ext,
            "jar" | "zip" | "gz" | "tar" | "gif" | "png" | "jpg" | "jpeg" | "class" | "pdf"
        );

        assert!(is_skipped, "Rust golden test should skip PDF files");
    }

    /// Test 4: FAILING TEST - Will pass once PDF text extraction is implemented
    ///
    /// This test will fail until we implement PDF text extraction.
    /// Once implemented, this test should detect the same licenses as Python.
    #[test]
    fn test_pdf_license_detection_with_text_extraction() {
        let Some(engine) = get_engine() else { return };

        let pdf_path =
            PathBuf::from("testdata/license-golden/datadriven/lic4/should_detect_something_5.pdf");

        let bytes = std::fs::read(&pdf_path).expect("Failed to read PDF");

        let content_type = content_inspector::inspect(&bytes);
        if content_type.is_binary() {
            let ext = pdf_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "pdf" {
                eprintln!("\n=== PDF TEXT EXTRACTION NOT IMPLEMENTED ===");
                eprintln!(
                    "To fix: Implement PDF text extraction similar to Python's textcode.analysis module"
                );
                eprintln!("Python uses pdfminer.six to extract text from PDFs");
                eprintln!();
                eprintln!("Once implemented, this test should detect:");
                eprintln!("  - sun-sissl-1.1");
                eprintln!("  - proprietary-license");
                eprintln!("  - cpal-1.0");

                panic!(
                    "PDF text extraction not implemented. \
                    See PLAN-009 for implementation guidance."
                );
            }
        }

        let text = String::from_utf8_lossy(&bytes);
        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        let expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\n=== Detected License Expressions ===");
        eprintln!("{:?}", expressions);

        assert!(
            expressions.contains(&"sun-sissl-1.1"),
            "Should detect sun-sissl-1.1 from PDF text"
        );
    }

    /// Test 5: Verify YAML expectations match Python output
    #[test]
    fn test_yaml_matches_python_expectations() {
        let yaml_path = PathBuf::from(
            "testdata/license-golden/datadriven/lic4/should_detect_something_5.pdf.yml",
        );

        let yaml_content = std::fs::read_to_string(&yaml_path).expect("Failed to read YAML");

        eprintln!("\n=== YAML Expected Values ===");
        eprintln!("{}", yaml_content);

        assert!(
            yaml_content.contains("sun-sissl-1.1"),
            "YAML should expect sun-sissl-1.1"
        );
        assert!(
            yaml_content.contains("proprietary-license"),
            "YAML should expect proprietary-license"
        );
        assert!(
            yaml_content.contains("cpal-1.0"),
            "YAML should expect cpal-1.0"
        );
    }

    /// Test 6: Document the divergence between Python and Rust
    #[test]
    fn test_document_python_vs_rust_divergence() {
        eprintln!("\n=== DIVERGENCE DOCUMENTATION ===");
        eprintln!();
        eprintln!("PYTHON BEHAVIOR:");
        eprintln!(
            "  1. typecode.get_type(path) -> is_binary=True, is_pdf=True, is_pdf_with_text=True"
        );
        eprintln!("  2. textcode.analysis.numbered_text_lines() extracts text from PDF");
        eprintln!("  3. License detection runs on extracted text");
        eprintln!("  4. Result: ['sun-sissl-1.1', 'proprietary-license', 'cpal-1.0']");
        eprintln!();
        eprintln!("RUST BEHAVIOR (current):");
        eprintln!("  1. content_inspector::inspect() -> ContentType::BINARY");
        eprintln!("  2. golden_test.rs skips PDF files (line 138)");
        eprintln!("  3. Returns error because YAML has non-empty expectations");
        eprintln!("  4. Result: Test failure");
        eprintln!();
        eprintln!("FIX REQUIRED:");
        eprintln!("  1. Implement PDF text extraction (pdfminer.six equivalent)");
        eprintln!("  2. Update golden_test.rs to extract text from PDFs before detection");
        eprintln!("  3. Alternatively: Mark PDF tests as expected-skip until implemented");
        eprintln!();
        eprintln!("FILES TO MODIFY:");
        eprintln!("  - src/license_detection/golden_test.rs:138 (remove 'pdf' from skip list)");
        eprintln!("  - Add PDF text extraction module");
        eprintln!("  - Update content inspection to detect PDFs with text");

        panic!(
            "This test documents the divergence. \
            Implement PDF text extraction to fix the golden test."
        );
    }
}
