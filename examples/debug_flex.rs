use scancode_rust::license_detection::LicenseDetectionEngine;
use std::path::Path;

fn main() {
    let rules_path = Path::new("reference/scancode-toolkit/src/licensedcode/data/rules");
    let engine = LicenseDetectionEngine::new(rules_path).expect("Failed to create engine");

    let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic1/flex-readme.txt")
        .expect("Failed to read file");

    let detections = engine.detect(&text, false).expect("Detection failed");

    println!("\n=== FINAL RESULT ===");
    println!("{} detections", detections.len());
    for (i, d) in detections.iter().enumerate() {
        println!(
            "Detection {}: expr={:?}, {} matches",
            i,
            d.license_expression,
            d.matches.len()
        );
        for (j, m) in d.matches.iter().enumerate() {
            println!(
                "  Match {}: {} rule={} lines {}-{} tokens {}-{} matcher={} matcher_order={} qspan_positions={:?}",
                j,
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matcher,
                m.matcher_order(),
                m.qspan_positions.as_ref().map(|p| p.len())
            );
        }
    }
}
