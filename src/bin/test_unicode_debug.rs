//! Debug unicode test

use std::path::PathBuf;

fn main() {
    let path =
        PathBuf::from("testdata/license-golden/datadriven/external/fossology-licenses/unicode.txt");
    let bytes = std::fs::read(&path).unwrap();

    // Method 1: Direct read (like debug pipeline)
    let text1 = String::from_utf8_lossy(&bytes).into_owned();
    println!("Method 1 (direct): {} bytes", text1.len());

    // Method 2: Extract text for detection (like golden test)
    let text2 = scancode_rust::utils::file_text::extract_text_for_detection(&bytes, &path)
        .map(|ft| ft.text)
        .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
    println!("Method 2 (extract): {} bytes", text2.len());

    // Check if they're the same
    println!("\nTexts are equal: {}", text1 == text2);

    // Now run detection on both
    let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    let engine =
        scancode_rust::license_detection::LicenseDetectionEngine::new(&rules_path).unwrap();

    let matches1 = engine.detect_matches(&text1, false).unwrap();
    let matches2 = engine.detect_matches(&text2, false).unwrap();

    println!("\nMethod 1 matches: {}", matches1.len());
    let exprs1: Vec<&str> = matches1
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    println!("  Expressions: {:?}", exprs1);

    println!("\nMethod 2 matches: {}", matches2.len());
    let exprs2: Vec<&str> = matches2
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    println!("  Expressions: {:?}", exprs2);

    // Expected
    let expected: Vec<&str> = vec!["unicode-tou", "unicode", "unicode"];
    println!("\nExpected: {:?}", expected);
    println!("Method 1 matches expected: {}", exprs1 == expected);
    println!("Method 2 matches expected: {}", exprs2 == expected);
}
