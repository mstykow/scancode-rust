use scancode_rust::license_detection::{detect_matches, LicenseDetectionEngine};
use std::path::Path;

fn main() {
    let rules_path = Path::new("reference/scancode-toolkit/src/licensedcode/data");
    let engine = LicenseDetectionEngine::new(rules_path).expect("Failed to create engine");

    let test_file = Path::new("testdata/license-golden/datadriven/external/glc/CC-BY-SA-1.0.t1");
    let content = std::fs::read_to_string(test_file).expect("Failed to read test file");

    println!("=== Analyzing CC-BY-SA-1.0.t1 ===");
    println!("File length: {} bytes\n", content.len());

    let matches = detect_matches(&engine, &content);

    println!("Total matches: {}", matches.len());
    println!("\nMatches by license_expression:");

    let mut expr_counts = std::collections::HashMap::new();
    for m in &matches {
        *expr_counts
            .entry(m.license_expression.as_str())
            .or_insert(0) += 1;
    }

    for (expr, count) in expr_counts.iter() {
        println!("  {}: {} matches", expr, count);
    }

    println!("\n=== Detailed matches ===");
    for (i, m) in matches.iter().enumerate() {
        println!("\nMatch {}:", i + 1);
        println!("  license_expression: {}", m.license_expression);
        println!("  rule_identifier: {}", m.rule_identifier);
        println!("  score: {:.2}", m.score);
        println!("  match_coverage: {:.2}", m.match_coverage);
        println!("  rule_relevance: {}", m.rule_relevance);
        println!("  start_line: {}, end_line: {}", m.start_line, m.end_line);
        println!(
            "  matched_length: {}, rule_length: {}",
            m.matched_length, m.rule_length
        );
        println!("  is_license_reference: {}", m.is_license_reference);
        println!(
            "  matched_text: {:?}",
            m.matched_text
                .as_ref()
                .map(|t| if t.len() > 100 { &t[..100] } else { t.as_str() })
        );
    }
}
