//! Investigation test for filter_dupes regression
//!
//! Cases:
//! 1. DNSDigest.c - Expected 3 "apache-2.0", Actual has 2
//! 2. sa11xx_base.c - Expected 2 "mpl-1.1 OR gpl-2.0", Actual has 1
//! 3. ar-ER.js.map - Expected 1 "mit", Actual has 2
//! 4. lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt - Expected 1 expression, Actual has 5

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

    #[test]
    fn test_dns_digest() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from(
            "testdata/license-golden/datadriven/external/fossology-tests/APSL/DNSDigest.c",
        );
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== DNSDigest.c DETECTIONS ===");
        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            for (j, m) in det.matches.iter().enumerate() {
                eprintln!(
                    "    Match {}: {} (score: {:.2}, rule: {}, lines: {}-{})",
                    j + 1,
                    m.license_expression,
                    m.score,
                    m.rule_identifier,
                    m.start_line,
                    m.end_line
                );
            }
        }

        let apache_count = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .filter(|m| m.license_expression == "apache-2.0")
            .count();
        eprintln!("\nApache-2.0 count: {} (expected 3)", apache_count);
    }

    #[test]
    fn test_sa11xx_base() {
        let Some(engine) = get_engine() else { return };
        let path =
            PathBuf::from("testdata/license-golden/datadriven/external/slic-tests/sa11xx_base.c");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== sa11xx_base.c DETECTIONS ===");
        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            for (j, m) in det.matches.iter().enumerate() {
                eprintln!(
                    "    Match {}: {} (score: {:.2}, rule: {}, lines: {}-{})",
                    j + 1,
                    m.license_expression,
                    m.score,
                    m.rule_identifier,
                    m.start_line,
                    m.end_line
                );
            }
        }

        let mpl_gpl_count = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .filter(|m| m.license_expression == "mpl-1.1 OR gpl-2.0")
            .count();
        eprintln!("\nmpl-1.1 OR gpl-2.0 count: {} (expected 2)", mpl_gpl_count);
    }

    #[test]
    fn test_ar_er_js_map() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from("testdata/license-golden/datadriven/lic2/ar-ER.js.map");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== ar-ER.js.map DETECTIONS ===");
        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            for (j, m) in det.matches.iter().enumerate() {
                eprintln!(
                    "    Match {}: {} (score: {:.2}, rule: {}, lines: {}-{})",
                    j + 1,
                    m.license_expression,
                    m.score,
                    m.rule_identifier,
                    m.start_line,
                    m.end_line
                );
            }
        }

        let mit_count = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .filter(|m| m.license_expression == "mit")
            .count();
        eprintln!("\nmit count: {} (expected 1)", mit_count);
    }

    #[test]
    fn test_lgpl_wxwindows() {
        let Some(engine) = get_engine() else { return };
        let path = PathBuf::from("testdata/license-golden/datadriven/lic3/lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };

        let detections = engine
            .detect(&text, false)
            .expect("Detection should succeed");

        eprintln!("\n=== lgpl-2.0-plus_with_wxwindows DETECTIONS ===");
        for (i, det) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  license_expression: {:?}", det.license_expression);
            for (j, m) in det.matches.iter().enumerate() {
                eprintln!(
                    "    Match {}: {} (score: {:.2}, rule: {}, lines: {}-{})",
                    j + 1,
                    m.license_expression,
                    m.score,
                    m.rule_identifier,
                    m.start_line,
                    m.end_line
                );
            }
        }

        let unique_expressions: Vec<_> = detections
            .iter()
            .flat_map(|d| d.matches.iter())
            .map(|m| m.license_expression.as_str())
            .collect();
        eprintln!("\nUnique expressions: {:?}", unique_expressions);
        eprintln!(
            "Expression count: {} (expected 1)",
            unique_expressions.len()
        );
    }
}
