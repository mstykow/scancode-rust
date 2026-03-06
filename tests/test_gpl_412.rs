use scancode_rust::license_detection::LicenseDetectionEngine;

fn main() {
    let text = r#"License GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law."#;

    println!("Query text:\n{}\n", text);

    let engine = LicenseDetectionEngine::new();
    let matches = engine.detect_licenses(text).unwrap();

    println!("Found {} matches:", matches.len());
    for m in &matches {
        println!(
            "  {} - rule: {}, matcher: {}, lines: {}-{}, coverage: {:.1}%",
            m.license_expression,
            m.rule_identifier,
            m.matcher,
            m.start_line,
            m.end_line,
            m.match_coverage
        );
    }

    println!("\nChecking for gpl-2.0-plus_412.RULE:");
    let found_412 = matches
        .iter()
        .any(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE");
    println!("  Found: {}", found_412);
}
