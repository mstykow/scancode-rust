//! Tests for Autotools configure script parser.

use crate::models::PackageType;

use super::PackageParser;
use super::autotools::AutotoolsConfigureParser;
use std::path::PathBuf;

#[test]
fn test_is_match() {
    // Should match configure
    assert!(AutotoolsConfigureParser::is_match(&PathBuf::from(
        "configure"
    )));
    assert!(AutotoolsConfigureParser::is_match(&PathBuf::from(
        "/path/to/myproject/configure"
    )));

    // Should match configure.ac
    assert!(AutotoolsConfigureParser::is_match(&PathBuf::from(
        "configure.ac"
    )));
    assert!(AutotoolsConfigureParser::is_match(&PathBuf::from(
        "/path/to/myproject/configure.ac"
    )));

    // Should NOT match configure.in (deprecated legacy format)
    assert!(!AutotoolsConfigureParser::is_match(&PathBuf::from(
        "configure.in"
    )));

    // Should NOT match other files
    assert!(!AutotoolsConfigureParser::is_match(&PathBuf::from(
        "Makefile"
    )));
    assert!(!AutotoolsConfigureParser::is_match(&PathBuf::from(
        "Makefile.in"
    )));
    assert!(!AutotoolsConfigureParser::is_match(&PathBuf::from(
        "Makefile.am"
    )));
    assert!(!AutotoolsConfigureParser::is_match(&PathBuf::from(
        "config.h"
    )));
}

#[test]
fn test_parent_dir_name_extraction() {
    let path = PathBuf::from("testdata/autotools/myproject/configure");
    let package_data = AutotoolsConfigureParser::extract_first_package(&path);

    assert_eq!(package_data.package_type, Some(PackageType::Autotools));
    assert_eq!(package_data.name, Some("myproject".to_string()));
    assert_eq!(package_data.version, None);
    assert_eq!(package_data.homepage_url, None);
}

#[test]
fn test_configure_ac() {
    let path = PathBuf::from("testdata/autotools/another-project/configure.ac");
    let package_data = AutotoolsConfigureParser::extract_first_package(&path);

    assert_eq!(package_data.package_type, Some(PackageType::Autotools));
    assert_eq!(package_data.name, Some("another-project".to_string()));
}

#[test]
fn test_nested_path() {
    let path = PathBuf::from("/usr/local/src/my-awesome-project/configure");
    let package_data = AutotoolsConfigureParser::extract_first_package(&path);

    assert_eq!(package_data.package_type, Some(PackageType::Autotools));
    assert_eq!(package_data.name, Some("my-awesome-project".to_string()));
}

#[test]
fn test_root_path_edge_case() {
    // Edge case: configure at root level (no parent)
    let path = PathBuf::from("configure");
    let package_data = AutotoolsConfigureParser::extract_first_package(&path);

    assert_eq!(package_data.package_type, Some(PackageType::Autotools));
    // When at root level, parent() returns None, so name should be None
    assert_eq!(package_data.name, None);
}
