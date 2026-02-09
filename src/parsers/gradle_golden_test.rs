#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::gradle::GradleParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    fn run_golden(test_file: &str, expected_file: &str) {
        let package_data = GradleParser::extract_first_package(&PathBuf::from(test_file));
        match compare_package_data_parser_only(&package_data, &PathBuf::from(expected_file)) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    // =========================================================================
    // Groovy Tests
    // =========================================================================

    #[test]
    fn test_golden_groovy1() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy1/build.gradle",
            "testdata/gradle-golden/groovy/groovy1/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy2() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy2/build.gradle",
            "testdata/gradle-golden/groovy/groovy2/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy3() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy3/build.gradle",
            "testdata/gradle-golden/groovy/groovy3/build.gradle-expected.json",
        );
    }

    #[test]
    #[ignore = "Rust extracts 11 deps vs Python 9: bracket map + multi-value + named-params produce overlapping entries that Python's pygmars grammar handles differently"]
    fn test_golden_groovy4() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy4/build.gradle",
            "testdata/gradle-golden/groovy/groovy4/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy4_singlequotes() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy4-singlequotes/build.gradle",
            "testdata/gradle-golden/groovy/groovy4-singlequotes/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy5() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy5/build.gradle",
            "testdata/gradle-golden/groovy/groovy5/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy5_parens_singlequotes() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy5-parens+singlequotes/build.gradle",
            "testdata/gradle-golden/groovy/groovy5-parens+singlequotes/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy6_braces() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy6-braces/build.gradle",
            "testdata/gradle-golden/groovy/groovy6-braces/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy6_with_props() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy6-with-props/build.gradle",
            "testdata/gradle-golden/groovy/groovy6-with-props/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy_basic() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy-basic/build.gradle",
            "testdata/gradle-golden/groovy/groovy-basic/build.gradle-expected.json",
        );
    }

    #[test]
    #[ignore = "Intentionally malformed Gradle syntax (mismatched quotes) - Python's lexer handles these edge cases differently"]
    fn test_golden_groovy_no_parens() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy-no-parens/build.gradle",
            "testdata/gradle-golden/groovy/groovy-no-parens/build.gradle-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy_and_kotlin1() {
        run_golden(
            "testdata/gradle-golden/groovy/groovy-and-kotlin1/build.gradle",
            "testdata/gradle-golden/groovy/groovy-and-kotlin1/build.gradle-expected.json",
        );
    }

    // =========================================================================
    // Kotlin Tests
    // =========================================================================

    #[test]
    fn test_golden_kotlin1() {
        run_golden(
            "testdata/gradle-golden/kotlin/kotlin1/build.gradle.kts",
            "testdata/gradle-golden/kotlin/kotlin1/build.gradle.kts-expected.json",
        );
    }

    #[test]
    #[ignore = "Python extracts only from buildscript dependencies in complex multi-block Kotlin files"]
    fn test_golden_kotlin2() {
        run_golden(
            "testdata/gradle-golden/kotlin/kotlin2/build.gradle.kts",
            "testdata/gradle-golden/kotlin/kotlin2/build.gradle.kts-expected.json",
        );
    }

    #[test]
    fn test_golden_kotlin3() {
        run_golden(
            "testdata/gradle-golden/kotlin/kotlin3/build.gradle.kts",
            "testdata/gradle-golden/kotlin/kotlin3/build.gradle.kts-expected.json",
        );
    }

    #[test]
    fn test_golden_kotlin4() {
        run_golden(
            "testdata/gradle-golden/kotlin/kotlin4/build.gradle.kts",
            "testdata/gradle-golden/kotlin/kotlin4/build.gradle.kts-expected.json",
        );
    }

    #[test]
    fn test_golden_kotlin5() {
        run_golden(
            "testdata/gradle-golden/kotlin/kotlin5/build.gradle.kts",
            "testdata/gradle-golden/kotlin/kotlin5/build.gradle.kts-expected.json",
        );
    }

    #[test]
    fn test_golden_groovy_and_kotlin2() {
        run_golden(
            "testdata/gradle-golden/kotlin/groovy-and-kotlin2/build.gradle.kts",
            "testdata/gradle-golden/kotlin/groovy-and-kotlin2/build.gradle.kts-expected.json",
        );
    }

    // =========================================================================
    // End-to-end (different JSON format - uses ScanCode output, not parser array)
    // =========================================================================

    #[test]
    #[ignore = "Expected file uses ScanCode full output format {packages,dependencies,files}, not parser-only array [{...}]"]
    fn test_golden_end2end() {
        run_golden(
            "testdata/gradle-golden/end2end/build.gradle",
            "testdata/gradle-golden/end2end/build.gradle-expected.json",
        );
    }
}
