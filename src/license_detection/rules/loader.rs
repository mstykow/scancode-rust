//! Parse .LICENSE and .RULE files.

use crate::license_detection::models::{License, Rule};
use anyhow::{Context, Result, anyhow};
use log::warn;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn deserialize_yes_no_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize, Serialize)]
    #[serde(untagged)]
    enum YesNoOrBool {
        String(String),
        Bool(bool),
    }

    match YesNoOrBool::deserialize(deserializer)? {
        YesNoOrBool::Bool(b) => Ok(Some(b)),
        YesNoOrBool::String(s) => {
            let lower = s.to_lowercase();
            if lower == "yes" || lower == "true" || lower == "1" {
                Ok(Some(true))
            } else if lower == "no" || lower == "false" || lower == "0" {
                Ok(Some(false))
            } else {
                Ok(None)
            }
        }
    }
}

trait ParseNumber {
    fn as_u8(&self) -> Option<u8>;
}

impl ParseNumber for serde_yaml::Number {
    fn as_u8(&self) -> Option<u8> {
        self.as_i64()
            .and_then(|n| {
                if n >= 0 && n <= u8::MAX as i64 {
                    Some(n as u8)
                } else {
                    None
                }
            })
            .or_else(|| {
                self.as_f64().and_then(|f| {
                    if f >= 0.0 && f <= u8::MAX as f64 {
                        Some(f as u8)
                    } else {
                        None
                    }
                })
            })
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LicenseFrontmatter {
    #[serde(default)]
    key: Option<String>,

    #[serde(default)]
    short_name: Option<String>,

    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    category: Option<String>,

    #[serde(default)]
    owner: Option<String>,

    #[serde(default)]
    homepage_url: Option<String>,

    #[serde(default)]
    notes: Option<String>,

    #[serde(default)]
    spdx_license_key: Option<String>,

    #[serde(default)]
    other_spdx_license_keys: Option<Vec<String>>,

    #[serde(default)]
    osi_license_key: Option<String>,

    #[serde(default)]
    text_urls: Option<Vec<String>>,

    #[serde(default)]
    osi_url: Option<String>,

    #[serde(default)]
    faq_url: Option<String>,

    #[serde(default)]
    other_urls: Option<Vec<String>>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_deprecated: Option<bool>,

    #[serde(default)]
    replaced_by: Option<Vec<String>>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_exception: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_unknown: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_generic: Option<bool>,

    #[serde(default)]
    minimum_coverage: Option<serde_yaml::Number>,

    #[serde(default)]
    standard_notice: Option<String>,

    #[serde(default)]
    ignorable_copyrights: Option<Vec<String>>,

    #[serde(default)]
    ignorable_holders: Option<Vec<String>>,

    #[serde(default)]
    ignorable_authors: Option<Vec<String>>,

    #[serde(default)]
    ignorable_urls: Option<Vec<String>>,

    #[serde(default)]
    ignorable_emails: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RuleFrontmatter {
    #[serde(default)]
    license_expression: Option<String>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_text: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_notice: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_reference: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_tag: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_intro: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_clue: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_false_positive: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_required_phrase: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    skip_for_required_phrase_generation: Option<bool>,

    #[serde(default)]
    relevance: Option<serde_yaml::Number>,

    #[serde(default)]
    minimum_coverage: Option<serde_yaml::Number>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_continuous: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_deprecated: Option<bool>,

    #[serde(default)]
    referenced_filenames: Option<Vec<String>>,

    #[serde(default)]
    replaced_by: Option<Vec<String>>,

    #[serde(default)]
    ignorable_urls: Option<Vec<String>>,

    #[serde(default)]
    ignorable_emails: Option<Vec<String>>,

    #[serde(default)]
    notes: Option<String>,

    #[serde(default)]
    ignorable_copyrights: Option<Vec<String>>,

    #[serde(default)]
    ignorable_holders: Option<Vec<String>>,

    #[serde(default)]
    ignorable_authors: Option<Vec<String>>,

    #[serde(default)]
    language: Option<String>,
}

pub fn parse_rule_file(path: &Path) -> Result<Rule> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read rule file: {}", path.display()))?;

    if content.len() < 6 {
        return Err(anyhow!("Rule file content too short: {}", path.display()));
    }

    let identifier = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.RULE")
        .to_string();

    let parts: Vec<&str> = content.split("---").collect();

    if parts.len() < 3 {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(anyhow!(
                "Rule file is empty or has no content: {}",
                path.display()
            ));
        }
        return Err(anyhow!(
            "Rule file missing delimiter '---': {}",
            path.display()
        ));
    }

    let yaml_content = parts
        .get(1)
        .ok_or_else(|| anyhow!("Missing YAML frontmatter in {}", path.display()))?;
    let text_content = parts.get(2).ok_or_else(|| {
        anyhow!(
            "Missing text content after frontmatter in {}",
            path.display()
        )
    })?;

    let trimmed_text = text_content.trim_start_matches('\n').trim();

    if trimmed_text.is_empty() {
        return Err(anyhow!(
            "Rule file has empty text content: {}",
            path.display()
        ));
    }

    let fm: RuleFrontmatter = match serde_yaml::from_str(yaml_content) {
        Ok(fm) => fm,
        Err(e) => {
            return Err(anyhow!(
                "Failed to parse rule frontmatter YAML in {}: {}\nContent was:\n{}",
                path.display(),
                e,
                yaml_content
            ));
        }
    };

    let is_false_positive = fm.is_false_positive.unwrap_or(false);

    let license_expression = match fm.license_expression {
        Some(expr) => expr,
        None if is_false_positive => "unknown".to_string(),
        None => {
            return Err(anyhow!(
                "Rule file missing required field 'license_expression': {}",
                path.display()
            ));
        }
    };

    let relevance = match fm.relevance {
        Some(num) => num.as_u8().unwrap_or(100),
        None => 100,
    };

    let minimum_coverage = fm.minimum_coverage.and_then(|n| n.as_u8());

    Ok(Rule {
        identifier,
        license_expression,
        text: trimmed_text.to_string(),
        tokens: vec![],
        is_license_text: fm.is_license_text.unwrap_or(false),
        is_license_notice: fm.is_license_notice.unwrap_or(false),
        is_license_reference: fm.is_license_reference.unwrap_or(false),
        is_license_tag: fm.is_license_tag.unwrap_or(false),
        is_license_intro: fm.is_license_intro.unwrap_or(false),
        is_license_clue: fm.is_license_clue.unwrap_or(false),
        is_false_positive: fm.is_false_positive.unwrap_or(false),
        is_required_phrase: fm.is_required_phrase.unwrap_or(false),
        is_from_license: false,
        relevance,
        minimum_coverage,
        is_continuous: fm.is_continuous.unwrap_or(false),
        referenced_filenames: fm.referenced_filenames.filter(|v| !v.is_empty()),
        ignorable_urls: fm.ignorable_urls.filter(|v| !v.is_empty()),
        ignorable_emails: fm.ignorable_emails.filter(|v| !v.is_empty()),
        ignorable_copyrights: fm.ignorable_copyrights.filter(|v| !v.is_empty()),
        ignorable_holders: fm.ignorable_holders.filter(|v| !v.is_empty()),
        ignorable_authors: fm.ignorable_authors.filter(|v| !v.is_empty()),
        language: fm.language,
        notes: fm.notes.filter(|s| !s.trim().is_empty()),
        length_unique: 0,
        high_length_unique: 0,
        high_length: 0,
        min_matched_length: 0,
        min_high_matched_length: 0,
        min_matched_length_unique: 0,
        min_high_matched_length_unique: 0,
        is_small: false,
        is_tiny: false,
    })
}

pub fn parse_license_file(path: &Path) -> Result<License> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read license file: {}", path.display()))?;

    if content.len() < 6 {
        return Err(anyhow!(
            "License file content too short: {}",
            path.display()
        ));
    }

    let parts: Vec<&str> = content.split("---").collect();

    if parts.len() < 3 {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(anyhow!(
                "License file is empty or has no content: {}",
                path.display()
            ));
        }
        return Err(anyhow!(
            "License file missing delimiter '---': {}",
            path.display()
        ));
    }

    let yaml_content = parts
        .get(1)
        .ok_or_else(|| anyhow!("Missing YAML frontmatter in {}", path.display()))?;
    let text_content = parts.get(2).ok_or_else(|| {
        anyhow!(
            "Missing text content after frontmatter in {}",
            path.display()
        )
    })?;

    let trimmed_text = text_content.trim_start_matches('\n').trim();

    let fm: LicenseFrontmatter = match serde_yaml::from_str(yaml_content) {
        Ok(fm) => fm,
        Err(e) => {
            return Err(anyhow!(
                "Failed to parse license frontmatter YAML in {}: {}\nContent was:\n{}",
                path.display(),
                e,
                yaml_content
            ));
        }
    };

    let key = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        anyhow!(
            "Cannot extract key from license file path: {}",
            path.display()
        )
    })?;

    if let Some(fm_key) = fm.key
        && fm_key != key
    {
        return Err(anyhow!(
            "License key mismatch: filename '{}' vs frontmatter key '{}' in file: {}",
            key,
            fm_key,
            path.display()
        ));
    }

    let is_deprecated = fm.is_deprecated.unwrap_or(false);

    if trimmed_text.is_empty()
        && !is_deprecated
        && !fm.is_unknown.unwrap_or(false)
        && !fm.is_generic.unwrap_or(false)
    {
        return Err(anyhow!(
            "License file has empty text content and is not deprecated/unknown/generic: {}",
            path.display()
        ));
    }

    let mut urls = vec![];
    if let Some(mut u) = fm.text_urls {
        urls.append(&mut u);
    }
    if let Some(u) = fm.other_urls {
        urls.extend(u);
    }
    if let Some(u) = fm.osi_url {
        urls.push(u);
    }
    if let Some(u) = fm.faq_url {
        urls.push(u);
    }
    if let Some(u) = fm.homepage_url {
        urls.push(u);
    }

    Ok(License {
        key: key.to_string(),
        name: fm
            .name
            .unwrap_or_else(|| fm.short_name.clone().unwrap_or_else(|| key.to_string())),
        spdx_license_key: fm.spdx_license_key,
        category: fm.category,
        text: trimmed_text.to_string(),
        reference_urls: urls,
        notes: fm.notes.filter(|s| !s.trim().is_empty()),
        is_deprecated,
        replaced_by: fm.replaced_by.unwrap_or_default(),
        minimum_coverage: fm.minimum_coverage.and_then(|n| n.as_u8()),
        ignorable_copyrights: fm.ignorable_copyrights.filter(|v| !v.is_empty()),
        ignorable_holders: fm.ignorable_holders.filter(|v| !v.is_empty()),
        ignorable_authors: fm.ignorable_authors.filter(|v| !v.is_empty()),
        ignorable_urls: fm.ignorable_urls.filter(|v| !v.is_empty()),
        ignorable_emails: fm.ignorable_emails.filter(|v| !v.is_empty()),
    })
}

fn load_rules_from_dir(dir: &Path, pattern: &str) -> Result<Vec<Rule>> {
    let mut rules = Vec::new();

    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read rules directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry
            .with_context(|| format!("Failed to read directory entry in: {}", dir.display()))?;
        let path = entry.path();

        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some(pattern.trim_start_matches('.'))
        {
            match parse_rule_file(&path) {
                Ok(rule) => rules.push(rule),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse rule file {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(rules)
}

fn load_licenses_from_dir(dir: &Path, pattern: &str) -> Result<Vec<License>> {
    let mut licenses = Vec::new();

    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read licenses directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry
            .with_context(|| format!("Failed to read directory entry in: {}", dir.display()))?;
        let path = entry.path();

        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some(pattern.trim_start_matches('.'))
        {
            match parse_license_file(&path) {
                Ok(license) => licenses.push(license),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse license file {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(licenses)
}

pub fn load_rules_from_directory(dir: &Path) -> Result<Vec<Rule>> {
    let rules = load_rules_from_dir(dir, ".RULE")?;
    validate_rules(&rules);
    Ok(rules)
}

pub fn load_licenses_from_directory(dir: &Path) -> Result<Vec<License>> {
    load_licenses_from_dir(dir, ".LICENSE")
}

/// Validate loaded rules for common issues.
///
/// Checks for:
/// 1. Duplicate rule texts (warns if found)
/// 2. Empty license expressions for non-false-positive rules (warns if found)
///
/// Corresponds to Python:
/// - `models.py:validate()` for license expression validation
/// - `index.py:_add_rules()` for duplicate detection via hash
fn validate_rules(rules: &[Rule]) {
    let mut seen_texts: HashSet<&str> = HashSet::new();
    let mut duplicate_count = 0;

    for rule in rules {
        if !seen_texts.insert(&rule.text) {
            warn!(
                "Duplicate rule text found for license_expression: {}",
                rule.license_expression
            );
            duplicate_count += 1;
        }

        if !rule.is_false_positive && rule.license_expression.trim().is_empty() {
            warn!("Rule has empty license_expression but is not marked as false_positive");
        }
    }

    if duplicate_count > 0 {
        warn!(
            "Found {} duplicate rule text(s) during rule validation",
            duplicate_count
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_number_as_u8() {
        let num_int: serde_yaml::Number = serde_yaml::from_str("100").unwrap();
        assert_eq!(num_int.as_u8(), Some(100));

        let num_out_of_range: serde_yaml::Number = serde_yaml::from_str("500").unwrap();
        assert_eq!(num_out_of_range.as_u8(), None);

        let num_float: serde_yaml::Number = serde_yaml::from_str("90.5").unwrap();
        assert_eq!(num_float.as_u8(), Some(90));
    }

    #[test]
    fn test_parse_simple_license_file() {
        let dir = tempdir().unwrap();
        let license_path = dir.path().join("mit.LICENSE");
        fs::write(
            &license_path,
            r#"---
key: mit
short_name: MIT License
name: MIT License
category: Permissive
spdx_license_key: MIT
---
MIT License text here"#,
        )
        .unwrap();

        let license = parse_license_file(&license_path).unwrap();
        assert_eq!(license.key, "mit");
        assert_eq!(license.name, "MIT License");
        assert!(license.text.contains("MIT License text"));
    }

    #[test]
    fn test_parse_simple_rule_file() {
        let dir = tempdir().unwrap();
        let rule_path = dir.path().join("mit_1.RULE");
        fs::write(
            &rule_path,
            r#"---
license_expression: mit
is_license_reference: yes
relevance: 90
referenced_filenames:
    - MIT.txt
---
MIT.txt"#,
        )
        .unwrap();

        let rule = parse_rule_file(&rule_path).unwrap();
        assert_eq!(rule.license_expression, "mit");
        assert_eq!(rule.text, "MIT.txt");
        assert!(rule.is_license_reference);
        assert_eq!(rule.relevance, 90);
    }

    #[test]
    fn test_deserialize_yes_no_bool() {
        let dir = tempdir().unwrap();
        let rule_path = dir.path().join("test.RULE");

        fs::write(
            &rule_path,
            r#"---
license_expression: mit
is_license_notice: yes
is_license_tag: no
---
MIT License"#,
        )
        .unwrap();

        let rule = parse_rule_file(&rule_path).unwrap();
        assert!(rule.is_license_notice);
        assert!(!rule.is_license_tag);
    }

    #[test]
    fn test_load_licenses_from_reference() {
        let path = Path::new("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let licenses = load_licenses_from_directory(path).unwrap();
        assert!(!licenses.is_empty());

        let mit = licenses.iter().find(|l| l.key == "mit");
        assert!(mit.is_some(), "MIT license should be loaded");
        let mit = mit.unwrap();
        assert_eq!(mit.name, "MIT License");
        assert_eq!(mit.spdx_license_key, Some("MIT".to_string()));
        assert!(!mit.text.is_empty());
    }

    #[test]
    fn test_load_rules_from_reference() {
        let path = Path::new("reference/scancode-toolkit/src/licensedcode/data/rules");
        if !path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let rules = load_rules_from_directory(path).unwrap();
        assert!(!rules.is_empty());

        let mit_rule = rules
            .iter()
            .find(|r| r.license_expression == "mit" && r.text.contains("MIT.txt"));
        assert!(mit_rule.is_some(), "MIT reference rule should be loaded");
        let mit_rule = mit_rule.unwrap();
        assert!(mit_rule.is_license_reference);
        assert_eq!(mit_rule.relevance, 90);
    }

    #[test]
    fn test_validate_rules_detects_duplicates() {
        let rules = vec![
            Rule {
                identifier: "mit.LICENSE".to_string(),
                license_expression: "mit".to_string(),
                text: "MIT License".to_string(),
                tokens: vec![],
                is_license_text: true,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: false,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 0,
                high_length_unique: 0,
                high_length: 0,
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
            },
            Rule {
                identifier: "apache-2.0.LICENSE".to_string(),
                license_expression: "apache-2.0".to_string(),
                text: "MIT License".to_string(),
                tokens: vec![],
                is_license_text: true,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: false,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 0,
                high_length_unique: 0,
                high_length: 0,
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
            },
        ];

        validate_rules(&rules);
    }

    #[test]
    fn test_validate_rules_accepts_false_positive_without_expression() {
        let rules = vec![Rule {
            identifier: "fp.RULE".to_string(),
            license_expression: "".to_string(),
            text: "Some text".to_string(),
            tokens: vec![],
            is_license_text: false,
            is_license_notice: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_intro: false,
            is_license_clue: false,
            is_false_positive: true,
            is_required_phrase: false,
            is_from_license: false,
            relevance: 100,
            minimum_coverage: None,
            is_continuous: false,
            referenced_filenames: None,
            ignorable_urls: None,
            ignorable_emails: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            language: None,
            notes: Some("False positive for common pattern".to_string()),
            length_unique: 0,
            high_length_unique: 0,
            high_length: 0,
            min_matched_length: 0,
            min_high_matched_length: 0,
            min_matched_length_unique: 0,
            min_high_matched_length_unique: 0,
            is_small: false,
            is_tiny: false,
        }];

        validate_rules(&rules);
    }

    #[test]
    fn test_validate_rules_no_duplicates() {
        let rules = vec![
            Rule {
                identifier: "mit.LICENSE".to_string(),
                license_expression: "mit".to_string(),
                text: "MIT License".to_string(),
                tokens: vec![],
                is_license_text: true,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: false,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 0,
                high_length_unique: 0,
                high_length: 0,
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
            },
            Rule {
                identifier: "apache-2.0.LICENSE".to_string(),
                license_expression: "apache-2.0".to_string(),
                text: "Apache License".to_string(),
                tokens: vec![],
                is_license_text: true,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: false,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 0,
                high_length_unique: 0,
                high_length: 0,
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
            },
        ];

        validate_rules(&rules);
    }
}
