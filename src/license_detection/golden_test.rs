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

        /// Run this test against the detection engine
        fn run(&self, engine: &LicenseDetectionEngine) -> Result<(), String> {
            let text = fs::read_to_string(&self.test_file).map_err(|e| {
                format!(
                    "Failed to read test file {}: {}",
                    self.test_file.display(),
                    e
                )
            })?;

            let detections = engine.detect(&text).map_err(|e| {
                format!("Detection failed for {}: {:?}", self.test_file.display(), e)
            })?;

            let actual: Vec<&str> = detections
                .iter()
                .map(|d| d.license_expression.as_deref().unwrap_or(""))
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

        let tests = discover_tests(dir);
        result.total = tests.len();

        println!("\n{}: Running {} tests...", suite_name, tests.len());

        for test in &tests {
            if test.yaml.expected_failure {
                result.skipped += 1;
                continue;
            }

            match test.run(engine) {
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
    fn test_golden_lic2() {
        let result = run_suite("lic2", &PathBuf::from(format!("{}/lic2", GOLDEN_DIR)));
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(result.failed, 0, "lic2 had {} failures", result.failed);
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
    fn test_golden_lic4() {
        let result = run_suite("lic4", &PathBuf::from(format!("{}/lic4", GOLDEN_DIR)));
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(result.failed, 0, "lic4 had {} failures", result.failed);
    }

    #[test]
    fn test_golden_external() {
        let result = run_suite(
            "external",
            &PathBuf::from(format!("{}/external", GOLDEN_DIR)),
        );
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(result.failed, 0, "external had {} failures", result.failed);
    }

    #[test]
    fn test_golden_unknown() {
        let result = run_suite("unknown", &PathBuf::from(format!("{}/unknown", GOLDEN_DIR)));
        if result.failed > 0 {
            println!("\n{} failures:", result.failed);
            for (name, err) in &result.failures {
                println!("  - {}: {}", name, err.lines().next().unwrap_or(err));
            }
        }
        assert_eq!(result.failed, 0, "unknown had {} failures", result.failed);
    }

    #[test]
    #[ignore = "Redundant - runs all suites which are tested individually"]
    fn test_golden_summary() {
        let Some(_engine) = ensure_engine() else {
            eprintln!("Skipping summary: engine not available");
            return;
        };

        let suites = [
            ("lic1", "lic1"),
            ("lic2", "lic2"),
            ("lic3", "lic3"),
            ("lic4", "lic4"),
            ("external", "external"),
            ("unknown", "unknown"),
        ];

        let mut total_tests = 0;
        let mut total_passed = 0;
        let mut total_failed = 0;
        let mut total_skipped = 0;

        for (name, subdir) in suites.iter() {
            let result = run_suite(name, &PathBuf::from(format!("{}/{}", GOLDEN_DIR, subdir)));
            total_tests += result.total;
            total_passed += result.passed;
            total_failed += result.failed;
            total_skipped += result.skipped;
        }

        println!("\n========================================");
        println!("License Golden Test Summary");
        println!("========================================");
        println!("Total tests:  {}", total_tests);
        println!("Passed:       {}", total_passed);
        println!("Failed:       {}", total_failed);
        println!("Skipped:      {}", total_skipped);
        println!(
            "Pass rate:    {:.1}%",
            if total_tests > 0 {
                (total_passed as f64 / total_tests as f64) * 100.0
            } else {
                0.0
            }
        );
        println!("========================================");
    }

    #[test]
    fn debug_double_isc() {
        let Some(engine) = ensure_engine() else {
            eprintln!("Engine not available");
            return;
        };

        let text =
            fs::read_to_string("testdata/license-golden/datadriven/lic1/double_isc.txt").unwrap();
        let detections = engine.detect(&text).unwrap();

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

        let detections = engine.detect(&text).unwrap();

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
}
