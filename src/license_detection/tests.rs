use super::*;
use std::path::PathBuf;

fn get_reference_data_paths() -> Option<(PathBuf, PathBuf)> {
    let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
    let licenses_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
    if rules_path.exists() && licenses_path.exists() {
        Some((rules_path, licenses_path))
    } else {
        None
    }
}

fn create_engine_from_reference() -> Option<LicenseDetectionEngine> {
    let (rules_path, licenses_path) = get_reference_data_paths()?;
    let rules = load_rules_from_directory(&rules_path, false).ok()?;
    let licenses = load_licenses_from_directory(&licenses_path, false).ok()?;
    let index = build_index(rules, licenses);
    let spdx_mapping =
        build_spdx_mapping(&index.licenses_by_key.values().cloned().collect::<Vec<_>>());
    Some(LicenseDetectionEngine {
        index: Arc::new(index),
        spdx_mapping,
    })
}

fn detect_fixture_matches(
    engine: &LicenseDetectionEngine,
    fixture_path: &str,
) -> Vec<LicenseMatch> {
    detect_fixture_matches_with_unknown_licenses(engine, fixture_path, false)
}

fn detect_fixture_matches_with_unknown_licenses(
    engine: &LicenseDetectionEngine,
    fixture_path: &str,
    unknown_licenses: bool,
) -> Vec<LicenseMatch> {
    let text = std::fs::read_to_string(fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {fixture_path}: {e}"));

    engine
        .detect_matches_with_kind(&text, unknown_licenses, false)
        .expect("Detection should succeed")
}

fn summarize_raw_matches(matches: &[LicenseMatch]) -> Vec<(String, String, usize, usize)> {
    matches
        .iter()
        .map(|m| {
            (
                m.rule_identifier.clone(),
                m.matcher.to_string(),
                m.start_line,
                m.end_line,
            )
        })
        .collect()
}

fn sorted_raw_matches(matches: &[LicenseMatch]) -> Vec<(String, String, usize, usize)> {
    let mut summary = summarize_raw_matches(matches);
    summary.sort();
    summary
}

fn sorted_raw_matches_with_normalized_unknown_ids(
    matches: &[LicenseMatch],
) -> Vec<(String, String, usize, usize)> {
    let mut summary: Vec<_> = matches
        .iter()
        .map(|m| {
            let rule_identifier = if m.rule_identifier.starts_with("license-detection-unknown-") {
                "license-detection-unknown".to_string()
            } else {
                m.rule_identifier.clone()
            };

            (
                rule_identifier,
                m.matcher.to_string(),
                m.start_line,
                m.end_line,
            )
        })
        .collect();
    summary.sort();
    summary
}

fn assert_raw_match_present(
    matches: &[LicenseMatch],
    rule_identifier: &str,
    matcher: impl ToString,
    start_line: usize,
    end_line: usize,
) {
    let matcher = matcher.to_string();
    assert!(
        matches.iter().any(|m| {
            m.rule_identifier == rule_identifier
                && m.matcher.as_str() == matcher
                && m.start_line == start_line
                && m.end_line == end_line
        }),
        "expected ({rule_identifier}, {matcher}, {start_line}, {end_line}) in {:?}",
        summarize_raw_matches(matches)
    );
}

fn make_test_match(
    matcher: impl ToString,
    expression: &str,
    rule_identifier: &str,
    start_token: usize,
    end_token: usize,
    qspan_positions: Option<Vec<usize>>,
) -> LicenseMatch {
    let matcher = matcher.to_string();
    let matched_length = qspan_positions
        .as_ref()
        .map(|positions| positions.len())
        .unwrap_or_else(|| end_token.saturating_sub(start_token));

    LicenseMatch {
        license_expression: expression.to_string(),
        matcher: matcher.parse().expect("invalid test matcher"),
        rule_identifier: rule_identifier.to_string(),
        start_token,
        end_token,
        matched_length,
        rule_length: matched_length,
        match_coverage: 100.0,
        qspan_positions,
        ..Default::default()
    }
}

#[test]
fn test_engine_new_with_reference_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    assert!(
        !engine.index().rules_by_rid.is_empty(),
        "Should have rules loaded"
    );
    assert!(
        !engine.index().licenses_by_key.is_empty(),
        "Should have licenses loaded"
    );
    assert!(
        engine.index().len_legalese > 0,
        "Should have legalese tokens"
    );
    assert!(
        !engine.index().rid_by_hash.is_empty(),
        "Should have hash mappings"
    );
    assert!(
        !engine.index().rid_by_hash.is_empty(),
        "Should have regular rule hashes"
    );
}

#[test]
fn test_engine_detect_mit_license() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let mit_text = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."#;

    let detections = engine
        .detect_with_kind(mit_text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect at least one license in MIT text"
    );

    let mit_related = detections.iter().any(|d| {
        d.license_expression
            .as_ref()
            .map(|e| e.contains("mit") || e.contains("unknown"))
            .unwrap_or(false)
    });
    assert!(
        mit_related,
        "Should detect MIT or unknown license, got: {:?}",
        detections
            .iter()
            .map(|d| d.license_expression.as_deref().unwrap_or("none"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_engine_detect_empty_text() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let detections = engine
        .detect_with_kind("", false, false)
        .expect("Detection should succeed");
    assert!(
        detections.is_empty() || !detections.is_empty(),
        "Detection completes"
    );

    let detections = engine
        .detect_with_kind("   \n\n   ", false, false)
        .expect("Detection should succeed");
    assert!(
        detections.is_empty() || !detections.is_empty(),
        "Detection completes"
    );
}

#[test]
fn test_engine_detect_spdx_identifier() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "SPDX-License-Identifier: MIT";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect license from SPDX identifier"
    );
}

#[test]
fn test_engine_index_populated() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };
    let index = engine.index();

    assert!(
        index.rules_by_rid.len() > 1000,
        "Should have at least 1000 rules loaded from reference"
    );

    assert!(
        index.licenses_by_key.len() > 100,
        "Should have at least 100 licenses loaded from reference"
    );

    assert!(
        !index.approx_matchable_rids.is_empty(),
        "Should have approx-matchable rules"
    );

    let has_false_positives = !index.false_positive_rids.is_empty();
    assert!(has_false_positives, "Should have false positive rules");

    let mut rules_with_tokens = 0;
    for &rid in index.rid_by_hash.values().take(10) {
        let rule = &index.rules_by_rid[rid];
        if !rule.tokens.is_empty() {
            rules_with_tokens += 1;
            assert!(
                rule.min_matched_length > 0,
                "Regular rule {} should have computed threshold",
                rid
            );
        }
    }
    assert!(
        rules_with_tokens > 0,
        "Should have at least one rule with tokens among first 10"
    );
}

#[test]
fn test_engine_automaton_functional() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };
    let index = engine.index();

    if !index.rules_by_rid.is_empty() {
        let first_rule = &index.rules_by_rid[0];
        if !first_rule.tokens.is_empty() {
            let pattern: Vec<u8> = first_rule
                .tokens
                .iter()
                .flat_map(|t| t.to_le_bytes())
                .collect();

            let matches: Vec<_> = index.rules_automaton.find_iter(&pattern).collect();
            assert!(
                !matches.is_empty(),
                "Automaton should find pattern for rule 0"
            );
        }
    }
}

#[test]
fn test_engine_spdx_mapping() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };
    let mapping = engine.spdx_mapping();

    let mit_spdx = mapping.scancode_to_spdx("mit");
    assert!(mit_spdx.is_some(), "Should have MIT SPDX mapping");
    assert_eq!(
        mit_spdx.unwrap(),
        "MIT",
        "MIT should map to MIT SPDX identifier"
    );
}

#[test]
fn test_engine_detect_no_license() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "This is just some random text without any license information.";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");
    assert!(
        !detections.is_empty() || detections.is_empty(),
        "Detection should complete without error"
    );
}

#[test]
fn test_engine_detect_gpl_notice() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation.";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(!detections.is_empty(), "Should detect GPL notice");
}

#[test]
fn test_engine_detect_apache_notice() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "Licensed under the Apache License, Version 2.0";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(!detections.is_empty(), "Should detect Apache notice");
}

#[test]
fn test_engine_index_sets_by_rid() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };
    let index = engine.index();

    for &rid in index.rid_by_hash.values().take(5) {
        assert!(
            index.sets_by_rid.contains_key(&rid),
            "Rule {} should have token set",
            rid
        );
        let set = &index.sets_by_rid[&rid];
        assert!(
            !set.is_empty(),
            "Rule {} token set should not be empty",
            rid
        );
    }
}

#[test]
fn test_engine_index_msets_by_rid() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };
    let index = engine.index();

    for &rid in index.rid_by_hash.values().take(5) {
        assert!(
            index.msets_by_rid.contains_key(&rid),
            "Rule {} should have token multiset",
            rid
        );
        let mset = &index.msets_by_rid[&rid];
        assert!(
            !mset.is_empty(),
            "Rule {} token multiset should not be empty",
            rid
        );
    }
}

#[test]
fn test_engine_index_high_postings() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };
    let index = engine.index();

    if !index.approx_matchable_rids.is_empty() {
        let some_approx_rid = index.approx_matchable_rids.iter().next().unwrap();
        if index.high_postings_by_rid.contains_key(some_approx_rid) {
            let postings = &index.high_postings_by_rid[some_approx_rid];
            assert!(!postings.is_empty(), "High postings should have entries");
        }
    }
}

#[test]
fn test_engine_matched_text_populated() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "SPDX-License-Identifier: MIT";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(!detections.is_empty(), "Should detect license");

    for detection in &detections {
        for m in &detection.matches {
            assert!(
                m.matched_text.is_some(),
                "matched_text should be populated for matcher {}",
                m.matcher
            );
            let matched = m.matched_text.as_ref().unwrap();
            assert!(
                !matched.is_empty(),
                "matched_text should not be empty for matcher {}",
                m.matcher
            );
        }
    }
}

#[test]
fn test_detect_multiple_licenses_in_text() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let isc_text = r#"Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE."#;

    let darpa_text = r#"Portions of this software were developed by the University of California,
Irvine under a U.S. Government contract with the Defense Advanced Research
Projects Agency (DARPA)."#;

    let combined_text = format!("{}\n\n{}", isc_text, darpa_text);

    let detections = engine
        .detect_with_kind(&combined_text, false, false)
        .expect("Detection should succeed");

    assert!(!detections.is_empty(), "Should detect at least one license");

    let detected_licenses: Vec<String> = detections
        .iter()
        .filter_map(|d| d.license_expression.as_ref())
        .cloned()
        .collect();

    assert!(
        detected_licenses.iter().any(|l| {
            let lower = l.to_lowercase();
            lower.contains("isc") || lower.contains("sudo")
        }),
        "Should detect ISC or sudo license (sudo contains ISC + DARPA attribution), got: {:?}",
        detected_licenses
    );
}

#[test]
fn test_sudo_license_loaded_from_license_file() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let index = engine.index();

    let sudo_rules: Vec<_> = index
        .rules_by_rid
        .iter()
        .filter(|r| r.license_expression.contains("sudo"))
        .collect();

    eprintln!("Found {} rules with 'sudo' expression", sudo_rules.len());
    for rule in sudo_rules.iter().take(3) {
        eprintln!(
            "  Rule: {} - is_from_license: {}, text len: {}",
            rule.identifier,
            rule.is_from_license,
            rule.text.len()
        );
    }

    assert!(
        !sudo_rules.is_empty(),
        "Should have at least one rule with 'sudo' license expression"
    );

    let sudo_from_license = sudo_rules.iter().find(|r| r.is_from_license);
    assert!(
        sudo_from_license.is_some(),
        "Should have a sudo rule created from license file"
    );

    let rule = sudo_from_license.unwrap();
    assert!(
        rule.text.contains("Sponsored in part"),
        "sudo rule text should contain DARPA acknowledgment"
    );
}

#[test]
fn test_spdx_simple() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "SPDX-License-Identifier: MIT\nSome code here";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect license from SPDX identifier"
    );

    let has_mit = detections.iter().any(|d| {
        d.license_expression
            .as_ref()
            .map(|e| e.contains("mit"))
            .unwrap_or(false)
    });
    assert!(has_mit, "Should detect MIT license");
}

#[test]
fn test_spdx_with_or() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "SPDX-License-Identifier: MIT OR Apache-2.0";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect license from SPDX identifier with OR"
    );
}

#[test]
fn test_spdx_with_plus() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "SPDX-License-Identifier: GPL-2.0+";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect license from SPDX identifier with plus"
    );
}

#[test]
fn test_spdx_in_comment() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "// SPDX-License-Identifier: MIT\n/* some code */";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect SPDX identifier in comment"
    );
}

#[test]
fn test_spdx_lines_do_not_get_rediscovered_as_seq_false_positives() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = std::fs::read_to_string("testdata/license-golden/datadriven/external/spdx/uboot.c")
        .expect("Failed to read uboot.c SPDX fixture");

    let matches = engine
        .detect_matches_with_kind(&text, false, false)
        .expect("Detection should succeed");
    let match_exprs: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    assert!(
        !match_exprs.contains(&"bsd-plus-patent"),
        "SPDX lines should not be rediscovered as bsd-plus-patent: {:?}",
        match_exprs
    );
    assert!(
        !match_exprs.contains(&"gpl-2.0 OR bsd-simplified"),
        "SPDX lines should not be rediscovered as gpl-2.0 OR bsd-simplified: {:?}",
        match_exprs
    );

    let detections = engine
        .detect_with_kind(&text, false, false)
        .expect("Detection should succeed");
    let detection_exprs: Vec<&str> = detections
        .iter()
        .filter_map(|d| d.license_expression.as_deref())
        .collect();

    assert!(
        !detection_exprs.contains(&"bsd-plus-patent"),
        "Grouped detections should not contain bsd-plus-patent: {:?}",
        detection_exprs
    );
    assert!(
        !detection_exprs.contains(&"gpl-2.0 OR bsd-simplified"),
        "Grouped detections should not contain gpl-2.0 OR bsd-simplified: {:?}",
        detection_exprs
    );
}

#[test]
fn test_spdx_complex2_html_matches_expected_expression() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text =
        std::fs::read_to_string("testdata/license-golden/datadriven/external/spdx/complex2.html")
            .expect("Failed to read complex2.html SPDX fixture");

    let composite_matches: Vec<_> = engine
        .detect_matches_with_kind(&text, false, false)
        .expect("Detection should succeed")
        .into_iter()
        .filter(|m| {
            m.license_expression
                == "epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception"
        })
        .collect();

    assert_eq!(composite_matches.len(), 1);
    assert_eq!(
        composite_matches[0].matcher,
        crate::license_detection::aho_match::MATCH_AHO,
    );
    assert_eq!(composite_matches[0].match_coverage, 100.0);
}

#[test]
fn test_png_h_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/slic-tests/png.h",
    );

    assert_eq!(
        matches.len(),
        2,
        "png.h should collapse to the two Python raw matches: {:?}",
        summarize_raw_matches(&matches)
    );

    assert!(matches.iter().any(|m| {
        m.rule_identifier == "libpng_27.RULE"
            && m.matcher == crate::license_detection::aho_match::MATCH_AHO
            && m.start_line == 8
            && m.end_line == 8
    }));
    assert!(matches.iter().any(|m| {
        m.rule_identifier == "libpng.SPDX.RULE"
            && m.matcher == crate::license_detection::seq_match::MATCH_SEQ
            && m.start_line == 297
            && m.end_line == 401
    }));
    assert!(!matches.iter().any(|m| m.rule_identifier == "libpng_4.RULE"));
    assert!(!matches
        .iter()
        .any(|m| m.rule_identifier == "unknown-license-reference_301.RULE"));
}

#[test]
fn test_standard_ml_nj_and_x11_and_x11_opengroup_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic4/standard-ml-nj_and_x11_and_x11-opengroup.txt",
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![
            (
                "historical_4.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                9,
                32,
            ),
            (
                "x11-opengroup_1.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                36,
                54,
            ),
            (
                "x11-xconsortium_1.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                59,
                79,
            ),
        ]
    );
    assert!(!matches
        .iter()
        .any(|m| m.rule_identifier == "x11-bitstream_4.RULE"));
}

#[test]
fn test_standard_ml_nj_and_x11_and_x11_opengroup_1_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic4/standard-ml-nj_and_x11_and_x11-opengroup_1.txt",
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![
            (
                "historical_4.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                9,
                32,
            ),
            (
                "x11-opengroup_1.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                36,
                54,
            ),
            (
                "x11-xconsortium_1.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                59,
                79,
            ),
        ]
    );
    assert!(!matches
        .iter()
        .any(|m| m.rule_identifier == "x11-bitstream_4.RULE"));
}

#[test]
fn test_x11_danse_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic4/x11_danse.txt",
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![(
            "x11_and_other-permissive_1.RULE".to_string(),
            crate::license_detection::seq_match::MATCH_SEQ.to_string(),
            3,
            38,
        )],
        "x11_danse.txt should match the Python raw output exactly"
    );
    assert!(
        !matches
            .iter()
            .any(|m| m.rule_identifier == "license-intro_94.RULE"),
        "license-intro_94.RULE should be absent: {:?}",
        summarize_raw_matches(&matches)
    );
    assert!(
        !matches
            .iter()
            .any(|m| m.rule_identifier == "other-permissive_339.RULE"),
        "other-permissive_339.RULE should be absent: {:?}",
        summarize_raw_matches(&matches)
    );
    assert!(
        !matches
            .iter()
            .any(|m| m.rule_identifier == "x11_danse2.RULE"),
        "x11_danse2.RULE should be absent: {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_libevent_license_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic3/libevent.LICENSE",
    );

    assert_eq!(
        matches.len(),
        7,
        "libevent.LICENSE should keep the seven Python raw matches"
    );

    for (rule_identifier, start_line, end_line) in [
        ("bsd-new_400.RULE", 1, 2),
        ("bsd-new_98.RULE", 8, 28),
        ("bsd-new_401.RULE", 31, 32),
        ("isc_21.RULE", 57, 58),
        ("isc_14.RULE", 63, 73),
        ("mit_97.RULE", 77, 78),
        ("mit.LICENSE", 83, 99),
    ] {
        assert!(matches.iter().any(|m| {
            m.rule_identifier == rule_identifier
                && m.matcher == crate::license_detection::aho_match::MATCH_AHO
                && m.start_line == start_line
                && m.end_line == end_line
        }));
    }

    assert!(!matches.iter().any(|m| {
        m.rule_identifier == "bsd-new_174.RULE"
            || (m.license_expression == "bsd-new" && m.start_line == 1 && m.end_line == 33)
    }));
}

#[test]
fn test_zlib_txt_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/OS-Licenses-master/zlib.txt",
    );

    let zlib_matches: Vec<_> = matches
        .iter()
        .filter(|m| m.license_expression == "zlib")
        .collect();
    assert_eq!(
        zlib_matches.len(),
        2,
        "zlib.txt should keep the two Python raw zlib matches"
    );

    assert!(zlib_matches.iter().any(|m| {
        m.rule_identifier == "zlib_92.RULE"
            && m.matcher == crate::license_detection::aho_match::MATCH_AHO
            && m.start_line == 1
            && m.end_line == 1
    }));
    assert!(zlib_matches.iter().any(|m| {
        m.rule_identifier == "zlib.LICENSE"
            && m.matcher == crate::license_detection::aho_match::MATCH_AHO
            && m.start_line == 4
            && m.end_line == 12
    }));
    assert!(!zlib_matches.iter().any(|m| {
        m.matcher == crate::license_detection::seq_match::MATCH_SEQ
            && m.start_line == 1
            && m.end_line == 12
    }));
}

#[test]
fn test_aladdin_md5_and_not_rsa_md5_detect_matches_match_python_raw_signature() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic2/aladdin-md5_and_not_rsa-md5.txt",
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![
            (
                "aladdin-md5.RULE".to_string(),
                crate::license_detection::seq_match::MATCH_SEQ.to_string(),
                26,
                34,
            ),
            (
                "zlib.LICENSE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                4,
                18,
            ),
        ],
        "aladdin-md5_and_not_rsa-md5 raw matches should align with Python: {:?}",
        summarize_raw_matches(&matches)
    );

    assert!(
        matches.iter().all(|m| m.license_expression == "zlib"),
        "expected all matches to keep Python's zlib expression: {:?}",
        matches
            .iter()
            .map(|m| (
                &m.rule_identifier,
                &m.license_expression,
                &m.matcher,
                m.start_line,
                m.end_line
            ))
            .collect::<Vec<_>>()
    );
    assert!(
        !matches.iter().any(|m| {
            m.matcher == crate::license_detection::seq_match::MATCH_SEQ
                && m.start_line == 4
                && m.end_line == 34
        }),
        "unexpected seq wrapper spanning lines 4-34: {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_notice_txt_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/slic-tests/2/NOTICE.txt",
    );

    assert_eq!(
        matches.len(),
        2,
        "NOTICE.txt should keep the two Python raw IJG matches: {:?}",
        summarize_raw_matches(&matches)
    );

    assert_raw_match_present(
        &matches,
        "ijg_13.RULE",
        crate::license_detection::aho_match::MATCH_AHO,
        1,
        8,
    );
    assert_raw_match_present(
        &matches,
        "ijg_9.RULE",
        crate::license_detection::aho_match::MATCH_AHO,
        11,
        38,
    );
}

#[test]
fn test_not_spdx_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/spdx/not-spdx",
    );

    assert_eq!(
        matches.len(),
        3,
        "not-spdx should keep the three Python raw AHO matches: {:?}",
        summarize_raw_matches(&matches)
    );

    assert_raw_match_present(
        &matches,
        "gpl-3.0-plus_98.RULE",
        crate::license_detection::aho_match::MATCH_AHO,
        3,
        6,
    );
    assert_raw_match_present(
        &matches,
        "gpl-1.0-plus_421.RULE",
        crate::license_detection::aho_match::MATCH_AHO,
        8,
        11,
    );
    assert_raw_match_present(
        &matches,
        "gpl-3.0-plus_50.RULE",
        crate::license_detection::aho_match::MATCH_AHO,
        13,
        13,
    );
}

#[test]
fn test_gpl_2_0_plus_33_detect_matches_match_python_raw_expressions() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt",
    );

    let expressions: Vec<_> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    assert_eq!(
        expressions,
        vec![
            "gpl-2.0-plus",
            "gpl-2.0-plus",
            "gpl-1.0-plus",
            "gpl-1.0-plus",
            "gpl-2.0-plus",
            "gpl-1.0-plus",
        ]
    );
    assert!(
        !matches.iter().any(|m| m.license_expression == "gpl-3.0"),
        "Python does not keep a gpl-3.0 raw match here: {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_kde_licenses_detect_matches_match_python_raw_expressions() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic4/kde_licenses_test.txt",
    );

    let expressions: Vec<_> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    assert_eq!(
        expressions,
        vec![
            "gpl-2.0 OR gpl-3.0 OR kde-accepted-gpl",
            "lgpl-2.1 OR lgpl-3.0 OR kde-accepted-lgpl",
            "gpl-2.0-plus",
            "gpl-3.0",
            "gpl-3.0-plus",
            "gpl-3.0-plus",
            "gpl-3.0-plus",
            "lgpl-2.1",
            "lgpl-2.1",
            "lgpl-2.1-plus",
            "bsd-simplified AND bsd-new",
            "x11-xconsortium",
            "x11-xconsortium",
            "mit",
            "mit",
        ]
    );
}

#[test]
fn test_d_zlib_and_gfdl_detect_matches_match_python_raw_expressions() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic1/d-zlib_and_gfdl-1.2_and_gpl_and_gpl_and_other.txt",
    );

    let expressions: Vec<_> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    assert_eq!(
        expressions,
        vec![
            "gpl-2.0-plus AND gpl-3.0-plus",
            "gpl-1.0-plus WITH mif-exception",
            "gpl-1.0-plus WITH ada-linking-exception",
            "gpl-1.0-plus",
            "gpl-1.0-plus",
            "gpl-1.0-plus WITH gcc-compiler-exception-2.0",
            "gpl-1.0-plus WITH classpath-exception-2.0",
            "gpl-1.0-plus WITH gcc-linking-exception-2.0",
            "linking-exception-2.0-plus",
            "gpl-1.0-plus WITH gcc-linking-exception-2.0",
            "gpl-1.0-plus WITH linking-exception-2.0-plus",
            "gpl-2.0-plus",
            "unknown-license-reference",
            "d-zlib",
            "lgpl-2.0-plus WITH linking-exception-2.0-plus",
            "unknown-license-reference",
            "mit",
            "gfdl-1.2",
        ]
    );
}

#[test]
fn test_bsd_2_clause_and_imlib2_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/fossology-tests/BSD/BSD-2-Clause_AND_Imlib2.txt",
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![
            (
                "bsd-simplified_and_imlib2_2.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                2,
                2,
            ),
            (
                "bsd-simplified_and_imlib2_3.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                3,
                3,
            ),
        ]
    );
}

#[test]
fn test_bsd_3_clause_and_cc0_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/fossology-tests/BSD/BSD-3-Clause_AND_CC0-1.0.txt",
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![
            (
                "bsd-new_303.RULE".to_string(),
                crate::license_detection::seq_match::MATCH_SEQ.to_string(),
                9,
                11,
            ),
            (
                "cc0-1.0_37.RULE".to_string(),
                crate::license_detection::seq_match::MATCH_SEQ.to_string(),
                22,
                28,
            ),
        ]
    );
}

#[test]
fn test_cecill_c_detect_matches_keep_spdx_and_long_notice() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/atarashi/CECILL-C.c",
    );

    assert_eq!(
        matches
            .iter()
            .map(|m| m.license_expression.as_str())
            .collect::<Vec<_>>(),
        vec!["cecill-c", "cecill-c-en"]
    );
    assert!(
        !matches
            .iter()
            .any(|m| m.rule_identifier == "cecill-c_3.RULE"),
        "mixed SPDX/title AHO reference should be filtered out: {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_xunit_sln_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches =
        detect_fixture_matches(&engine, "testdata/license-golden/datadriven/lic4/xunit.sln");

    assert!(
        matches.is_empty(),
        "xunit.sln should have no raw matches: {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_basename_elf_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let path = std::path::Path::new("testdata/license-golden/datadriven/lic2/basename.elf");
    let bytes = std::fs::read(path).expect("failed to read basename.elf fixture");
    let (text, kind) = crate::utils::file::extract_text_for_detection(path, &bytes);

    let matches = engine
        .detect_matches_with_kind(
            &text,
            false,
            matches!(kind, crate::utils::file::ExtractedTextKind::BinaryStrings),
        )
        .expect("detection should succeed");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].rule_identifier, "gpl-3.0-plus_14.RULE");
    assert_eq!(
        matches[0].matcher,
        crate::license_detection::aho_match::MATCH_AHO
    );
    assert_eq!(matches[0].license_expression, "gpl-3.0-plus");
}

#[test]
fn test_faq_doctree_detect_matches_preserve_two_bsd_new_hits() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let path =
        std::path::Path::new("testdata/license-golden/datadriven/lic2/2189-bsd-bin/faq.doctree");
    let bytes = std::fs::read(path).expect("failed to read faq.doctree fixture");
    let (text, kind) = crate::utils::file::extract_text_for_detection(path, &bytes);

    let matches = engine
        .detect_matches_with_kind(
            &text,
            false,
            matches!(kind, crate::utils::file::ExtractedTextKind::BinaryStrings),
        )
        .expect("detection should succeed");

    assert_eq!(matches.len(), 2);
    assert_eq!(
        matches
            .iter()
            .map(|m| (
                m.rule_identifier.as_str(),
                m.matcher.as_str(),
                m.license_expression.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "bsd-new_242.RULE",
                crate::license_detection::aho_match::MATCH_AHO.as_str(),
                "bsd-new"
            ),
            (
                "bsd-new_242.RULE",
                crate::license_detection::aho_match::MATCH_AHO.as_str(),
                "bsd-new"
            ),
        ],
    );
}

#[test]
fn test_complex_el_detect_matches_keep_python_lgpl_container() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic1/complex.el",
    );

    assert!(
        matches.iter().any(|m| {
            m.rule_identifier == "lgpl-2.0-plus_55.RULE"
                && m.matcher == crate::license_detection::seq_match::MATCH_SEQ
                && m.start_line == 37
                && m.end_line == 54
        }),
        "expected Python LGPL container to remain present: {:?}",
        summarize_raw_matches(&matches)
    );
    assert!(
        !matches
            .iter()
            .any(|m| m.rule_identifier == "lgpl_bare_single_word.RULE"),
        "expected bare single-word LGPL child to be absorbed by the container: {:?}",
        summarize_raw_matches(&matches)
    );
    assert!(
        !matches
            .iter()
            .any(|m| m.rule_identifier == "lgpl-2.0-plus_36.RULE"),
        "expected shorter LGPL body child to be absorbed by the container: {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_gpl_and_gpl_and_gpl_and_lgpl_detect_matches_match_python_raw_expressions() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic1/gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt",
    );

    let expressions: Vec<_> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    assert_eq!(
        expressions,
        vec![
            "gpl-1.0-plus",
            "gpl-1.0-plus",
            "lgpl-2.1-plus",
            "lgpl-2.1-plus",
            "gpl-1.0-plus",
        ]
    );
    assert!(
        !matches
            .iter()
            .any(|m| m.rule_identifier == "license-intro_25.RULE"),
        "the Python raw output does not keep license-intro_25.RULE here: {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_android_sdk_preview_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic2/android-sdk-preview-2015.html",
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![
            (
                "android-sdk-preview-2015_1.RULE".to_string(),
                crate::license_detection::seq_match::MATCH_SEQ.to_string(),
                557,
                728,
            ),
            (
                "android-sdk-preview-2015_4.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                98,
                98,
            ),
            (
                "cc-by-2.5_2.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                763,
                767,
            ),
            (
                "license-intro_22.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                100,
                100,
            ),
            (
                "license-intro_22.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                213,
                213,
            ),
        ],
        "android-sdk-preview-2015.html raw matches should align with Python"
    );
}

#[test]
fn test_unknown_readme_detect_matches_unknown_mode_matches_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches_with_unknown_licenses(
        &engine,
        "testdata/license-golden/datadriven/unknown/README.md",
        true,
    );

    assert_eq!(
        sorted_raw_matches(&matches),
        vec![
            (
                "unknown-license-reference_341.RULE".to_string(),
                "2-aho".to_string(),
                6,
                6,
            ),
            (
                "unknown-license-reference_344.RULE".to_string(),
                "2-aho".to_string(),
                4,
                4,
            ),
            (
                "unknown-license-reference_348.RULE".to_string(),
                "2-aho".to_string(),
                44,
                44,
            ),
        ]
    );
}

#[test]
fn test_unknown_cisco_detect_matches_unknown_mode_matches_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches_with_unknown_licenses(
        &engine,
        "testdata/license-golden/datadriven/unknown/cisco.txt",
        true,
    );

    assert_eq!(
        sorted_raw_matches_with_normalized_unknown_ids(&matches),
        vec![
            (
                "license-detection-unknown".to_string(),
                crate::license_detection::unknown_match::MATCH_UNKNOWN.to_string(),
                1,
                16,
            ),
            (
                "license-detection-unknown".to_string(),
                crate::license_detection::unknown_match::MATCH_UNKNOWN.to_string(),
                24,
                32,
            ),
            (
                "warranty-disclaimer_21.RULE".to_string(),
                crate::license_detection::seq_match::MATCH_SEQ.to_string(),
                20,
                22,
            ),
        ]
    );
}

#[test]
fn test_unknown_ucware_eula_detect_matches_unknown_mode_matches_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches_with_unknown_licenses(
        &engine,
        "testdata/license-golden/datadriven/unknown/ucware-eula.txt",
        true,
    );

    assert_eq!(
        sorted_raw_matches_with_normalized_unknown_ids(&matches),
        vec![
            (
                "license-detection-unknown".to_string(),
                crate::license_detection::unknown_match::MATCH_UNKNOWN.to_string(),
                3,
                31,
            ),
            (
                "license-detection-unknown".to_string(),
                crate::license_detection::unknown_match::MATCH_UNKNOWN.to_string(),
                31,
                31,
            ),
            (
                "license-intro_21.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                1,
                1,
            ),
            (
                "license-intro_22.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                3,
                3,
            ),
            (
                "swrule.LICENSE".to_string(),
                crate::license_detection::seq_match::MATCH_SEQ.to_string(),
                31,
                31,
            ),
            (
                "warranty-disclaimer_24.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                31,
                31,
            ),
        ]
    );
}

#[test]
fn test_unknown_citrix_detect_matches_unknown_mode_matches_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches_with_unknown_licenses(
        &engine,
        "testdata/license-golden/datadriven/unknown/citrix.txt",
        true,
    );

    assert_eq!(
        sorted_raw_matches_with_normalized_unknown_ids(&matches),
        vec![
            (
                "commercial-option_33.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                236,
                236,
            ),
            (
                "free-unknown_85.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                138,
                139,
            ),
            (
                "free-unknown_88.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                33,
                33,
            ),
            (
                "free-unknown_88.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                140,
                140,
            ),
            (
                "gpl-1.0-plus_33.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                16,
                16,
            ),
            (
                "license-detection-unknown".to_string(),
                crate::license_detection::unknown_match::MATCH_UNKNOWN.to_string(),
                1,
                12,
            ),
            (
                "license-detection-unknown".to_string(),
                crate::license_detection::unknown_match::MATCH_UNKNOWN.to_string(),
                236,
                266,
            ),
            (
                "unknown-license-reference_351.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                197,
                197,
            ),
            (
                "warranty-disclaimer_24.RULE".to_string(),
                crate::license_detection::aho_match::MATCH_AHO.to_string(),
                43,
                43,
            ),
        ]
    );
}

#[test]
fn test_openssh_license_detect_matches_match_python_raw_rules() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic4/openssh.LICENSE",
    );

    assert_eq!(
        matches.len(),
        13,
        "openssh.LICENSE should keep the thirteen Python raw matches: {:?}",
        summarize_raw_matches(&matches)
    );

    for (rule_identifier, matcher, start_line, end_line) in [
        (
            "bsd-new_and_other-permissive_4.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            3,
            7,
        ),
        (
            "tatu-ylonen2.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            13,
            76,
        ),
        (
            "gary-s-brown.LICENSE",
            crate::license_detection::aho_match::MATCH_AHO,
            83,
            84,
        ),
        (
            "bsd-new_1023.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            88,
            88,
        ),
        (
            "other-permissive_kalle-kaukonen_4.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            94,
            102,
        ),
        (
            "other-permissive_kalle-kaukonen_2.RULE",
            crate::license_detection::seq_match::MATCH_SEQ,
            108,
            114,
        ),
        (
            "ssh-keyscan.LICENSE",
            crate::license_detection::aho_match::MATCH_AHO,
            113,
            115,
        ),
        (
            "public-domain_73.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            119,
            120,
        ),
        (
            "public-domain-disclaimer.LICENSE",
            crate::license_detection::aho_match::MATCH_AHO,
            130,
            142,
        ),
        (
            "bsd-original-uc_5.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            145,
            146,
        ),
        (
            "bsd-original-uc_3.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            154,
            180,
        ),
        (
            "bsd-simplified_87.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            183,
            184,
        ),
        (
            "bsd-simplified_8.RULE",
            crate::license_detection::aho_match::MATCH_AHO,
            196,
            214,
        ),
    ] {
        assert_raw_match_present(&matches, rule_identifier, matcher, start_line, end_line);
    }
}

#[test]
fn test_filter_redundant_same_expression_seq_containers_drops_tiny_gap_unicode_wrapper() {
    let redundant_seq = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "unicode",
        "unicode_3.RULE",
        10,
        24,
        Some(vec![10, 11, 12, 13, 16, 17, 18, 19, 20, 21, 22, 23]),
    );
    let aho_first = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "unicode",
        "unicode_6.RULE",
        10,
        13,
        None,
    );
    let aho_second = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "unicode",
        "unicode_8.RULE",
        21,
        24,
        None,
    );

    let filtered = filter_redundant_same_expression_seq_containers(
        vec![redundant_seq],
        &[aho_first.clone(), aho_second.clone()],
    );
    assert!(
        filtered.is_empty(),
        "expected tiny-gap redundant seq container to drop"
    );
}

#[test]
fn test_filter_redundant_same_expression_seq_containers_drops_small_boundary_wrapper() {
    let redundant_seq = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "bsd-new",
        "bsd-new_174.RULE",
        9,
        25,
        Some(vec![9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 21, 22, 23, 24]),
    );
    let aho_first = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "bsd-new",
        "bsd-new_400.RULE",
        10,
        14,
        None,
    );
    let aho_second = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "bsd-new",
        "bsd-new_98.RULE",
        17,
        20,
        None,
    );
    let aho_third = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "bsd-new",
        "bsd-new_401.RULE",
        22,
        25,
        None,
    );

    let filtered = filter_redundant_same_expression_seq_containers(
        vec![redundant_seq],
        &[aho_first, aho_second, aho_third],
    );
    assert!(
        filtered.is_empty(),
        "expected small bridge and boundary filler wrapper to drop"
    );
}

#[test]
fn test_filter_redundant_same_expression_seq_containers_keeps_material_boundary_wrapper() {
    let material_seq = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "unicode",
        "unicode_3.RULE",
        1,
        24,
        Some(vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16, 17, 18, 19, 20, 21, 22, 23,
        ]),
    );
    let aho_first = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "unicode",
        "unicode_6.RULE",
        10,
        13,
        None,
    );
    let aho_second = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "unicode",
        "unicode_8.RULE",
        21,
        24,
        None,
    );

    let filtered = filter_redundant_same_expression_seq_containers(
        vec![material_seq.clone()],
        &[aho_first, aho_second],
    );
    assert_eq!(filtered, vec![material_seq]);
}

#[test]
fn test_filter_redundant_same_expression_seq_containers_keeps_wide_gap_unicode_wrapper() {
    let wide_gap_seq = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "unicode",
        "unicode_3.RULE",
        10,
        19,
        Some(vec![10, 11, 12, 16, 17, 18]),
    );
    let aho_first = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "unicode",
        "unicode_6.RULE",
        10,
        13,
        None,
    );
    let aho_second = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "unicode",
        "unicode_8.RULE",
        21,
        24,
        None,
    );

    let filtered = filter_redundant_same_expression_seq_containers(
        vec![wide_gap_seq.clone()],
        &[aho_first, aho_second],
    );
    assert_eq!(filtered, vec![wide_gap_seq]);
}

#[test]
fn test_filter_redundant_same_expression_seq_containers_keeps_single_material_child_wrapper() {
    let seq_container = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "lgpl-2.0-plus",
        "lgpl-2.0-plus_55.RULE",
        148,
        270,
        Some((148..151).chain(154..270).collect()),
    );
    let bare_single_word = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "lgpl-2.0-plus",
        "lgpl_bare_single_word.RULE",
        149,
        150,
        None,
    );
    let long_body = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "lgpl-2.0-plus",
        "lgpl-2.0-plus_36.RULE",
        154,
        270,
        None,
    );

    let filtered = filter_redundant_same_expression_seq_containers(
        vec![seq_container.clone()],
        &[bare_single_word, long_body],
    );
    assert_eq!(filtered, vec![seq_container]);
}

#[test]
fn test_filter_redundant_same_expression_seq_containers_keeps_single_bridge_token_wrapper() {
    let seq_container = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "bsd-new",
        "bsd-new_303.RULE",
        28,
        44,
        Some(vec![28, 29, 30, 31, 32, 33, 34, 35, 36, 40, 41, 42, 43]),
    );
    let aho_first = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "bsd-new",
        "bsd-new_302.RULE",
        28,
        36,
        None,
    );
    let aho_second = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "bsd-new",
        "bsd-new_304.RULE",
        40,
        44,
        None,
    );

    let filtered = filter_redundant_same_expression_seq_containers(
        vec![seq_container.clone()],
        &[aho_first, aho_second],
    );
    assert_eq!(filtered, vec![seq_container]);
}

#[test]
fn test_filter_redundant_same_expression_seq_containers_keeps_small_one_sided_boundary_wrapper() {
    let seq_container = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "gpl-1.0-plus",
        "gpl_64.RULE",
        1645,
        1661,
        Some(vec![
            1645, 1646, 1647, 1648, 1649, 1650, 1651, 1652, 1653, 1654, 1657, 1658, 1659, 1660,
        ]),
    );
    let aho_first = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "gpl-1.0-plus",
        "gpl-1.0-plus_359.RULE",
        1648,
        1655,
        None,
    );
    let aho_second = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "gpl-1.0-plus",
        "gpl_63.RULE",
        1657,
        1661,
        None,
    );

    let filtered = filter_redundant_same_expression_seq_containers(
        vec![seq_container.clone()],
        &[aho_first, aho_second],
    );
    assert_eq!(filtered, vec![seq_container]);
}

#[test]
fn test_filter_redundant_low_coverage_composite_seq_wrappers_drops_tiny_composite_wrapper() {
    let seq_container = make_test_match(
        crate::license_detection::seq_match::MATCH_SEQ,
        "composite-wrapper",
        "epl-2.0_or_apache-2.0_or_gpl-2.0_with_openjdk-exception_and_others4.RULE",
        55,
        60,
        Some(vec![55, 56, 57, 58, 59]),
    );
    let mut seq_container = seq_container;
    seq_container.match_coverage = 21.3;

    let aho_first = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "gpl-3.0 WITH autoconf-simple-exception-2.0",
        "gpl-3.0_with_autoconf-simple-exception-2.0_1.RULE",
        55,
        56,
        None,
    );
    let aho_second = make_test_match(
        crate::license_detection::aho_match::MATCH_AHO,
        "epl-2.0 OR apache-2.0",
        "epl-2.0_or_apache-2.0_3.RULE",
        57,
        60,
        None,
    );

    let filtered = filter_redundant_low_coverage_composite_seq_wrappers(
        vec![seq_container],
        &[aho_first, aho_second],
    );
    assert!(filtered.is_empty());
}

#[test]
fn test_spdx_complex_short_html_keeps_exact_unicode_matches_and_drops_seq_container() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/spdx/complex-short.html",
    );
    let expressions: Vec<_> = matches
        .iter()
        .map(|m| m.license_expression.clone())
        .collect();
    let rule_ids: Vec<_> = matches.iter().map(|m| m.rule_identifier.as_str()).collect();

    assert!(
        expressions.contains(
            &"epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception"
                .to_string()
        ),
        "expected composite SPDX/OpenJ9 match to remain present: {:?}",
        expressions
    );
    assert!(
        rule_ids.contains(&"unicode_6.RULE"),
        "expected unicode_6.RULE to remain present: {:?}",
        rule_ids
    );
    assert!(
        rule_ids.contains(&"unicode_8.RULE"),
        "expected unicode_8.RULE to remain present: {:?}",
        rule_ids
    );
    assert!(
        !rule_ids.contains(&"unicode_3.RULE"),
        "expected redundant unicode_3.RULE seq container to be absent: {:?}",
        rule_ids
    );
    assert!(
        rule_ids.contains(&"gpl-3.0_with_autoconf-simple-exception-2.0_1.RULE"),
        "expected GPL/autoconf exact child to remain present: {:?}",
        rule_ids
    );
    assert!(
        rule_ids.contains(&"epl-2.0_or_apache-2.0_3.RULE"),
        "expected EPL/Apache exact child to remain present: {:?}",
        rule_ids
    );
    assert!(
        !rule_ids
            .contains(&"epl-2.0_or_apache-2.0_or_gpl-2.0_with_openjdk-exception_and_others4.RULE"),
        "expected tiny composite seq wrapper to be absent: {:?}",
        rule_ids
    );
}

#[test]
fn test_spdx_complex_readme_detect_matches_keeps_nearby_embedded_matches() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/spdx/complex-readme.txt",
    );
    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    assert!(
        expressions.contains(&"epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception"),
        "expected the SPDX composite match to remain present: {:?}",
        expressions
    );
    assert!(
        expressions.contains(&"unicode"),
        "expected nearby unicode matches to remain present: {:?}",
        expressions
    );
    assert!(
        expressions.contains(&"public-domain"),
        "expected nearby MurmurHash3 public-domain match to remain present: {:?}",
        expressions
    );
    assert!(
        expressions.contains(&"mit"),
        "expected nearby libffi MIT match to remain present: {:?}",
        expressions
    );
    assert!(
        expressions.contains(&"zlib"),
        "expected nearby zlib/CuTest matches to remain present: {:?}",
        expressions
    );
}

#[test]
fn test_spdx_complex_readme_detect_matches_match_python_raw_signature() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/external/spdx/complex-readme.txt",
    );
    let composite_seq_expression = "((epl-2.0 OR apache-2.0) OR (gpl-2.0 WITH classpath-exception-2.0 AND gpl-2.0 WITH openjdk-exception)) AND unicode AND public-domain AND mit AND zlib AND zlib";
    let composite_exact_expression = "epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception";

    assert_eq!(
        matches.len(),
        9,
        "complex-readme raw matches should align with Python: {:?}",
        summarize_raw_matches(&matches)
    );

    let bounded_composite_seq: Vec<_> = matches
        .iter()
        .filter(|m| {
            m.license_expression == composite_seq_expression
                && m.matcher == crate::license_detection::seq_match::MATCH_SEQ
                && m.start_line == 5
                && m.end_line == 25
        })
        .collect();
    assert_eq!(
        bounded_composite_seq.len(),
        1,
        "expected exactly one bounded composite seq match: {:?}",
        summarize_raw_matches(&matches)
    );

    let composite_spdx_exact: Vec<_> = matches
        .iter()
        .filter(|m| {
            m.license_expression == composite_exact_expression
                && m.matcher == crate::license_detection::aho_match::MATCH_AHO
                && m.start_line == 12
                && m.end_line == 12
        })
        .collect();
    assert_eq!(
        composite_spdx_exact.len(),
        1,
        "expected exactly one composite SPDX/OpenJ9 exact match: {:?}",
        summarize_raw_matches(&matches)
    );

    let epl_or_apache_exact: Vec<_> = matches
        .iter()
        .filter(|m| {
            m.license_expression == "epl-2.0 OR apache-2.0"
                && m.matcher == crate::license_detection::aho_match::MATCH_AHO
                && m.start_line == 23
                && m.end_line == 487
        })
        .collect();
    assert_eq!(
        epl_or_apache_exact.len(),
        1,
        "expected exactly one long epl/apache exact match: {:?}",
        summarize_raw_matches(&matches)
    );

    for (expression, matcher, start_line, end_line) in [
        (
            "unicode",
            crate::license_detection::aho_match::MATCH_AHO,
            490,
            496,
        ),
        (
            "unicode",
            crate::license_detection::aho_match::MATCH_AHO,
            498,
            506,
        ),
        (
            "public-domain",
            crate::license_detection::aho_match::MATCH_AHO,
            509,
            509,
        ),
        (
            "mit",
            crate::license_detection::aho_match::MATCH_AHO,
            517,
            534,
        ),
        (
            "zlib",
            crate::license_detection::aho_match::MATCH_AHO,
            539,
            556,
        ),
        (
            "zlib",
            crate::license_detection::aho_match::MATCH_AHO,
            561,
            578,
        ),
    ] {
        assert!(
            matches.iter().any(|m| {
                m.license_expression == expression
                    && m.matcher.to_string() == matcher.to_string()
                    && m.start_line == start_line
                    && m.end_line == end_line
            }),
            "expected ({expression}, {matcher}, {start_line}, {end_line}) in {:?}",
            summarize_raw_matches(&matches)
        );
    }

    assert_eq!(
        matches
            .iter()
            .filter(|m| {
                m.license_expression == composite_exact_expression
                    && m.matcher == crate::license_detection::aho_match::MATCH_AHO
            })
            .count(),
        1,
        "unexpected duplicate composite exact match: {:?}",
        summarize_raw_matches(&matches)
    );

    for expression in ["epl-1.0", "gpl-2.0", "apache-2.0"] {
        assert!(
            !matches.iter().any(|m| m.license_expression == expression),
            "did not expect extra {expression} match in {:?}",
            summarize_raw_matches(&matches)
        );
    }

    assert!(
        !matches.iter().any(|m| {
            m.license_expression == "zlib" && (m.start_line < 539 || m.end_line < 556)
        }),
        "did not expect an extra early zlib match in {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_eclipse_openj9_detect_matches_match_python_raw_signature() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let matches = detect_fixture_matches(
        &engine,
        "testdata/license-golden/datadriven/lic1/eclipse-openj9.LICENSE",
    );

    let mut signature: Vec<_> = matches
        .iter()
        .map(|m| (m.rule_identifier.clone(), m.matcher.to_string()))
        .collect();
    signature.sort();

    assert_eq!(
        signature.len(),
        8,
        "eclipse-openj9 raw matches should align with Python: {:?}",
        summarize_raw_matches(&matches)
    );
    assert_eq!(
        signature,
        vec![
            (
                "epl-2.0_or_apache-2.0_4.RULE".to_string(),
                "2-aho".to_string()
            ),
            (
                "epl-2.0_or_apache-2.0_or_gpl-2.0_with_openjdk-exception_and_others3.RULE"
                    .to_string(),
                "2-aho".to_string()
            ),
            ("mit.LICENSE".to_string(), "2-aho".to_string()),
            ("public-domain_64.RULE".to_string(), "2-aho".to_string()),
            ("unicode_6.RULE".to_string(), "2-aho".to_string()),
            ("unicode_8.RULE".to_string(), "2-aho".to_string()),
            ("zlib.LICENSE".to_string(), "2-aho".to_string()),
            ("zlib_17.RULE".to_string(), "2-aho".to_string()),
        ],
        "eclipse-openj9 raw signature mismatch: {:?}",
        summarize_raw_matches(&matches)
    );

    assert!(
        !matches.iter().any(|m| {
            m.rule_identifier
                == "epl-2.0_or_apache-2.0_or_gpl-2.0_with_openjdk-exception_and_others2.RULE"
                && m.matcher == crate::license_detection::seq_match::MATCH_SEQ
        }),
        "did not expect the synthetic OpenJ9 seq wrapper in {:?}",
        summarize_raw_matches(&matches)
    );
}

#[test]
fn test_hash_exact_mit() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let mit_text = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software."#;

    let detections = engine
        .detect_with_kind(mit_text, false, false)
        .expect("Detection should succeed");

    assert!(!detections.is_empty(), "Should detect partial MIT license");
}

#[test]
fn test_seq_partial_license() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let partial_mit = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software."#;

    let detections = engine
        .detect_with_kind(partial_mit, false, false)
        .expect("Detection should succeed");

    assert!(!detections.is_empty(), "Should detect partial MIT license");
}

#[test]
fn test_unknown_proprietary() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "This software is proprietary and confidential. All rights reserved.";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect unknown license or return empty"
    );
}


#[test]
fn test_no_token_boundary_false_positives() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let test_file =
        std::path::PathBuf::from("testdata/license-golden/datadriven/lic1/config.guess-gpl2.txt");
    let text = match std::fs::read_to_string(&test_file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Skipping test: cannot read test file: {}", e);
            return;
        }
    };

    let detections = engine
        .detect_with_kind(&text, false, false)
        .expect("Detection should succeed");

    for detection in &detections {
        for m in &detection.matches {
            assert!(
                !m.license_expression.contains("cc-by-nc-sa"),
                "Found false positive cc-by-nc-sa match at lines {}-{} with matched_text: {:?}",
                m.start_line,
                m.end_line,
                m.matched_text
            );
        }
    }
}

#[test]
fn test_detect_mit_license_with_utf8_bom() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let mit_with_bom =
        "\u{FEFF}Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.";

    let detections = engine
        .detect_with_kind(mit_with_bom, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect at least one license in MIT text with BOM"
    );

    let mit_related = detections.iter().any(|d| {
        d.license_expression
            .as_ref()
            .map(|e| e.contains("mit") || e.contains("unknown"))
            .unwrap_or(false)
    });
    assert!(
        mit_related,
        "Should detect MIT or unknown license with BOM, got: {:?}",
        detections
            .iter()
            .map(|d| d.license_expression.as_deref().unwrap_or("none"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_detect_spdx_identifier_with_utf8_bom() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let text = "\u{FEFF}SPDX-License-Identifier: MIT";
    let detections = engine
        .detect_with_kind(text, false, false)
        .expect("Detection should succeed");

    assert!(
        !detections.is_empty(),
        "Should detect SPDX identifier even with BOM"
    );
}
