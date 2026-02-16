//! SPDX license key mapping for license expressions.
//!
//! This module provides bidirectional mapping between ScanCode license keys and
//! SPDX license identifiers. It loads the mapping data from License objects and
//! provides functions to convert license expressions from ScanCode keys to SPDX keys.
//!
//! Based on the Python ScanCode Toolkit implementation:
//! - `build_spdx_license_expression()` in `reference/scancode-toolkit/src/licensedcode/cache.py`
//! - License.spdx_license_key in `reference/scancode-toolkit/src/licensedcode/models.py`

use std::collections::HashMap;

use crate::license_detection::expression::{
    LicenseExpression, expression_to_string, parse_expression,
};
use crate::license_detection::models::License;

/// Bidirectional mapping between ScanCode and SPDX license keys.
///
/// This structure enables conversion of license expressions from ScanCode-specific
/// license keys (lowercase, e.g., "mit", "gpl-2.0-plus") to SPDX license identifiers
/// (case-sensitive, e.g., "MIT", "GPL-2.0-or-later") and vice versa.
#[derive(Debug, Clone)]
pub struct SpdxMapping {
    /// Mapping from ScanCode license key to SPDX license key.
    ///
    /// Keys are lowercase ScanCode license keys. Values are SPDX license identifiers.
    scancode_to_spdx: HashMap<String, String>,

    /// Mapping from SPDX license key to ScanCode license key.
    ///
    /// Keys are SPDX license identifiers (case-sensitive). Values are lowercase ScanCode keys.
    /// When multiple ScanCode keys map to the same SPDX key, the first one encountered is used.
    #[allow(dead_code)]
    spdx_to_scancode: HashMap<String, String>,
}

impl SpdxMapping {
    /// Build a bidirectional SPDX mapping from a slice of License objects.
    ///
    /// This function extracts the `spdx_license_key` field from each License
    /// and builds the two-way mapping. For licenses without an SPDX equivalent,
    /// they are mapped to `LicenseRef-scancode-<key>` format.
    ///
    /// # Arguments
    /// * `licenses` - Slice of License objects to build mapping from
    ///
    /// # Returns
    /// A SpdxMapping with populated bidirectional mappings
    ///
    /// # Example
    /// ```
    /// use scancode_rust::license_detection::spdx_mapping::build_spdx_mapping;
    /// use scancode_rust::license_detection::models::License;
    ///
    /// let licenses = vec![
    ///     License {
    ///         key: "mit".to_string(),
    ///         name: "MIT License".to_string(),
    ///         spdx_license_key: Some("MIT".to_string()),
    ///         category: Some("Permissive".to_string()),
    ///         text: "MIT License text...".to_string(),
    ///         reference_urls: vec![],
    ///         notes: None,
    ///     },
    /// ];
    ///
    /// let mapping = build_spdx_mapping(&licenses);
    /// ```
    pub fn build_from_licenses(licenses: &[License]) -> Self {
        let mut scancode_to_spdx = HashMap::new();
        let mut spdx_to_scancode = HashMap::new();

        for license in licenses {
            let scancode_key = &license.key;

            if let Some(spdx_key) = &license.spdx_license_key {
                scancode_to_spdx.insert(scancode_key.clone(), spdx_key.clone());

                spdx_to_scancode
                    .entry(spdx_key.clone())
                    .or_insert_with(|| scancode_key.clone());
            } else {
                let licenseref_key = format!("LicenseRef-scancode-{}", scancode_key);
                scancode_to_spdx.insert(scancode_key.clone(), licenseref_key.clone());

                spdx_to_scancode
                    .entry(licenseref_key)
                    .or_insert_with(|| scancode_key.clone());
            }
        }

        Self {
            scancode_to_spdx,
            spdx_to_scancode,
        }
    }

    /// Convert a ScanCode license key to its SPDX equivalent.
    ///
    /// # Arguments
    /// * `scancode_key` - Lowercase ScanCode license key (e.g., "mit", "gpl-2.0-plus")
    ///
    /// # Returns
    /// Option containing SPDX license identifier, or None if key not found
    ///
    /// # Example
    /// ```
    /// use scancode_rust::license_detection::spdx_mapping::SpdxMapping;
    ///
    /// let spdx = SpdxMapping::scancode_to_spdx(&mapping, "mit");
    /// assert_eq!(spdx, Some("MIT".to_string()));
    /// ```
    pub fn scancode_to_spdx(&self, scancode_key: &str) -> Option<String> {
        self.scancode_to_spdx.get(scancode_key).cloned()
    }

    /// Convert an SPDX license key to its ScanCode equivalent.
    ///
    /// # Arguments
    /// * `spdx_key` - SPDX license identifier (e.g., "MIT", "GPL-2.0-or-later")
    ///
    /// # Returns
    /// Option containing lowercase ScanCode license key, or None if key not found
    ///
    /// # Example
    /// ```
    /// use scancode_rust::license_detection::spdx_mapping::SpdxMapping;
    ///
    /// let scancode = SpdxMapping::spdx_to_scancode(&mapping, "MIT");
    /// assert_eq!(scancode, Some("mit".to_string()));
    /// ```
    #[allow(dead_code)]
    pub fn spdx_to_scancode(&self, spdx_key: &str) -> Option<String> {
        self.spdx_to_scancode.get(spdx_key).cloned()
    }

    /// Convert a license expression from ScanCode keys to SPDX keys.
    ///
    /// This function parses the expression, replaces each license key with its SPDX
    /// equivalent, and serializes the result back to a string.
    ///
    /// # Arguments
    /// * `scancode_expr` - License expression string with ScanCode keys
    ///
    /// # Returns
    /// String containing the expression with SPDX keys, or parse error
    ///
    /// # Example
    /// ```
    /// use scancode_rust::license_detection::spdx_mapping::SpdxMapping;
    ///
    /// let spdx_expr = mapping.expression_scancode_to_spdx("mit OR gpl-2.0-plus");
    /// assert_eq!(spdx_expr?, "MIT OR LicenseRef-scancode-gpl-2.0-plus");
    /// ```
    pub fn expression_scancode_to_spdx(&self, scancode_expr: &str) -> Result<String, String> {
        let parsed = parse_expression(scancode_expr).map_err(|e| format!("Parse error: {}", e))?;
        let converted = self.convert_expression_to_spdx(&parsed);
        Ok(expression_to_string(&converted))
    }

    /// Internal function to convert a LicenseExpression from ScanCode to SPDX keys.
    fn convert_expression_to_spdx(&self, expr: &LicenseExpression) -> LicenseExpression {
        match expr {
            LicenseExpression::License(key) => {
                if let Some(spdx_key) = self.scancode_to_spdx(key) {
                    if spdx_key.starts_with("LicenseRef-") {
                        LicenseExpression::LicenseRef(spdx_key)
                    } else {
                        LicenseExpression::License(spdx_key)
                    }
                } else {
                    LicenseExpression::LicenseRef(format!("LicenseRef-scancode-{}", key))
                }
            }
            LicenseExpression::LicenseRef(key) => {
                if let Some(spdx_key) = self.scancode_to_spdx(key) {
                    LicenseExpression::LicenseRef(spdx_key)
                } else {
                    LicenseExpression::LicenseRef(key.clone())
                }
            }
            LicenseExpression::And { left, right } => LicenseExpression::And {
                left: Box::new(self.convert_expression_to_spdx(left)),
                right: Box::new(self.convert_expression_to_spdx(right)),
            },
            LicenseExpression::Or { left, right } => LicenseExpression::Or {
                left: Box::new(self.convert_expression_to_spdx(left)),
                right: Box::new(self.convert_expression_to_spdx(right)),
            },
            LicenseExpression::With { left, right } => LicenseExpression::With {
                left: Box::new(self.convert_expression_to_spdx(left)),
                right: Box::new(self.convert_expression_to_spdx(right)),
            },
        }
    }

    /// Get the number of ScanCode keys in the mapping.
    #[allow(dead_code)]
    pub fn scancode_count(&self) -> usize {
        self.scancode_to_spdx.len()
    }

    /// Get the number of SPDX keys in the mapping.
    #[allow(dead_code)]
    pub fn spdx_count(&self) -> usize {
        self.spdx_to_scancode.len()
    }
}

/// Build a bidirectional SPDX mapping from a slice of License objects.
///
/// Convenience function that creates a new SpdxMapping instance.
///
/// # Arguments
/// * `licenses` - Slice of License objects to build mapping from
///
/// # Returns
/// A SpdxMapping with populated bidirectional mappings
pub fn build_spdx_mapping(licenses: &[License]) -> SpdxMapping {
    SpdxMapping::build_from_licenses(licenses)
}

/// Convert a ScanCode license key to its SPDX equivalent.
///
/// Convenience function for key-level conversion.
///
/// # Arguments
/// * `mapping` - The SpdxMapping to use for conversion
/// * `scancode_key` - Lowercase ScanCode license key
///
/// # Returns
/// Option containing SPDX license identifier, or None if key not found
#[allow(dead_code)]
pub fn scancode_to_spdx(mapping: &SpdxMapping, scancode_key: &str) -> Option<String> {
    mapping.scancode_to_spdx(scancode_key)
}

/// Convert an SPDX license key to its ScanCode equivalent.
///
/// Convenience function for key-level conversion.
///
/// # Arguments
/// * `mapping` - The SpdxMapping to use for conversion
/// * `spdx_key` - SPDX license identifier
///
/// # Returns
/// Option containing lowercase ScanCode license key, or None if key not found
#[allow(dead_code)]
pub fn spdx_to_scancode(mapping: &SpdxMapping, spdx_key: &str) -> Option<String> {
    mapping.spdx_to_scancode(spdx_key)
}

/// Convert a license expression from ScanCode keys to SPDX keys.
///
/// Convenience function for expression-level conversion.
///
/// # Arguments
/// * `mapping` - The SpdxMapping to use for conversion
/// * `scancode_expr` - License expression string with ScanCode keys
///
/// # Returns
/// String containing the expression with SPDX keys, or error string
#[allow(dead_code)]
pub fn expression_scancode_to_spdx(
    mapping: &SpdxMapping,
    scancode_expr: &str,
) -> Result<String, String> {
    mapping.expression_scancode_to_spdx(scancode_expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_licenses() -> Vec<License> {
        vec![
            License {
                key: "mit".to_string(),
                name: "MIT License".to_string(),
                spdx_license_key: Some("MIT".to_string()),
                category: Some("Permissive".to_string()),
                text: "MIT License text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "gpl-2.0-plus".to_string(),
                name: "GNU GPL v2.0 or later".to_string(),
                spdx_license_key: Some("GPL-2.0-or-later".to_string()),
                category: Some("Copyleft".to_string()),
                text: "GPL text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "apache-2.0".to_string(),
                name: "Apache License 2.0".to_string(),
                spdx_license_key: Some("Apache-2.0".to_string()),
                category: Some("Permissive".to_string()),
                text: "Apache License text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "custom-1".to_string(),
                name: "Custom License 1".to_string(),
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                category: Some("Unstated License".to_string()),
                text: "Custom license text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
        ]
    }

    #[test]
    fn test_build_spdx_mapping() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_to_spdx("mit"), Some("MIT".to_string()));
        assert_eq!(
            mapping.scancode_to_spdx("gpl-2.0-plus"),
            Some("GPL-2.0-or-later".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("apache-2.0"),
            Some("Apache-2.0".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("custom-1"),
            Some("LicenseRef-scancode-custom-1".to_string())
        );
    }

    #[test]
    fn test_spdx_to_scancode() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.spdx_to_scancode("MIT"), Some("mit".to_string()));
        assert_eq!(
            mapping.spdx_to_scancode("GPL-2.0-or-later"),
            Some("gpl-2.0-plus".to_string())
        );
        assert_eq!(
            mapping.spdx_to_scancode("Apache-2.0"),
            Some("apache-2.0".to_string())
        );
        assert_eq!(
            mapping.spdx_to_scancode("LicenseRef-scancode-custom-1"),
            Some("custom-1".to_string())
        );
    }

    #[test]
    fn test_scancode_to_spdx_unknown_key() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_to_spdx("unknown-key"), None);
    }

    #[test]
    fn test_spdx_to_scancode_unknown_key() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.spdx_to_scancode("UNKNOWN-KEY"), None);
    }

    #[test]
    fn test_expression_scancode_to_spdx_simple() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("mit");
        assert_eq!(result.unwrap(), "MIT");
    }

    #[test]
    fn test_expression_scancode_to_spdx_and() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("mit AND gpl-2.0-plus");
        assert_eq!(result.unwrap(), "MIT AND GPL-2.0-or-later");
    }

    #[test]
    fn test_expression_scancode_to_spdx_or() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("mit OR apache-2.0");
        assert_eq!(result.unwrap(), "MIT OR Apache-2.0");
    }

    #[test]
    fn test_expression_scancode_to_spdx_with() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("gpl-2.0-plus WITH custom-1");
        assert!(result.is_ok());
        let spdx_expr = result.unwrap();
        assert!(spdx_expr.contains("GPL-2.0-or-later"));
        assert!(spdx_expr.contains("LicenseRef-scancode-custom-1"));
    }

    #[test]
    fn test_expression_scancode_to_spdx_complex() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("(mit OR apache-2.0) AND gpl-2.0-plus");
        assert!(result.is_ok());
        let spdx_expr = result.unwrap();
        assert!(spdx_expr.contains("MIT"));
        assert!(spdx_expr.contains("Apache-2.0"));
        assert!(spdx_expr.contains("GPL-2.0-or-later"));
    }

    #[test]
    fn test_expression_scancode_to_spdx_custom_license() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("custom-1");
        assert_eq!(result.unwrap(), "LicenseRef-scancode-custom-1");
    }

    #[test]
    fn test_expression_scancode_to_spdx_parentheses() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("(mit)");
        assert_eq!(result.unwrap(), "MIT");
    }

    #[test]
    fn test_expression_scancode_to_spdx_whitespace() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result1 = mapping.expression_scancode_to_spdx("mit AND apache-2.0");
        let result2 = mapping.expression_scancode_to_spdx("mit   AND   apache-2.0");
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_expression_scancode_to_spdx_cli_validate() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let scancode_expr = "mit OR gpl-2.0-plus";
        let spdx_expr = mapping.expression_scancode_to_spdx(scancode_expr);

        assert!(spdx_expr.is_ok());
        let exp_str = spdx_expr.unwrap();
        assert_eq!(exp_str, "MIT OR GPL-2.0-or-later");
    }

    #[test]
    fn test_expression_scancode_to_spdx_case_insensitive_input() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("MIT AND Apache-2.0");
        assert_eq!(result.unwrap(), "MIT AND Apache-2.0");
    }

    #[test]
    fn test_mapping_counts() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_count(), 4);
    }

    #[test]
    fn test_convenience_functions() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(scancode_to_spdx(&mapping, "mit"), Some("MIT".to_string()));
        assert_eq!(spdx_to_scancode(&mapping, "MIT"), Some("mit".to_string()));

        let expr_result = expression_scancode_to_spdx(&mapping, "mit OR apache-2.0");
        assert_eq!(expr_result.unwrap(), "MIT OR Apache-2.0");
    }

    #[test]
    fn test_expression_scancode_to_spdx_parse_error() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("mit @ apache-2.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_expression_scancode_to_spdx_empty_parens() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("()");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_from_licenses_empty() {
        let licenses: Vec<License> = vec![];
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_count(), 0);
        assert_eq!(mapping.spdx_count(), 0);
        assert!(mapping.scancode_to_spdx("mit").is_none());
        assert!(mapping.spdx_to_scancode("MIT").is_none());
    }

    #[test]
    fn test_spdx_to_scancode_with_licenseref() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.spdx_to_scancode("LicenseRef-scancode-custom-1");
        assert_eq!(result, Some("custom-1".to_string()));
    }

    #[test]
    fn test_mapping_with_all_spdx_keys() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_to_spdx("mit"), Some("MIT".to_string()));
        assert_eq!(
            mapping.scancode_to_spdx("gpl-2.0-plus"),
            Some("GPL-2.0-or-later".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("apache-2.0"),
            Some("Apache-2.0".to_string())
        );
    }

    #[test]
    fn test_expression_with_license_ref() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("custom-1");
        assert_eq!(result.unwrap(), "LicenseRef-scancode-custom-1");
    }

    #[test]
    fn test_multiple_scancode_keys_same_spdx() {
        let licenses = vec![
            License {
                key: "mit".to_string(),
                name: "MIT License".to_string(),
                spdx_license_key: Some("MIT".to_string()),
                category: Some("Permissive".to_string()),
                text: "MIT text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "mit-x11".to_string(),
                name: "MIT X11 License".to_string(),
                spdx_license_key: Some("MIT".to_string()),
                category: Some("Permissive".to_string()),
                text: "MIT X11 text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_to_spdx("mit"), Some("MIT".to_string()));
        assert_eq!(mapping.scancode_to_spdx("mit-x11"), Some("MIT".to_string()));
        assert_eq!(mapping.spdx_to_scancode("MIT"), Some("mit".to_string()));
    }

    #[test]
    fn test_deprecated_license_mapping() {
        let licenses = vec![
            License {
                key: "gpl-2.0".to_string(),
                name: "GNU General Public License 2.0".to_string(),
                spdx_license_key: Some("GPL-2.0".to_string()),
                category: Some("Copyleft".to_string()),
                text: "GPL 2.0 text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "gpl-2.0-old".to_string(),
                name: "GNU GPL 2.0 (deprecated)".to_string(),
                spdx_license_key: Some("GPL-2.0".to_string()),
                category: Some("Copyleft".to_string()),
                text: "Old GPL".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: true,
                replaced_by: vec!["gpl-2.0".to_string()],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(
            mapping.scancode_to_spdx("gpl-2.0"),
            Some("GPL-2.0".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("gpl-2.0-old"),
            Some("GPL-2.0".to_string())
        );
    }

    #[test]
    fn test_or_later_variants() {
        let licenses = vec![
            License {
                key: "gpl-2.0-plus".to_string(),
                name: "GPL 2.0 or later".to_string(),
                spdx_license_key: Some("GPL-2.0-or-later".to_string()),
                category: Some("Copyleft".to_string()),
                text: "GPL text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "gpl-3.0-plus".to_string(),
                name: "GPL 3.0 or later".to_string(),
                spdx_license_key: Some("GPL-3.0-or-later".to_string()),
                category: Some("Copyleft".to_string()),
                text: "GPL 3.0 text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "lgpl-2.1-plus".to_string(),
                name: "LGPL 2.1 or later".to_string(),
                spdx_license_key: Some("LGPL-2.1-or-later".to_string()),
                category: Some("Copyleft".to_string()),
                text: "LGPL text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(
            mapping.scancode_to_spdx("gpl-2.0-plus"),
            Some("GPL-2.0-or-later".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("gpl-3.0-plus"),
            Some("GPL-3.0-or-later".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("lgpl-2.1-plus"),
            Some("LGPL-2.1-or-later".to_string())
        );

        let result = mapping.expression_scancode_to_spdx("gpl-2.0-plus OR lgpl-2.1-plus");
        assert_eq!(result.unwrap(), "GPL-2.0-or-later OR LGPL-2.1-or-later");
    }

    #[test]
    fn test_with_exception_expressions() {
        let licenses = vec![
            License {
                key: "gpl-2.0".to_string(),
                name: "GPL 2.0".to_string(),
                spdx_license_key: Some("GPL-2.0-only".to_string()),
                category: Some("Copyleft".to_string()),
                text: "GPL text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "classpath-exception-2.0".to_string(),
                name: "Classpath Exception 2.0".to_string(),
                spdx_license_key: Some("Classpath-exception-2.0".to_string()),
                category: Some("Copyleft".to_string()),
                text: "Exception text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "gcc-exception-3.1".to_string(),
                name: "GCC Exception 3.1".to_string(),
                spdx_license_key: Some("GCC-exception-3.1".to_string()),
                category: Some("Copyleft".to_string()),
                text: "GCC exception text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("gpl-2.0 WITH classpath-exception-2.0");
        assert_eq!(result.unwrap(), "GPL-2.0-only WITH Classpath-exception-2.0");

        let result2 = mapping.expression_scancode_to_spdx("gpl-2.0 WITH gcc-exception-3.1");
        assert_eq!(result2.unwrap(), "GPL-2.0-only WITH GCC-exception-3.1");
    }

    #[test]
    fn test_deeply_nested_expression() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping
            .expression_scancode_to_spdx("((mit OR apache-2.0) AND gpl-2.0-plus) OR custom-1");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert!(expr.contains("MIT"));
        assert!(expr.contains("Apache-2.0"));
        assert!(expr.contains("GPL-2.0-or-later"));
        assert!(expr.contains("LicenseRef-scancode-custom-1"));
    }

    #[test]
    fn test_license_key_with_special_chars() {
        let licenses = vec![
            License {
                key: "bsd-2-clause".to_string(),
                name: "BSD 2-Clause".to_string(),
                spdx_license_key: Some("BSD-2-Clause".to_string()),
                category: Some("Permissive".to_string()),
                text: "BSD text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "boost-1.0".to_string(),
                name: "Boost 1.0".to_string(),
                spdx_license_key: Some("BSL-1.0".to_string()),
                category: Some("Permissive".to_string()),
                text: "Boost text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "unicode_dfs_2015".to_string(),
                name: "Unicode DFS 2015".to_string(),
                spdx_license_key: Some("Unicode-DFS-2015".to_string()),
                category: Some("Permissive".to_string()),
                text: "Unicode text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(
            mapping.scancode_to_spdx("bsd-2-clause"),
            Some("BSD-2-Clause".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("boost-1.0"),
            Some("BSL-1.0".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("unicode_dfs_2015"),
            Some("Unicode-DFS-2015".to_string())
        );
    }

    #[test]
    fn test_multiple_with_in_expression() {
        let licenses = vec![
            License {
                key: "gpl-2.0".to_string(),
                name: "GPL 2.0".to_string(),
                spdx_license_key: Some("GPL-2.0-only".to_string()),
                category: Some("Copyleft".to_string()),
                text: "GPL text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "exception-a".to_string(),
                name: "Exception A".to_string(),
                spdx_license_key: Some("Exception-A".to_string()),
                category: Some("Exception".to_string()),
                text: "Exception A text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "exception-b".to_string(),
                name: "Exception B".to_string(),
                spdx_license_key: Some("Exception-B".to_string()),
                category: Some("Exception".to_string()),
                text: "Exception B text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx(
            "(gpl-2.0 WITH exception-a) OR (gpl-2.0 WITH exception-b)",
        );
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert!(expr.contains("GPL-2.0-only WITH Exception-A"));
        assert!(expr.contains("GPL-2.0-only WITH Exception-B"));
    }

    #[test]
    fn test_licenseref_preserved_on_unknown_key() {
        let licenses: Vec<License> = vec![];
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("unknown-license");
        assert_eq!(result.unwrap(), "LicenseRef-scancode-unknown-license");
    }

    #[test]
    fn test_mixed_known_and_unknown_licenses() {
        let licenses = vec![License {
            key: "mit".to_string(),
            name: "MIT License".to_string(),
            spdx_license_key: Some("MIT".to_string()),
            category: Some("Permissive".to_string()),
            text: "MIT text".to_string(),
            reference_urls: vec![],
            notes: None,
            is_deprecated: false,
            replaced_by: vec![],
            minimum_coverage: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            ignorable_urls: None,
            ignorable_emails: None,
            other_spdx_license_keys: vec![],
        }];
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("mit AND unknown-license");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert!(expr.contains("MIT"));
        assert!(expr.contains("LicenseRef-scancode-unknown-license"));
    }

    #[test]
    fn test_expression_to_spdx_preserves_operator_case() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        let result = mapping.expression_scancode_to_spdx("mit and gpl-2.0-plus or apache-2.0");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert!(expr.contains(" AND "));
        assert!(expr.contains(" OR "));
    }

    #[test]
    fn test_common_license_mappings() {
        let licenses = vec![
            License {
                key: "mit".to_string(),
                name: "MIT License".to_string(),
                spdx_license_key: Some("MIT".to_string()),
                category: Some("Permissive".to_string()),
                text: "MIT text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "apache-2.0".to_string(),
                name: "Apache 2.0".to_string(),
                spdx_license_key: Some("Apache-2.0".to_string()),
                category: Some("Permissive".to_string()),
                text: "Apache text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "bsd-3-clause".to_string(),
                name: "BSD 3-Clause".to_string(),
                spdx_license_key: Some("BSD-3-Clause".to_string()),
                category: Some("Permissive".to_string()),
                text: "BSD text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "isc".to_string(),
                name: "ISC License".to_string(),
                spdx_license_key: Some("ISC".to_string()),
                category: Some("Permissive".to_string()),
                text: "ISC text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
            License {
                key: "mpl-2.0".to_string(),
                name: "MPL 2.0".to_string(),
                spdx_license_key: Some("MPL-2.0".to_string()),
                category: Some("Copyleft".to_string()),
                text: "MPL text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
                other_spdx_license_keys: vec![],
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_to_spdx("mit"), Some("MIT".to_string()));
        assert_eq!(
            mapping.scancode_to_spdx("apache-2.0"),
            Some("Apache-2.0".to_string())
        );
        assert_eq!(
            mapping.scancode_to_spdx("bsd-3-clause"),
            Some("BSD-3-Clause".to_string())
        );
        assert_eq!(mapping.scancode_to_spdx("isc"), Some("ISC".to_string()));
        assert_eq!(
            mapping.scancode_to_spdx("mpl-2.0"),
            Some("MPL-2.0".to_string())
        );
    }
}
