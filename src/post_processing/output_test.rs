use chrono::Utc;

use super::test_utils::{dir, file};
use super::*;
use crate::assembly;
use crate::models::{Copyright, Holder, Match, Package, Tallies};
use crate::scan_result_shaping::normalize_paths;
use serde_json::json;

#[test]
fn create_output_gates_summary_tallies_and_generated_sections() {
    let license_rel = "project/LICENSE".to_string();
    let mut disabled_license = file(&license_rel);
    disabled_license.is_generated = Some(true);
    disabled_license.tallies = Some(Tallies::default());

    let start = Utc::now();
    let end = start;
    let output_without_flags = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), disabled_license],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result: assembly::AssemblyResult {
                packages: vec![],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: false,
                include_tallies_by_facet: false,
                include_summary: false,
                include_license_clarity_score: false,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: false,
            },
        },
    );
    assert!(output_without_flags.summary.is_none());
    assert!(output_without_flags.tallies.is_none());
    assert!(output_without_flags.tallies_of_key_files.is_none());
    assert!(
        output_without_flags
            .files
            .iter()
            .all(|file| file.is_generated.is_none())
    );

    let mut enabled_license = file(&license_rel);
    enabled_license.is_generated = Some(true);
    enabled_license.license_expression = Some("mit".to_string());
    enabled_license.license_detections = vec![crate::models::LicenseDetection {
        license_expression: "mit".to_string(),
        license_expression_spdx: "MIT".to_string(),
        matches: vec![Match {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some(license_rel.clone()),
            start_line: 1,
            end_line: 1,
            matcher: Some("1-hash".to_string()),
            score: 100.0,
            matched_length: Some(10),
            match_coverage: Some(100.0),
            rule_relevance: Some(100),
            rule_identifier: None,
            rule_url: None,
            matched_text: None,
        }],
        identifier: None,
    }];

    let output_with_flags = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), enabled_license],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result: assembly::AssemblyResult {
                packages: vec![],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: false,
                include_tallies_by_facet: false,
                include_summary: true,
                include_license_clarity_score: true,
                include_tallies: true,
                include_tallies_with_details: true,
                include_tallies_of_key_files: true,
                include_generated: true,
            },
        },
    );
    assert!(output_with_flags.summary.is_some());
    assert!(output_with_flags.tallies.is_some());
    assert!(output_with_flags.tallies_of_key_files.is_some());
    assert!(
        output_with_flags
            .files
            .iter()
            .find(|file| file.path == license_rel)
            .is_some_and(|file| file.is_generated == Some(true) && file.tallies.is_some())
    );
}

#[test]
fn create_output_preserves_scanner_generated_flags_without_scan_root() {
    let start = Utc::now();
    let end = start;

    let mut generated = file("project/generated.c");
    generated.is_generated = Some(true);

    let mut plain = file("project/plain.c");
    plain.is_generated = Some(false);

    let mut missing = file("project/missing.c");
    missing.is_generated = None;

    let output = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), generated, plain, missing],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result: assembly::AssemblyResult {
                packages: vec![],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: false,
                include_tallies_by_facet: false,
                include_summary: false,
                include_license_clarity_score: false,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: true,
            },
        },
    );

    let generated_flags: Vec<_> = output
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.is_generated))
        .collect();

    assert_eq!(
        generated_flags,
        vec![
            ("project", Some(false)),
            ("project/generated.c", Some(true)),
            ("project/plain.c", Some(false)),
            ("project/missing.c", Some(false)),
        ]
    );
}

#[test]
fn create_output_score_only_keeps_clarity_without_full_summary_fields() {
    let start = Utc::now();
    let end = start;
    let mut license = file("project/LICENSE");
    license.license_expression = Some("mit".to_string());
    license.license_detections = vec![crate::models::LicenseDetection {
        license_expression: "mit".to_string(),
        license_expression_spdx: "MIT".to_string(),
        matches: vec![Match {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("project/LICENSE".to_string()),
            start_line: 1,
            end_line: 1,
            matcher: Some("1-hash".to_string()),
            score: 100.0,
            matched_length: Some(10),
            match_coverage: Some(100.0),
            rule_relevance: Some(100),
            rule_identifier: None,
            rule_url: None,
            matched_text: None,
        }],
        identifier: None,
    }];

    let output = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), license],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result: assembly::AssemblyResult {
                packages: vec![],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: false,
                include_tallies_by_facet: false,
                include_summary: false,
                include_license_clarity_score: true,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: false,
            },
        },
    );

    let summary = output.summary.expect("score-only summary exists");
    assert_eq!(summary.declared_license_expression.as_deref(), Some("mit"));
    assert!(summary.license_clarity_score.is_some());
    assert!(summary.declared_holder.is_none());
    assert!(summary.primary_language.is_none());
    assert!(summary.other_license_expressions.is_empty());
    assert!(summary.other_holders.is_empty());
    assert!(summary.other_languages.is_empty());
}

#[test]
fn create_output_preserves_file_level_license_clues_in_json_shape() {
    let start = Utc::now();
    let end = start;
    let mut clue_file = file("project/NOTICE");
    clue_file.license_clues = vec![Match {
        license_expression: "unknown-license-reference".to_string(),
        license_expression_spdx: "LicenseRef-scancode-unknown-license-reference".to_string(),
        from_file: Some("project/NOTICE".to_string()),
        start_line: 1,
        end_line: 2,
        matcher: Some("2-aho".to_string()),
        score: 100.0,
        matched_length: Some(19),
        match_coverage: Some(100.0),
        rule_relevance: Some(100),
        rule_identifier: Some("license-clue_1.RULE".to_string()),
        rule_url: Some("https://example.com/license-clue_1.RULE".to_string()),
        matched_text: Some(
            "This product currently only contains code developed by authors".to_string(),
        ),
    }];

    let output = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), clue_file],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result: assembly::AssemblyResult {
                packages: vec![],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: false,
                include_tallies_by_facet: false,
                include_summary: false,
                include_license_clarity_score: false,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: false,
            },
        },
    );

    let value = serde_json::to_value(&output).expect("output should serialize");
    let notice = value["files"]
        .as_array()
        .expect("files array")
        .iter()
        .find(|entry| entry["path"] == json!("project/NOTICE"))
        .expect("notice file present");

    assert_eq!(notice["license_detections"], json!([]));
    assert_eq!(
        notice["detected_license_expression_spdx"],
        serde_json::Value::Null
    );
    assert_eq!(
        notice["license_clues"][0]["license_expression"],
        "unknown-license-reference"
    );
    assert_eq!(notice["license_clues"][0]["matcher"], "2-aho");
}

#[test]
fn create_output_tallies_by_facet_does_not_leak_resource_tallies() {
    let start = Utc::now();
    let end = start;
    let mut source = file("project/src/lib.rs");
    source.programming_language = Some("Rust".to_string());

    let facet_defs = ["dev=*.rs".to_string()];
    let facet_rules = build_facet_rules(&facet_defs).expect("facet rules compile");

    let output = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), dir("project/src"), source],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 2,
            assembly_result: assembly::AssemblyResult {
                packages: vec![],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &facet_rules,
                include_classify: false,
                include_tallies_by_facet: true,
                include_summary: false,
                include_license_clarity_score: false,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: false,
            },
        },
    );

    assert!(output.tallies_by_facet.is_some());
    assert!(output.files.iter().all(|file| file.tallies.is_none()));
}

#[test]
fn create_output_promotes_package_metadata_without_summary_flags() {
    let start = Utc::now();
    let end = start;
    let package_uid = "pkg:npm/demo?uuid=test".to_string();
    let mut license = file("project/LICENSE");
    license.for_packages = vec![package_uid.clone()];
    license.copyrights = vec![Copyright {
        copyright: "Copyright Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    license.holders = vec![Holder {
        holder: "Example Corp.".to_string(),
        start_line: 1,
        end_line: 1,
    }];
    let package = Package {
        package_uid,
        datafile_paths: vec!["project/package.json".to_string()],
        ..super::test_utils::package("pkg:npm/demo?uuid=test", "project/package.json")
    };

    let output = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), license],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result: assembly::AssemblyResult {
                packages: vec![package],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: false,
                include_tallies_by_facet: false,
                include_summary: false,
                include_license_clarity_score: false,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: false,
            },
        },
    );

    assert_eq!(output.packages[0].holder.as_deref(), Some("Example Corp."));
    assert_eq!(
        output.packages[0].copyright.as_deref(),
        Some("Copyright Example Corp.")
    );
}

#[test]
fn create_output_summary_still_resolves_after_strip_root_normalization() {
    let start = Utc::now();
    let end = start;
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let root = temp.path().join("project");
    let manifest_path = root.join("demo.gemspec");
    std::fs::create_dir_all(&root).expect("root should exist");

    let mut manifest = file(manifest_path.to_str().unwrap());
    manifest.package_data = vec![crate::models::PackageData {
        package_type: Some(crate::models::PackageType::Gem),
        datasource_id: Some(crate::models::DatasourceId::Gemspec),
        declared_license_expression: Some("mit".to_string()),
        declared_license_expression_spdx: Some("MIT".to_string()),
        purl: Some("pkg:gem/demo@1.0.0".to_string()),
        ..Default::default()
    }];
    manifest.license_expression = Some("mit".to_string());
    manifest.license_detections = vec![crate::models::LicenseDetection {
        license_expression: "mit".to_string(),
        license_expression_spdx: "MIT".to_string(),
        matches: vec![Match {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("project/demo.gemspec".to_string()),
            start_line: 1,
            end_line: 1,
            matcher: Some("1-spdx-id".to_string()),
            score: 100.0,
            matched_length: Some(1),
            match_coverage: Some(100.0),
            rule_relevance: Some(100),
            rule_identifier: None,
            rule_url: None,
            matched_text: None,
        }],
        identifier: None,
    }];

    let mut files = vec![dir(root.to_str().unwrap()), manifest];
    normalize_paths(&mut files, root.to_str().unwrap(), true, false);
    let assembly_result = assembly::assemble(&mut files);

    let output = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files,
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result,
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: false,
                include_tallies_by_facet: false,
                include_summary: true,
                include_license_clarity_score: false,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: false,
            },
        },
    );

    assert_eq!(
        output
            .summary
            .and_then(|summary| summary.declared_license_expression),
        Some("mit".to_string())
    );
}

#[test]
fn create_output_classify_only_sets_key_file_flags() {
    let start = Utc::now();
    let end = start;

    let output = create_output(
        start,
        end,
        crate::scanner::ProcessResult {
            files: vec![dir("project"), file("project/README.md")],
            excluded_count: 0,
        },
        CreateOutputContext {
            total_dirs: 1,
            assembly_result: assembly::AssemblyResult {
                packages: vec![],
                dependencies: vec![],
            },
            license_references: vec![],
            license_rule_references: vec![],
            options: CreateOutputOptions {
                facet_rules: &[],
                include_classify: true,
                include_tallies_by_facet: false,
                include_summary: false,
                include_license_clarity_score: false,
                include_tallies: false,
                include_tallies_with_details: false,
                include_tallies_of_key_files: false,
                include_generated: false,
            },
        },
    );

    let readme = output
        .files
        .iter()
        .find(|file| file.path == "project/README.md")
        .expect("README should exist");

    assert!(readme.is_readme);
    assert!(readme.is_top_level);
    assert!(readme.is_key_file);
}
