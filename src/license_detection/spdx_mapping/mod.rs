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
    expression_to_string, parse_expression, LicenseExpression,
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
    pub fn scancode_count(&self) -> usize {
        self.scancode_to_spdx.len()
    }

    /// Get the number of SPDX keys in the mapping.
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
pub fn expression_scancode_to_spdx(
    mapping: &SpdxMapping,
    scancode_expr: &str,
) -> Result<String, String> {
    mapping.expression_scancode_to_spdx(scancode_expr)
}

#[cfg(test)]
mod test;
