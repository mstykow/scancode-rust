//! Investigation test for PLAN-016: unknown/scea.txt
//!
//! ## Issue
//! **Expected:** `["scea-1.0", "unknown-license-reference", "scea-1.0", "unknown", "unknown"]`
//! **Actual:** `["scea-1.0", "unknown-license-reference", "scea-1.0", "unknown"]`
//!
//! ## Differences
//! - Missing one `unknown` match at the end
//! - Python has 5 matches, Rust has 4 matches

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use once_cell::sync::Lazy;
    use std::path::PathBuf;

    static TEST_ENGINE: Lazy<Option<LicenseDetectionEngine>> = Lazy::new(|| {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    });

    fn get_engine() -> Option<&'static LicenseDetectionEngine> {
        TEST_ENGINE.as_ref()
    }

    fn read_test_file() -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/unknown/scea.txt");
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_plan_016_rust_detection() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file() else { return };

        let detections = engine
            .detect(&text, true)
            .expect("Detection should succeed");

        eprintln!("\n=== RUST DETECTIONS ===");
        eprintln!("Number of detections: {}", detections.len());

        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            eprintln!("  Number of matches: {}", det.matches.len());

            for (j, m) in det.matches.iter().enumerate() {
                eprintln!("    Match {}:", j + 1);
                eprintln!("      license_expression: {}", m.license_expression);
                eprintln!("      matcher: {}", m.matcher);
                eprintln!("      lines: {}-{}", m.start_line, m.end_line);
                eprintln!("      score: {:.2}", m.score);
                eprintln!("      rule_identifier: {}", m.rule_identifier);
            }
        }

        let all_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();

        eprintln!("\nAll license expressions: {:?}", all_expressions);

        let expected = vec![
            "scea-1.0",
            "unknown-license-reference",
            "scea-1.0",
            "unknown",
            "unknown",
        ];

        assert_eq!(
            all_expressions, expected,
            "Expected license expressions mismatch"
        );
    }

    #[test]
    fn test_plan_016_text_analysis() {
        let Some(text) = read_test_file() else { return };

        eprintln!("\n=== TEXT ANALYSIS ===");
        eprintln!("Total lines: {}", text.lines().count());

        for (i, line) in text.lines().enumerate() {
            eprintln!("{:3}: {}", i + 1, line);
        }
    }
}
