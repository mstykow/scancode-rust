use super::test_utils::{dir, file};
use super::*;
use crate::models::{Author, Copyright, Holder, Match, TallyEntry};

#[test]
fn compute_tallies_counts_file_findings_and_missing_values() {
    let mut mit_file = file("project/src/lib.rs");
    mit_file.programming_language = Some("Rust".to_string());
    mit_file.license_expression = Some("mit".to_string());
    mit_file.license_detections = vec![crate::models::LicenseDetection {
        license_expression: "mit".to_string(),
        license_expression_spdx: "MIT".to_string(),
        matches: vec![Match {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("project/src/lib.rs".to_string()),
            start_line: 1,
            end_line: 1,
            matcher: None,
            score: 100.0,
            matched_length: None,
            match_coverage: None,
            rule_relevance: None,
            rule_identifier: None,
            rule_url: None,
            matched_text: None,
        }],
        identifier: None,
    }];
    mit_file.copyrights = vec![Copyright {
        copyright: "Copyright (c) Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    mit_file.holders = vec![Holder {
        holder: "Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    mit_file.authors = vec![Author {
        author: "Alice".to_string(),
        start_line: 1,
        end_line: 1,
    }];

    let mut dual_license_file = file("project/src/main.c");
    dual_license_file.programming_language = Some("C".to_string());
    dual_license_file.license_expression = Some("apache-2.0 AND mit".to_string());
    dual_license_file.license_detections = vec![
        crate::models::LicenseDetection {
            license_expression: "apache-2.0".to_string(),
            license_expression_spdx: "Apache-2.0".to_string(),
            matches: vec![Match {
                license_expression: "apache-2.0".to_string(),
                license_expression_spdx: "Apache-2.0".to_string(),
                from_file: Some("project/src/main.c".to_string()),
                start_line: 1,
                end_line: 1,
                matcher: None,
                score: 100.0,
                matched_length: None,
                match_coverage: None,
                rule_relevance: None,
                rule_identifier: None,
                rule_url: None,
                matched_text: None,
            }],
            identifier: None,
        },
        crate::models::LicenseDetection {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            matches: vec![Match {
                license_expression: "mit".to_string(),
                license_expression_spdx: "MIT".to_string(),
                from_file: Some("project/src/main.c".to_string()),
                start_line: 2,
                end_line: 2,
                matcher: None,
                score: 100.0,
                matched_length: None,
                match_coverage: None,
                rule_relevance: None,
                rule_identifier: None,
                rule_url: None,
                matched_text: None,
            }],
            identifier: None,
        },
    ];
    dual_license_file.copyrights = vec![Copyright {
        copyright: "Copyright (c) Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    dual_license_file.holders = vec![Holder {
        holder: "Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    dual_license_file.authors = vec![Author {
        author: "Bob".to_string(),
        start_line: 1,
        end_line: 1,
    }];

    let empty_file = file("project/README.md");
    let tallies =
        compute_tallies(&[mit_file, dual_license_file, empty_file]).expect("tallies exist");

    assert_eq!(tallies.detected_license_expression.len(), 3);
    assert_eq!(tallies.copyrights[0].count, 2);
    assert_eq!(tallies.holders[0].count, 2);
    assert_eq!(tallies.authors.len(), 3);
    assert_eq!(
        tallies.programming_language,
        vec![
            TallyEntry {
                value: Some("C".to_string()),
                count: 1
            },
            TallyEntry {
                value: Some("Rust".to_string()),
                count: 1
            },
        ]
    );
}

#[test]
fn compute_key_file_tallies_only_counts_key_files_and_drops_missing_values() {
    let mut key_license = file("project/LICENSE");
    key_license.is_key_file = true;
    key_license.license_expression = Some("apache-2.0".to_string());
    key_license.copyrights = vec![Copyright {
        copyright: "Copyright (c) Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    key_license.holders = vec![Holder {
        holder: "Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];

    let mut key_readme = file("project/README.md");
    key_readme.is_key_file = true;
    key_readme.programming_language = Some("Markdown".to_string());
    key_readme.authors = vec![Author {
        author: "Alice".to_string(),
        start_line: 1,
        end_line: 1,
    }];

    let mut non_key_source = file("project/src/lib.rs");
    non_key_source.programming_language = Some("Rust".to_string());
    non_key_source.license_expression = Some("mit".to_string());

    let tallies = compute_key_file_tallies(&[key_license, key_readme, non_key_source])
        .expect("key-file tallies exist");

    assert_eq!(
        tallies.detected_license_expression[0].value.as_deref(),
        Some("apache-2.0")
    );
    assert_eq!(tallies.copyrights[0].count, 1);
    assert_eq!(tallies.holders[0].count, 1);
    assert_eq!(tallies.authors[0].value.as_deref(), Some("Alice"));
    assert_eq!(
        tallies.programming_language[0].value.as_deref(),
        Some("Markdown")
    );
}

#[test]
fn compute_detailed_tallies_assigns_file_and_directory_rollups() {
    let mut files = vec![
        dir("project"),
        dir("project/src"),
        dir("project/empty"),
        file("project/src/main.rs"),
        file("project/README.md"),
    ];

    files[3].license_expression = Some("mit".to_string());
    files[3].programming_language = Some("Rust".to_string());
    files[3].authors = vec![Author {
        author: "Alice".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    files[4].programming_language = Some("Markdown".to_string());

    compute_detailed_tallies(&mut files);

    let root = files.iter().find(|file| file.path == "project").unwrap();
    let src = files
        .iter()
        .find(|file| file.path == "project/src")
        .unwrap();
    let empty = files
        .iter()
        .find(|file| file.path == "project/empty")
        .unwrap();
    assert_eq!(
        src.tallies.as_ref().unwrap().programming_language[0]
            .value
            .as_deref(),
        Some("Rust")
    );
    assert!(
        root.tallies
            .as_ref()
            .unwrap()
            .detected_license_expression
            .len()
            >= 2
    );
    assert!(empty.tallies.as_ref().unwrap().is_empty());
}
