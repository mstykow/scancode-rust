//! Tests for SPDX license key mapping.

#[cfg(test)]
mod tests {
    use crate::license_detection::models::License;
    use crate::license_detection::spdx_mapping::*;

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
    fn test_scancode_to_spdx_unknown_key() {
        let licenses = create_test_licenses();
        let mapping = build_spdx_mapping(&licenses);

        assert_eq!(mapping.scancode_to_spdx("unknown-key"), None);
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

        assert!(mapping.scancode_to_spdx("mit").is_none());
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
