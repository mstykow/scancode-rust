//! Tests for rule/license file parsing.
//!
//! Tests for YAML frontmatter parsing edge cases including:
//! - PGP signatures in content
//! - Dashes in license text
//! - Various delimiter formats

use super::*;
use crate::license_detection::models::{License, Rule};
use anyhow::Result;

fn parse_license_from_str(content: &str, filename: &str) -> Result<License> {
    let temp_path = std::env::temp_dir().join(filename);
    std::fs::write(&temp_path, content)?;
    let result = parse_license_file(&temp_path);
    let _ = std::fs::remove_file(&temp_path);
    result
}

fn parse_rule_from_str(content: &str, filename: &str) -> Result<Rule> {
    let temp_path = std::env::temp_dir().join(filename);
    std::fs::write(&temp_path, content)?;
    let result = parse_rule_file(&temp_path);
    let _ = std::fs::remove_file(&temp_path);
    result
}

#[test]
fn test_parse_license_file_basic() {
    let content = r#"---
key: test-license
name: Test License
category: Permissive
---
This is the license text.
It has multiple lines."#;

    let result = parse_license_from_str(content, "test-license.LICENSE");
    assert!(result.is_ok(), "Should parse basic license: {:?}", result);
    let license = result.unwrap();
    assert_eq!(license.key, "test-license");
    assert_eq!(license.name, "Test License");
    assert!(license.text.contains("license text"));
}

#[test]
fn test_parse_rule_file_basic() {
    let content = r#"---
license_expression: mit
is_license_text: yes
---
Permission is hereby granted, free of charge."#;

    let result = parse_rule_from_str(content, "mit_1.RULE");
    assert!(result.is_ok(), "Should parse basic rule: {:?}", result);
    let rule = result.unwrap();
    assert_eq!(rule.license_expression, "mit");
    assert!(rule.text.contains("Permission"));
    assert!(rule.is_license_text);
}

#[test]
fn test_parse_license_empty_frontmatter() {
    let content = r#"---
---
This is the license text."#;

    let result = parse_license_from_str(content, "empty-fm.LICENSE");
    assert!(result.is_ok(), "Empty frontmatter should use defaults");
}

#[test]
fn test_parse_license_empty_text() {
    let content = r#"---
key: empty-text
name: Test License
---
"#;

    let result = parse_license_from_str(content, "empty-text.LICENSE");
    assert!(
        result.is_err(),
        "Empty text should fail for non-deprecated license"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("empty text content")
    );
}

#[test]
fn test_parse_license_empty_text_deprecated_allowed() {
    let content = r#"---
key: deprecated
name: Test License
is_deprecated: yes
---
"#;

    let result = parse_license_from_str(content, "deprecated.LICENSE");
    assert!(result.is_ok(), "Deprecated license can have empty text");
}

#[test]
fn test_parse_license_empty_text_unknown_allowed() {
    let content = r#"---
key: unknown
name: Test License
is_unknown: yes
---
"#;

    let result = parse_license_from_str(content, "unknown.LICENSE");
    assert!(result.is_ok(), "Unknown license can have empty text");
}

#[test]
fn test_parse_license_empty_text_generic_allowed() {
    let content = r#"---
key: generic
name: Test License
is_generic: yes
---
"#;

    let result = parse_license_from_str(content, "generic.LICENSE");
    assert!(result.is_ok(), "Generic license can have empty text");
}

#[test]
fn test_parse_rule_empty_text() {
    let content = r#"---
license_expression: mit
---
"#;

    let result = parse_rule_from_str(content, "empty-text.RULE");
    assert!(result.is_err(), "Rule with empty text should fail");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("empty text content")
    );
}

#[test]
fn test_parse_license_with_pgp_signature() {
    let content = r#"---
key: pgp-license
name: Test License
---
-----BEGIN PGP SIGNED MESSAGE-----
Hash: SHA256

This is the license text with a PGP signature.
-----END PGP SIGNATURE-----"#;

    let result = parse_license_from_str(content, "pgp-license.LICENSE");
    assert!(
        result.is_ok(),
        "Should parse license with PGP signature: {:?}",
        result
    );
    let license = result.unwrap();
    assert!(
        license.text.contains("PGP SIGNED MESSAGE"),
        "Text should contain PGP marker"
    );
    assert!(
        license.text.contains("license text"),
        "Text should contain actual license"
    );
}

#[test]
fn test_parse_license_with_dashes_in_content() {
    let content = r#"---
key: dashes-content
name: Test License
---
The Artistic License 1.0
--- 
This is a separator in the license text.
More text here.
--- end ---"#;

    let result = parse_license_from_str(content, "dashes-content.LICENSE");
    assert!(
        result.is_ok(),
        "Should parse license with dashes in content: {:?}",
        result
    );
    let license = result.unwrap();
    assert!(
        license.text.contains("separator"),
        "Text should contain content after dashes"
    );
    assert!(
        license.text.contains("--- end ---"),
        "Text should contain inline dashes"
    );
}

#[test]
fn test_parse_license_with_four_dash_delimiter() {
    let content = r#"----
key: four-dash
name: Test License
----
This is the license text."#;

    let result = parse_license_from_str(content, "four-dash.LICENSE");
    assert!(
        result.is_ok(),
        "Should parse license with 4-dash delimiter: {:?}",
        result
    );
    let license = result.unwrap();
    assert!(
        license.text.contains("license text"),
        "Text should be extracted correctly"
    );
}

#[test]
fn test_parse_rule_with_pgp_signature() {
    let content = r#"---
license_expression: test-rule
---
-----BEGIN PGP SIGNED MESSAGE-----

Rule text here.
-----END PGP SIGNATURE-----"#;

    let result = parse_rule_from_str(content, "pgp-rule.RULE");
    assert!(
        result.is_ok(),
        "Should parse rule with PGP signature: {:?}",
        result
    );
    let rule = result.unwrap();
    assert!(
        rule.text.contains("PGP SIGNED MESSAGE"),
        "Text should contain PGP marker"
    );
}

#[test]
fn test_parse_rule_false_positive() {
    let content = r#"---
license_expression: gpl-2.0-plus
is_false_positive: yes
---
GPL"#;

    let result = parse_rule_from_str(content, "false-positive.RULE");
    assert!(result.is_ok(), "Should parse false positive rule");
    let rule = result.unwrap();
    assert!(rule.is_false_positive);
}

#[test]
fn test_parse_license_minimum_coverage() {
    let content = r#"---
key: min-coverage
name: Test License
minimum_coverage: 80
---
License text here."#;

    let result = parse_license_from_str(content, "min-coverage.LICENSE");
    assert!(result.is_ok());
    let license = result.unwrap();
    assert_eq!(license.minimum_coverage, Some(80));
}

#[test]
fn test_parse_rule_minimum_coverage() {
    let content = r#"---
license_expression: test
minimum_coverage: 90
---
Rule text."#;

    let result = parse_rule_from_str(content, "min-coverage.RULE");
    assert!(result.is_ok());
    let rule = result.unwrap();
    assert_eq!(rule.minimum_coverage, Some(90));
}

#[test]
fn test_parse_license_no_key_in_frontmatter() {
    let content = r#"---
name: Test License
---
Text."#;

    let result = parse_license_from_str(content, "no-key.LICENSE");
    assert!(result.is_ok(), "Missing key should use filename");
    let license = result.unwrap();
    assert_eq!(license.key, "no-key");
}

#[test]
fn test_parse_rule_with_all_boolean_flags() {
    let content = r#"---
license_expression: test
is_license_text: yes
is_license_notice: yes
is_license_reference: yes
is_license_tag: yes
is_license_intro: yes
is_license_clue: yes
is_false_positive: yes
is_continuous: yes
---
Text."#;

    let result = parse_rule_from_str(content, "flags.RULE");
    assert!(result.is_ok());
    let rule = result.unwrap();
    assert!(rule.is_license_text);
    assert!(rule.is_license_notice);
    assert!(rule.is_license_reference);
    assert!(rule.is_license_tag);
    assert!(rule.is_license_intro);
    assert!(rule.is_license_clue);
    assert!(rule.is_false_positive);
    assert!(rule.is_continuous);
}

#[test]
fn test_parse_license_with_multiline_yaml() {
    let content = r#"---
key: multiline
name: Test License
notes: |
    Line 1
    Line 2
    Line 3
---
Text."#;

    let result = parse_license_from_str(content, "multiline.LICENSE");
    assert!(result.is_ok());
    let license = result.unwrap();
    assert!(license.notes.as_ref().unwrap().contains("Line 1"));
    assert!(license.notes.as_ref().unwrap().contains("Line 2"));
}

#[test]
fn test_parse_license_with_trailing_whitespace_on_delimiter() {
    let content = "---  \nkey: ws-delimiter\nname: Test\n---  \nText.";

    let result = parse_license_from_str(content, "ws-delimiter.LICENSE");
    assert!(
        result.is_ok(),
        "Should handle trailing whitespace on delimiter"
    );
    let license = result.unwrap();
    assert!(license.text.contains("Text"));
}
