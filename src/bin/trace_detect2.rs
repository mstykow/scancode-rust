use scancode_rust::license_detection::LicenseDetectionEngine;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("testdata/license-golden/datadriven/external/fossology-licenses/unicode.txt");
    let bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    let engine = LicenseDetectionEngine::new(&rules_path).unwrap();

    println!("=== detect_matches() result ===");
    let matches = engine.detect_matches(&text, false).unwrap();
    println!("Match count: {}", matches.len());
    for m in &matches {
        println!("  {} (license: {}, qstart={}, end_token={}, matcher={})", 
            m.rule_identifier, m.license_expression, m.qstart(), m.end_token, m.matcher);
    }
}
