//! Golden tests for license detection against Python ScanCode reference.
//!
//! These tests validate that the Rust license detection engine produces
//! correct results compared to the Python reference implementation.
//!
//! ## Test Data
//!
//! Test data is copied from `reference/scancode-toolkit/tests/licensedcode/data/datadriven/`:
//! - `lic1/` - ~291 test cases
//! - `lic2/` - ~340 test cases  
//! - `lic3/` - ~292 test cases
//! - `lic4/` - ~345 test cases
//! - `external/` - External license references
//! - `unknown/` - Unknown license detection
//!
//! Each test consists of:
//! - A source file to scan (e.g., `mit.c`)
//! - A YAML expectation file with expected `license_expressions`
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test license_detection_golden
//! ```

#[cfg(test)]
mod golden_tests {
    use crate::license_detection::LicenseDetectionEngine;
    use crate::utils::file_text::extract_text_for_detection;
    use once_cell::sync::Lazy;
    use serde::Deserialize;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Once;

    const GOLDEN_DIR: &str = "testdata/license-golden/datadriven";

    /// Shared engine instance - created once and reused across all tests
    static TEST_ENGINE: Lazy<Option<LicenseDetectionEngine>> = Lazy::new(|| {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            eprintln!("Reference data not available at {:?}", data_path);
            return None;
        }
        match LicenseDetectionEngine::new(&data_path) {
            Ok(engine) => {
                eprintln!("License detection engine initialized for tests");
                Some(engine)
            }
            Err(e) => {
                eprintln!("Failed to create engine: {:?}", e);
                None
            }
        }
    });

    /// Initialize engine once before any tests run
    static INIT: Once = Once::new();

    fn ensure_engine() -> Option<&'static LicenseDetectionEngine> {
        INIT.call_once(|| {
            let _ = &*TEST_ENGINE;
        });
        TEST_ENGINE.as_ref()
    }

    /// Represents the YAML expectation file format
    #[derive(Debug, Deserialize, Default)]
    struct LicenseTestYaml {
        #[serde(default)]
        license_expressions: Vec<String>,
        #[serde(default)]
        expected_failure: bool,
    }

    /// A single golden test case
    struct LicenseGoldenTest {
        name: String,
        test_file: PathBuf,
        yaml: LicenseTestYaml,
    }

    impl LicenseGoldenTest {
        /// Load a test from its YAML file
        fn load(yaml_path: &Path) -> Result<Self, String> {
            let content = fs::read_to_string(yaml_path)
                .map_err(|e| format!("Failed to read {}: {}", yaml_path.display(), e))?;

            let yaml: LicenseTestYaml = serde_yaml::from_str(&content)
                .map_err(|e| format!("Failed to parse YAML {}: {}", yaml_path.display(), e))?;

            let test_file = yaml_path.with_extension("");

            // Use relative path from GOLDEN_DIR as name for uniqueness
            let name = yaml_path
                .strip_prefix(PathBuf::from(GOLDEN_DIR).parent().unwrap_or(Path::new("")))
                .unwrap_or(yaml_path)
                .with_extension("")
                .to_string_lossy()
                .replace('\\', "/");

            Ok(Self {
                name,
                test_file,
                yaml,
            })
        }

        /// Read file content using production text extraction.
        /// Returns None for files that should be skipped.
        fn read_test_file_content(&self) -> Result<Option<String>, String> {
            let text = fs::read(&self.test_file)
                .map(|buffer| extract_text_for_detection(&buffer, &self.test_file))
                .map(|opt| opt.map(|ft| ft.text))
                .map_err(|e| format!("Failed to read {}: {}", self.test_file.display(), e))?;
            
            // Handle source map files specially - extract content from JSON
            let text = match text {
                Some(t) => t,
                None => return Ok(None),
            };
            
            if crate::utils::sourcemap::is_sourcemap(&self.test_file) {
                if let Some(sourcemap_content) = crate::utils::sourcemap::extract_sourcemap_content(&text) {
                    Ok(Some(sourcemap_content))
                } else {
                    Ok(Some(text))
                }
            } else {
                Ok(Some(text))
            }
        }

        /// Run this test against the detection engine
        fn run(
            &self,
            engine: &LicenseDetectionEngine,
            unknown_licenses: bool,
        ) -> Result<(), String> {
            let text = match self.read_test_file_content()? {
                Some(t) => t,
                None => {
                    let expected: Vec<&str> = self
                        .yaml
                        .license_expressions
                        .iter()
                        .map(|s| s.as_str())
                        .collect();

                    if !expected.is_empty() {
                        return Err(format!(
                            "Binary file {} has unexpected non-empty license_expressions: {:?}",
                            self.name, expected
                        ));
                    }
                    return Ok(());
                }
            };

            let detections = engine.detect(&text, unknown_licenses).map_err(|e| {
                format!("Detection failed for {}: {:?}", self.test_file.display(), e)
            })?;

            let actual: Vec<&str> = detections
                .iter()
                .flat_map(|d| d.matches.iter())
                .map(|m| m.license_expression.as_str())
                .collect();

            let expected: Vec<&str> = self
                .yaml
                .license_expressions
                .iter()
                .map(|s| s.as_str())
                .collect();

            if actual != expected {
                return Err(format!(
                    "license_expressions mismatch for {}:  Expected: {:?}  Actual:   {:?}",
                    self.name, expected, actual
                ));
            }

            Ok(())
        }
    }

    /// Discover all golden tests in a directory (recursively)
    fn discover_tests(dir: &Path) -> Vec<LicenseGoldenTest> {
        let mut tests = Vec::new();
        discover_tests_recursive(dir, &mut tests);
        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }

    fn discover_tests_recursive(dir: &Path, tests: &mut Vec<LicenseGoldenTest>) {
        if !dir.exists() {
            return;
        }

        let entries: Vec<_> = match fs::read_dir(dir) {
            Ok(e) => e.filter_map(|e| e.ok()).collect(),
            Err(_) => return,
        };

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                discover_tests_recursive(&path, tests);
            } else if path.extension().is_some_and(|e| e == "yml")
                && let Ok(test) = LicenseGoldenTest::load(&path)
            {
                tests.push(test);
            }
        }
    }

    /// Result of running a test suite
    struct SuiteResult {
        total: usize,
        passed: usize,
        failed: usize,
        skipped: usize,
        failures: Vec<(String, String)>,
    }

    /// Run a complete test suite using the shared engine
    fn run_suite(suite_name: &str, dir: &Path) -> SuiteResult {
        run_suite_range(suite_name, dir, None, None, false)
    }

    /// Run a complete test suite with unknown_licenses enabled
    fn run_suite_unknown(suite_name: &str, dir: &Path) -> SuiteResult {
        run_suite_range(suite_name, dir, None, None, true)
    }

    /// Run a subset of a test suite (for splitting large suites)
    fn run_suite_range(
        suite_name: &str,
        dir: &Path,
        start: Option<usize>,
        end: Option<usize>,
        unknown_licenses: bool,
    ) -> SuiteResult {
        let mut result = SuiteResult {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
        };

        let Some(engine) = ensure_engine() else {
            eprintln!("Skipping {}: engine not available", suite_name);
            return result;
        };

        let mut tests = discover_tests(dir);
        let total_tests = tests.len();

        let start_idx = start.unwrap_or(0);
        let end_idx = end.unwrap_or(total_tests).min(total_tests);

        if start_idx >= total_tests {
            return result;
        }

        tests = tests.split_off(start_idx);
        if end_idx < total_tests {
            tests.truncate(end_idx - start_idx);
        }

        result.total = tests.len();

        println!(
            "\n{}: Running {} tests ({}-{} of {})...",
            suite_name,
            tests.len(),
            start_idx,
            end_idx,
            total_tests
        );

        for test in &tests {
            if test.yaml.expected_failure {
                result.skipped += 1;
                continue;
            }

            match test.run(engine, unknown_licenses) {
                Ok(()) => result.passed += 1,
                Err(e) => {
                    result.failed += 1;
                    result.failures.push((test.name.clone(), e));
                }
            }
        }

        println!(
            "{}: {} passed, {} failed, {} skipped",
            suite_name, result.passed, result.failed, result.skipped
        );

        result
    }

    #[test]
    fn test_golden_lic1() {
        let result = run_suite("lic1", &PathBuf::from(format!("{}/lic1", GOLDEN_DIR)));
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(result.failed, 0, "lic1 had {} failures", result.failed);
    }

    #[test]
    fn test_golden_lic2_part1() {
        let result = run_suite_range(
            "lic2-part1",
            &PathBuf::from(format!("{}/lic2", GOLDEN_DIR)),
            Some(0),
            Some(285),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "lic2-part1 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_lic2_part2() {
        let result = run_suite_range(
            "lic2-part2",
            &PathBuf::from(format!("{}/lic2", GOLDEN_DIR)),
            Some(285),
            Some(570),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "lic2-part2 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_lic2_part3() {
        let result = run_suite_range(
            "lic2-part3",
            &PathBuf::from(format!("{}/lic2", GOLDEN_DIR)),
            Some(570),
            None,
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "lic2-part3 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_lic3() {
        let result = run_suite("lic3", &PathBuf::from(format!("{}/lic3", GOLDEN_DIR)));
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(result.failed, 0, "lic3 had {} failures", result.failed);
    }

    #[test]
    fn test_golden_lic4_part1() {
        let result = run_suite_range(
            "lic4-part1",
            &PathBuf::from(format!("{}/lic4", GOLDEN_DIR)),
            Some(0),
            Some(175),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "lic4-part1 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_lic4_part2() {
        let result = run_suite_range(
            "lic4-part2",
            &PathBuf::from(format!("{}/lic4", GOLDEN_DIR)),
            Some(175),
            None,
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "lic4-part2 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part1() {
        let result = run_suite_range(
            "external-part1",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(0),
            Some(285),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part1 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part2() {
        let result = run_suite_range(
            "external-part2",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(285),
            Some(570),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part2 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part3() {
        let result = run_suite_range(
            "external-part3",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(570),
            Some(855),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part3 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part4() {
        let result = run_suite_range(
            "external-part4",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(855),
            Some(1140),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part4 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part5() {
        let result = run_suite_range(
            "external-part5",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(1140),
            Some(1425),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part5 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part6() {
        let result = run_suite_range(
            "external-part6",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(1425),
            Some(1710),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part6 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part7() {
        let result = run_suite_range(
            "external-part7",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(1710),
            Some(1995),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part7 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part8() {
        let result = run_suite_range(
            "external-part8",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(1995),
            Some(2280),
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part8 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_external_part9() {
        let result = run_suite_range(
            "external-part9",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
            Some(2280),
            None,
            false,
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(
            result.failed, 0,
            "external-part9 had {} failures",
            result.failed
        );
    }

    #[test]
    fn test_golden_unknown() {
        let result =
            run_suite_unknown("unknown", &PathBuf::from(format!("{}/unknown", GOLDEN_DIR)));
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(result.failed, 0, "unknown had {} failures", result.failed);
    }

    #[test]
    fn debug_double_isc() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Engine not available");
            return;
        };

        let text =
            fs::read_to_string("testdata/license-golden/datadriven/lic1/double_isc.txt").unwrap();
        let detections = engine.detect(&text, false).unwrap();

        let actual: Vec<&str> = detections
            .iter()
            .map(|d| d.license_expression.as_deref().unwrap_or(""))
            .collect();

        eprintln!("Expected: {:?}", vec!["isc", "isc", "sudo"]);
        eprintln!("Actual:   {:?}", actual);

        for (i, d) in detections.iter().enumerate() {
            eprintln!(
                "Detection {}: {:?} ({} matches)",
                i,
                d.license_expression,
                d.matches.len()
            );
            for m in &d.matches {
                eprintln!(
                    "  Match: {} lines {}-{} score={} len={} rule_id={}",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.score,
                    m.matched_length,
                    m.rule_identifier
                );
            }
        }
    }

    #[test]
    fn debug_glassfish_token_analysis() {
        let engine = match ensure_engine() {
            Some(e) => e,
            None => {
                eprintln!("Engine not available, skipping test");
                return;
            }
        };

        let text = match std::fs::read_to_string(
            "testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt",
        ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Could not read glassfish test file: {}", e);
                return;
            }
        };

        let query = match crate::license_detection::query::Query::new(&text, engine.index()) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("Failed to create query: {}", e);
                return;
            }
        };

        eprintln!("=== Glassfish File Token Analysis ===");
        eprintln!("Query tokens (known only): {}", query.tokens.len());
        eprintln!(
            "Unknown tokens total: {}",
            query.unknowns_by_pos.values().sum::<usize>()
        );
        eprintln!(
            "Stopwords total: {}",
            query.stopwords_by_pos.values().sum::<usize>()
        );
        eprintln!("High matchables: {}", query.high_matchables.len());
        eprintln!("Low matchables: {}", query.low_matchables.len());
        eprintln!("len_legalese: {}", engine.index().len_legalese);

        // Show first 20 known token IDs
        let known_tokens: Vec<_> = query.tokens.iter().take(20).collect();
        eprintln!("First 20 known token IDs: {:?}", known_tokens);
    }

    #[test]
    fn debug_glassfish_detection() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Engine not available, skipping test");
            return;
        };

        let text = match std::fs::read_to_string(
            "testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt",
        ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Could not read glassfish test file: {}", e);
                return;
            }
        };

        let detections = engine.detect(&text, false).unwrap();

        let actual: Vec<&str> = detections
            .iter()
            .map(|d| d.license_expression.as_deref().unwrap_or(""))
            .collect();

        eprintln!("Expected: {:?}", vec!["cddl-1.0 OR gpl-2.0"]);
        eprintln!("Actual:   {:?}", actual);

        for (i, d) in detections.iter().enumerate() {
            eprintln!("\nDetection {}:", i + 1);
            eprintln!("  expression: {:?}", d.license_expression);
            for m in &d.matches {
                eprintln!(
                    "    match: {} score={:.1} matcher={} lines={}-{}",
                    m.license_expression, m.score, m.matcher, m.start_line, m.end_line
                );
            }
        }
    }

    #[test]
    fn debug_short_license_detection() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Engine not available");
            return;
        };

        // Test the lgpl bare word detection
        let lgpl_text =
            std::fs::read_to_string("testdata/license-golden/datadriven/lic4/lgpl_21.txt").unwrap();
        let detections = engine.detect(&lgpl_text, false).unwrap();
        let actual: Vec<&str> = detections
            .iter()
            .map(|d| d.license_expression.as_deref().unwrap_or(""))
            .collect();
        eprintln!("lgpl_21.txt: Expected: lgpl-2.0-plus, Actual: {:?}", actual);
        assert!(
            actual.contains(&"lgpl-2.0-plus"),
            "lgpl_21.txt should detect lgpl-2.0-plus"
        );
    }

    #[test]
    fn debug_gpl_12() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Engine not available");
            return;
        };

        let text =
            fs::read_to_string("testdata/license-golden/datadriven/lic1/gpl_12.txt").unwrap();
        let detections = engine.detect(&text, false).unwrap();

        eprintln!("Expected: {:?}", vec!["gpl-1.0-plus", "gpl-2.0-plus"]);
        eprintln!(
            "Actual:   {:?}",
            detections
                .iter()
                .map(|d| d.license_expression.as_deref().unwrap_or(""))
                .collect::<Vec<_>>()
        );

        for (i, d) in detections.iter().enumerate() {
            eprintln!(
                "\nDetection {}: {:?} ({} matches)",
                i,
                d.license_expression,
                d.matches.len()
            );
            for m in &d.matches {
                eprintln!(
                    "  Match: {} lines {}-{} score={:.1}",
                    m.license_expression, m.start_line, m.end_line, m.score
                );
            }
        }
    }

    #[test]
    fn debug_crapl_0_1() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Engine not available");
            return;
        };

        let text =
            fs::read_to_string("testdata/license-golden/datadriven/lic1/crapl-0.1.txt").unwrap();
        let detections = engine.detect(&text, false).unwrap();

        eprintln!("\n========================================");
        eprintln!("DEBUG: crapl-0.1.txt detection");
        eprintln!("========================================");
        eprintln!("Text:\n{}", text);
        eprintln!();

        eprintln!("Expected: {:?}", vec!["crapl-0.1"]);
        eprintln!(
            "Actual:   {:?}",
            detections
                .iter()
                .map(|d| d.license_expression.as_deref().unwrap_or(""))
                .collect::<Vec<_>>()
        );

        for (i, d) in detections.iter().enumerate() {
            eprintln!(
                "\nDetection {}: {:?} ({} matches)",
                i,
                d.license_expression,
                d.matches.len()
            );
            for m in &d.matches {
                eprintln!(
                    "  Match: {} lines {}-{} score={:.1} matcher={}",
                    m.license_expression, m.start_line, m.end_line, m.score, m.matcher
                );
            }
        }

        let index = engine.index();
        let crapl_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("crapl"))
            .collect();

        eprintln!("\n========================================");
        eprintln!("CRAPL rules in index:");
        eprintln!("========================================");
        for rule in &crapl_rules {
            eprintln!(
                "  {} - tokens: {}, is_license_notice: {}",
                rule.identifier,
                rule.tokens.len(),
                rule.is_license_notice
            );
        }

        eprintln!("\n========================================");
        eprintln!("Checking automaton for crapl rules:");
        eprintln!("========================================");

        for rule in &crapl_rules {
            let rid = index
                .rules_by_rid
                .iter()
                .position(|r| r.identifier == rule.identifier)
                .unwrap();
            let tokens = &index.tids_by_rid[rid];
            let pattern: Vec<u8> = tokens.iter().flat_map(|t| t.to_le_bytes()).collect();

            let matches: Vec<_> = index.rules_automaton.find_iter(&pattern).collect();
            eprintln!(
                "  {} (rid={}): {} automaton matches",
                rule.identifier,
                rid,
                matches.len()
            );
        }

        eprintln!("\n========================================");
        eprintln!("Query tokenization check:");
        eprintln!("========================================");

        let crapl_3_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "crapl-0.1_3.RULE")
            .expect("crapl-0.1_3.RULE not found");
        let crapl_3_tokens = &index.tids_by_rid[crapl_3_rid];
        eprintln!(
            "crapl-0.1_3.RULE tokens ({}): {:?}",
            crapl_3_tokens.len(),
            crapl_3_tokens
        );

        let query = crate::license_detection::query::Query::new(&text, index)
            .expect("Failed to create query");
        eprintln!("Query tokens ({}): {:?}", query.tokens.len(), &query.tokens);

        if !crapl_3_tokens.is_empty() {
            let first_tid = crapl_3_tokens[0];
            let positions: Vec<_> = query
                .tokens
                .iter()
                .enumerate()
                .filter(|(_, t)| **t == first_tid)
                .map(|(i, _)| i)
                .collect();
            eprintln!(
                "First token {} appears at positions: {:?}",
                first_tid, positions
            );

            for pos in positions {
                let mut match_len = 0;
                for (i, &rule_tid) in crapl_3_tokens.iter().enumerate() {
                    if pos + i < query.tokens.len() && query.tokens[pos + i] == rule_tid {
                        match_len += 1;
                    } else {
                        break;
                    }
                }
                eprintln!(
                    "  At pos {}: {} tokens match (need {})",
                    pos,
                    match_len,
                    crapl_3_tokens.len()
                );
                if match_len == crapl_3_tokens.len() {
                    eprintln!("    FULL MATCH FOUND!");
                }
            }
        }

        eprintln!("\n========================================");
        eprintln!("Matchables check:");
        eprintln!("========================================");
        let whole_run = query.whole_query_run();
        let matchables = whole_run.matchables(true);
        eprintln!("Matchables: {:?}", matchables);
        eprintln!("Matchables len: {}", matchables.len());

        let crapl_3_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "crapl-0.1_3.RULE")
            .unwrap();
        let crapl_3_tokens = &index.tids_by_rid[crapl_3_rid];

        let first_tid = crapl_3_tokens[0];
        let positions: Vec<_> = query
            .tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| **t == first_tid)
            .map(|(i, _)| i)
            .collect();

        for pos in positions {
            if pos + crapl_3_tokens.len() <= query.tokens.len() {
                let all_match = crapl_3_tokens
                    .iter()
                    .enumerate()
                    .all(|(i, &tid)| query.tokens[pos + i] == tid);
                if all_match {
                    let qstart = pos;
                    let qend = pos + crapl_3_tokens.len();
                    let is_matchable = (qstart..qend).all(|p| matchables.contains(&p));
                    eprintln!("Full match at pos {}: is_matchable = {}", pos, is_matchable);
                    if !is_matchable {
                        eprintln!("  Non-matchable positions:");
                        for p in qstart..qend {
                            if !matchables.contains(&p) {
                                eprintln!(
                                    "    Position {} is NOT matchable (token {})",
                                    p, query.tokens[p]
                                );
                            }
                        }
                    }
                }
            }
        }

        eprintln!("\n========================================");
        eprintln!("Running Aho-Corasick match directly:");
        eprintln!("========================================");
        let aho_matches = crate::license_detection::aho_match::aho_match(index, &whole_run);
        eprintln!("Aho matches found: {}", aho_matches.len());
        for m in &aho_matches {
            eprintln!(
                "  {} lines {}-{} score={:.1} rule_id={}",
                m.license_expression, m.start_line, m.end_line, m.score, m.rule_identifier
            );
        }

        eprintln!("\n========================================");
        eprintln!("Checking refine_matches filters:");
        eprintln!("========================================");

        // Check filter_false_positive_matches
        for m in &aho_matches {
            if let Some(rid) = m
                .rule_identifier
                .strip_prefix('#')
                .and_then(|s| s.parse::<usize>().ok())
            {
                eprintln!(
                    "Rule {} (rid={}): is_false_positive = {}",
                    m.rule_identifier,
                    rid,
                    index.false_positive_rids.contains(&rid)
                );
            }
        }

        // Check filter_contained_matches
        eprintln!("\nChecking containment:");
        for i in 0..aho_matches.len() {
            for j in 0..aho_matches.len() {
                if i != j {
                    let a = &aho_matches[i];
                    let b = &aho_matches[j];
                    if a.start_token <= b.start_token && a.end_token >= b.end_token {
                        eprintln!(
                            "  {} (lines {}-{}, tokens {}-{}) CONTAINS {} (lines {}-{}, tokens {}-{})",
                            a.rule_identifier,
                            a.start_line,
                            a.end_line,
                            a.start_token,
                            a.end_token,
                            b.rule_identifier,
                            b.start_line,
                            b.end_line,
                            b.start_token,
                            b.end_token
                        );
                    }
                }
            }
        }

        eprintln!("\n========================================");
        eprintln!("Running refine_matches:");
        eprintln!("========================================");
        let all_matches: Vec<_> = aho_matches.clone();
        let refined =
            crate::license_detection::match_refine::refine_matches(index, all_matches, &query);
        eprintln!("Refined matches: {}", refined.len());
        for m in &refined {
            eprintln!(
                "  {} lines {}-{} score={:.1} rule_id={}",
                m.license_expression, m.start_line, m.end_line, m.score, m.rule_identifier
            );
        }

        eprintln!("\n========================================");
        eprintln!("Running detection grouping:");
        eprintln!("========================================");
        use crate::license_detection::detection::{
            create_detection_from_group, group_matches_by_region,
            populate_detection_from_group_with_spdx,
        };

        let mut sorted = refined.clone();
        crate::license_detection::detection::sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);
        eprintln!("Number of groups: {}", groups.len());
        for (i, group) in groups.iter().enumerate() {
            eprintln!(
                "Group {} (lines {}-{}):",
                i, group.start_line, group.end_line
            );
            for m in &group.matches {
                eprintln!(
                    "  {} lines {}-{} is_license_intro={} is_license_clue={}",
                    m.license_expression,
                    m.start_line,
                    m.end_line,
                    m.is_license_intro,
                    m.is_license_clue
                );
            }

            let mut detection = create_detection_from_group(group);
            populate_detection_from_group_with_spdx(&mut detection, group, engine.spdx_mapping());
            eprintln!("  Detection: {:?}", detection.license_expression);
            eprintln!("  Detection log: {:?}", detection.detection_log);
        }

        eprintln!("\n========================================");
        eprintln!("Running post_process_detections:");
        eprintln!("========================================");
        let detections: Vec<_> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(
                    &mut detection,
                    group,
                    engine.spdx_mapping(),
                );
                detection
            })
            .collect();

        eprintln!(
            "Before post_process_detections: {} detections",
            detections.len()
        );
        for (i, d) in detections.iter().enumerate() {
            eprintln!("  Detection {}: {:?}", i, d.license_expression);
        }

        let processed =
            crate::license_detection::detection::post_process_detections(detections, 0.0);
        eprintln!(
            "After post_process_detections: {} detections",
            processed.len()
        );
        for (i, d) in processed.iter().enumerate() {
            eprintln!("  Detection {}: {:?}", i, d.license_expression);
            for m in &d.matches {
                eprintln!("    Match: {}", m.license_expression);
            }
        }

        eprintln!("\n========================================");
        eprintln!("Comparing with engine.detect():");
        eprintln!("========================================");

        // Let's manually trace through the engine.detect() pipeline
        let query = crate::license_detection::query::Query::new(&text, index).unwrap();
        let mut all_matches = Vec::new();
        let mut matched_qspans = Vec::new();

        // Phase 1: Hash, SPDX, Aho-Corasick matching
        let whole_run = query.whole_query_run();

        let hash_matches = crate::license_detection::hash_match::hash_match(index, &whole_run);
        eprintln!("Hash matches: {}", hash_matches.len());
        all_matches.extend(hash_matches);

        let spdx_matches = crate::license_detection::spdx_lid::spdx_lid_match(index, &query);
        eprintln!("SPDX matches: {}", spdx_matches.len());
        all_matches.extend(spdx_matches);

        let aho_matches = crate::license_detection::aho_match::aho_match(index, &whole_run);
        eprintln!("Aho matches: {}", aho_matches.len());
        for m in &aho_matches {
            eprintln!(
                "  Aho: {} lines {}-{} coverage={}",
                m.license_expression, m.start_line, m.end_line, m.match_coverage
            );
            if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                matched_qspans.push(crate::license_detection::query::PositionSpan::new(
                    m.start_token,
                    m.end_token - 1,
                ));
            }
        }
        all_matches.extend(aho_matches);

        eprintln!("\nMatched qspans after Phase 1: {}", matched_qspans.len());

        // Check what happens after Phase 2 (near-dupe detection)
        let near_dupe_candidates =
            crate::license_detection::seq_match::compute_candidates_with_msets(
                index,
                &whole_run,
                true,
                crate::license_detection::seq_match::MAX_NEAR_DUPE_CANDIDATES,
            );
        eprintln!("\nNear-dupe candidates: {}", near_dupe_candidates.len());

        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches = crate::license_detection::seq_match::seq_match_with_candidates(
                index,
                &whole_run,
                &near_dupe_candidates,
            );
            eprintln!("Near-dupe matches: {}", near_dupe_matches.len());
            for m in &near_dupe_matches {
                eprintln!(
                    "  Near-dupe: {} lines {}-{} coverage={}",
                    m.license_expression, m.start_line, m.end_line, m.match_coverage
                );
            }
        }

        // Phase 3: Regular sequence matching
        let seq_candidates = crate::license_detection::seq_match::compute_candidates_with_msets(index, &whole_run, false, 50);
        let seq_matches = crate::license_detection::seq_match::seq_match_with_candidates(index, &whole_run, &seq_candidates);
        eprintln!("\nSeq matches: {}", seq_matches.len());
        for m in &seq_matches {
            eprintln!(
                "  Seq: {} lines {}-{} coverage={}",
                m.license_expression, m.start_line, m.end_line, m.match_coverage
            );
        }
        all_matches.extend(seq_matches.clone());

        // Now let's run refine_matches on ALL matches
        eprintln!("\n========================================");
        eprintln!("Refining ALL matches:");
        eprintln!("========================================");

        let refined_all = crate::license_detection::match_refine::refine_matches(
            index,
            all_matches.clone(),
            &query,
        );
        eprintln!("\nRefined matches (from all phases): {}", refined_all.len());
        for m in refined_all.iter().take(10) {
            eprintln!(
                "  {} lines {}-{} coverage={:.1} rule_id={}",
                m.license_expression, m.start_line, m.end_line, m.match_coverage, m.rule_identifier
            );
        }

        // Now let's see what the actual engine.detect() returns
        let engine_detections = engine.detect(&text, false).unwrap();
        eprintln!(
            "\nengine.detect() returned {} detections",
            engine_detections.len()
        );
    }
}


