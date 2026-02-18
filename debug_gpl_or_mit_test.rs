use scancode_rust::license_detection::LicenseDetectionEngine;
use std::path::PathBuf;

fn main() {
    let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    let engine = LicenseDetectionEngine::new(&data_path).expect("Failed to create engine");

    let text = r#"/*! HTML5 Shiv vpre3.6 | @afarkas @jdalton @jon_neal @rem | MIT/GPL2 Licensed
  Uncompressed source: https://github.com/aFarkas/html5shiv  */"#;

    println!("Input text:\n{}\n", text);

    let detections = engine.detect(text).expect("Detection failed");

    println!("Number of detections: {}", detections.len());

    for (i, d) in detections.iter().enumerate() {
        println!("Detection {}:", i);
        println!("  license_expression: {:?}", d.license_expression);
        println!("  detection_log: {:?}", d.detection_log);
        println!("  matches: {} matches", d.matches.len());
        for m in &d.matches {
            println!(
                "    Match: {} @ lines {}-{} matcher={} score={:.2} coverage={:.1}% rule_id={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.matcher,
                m.score,
                m.match_coverage,
                m.rule_identifier
            );
        }
    }

    println!("\n--- Searching for 'gpl_or_mit' rules in index ---");
    let index = engine.index();
    for (rid, rule) in index.rules_by_rid.iter().enumerate() {
        if rule.license_expression.contains("mit") && rule.license_expression.contains("gpl") {
            println!(
                "Rule #{}: {} = '{}'",
                rid, rule.license_expression, rule.text
            );
            println!("  tokens: {:?}", rule.tokens);
            println!("  is_license_tag: {}", rule.is_license_tag);
            println!("  is_false_positive: {}", rule.is_false_positive);
        }
    }
}
