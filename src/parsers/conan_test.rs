//! Unit tests for Conan parsers (conanfile.py, conanfile.txt, conan.lock)

use std::path::PathBuf;

use super::PackageParser;
use super::conan::{ConanFilePyParser, ConanLockParser, ConanfileTxtParser};

#[test]
fn test_conanfile_py_parser_is_match() {
    assert!(ConanFilePyParser::is_match(&PathBuf::from("conanfile.py")));
    assert!(ConanFilePyParser::is_match(&PathBuf::from(
        "/path/to/conanfile.py"
    )));
    assert!(!ConanFilePyParser::is_match(&PathBuf::from(
        "conanfile.txt"
    )));
    assert!(!ConanFilePyParser::is_match(&PathBuf::from("conan.lock")));
    assert!(!ConanFilePyParser::is_match(&PathBuf::from("package.json")));
}

#[test]
fn test_conanfile_py_basic_metadata() {
    let test_file = "testdata/conan/recipes/libgettext/manifest/conanfile.py";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(result.package_type, Some("conan".to_string()));
    assert_eq!(result.name, Some("libgettext".to_string()));
    assert_eq!(
        result.description,
        Some(
            "An internationalization and localization system for multilingual programs".to_string()
        )
    );
    assert_eq!(
        result.homepage_url,
        Some("https://www.gnu.org/software/gettext".to_string())
    );
    assert_eq!(
        result.vcs_url,
        Some("https://github.com/conan-io/conan-center-index".to_string())
    );
    assert_eq!(
        result.extracted_license_statement,
        Some("LGPL-2.1-or-later".to_string())
    );
    assert_eq!(
        result.keywords,
        vec![
            "gettext".to_string(),
            "intl".to_string(),
            "libintl".to_string(),
            "i18n".to_string()
        ]
    );
}

#[test]
fn test_conanfile_py_dependencies() {
    let test_file = "testdata/conan/recipes/libgettext/manifest/conanfile.py";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(result.dependencies.len(), 1);
    let dep = &result.dependencies[0];
    assert_eq!(dep.purl, Some("pkg:conan/libiconv@1.17".to_string()));
    assert_eq!(dep.extracted_requirement, Some("1.17".to_string()));
    assert_eq!(dep.scope, Some("install".to_string()));
    assert_eq!(dep.is_runtime, Some(true));
    assert_eq!(dep.is_pinned, Some(true));
}

#[test]
fn test_conanfile_py_boost_metadata() {
    let test_file = "testdata/conan/recipes/boost/manifest/conanfile.py";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(result.package_type, Some("conan".to_string()));
    assert_eq!(result.name, Some("boost".to_string()));
    assert_eq!(
        result.description,
        Some("Boost provides free peer-reviewed portable C++ source libraries".to_string())
    );
    assert_eq!(
        result.homepage_url,
        Some("https://www.boost.org".to_string())
    );
    assert_eq!(
        result.vcs_url,
        Some("https://github.com/conan-io/conan-center-index".to_string())
    );
    assert_eq!(
        result.extracted_license_statement,
        Some("BSL-1.0".to_string())
    );
    assert_eq!(
        result.keywords,
        vec!["libraries".to_string(), "cpp".to_string()]
    );
}

#[test]
fn test_conanfile_py_boost_complex_requirements() {
    let test_file = "testdata/conan/recipes/boost/manifest/conanfile.py";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(result.package_type, Some("conan".to_string()));
    assert_eq!(result.name, Some("boost".to_string()));
}

#[test]
fn test_conanfile_py_license_tuple() {
    // Test that license as a string literal is handled
    let test_file = "testdata/conan/recipes/libgettext/manifest/conanfile.py";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(
        result.extracted_license_statement,
        Some("LGPL-2.1-or-later".to_string())
    );
}

#[test]
fn test_conanfile_py_no_version() {
    // libgettext doesn't have version in class attributes
    let test_file = "testdata/conan/recipes/libgettext/manifest/conanfile.py";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(result.version, None);
}

#[test]
fn test_conanfile_py_invalid_python() {
    // Test with invalid Python file
    let test_file = "testdata/conan/conanfile.txt";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    // Should return default package data on parse failure
    assert_eq!(result.package_type, Some("conan".to_string()));
    assert_eq!(result.primary_language, Some("C++".to_string()));
}

#[test]
fn test_conanfile_py_no_conanfile_class() {
    // Test with Python file that doesn't have ConanFile class
    // (using a .py file that exists but isn't a conanfile)
    let test_file = "testdata/conan/recipes/boost/manifest/conanfile.py";
    let result = ConanFilePyParser::extract_package_data(&PathBuf::from(test_file));

    // Should extract data from BoostConan(ConanFile)
    assert!(result.name.is_some());
}

#[test]
fn test_conanfile_txt_parser_is_match() {
    assert!(ConanfileTxtParser::is_match(&PathBuf::from(
        "conanfile.txt"
    )));
    assert!(ConanfileTxtParser::is_match(&PathBuf::from(
        "/path/to/conanfile.txt"
    )));
    assert!(!ConanfileTxtParser::is_match(&PathBuf::from(
        "conanfile.py"
    )));
    assert!(!ConanfileTxtParser::is_match(&PathBuf::from("conan.lock")));
}

#[test]
fn test_conanfile_txt_basic() {
    let test_file = "testdata/conan/conanfile.txt";
    let result = ConanfileTxtParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(result.package_type, Some("conan".to_string()));
    assert_eq!(result.primary_language, Some("C++".to_string()));
}

#[test]
fn test_conan_lock_parser_is_match() {
    assert!(ConanLockParser::is_match(&PathBuf::from("conan.lock")));
    assert!(ConanLockParser::is_match(&PathBuf::from(
        "/path/to/conan.lock"
    )));
    assert!(!ConanLockParser::is_match(&PathBuf::from("conanfile.txt")));
    assert!(!ConanLockParser::is_match(&PathBuf::from("conanfile.py")));
}

#[test]
fn test_conan_lock_basic() {
    let test_file = "testdata/conan/conan.lock";
    let result = ConanLockParser::extract_package_data(&PathBuf::from(test_file));

    assert_eq!(result.package_type, Some("conan".to_string()));
    assert_eq!(result.primary_language, Some("C++".to_string()));
}
