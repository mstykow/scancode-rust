//! Investigation test for PLAN-008: should_detect_something_4.pdf
//!
//! ## Issue
//! PDF file should have `generic-cla` license expression (detected by Python).
//!
//! **Python Expected:** `["generic-cla"]`
//! **Rust Actual:** Skips PDF files entirely in golden test framework
//!
//! ## Root Cause
//! The golden_test.rs framework (lines 136-141) explicitly skips PDF files:
//! ```rust
//! if matches!(ext, "jar" | "zip" | "gz" | "tar" | "gif" | "png" | "jpg" | "jpeg" | "class" | "pdf") {
//!     return Ok(None);
//! }
//! ```
//!
//! But Python's typecode system detects that PDFs contain text and extracts it
//! for license detection. The PDF contains a Sun Microsystems Contributor
//! Agreement which matches `generic-cla`.
//!
//! ## Fix Required
//! 1. Implement PDF text extraction in Rust (using pdftotext or pdf-extract crate)
//! 2. OR: Update golden test to check if PDF has extractable text before skipping
//! 3. OR: Accept that Rust doesn't support PDF text extraction (document as known limitation)

use std::path::PathBuf;

use crate::license_detection::LicenseDetectionEngine;

const PDF_PATH: &str = "testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf";
const YAML_PATH: &str = "testdata/license-golden/datadriven/lic4/should_detect_something_4.pdf.yml";

#[test]
fn test_pdf_exists_and_is_valid() {
    let pdf_path = PathBuf::from(PDF_PATH);
    assert!(pdf_path.exists(), "PDF file should exist at {}", PDF_PATH);

    let bytes = std::fs::read(&pdf_path).expect("Should read PDF file");
    assert!(
        bytes.len() > 1000,
        "PDF file should have substantial content"
    );

    assert!(bytes.starts_with(b"%PDF-"), "File should be a valid PDF");
}

#[test]
fn test_pdf_yaml_expects_generic_cla() {
    let yaml_content = std::fs::read_to_string(YAML_PATH).expect("Should read YAML file");

    assert!(
        yaml_content.contains("generic-cla"),
        "YAML should expect generic-cla, got: {}",
        yaml_content
    );
}

#[test]
fn test_pdf_contains_extractable_text_via_pdftotext() {
    let pdf_path = PathBuf::from(PDF_PATH);

    let output = std::process::Command::new("pdftotext")
        .arg(&pdf_path)
        .arg("-")
        .output();

    match output {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(
                text.contains("Sun Microsystems") || text.contains("Contributor Agreement"),
                "PDF should contain Sun Contributor Agreement text via pdftotext, got: {}",
                &text[..text.len().min(500)]
            );

            assert!(
                text.contains("contribution") && text.contains("copyright"),
                "PDF should contain license-relevant text (contribution, copyright), got first 500 chars: {}",
                &text[..text.len().min(500)]
            );
        }
        Err(e) => {
            eprintln!("pdftotext not available: {}", e);
            eprintln!("This test requires pdftotext to be installed to verify PDF text extraction");
        }
    }
}

#[test]
fn test_license_detection_on_pdf_text_content() {
    let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    if !data_path.exists() {
        eprintln!("Skipping: reference data not available");
        return;
    }

    let engine = match LicenseDetectionEngine::new(&data_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping: failed to create engine: {:?}", e);
            return;
        }
    };

    let output = std::process::Command::new("pdftotext")
        .arg(PDF_PATH)
        .arg("-")
        .output();

    let text = match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
        Err(_) => {
            eprintln!("Skipping: pdftotext not available");
            return;
        }
    };

    if text.is_empty() {
        eprintln!("Skipping: PDF text extraction returned empty");
        return;
    }

    let detections = engine
        .detect(&text, false)
        .expect("Detection should succeed");

    let detected: Vec<&str> = detections
        .iter()
        .flat_map(|d| d.matches.iter())
        .map(|m| m.license_expression.as_str())
        .collect();

    eprintln!("Detected licenses from PDF text: {:?}", detected);

    assert!(
        detected.contains(&"generic-cla"),
        "Detection on PDF text should find generic-cla, got: {:?}",
        detected
    );
}

#[test]
fn test_rust_golden_test_skips_pdf_files() {
    use content_inspector::{ContentType, inspect};

    let bytes = std::fs::read(PDF_PATH).expect("Should read PDF");
    let content_type = inspect(&bytes);

    eprintln!("Content type detected: {:?}", content_type);

    let path = PathBuf::from(PDF_PATH);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    eprintln!("Extension: {}", ext);

    let is_skipped = matches!(
        content_type,
        ContentType::BINARY
            | ContentType::UTF_16LE
            | ContentType::UTF_16BE
            | ContentType::UTF_32LE
            | ContentType::UTF_32BE
    ) && matches!(
        ext,
        "jar" | "zip" | "gz" | "tar" | "gif" | "png" | "jpg" | "jpeg" | "class" | "pdf"
    );

    assert!(
        is_skipped,
        "Rust golden test framework should skip PDF files based on extension"
    );

    eprintln!("Current Rust behavior: Skips PDF files (returns None)");
    eprintln!("Python behavior: Extracts text from PDFs and detects licenses");
    eprintln!("This is the divergence point!");
}

#[test]
fn test_python_handles_pdfs_with_text() {
    eprintln!("Python's typecode module detects that PDFs 'contain_text'");
    eprintln!("Python's licensedcode/query.py checks T.contains_text before returning None");
    eprintln!("If contains_text is true, Python extracts text via pdftotext/pdftotext");
    eprintln!("The extracted text then goes through normal license detection");
    eprintln!();
    eprintln!("Rust needs to either:");
    eprintln!("1. Implement PDF text extraction (feature parity)");
    eprintln!("2. Or document this as a known limitation");
}
