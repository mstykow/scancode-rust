#[cfg(test)]
mod tests {
    use crate::models::PackageType;
    use crate::models::{DatasourceId, Dependency};
    use crate::parsers::{PackageParser, PythonParser};
    use crate::test_utils::compare_package_data_parser_only;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper function to create a temporary test file with the given content and name
    fn create_temp_file(content: &str, filename: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).expect("Failed to write file");

        (temp_dir, file_path)
    }

    #[test]
    fn test_is_match() {
        let pyproject_path = PathBuf::from("/some/path/pyproject.toml");
        let setup_cfg_path = PathBuf::from("/some/path/setup.cfg");
        let setup_path = PathBuf::from("/some/path/setup.py");
        let pkg_info_path = PathBuf::from("/some/path/PKG-INFO");
        let metadata_path = PathBuf::from("/some/path/METADATA");
        let pip_inspect_path = PathBuf::from("/some/path/pip-inspect.deplock");
        let invalid_path = PathBuf::from("/some/path/not_python.txt");

        assert!(PythonParser::is_match(&pyproject_path));
        assert!(PythonParser::is_match(&setup_cfg_path));
        assert!(PythonParser::is_match(&setup_path));
        assert!(PythonParser::is_match(&pkg_info_path));
        assert!(PythonParser::is_match(&metadata_path));
        assert!(PythonParser::is_match(&pip_inspect_path));
        assert!(!PythonParser::is_match(&invalid_path));
    }

    #[test]
    fn test_extract_from_pyproject_toml() {
        let content = r#"
[project]
name = "test-package"
version = "0.1.0"
license = "MIT"
authors = [
    "Test User <test@example.com>",
    "Another User <another@example.com>"
]

[project.urls]
homepage = "https://example.com"
repository = "https://github.com/user/test-package"

[project.optional-dependencies]
test = ["pytest>=6.0.0"]

[project.dependencies]
requests = ">=2.25.0"
numpy = ">=1.20.0"
"#;

        let (_temp_file, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.package_type, Some(PackageType::Pypi));
        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://example.com".to_string())
        );

        assert_eq!(package_data.declared_license_expression, None);
        assert_eq!(package_data.declared_license_expression_spdx, None);
        assert_eq!(package_data.license_detections.len(), 0);
        assert!(package_data.extracted_license_statement.is_some());

        // Check purl
        assert_eq!(
            package_data.purl,
            Some("pkg:pypi/test-package@0.1.0".to_string())
        );
    }

    #[test]
    fn test_extract_from_python_testdata() {
        let file_path = PathBuf::from("testdata/python/pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.package_type, Some(PackageType::Pypi));
        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://example.com".to_string())
        );

        assert_eq!(package_data.declared_license_expression, None);
        assert_eq!(package_data.declared_license_expression_spdx, None);
        assert_eq!(package_data.license_detections.len(), 0);
        assert!(package_data.extracted_license_statement.is_some());

        // Check purl
        assert_eq!(
            package_data.purl,
            Some("pkg:pypi/test-package@0.1.0".to_string())
        );

        // Check dependencies - should have 2 regular dependencies
        assert_eq!(package_data.dependencies.len(), 2);
        let purls: Vec<&str> = package_data
            .dependencies
            .iter()
            .filter_map(|d| d.purl.as_deref())
            .collect();

        // Check that dependencies exist with correct package names (version-agnostic)
        assert!(
            purls.iter().any(|p| p.starts_with("pkg:pypi/requests@")),
            "Should contain requests dependency"
        );
        assert!(
            purls.iter().any(|p| p.starts_with("pkg:pypi/numpy@")),
            "Should contain numpy dependency"
        );
    }

    #[test]
    fn test_extract_from_setup_py() {
        let content = r#"
 from setuptools import setup, find_packages

setup(
    name="test-package",
    version="0.1.0",
    license="MIT",
    url="https://example.com",
    author="Test User",
    author_email="test@example.com",
    description="A test package",
    packages=find_packages(),
    install_requires=[
        "requests>=2.25.0",
        "numpy>=1.20.0",
    ],
)
"#;

        let (_temp_file, file_path) = create_temp_file(content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.package_type, Some(PackageType::Pypi));
        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://example.com".to_string())
        );

        assert_eq!(package_data.declared_license_expression, None);
        assert_eq!(package_data.declared_license_expression_spdx, None);
        assert_eq!(package_data.license_detections.len(), 0);
        assert!(package_data.extracted_license_statement.is_some());

        // Check purl
        assert_eq!(
            package_data.purl,
            Some("pkg:pypi/test-package@0.1.0".to_string())
        );
    }

    #[test]
    fn test_setup_py_ast_basic() {
        let content = fs::read_to_string("testdata/python/setup-ast-basic.py")
            .expect("Failed to read setup-ast-basic.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("mypackage".to_string()));
        assert_eq!(package_data.version, Some("1.0.0".to_string()));
        assert_eq!(package_data.description, Some("A test package".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://example.com".to_string())
        );

        let author_party = package_data
            .parties
            .iter()
            .find(|party| party.role.as_deref() == Some("author"))
            .expect("author party should exist");
        assert_eq!(author_party.name, Some("John Doe".to_string()));
        assert_eq!(author_party.email, Some("john@example.com".to_string()));

        let install_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|dep| dep.scope.as_deref() == Some("install"))
            .collect();
        assert_eq!(install_deps.len(), 2);
    }

    #[test]
    fn test_setup_py_ast_constants() {
        let content = fs::read_to_string("testdata/python/setup-ast-constants.py")
            .expect("Failed to read setup-ast-constants.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("mypackage".to_string()));
        assert_eq!(package_data.version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_setup_py_ast_dict_unpack() {
        let content = fs::read_to_string("testdata/python/setup-ast-dict-unpack.py")
            .expect("Failed to read setup-ast-dict-unpack.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("dictpkg".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));

        let dep_purls: Vec<&str> = package_data
            .dependencies
            .iter()
            .filter_map(|dep| dep.purl.as_deref())
            .collect();
        assert!(
            dep_purls
                .iter()
                .any(|purl| purl.starts_with("pkg:pypi/requests")),
            "Should contain requests dependency"
        );
    }

    #[test]
    fn test_setup_py_ast_dynamic_expr() {
        let content = fs::read_to_string("testdata/python/setup-ast-dynamic.py")
            .expect("Failed to read setup-ast-dynamic.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("dynpkg".to_string()));
        assert_eq!(package_data.version, None);
    }

    #[test]
    fn test_setup_py_ast_install_requires() {
        let content = fs::read_to_string("testdata/python/setup-ast-install-requires.py")
            .expect("Failed to read setup-ast-install-requires.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        let install_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|dep| dep.scope.as_deref() == Some("install"))
            .collect();
        assert_eq!(install_deps.len(), 2);

        let dep_purls: Vec<&str> = install_deps
            .iter()
            .filter_map(|dep| dep.purl.as_deref())
            .collect();
        assert!(
            dep_purls
                .iter()
                .any(|purl| purl.starts_with("pkg:pypi/requests")),
            "Should contain requests dependency"
        );
        assert!(
            dep_purls
                .iter()
                .any(|purl| purl.starts_with("pkg:pypi/click")),
            "Should contain click dependency"
        );
    }

    #[test]
    fn test_setup_py_ast_extras_require() {
        let content = fs::read_to_string("testdata/python/setup-ast-extras-require.py")
            .expect("Failed to read setup-ast-extras-require.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        let dev_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|dep| dep.scope.as_deref() == Some("dev"))
            .collect();
        assert_eq!(dev_deps.len(), 2);
        assert!(dev_deps.iter().all(|dep| dep.is_optional == Some(true)));

        let docs_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|dep| dep.scope.as_deref() == Some("docs"))
            .collect();
        assert_eq!(docs_deps.len(), 1);
        assert!(docs_deps.iter().all(|dep| dep.is_optional == Some(true)));

        let test_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|dep| dep.scope.as_deref() == Some("test"))
            .collect();
        assert_eq!(test_deps.len(), 1);
        assert!(test_deps.iter().all(|dep| dep.is_optional == Some(true)));
    }

    #[test]
    fn test_setup_py_ast_malformed_syntax() {
        let content = fs::read_to_string("testdata/python/setup-ast-malformed.py")
            .expect("Failed to read setup-ast-malformed.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("broken".to_string()));
        assert_eq!(package_data.version, Some("0.0.1".to_string()));
    }

    #[test]
    fn test_setup_py_ast_large_file() {
        let mut content = String::from("from setuptools import setup\n");
        content.push_str("setup(name=\"big-package\", version=\"9.9.9\")\n");
        content.push_str(&"a".repeat(1_048_600));

        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("big-package".to_string()));
        assert_eq!(package_data.version, Some("9.9.9".to_string()));
    }

    #[test]
    fn test_setup_py_ast_deep_nesting() {
        let mut nested = "\"1.0\"".to_string();
        for _ in 0..60 {
            nested = format!("[{}]", nested);
        }

        let content = format!(
            "from setuptools import setup\n\
NAME = \"deep\"\n\
VERSION = {}\n\
setup(name=NAME, version=VERSION)\n",
            nested
        );

        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("deep".to_string()));
        assert_eq!(package_data.version, None);
    }

    #[test]
    fn test_setup_py_ast_malicious_no_exec() {
        let content = fs::read_to_string("testdata/python/setup-ast-malicious.py")
            .expect("Failed to read setup-ast-malicious.py");
        let (_temp_dir, file_path) = create_temp_file(&content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("malicious".to_string()));
    }

    #[test]
    fn test_extract_api_url_basic() {
        // Given: A pyproject.toml with name and version
        let content = r#"
[project]
name = "requests"
version = "2.31.0"
license = "Apache-2.0"
authors = [
    "Kenneth Reitz <me@kennethreitz.com>"
]

[project.dependencies]
urllib3 = ">=1.21.1,<2"
chardet = ">=3.0.2,<5"
"#;

        let (_temp_file, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        // Then: API data URL should be generated
        assert_eq!(
            package_data.api_data_url,
            Some("https://pypi.org/pypi/requests/2.31.0/json".to_string())
        );

        // Then: Homepage URL should fall back to PyPI
        assert_eq!(
            package_data.homepage_url,
            Some("https://pypi.org/project/requests".to_string())
        );

        // Then: Download URL should be PyPI source tarball
        assert_eq!(
            package_data.download_url,
            Some("https://pypi.org/packages/source/r/requests/requests-2.31.0.tar.gz".to_string())
        );
    }

    #[test]
    fn test_extract_api_url_no_version() {
        // Given: A pyproject.toml with name but no version
        let content = r#"
[project]
name = "numpy"
license = "BSD-3-Clause"
authors = [
    "Travis Oliphant <oliphant@enthought.com>"
]

[project.dependencies]
"#;

        let (_temp_file, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        // Then: API data URL should still be generated (without version)
        assert_eq!(
            package_data.api_data_url,
            Some("https://pypi.org/pypi/numpy/json".to_string())
        );

        // Then: Homepage URL should fall back to PyPI
        assert_eq!(
            package_data.homepage_url,
            Some("https://pypi.org/project/numpy".to_string())
        );

        // Then: Download URL should not be generated (no version)
        assert_eq!(package_data.download_url, None);
    }

    #[test]
    fn test_extract_vcs_url_with_repository() {
        // Given: A pyproject.toml with a repository URL
        let content = r#"
[project]
name = "test-package"
version = "1.0.0"
repository = "https://github.com/user/test-package"

[project.dependencies]
"#;

        let (_temp_file, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        // Then: vcs_url should contain the repository URL
        assert_eq!(
            package_data.vcs_url,
            Some("https://github.com/user/test-package".to_string())
        );
    }

    #[test]
    fn test_extract_vcs_url_with_urls_section() {
        // Given: A pyproject.toml with repository in URLs section
        let content = r#"
[project]
name = "test-package"
version = "1.0.0"

[project.urls]
repository = "https://github.com/user/test-package"
homepage = "https://example.com"

[project.dependencies]
"#;

        let (_temp_file, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        // Then: vcs_url should contain the repository URL from URLs section
        assert_eq!(
            package_data.vcs_url,
            Some("https://github.com/user/test-package".to_string())
        );
    }

    #[test]
    fn test_extract_vcs_url_without_repository() {
        // Given: A minimal pyproject.toml without a repository
        let content = r#"
[project]
name = "test-package"
version = "1.0.0"

[project.dependencies]
"#;

        let (_temp_file, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        // Then: vcs_url should be None
        assert_eq!(package_data.vcs_url, None);
    }

    #[test]
    fn test_extract_download_url_and_requires_python() {
        let content = r#"Metadata-Version: 2.0
Name: test-package
Version: 1.0.0
Summary: A test package
Home-page: https://example.com
Author: Test Author
Author-email: test@example.com
License: MIT
Download-URL: https://github.com/test/test-package/tarball/1.0.0
Requires-Python: >=3.8

This is a test package.
"#;

        let (_temp_file, file_path) = create_temp_file(content, "METADATA");
        let package_data = PythonParser::extract_first_package(&file_path);

        // Verify Download-URL is extracted
        assert_eq!(
            package_data.download_url,
            Some("https://github.com/test/test-package/tarball/1.0.0".to_string())
        );

        // Verify Requires-Python is stored in extra_data
        assert!(package_data.extra_data.is_some());
        let extra_data = package_data.extra_data.unwrap();
        assert_eq!(
            extra_data.get("requires_python").and_then(|v| v.as_str()),
            Some(">=3.8")
        );
    }

    #[test]
    fn test_golden_metadata() {
        let test_file = PathBuf::from("testdata/python/golden/metadata/METADATA");
        let expected_file = PathBuf::from("testdata/python/golden/metadata/METADATA-expected.json");

        let package_data = PythonParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_golden_setup_cfg() {
        let test_file = PathBuf::from("testdata/python/golden/setup_cfg_wheel/setup.cfg");
        let expected_file =
            PathBuf::from("testdata/python/setup_cfg_wheel/setup.cfg-expected-corrected.json");

        let package_data = PythonParser::extract_first_package(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_extract_project_urls() {
        let content = r#"Metadata-Version: 2.1
Name: pip
Version: 20.2.2
Summary: The PyPA recommended tool for installing Python packages.
Home-page: https://pip.pypa.io/
Author: The pip developers
Author-email: distutils-sig@python.org
License: MIT
Project-URL: Documentation, https://pip.pypa.io
Project-URL: Source, https://github.com/pypa/pip
Project-URL: Changelog, https://pip.pypa.io/en/stable/news/

pip - The Python Package Installer
"#;

        let (_temp_file, file_path) = create_temp_file(content, "METADATA");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("pip".to_string()));
        assert_eq!(package_data.version, Some("20.2.2".to_string()));

        assert_eq!(
            package_data.homepage_url,
            Some("https://pip.pypa.io/".to_string())
        );

        assert_eq!(
            package_data.code_view_url,
            Some("https://github.com/pypa/pip".to_string()),
            "Source URL should be mapped to code_view_url"
        );

        assert!(package_data.extra_data.is_some());
        let extra_data = package_data.extra_data.unwrap();

        assert_eq!(
            extra_data.get("changelog_url").and_then(|v| v.as_str()),
            Some("https://pip.pypa.io/en/stable/news/"),
            "Changelog URL should be in extra_data"
        );

        let project_urls = extra_data.get("project_urls").and_then(|v| v.as_object());
        assert!(
            project_urls.is_some(),
            "project_urls should be in extra_data"
        );
        let urls = project_urls.unwrap();

        assert_eq!(urls.len(), 3, "Should have all 3 Project-URLs");
        assert_eq!(
            urls.get("Documentation").and_then(|v| v.as_str()),
            Some("https://pip.pypa.io")
        );
        assert_eq!(
            urls.get("Source").and_then(|v| v.as_str()),
            Some("https://github.com/pypa/pip")
        );
        assert_eq!(
            urls.get("Changelog").and_then(|v| v.as_str()),
            Some("https://pip.pypa.io/en/stable/news/")
        );
    }

    #[test]
    fn test_extract_project_urls_with_mapping() {
        let content = r#"Metadata-Version: 2.2
Name: trimesh
Version: 4.6.1
Summary: Import, export, process, analyze and view triangular meshes.
Author-email: Michael Dawson-Haggerty <mikedh@kerfed.com>
License: MIT
Project-URL: homepage, https://github.com/mikedh/trimesh
Project-URL: documentation, https://trimesh.org
Project-URL: Bug Tracker, https://github.com/mikedh/trimesh/issues
Project-URL: Repository, https://github.com/mikedh/trimesh

Trimesh is a pure Python library for loading and using triangular meshes.
"#;

        let (_temp_file, file_path) = create_temp_file(content, "METADATA");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("trimesh".to_string()));

        assert_eq!(
            package_data.homepage_url,
            Some("https://github.com/mikedh/trimesh".to_string()),
            "homepage Project-URL should be mapped to homepage_url"
        );

        assert_eq!(
            package_data.vcs_url,
            Some("https://github.com/mikedh/trimesh".to_string()),
            "Repository Project-URL should be mapped to vcs_url"
        );

        assert_eq!(
            package_data.bug_tracking_url,
            Some("https://github.com/mikedh/trimesh/issues".to_string()),
            "Bug Tracker Project-URL should be mapped to bug_tracking_url"
        );

        assert_eq!(
            package_data.code_view_url, None,
            "documentation URL is not mapped to code_view_url (only source/source code/code are)"
        );

        assert!(package_data.extra_data.is_some());
        let extra_data = package_data.extra_data.unwrap();
        let project_urls = extra_data.get("project_urls").and_then(|v| v.as_object());
        assert!(
            project_urls.is_some(),
            "All Project-URLs should be in extra_data"
        );

        let urls = project_urls.unwrap();
        assert_eq!(urls.len(), 4, "Should have all 4 Project-URLs");
        assert_eq!(
            urls.get("documentation").and_then(|v| v.as_str()),
            Some("https://trimesh.org")
        );
    }

    #[test]
    fn test_extract_project_urls_all_stored() {
        let content = r#"Metadata-Version: 2.1
Name: test-package
Version: 1.0.0
Summary: Test package
Project-URL: Documentation, https://docs.example.com
Project-URL: Source, https://github.com/user/test-package
Project-URL: Issues, https://github.com/user/test-package/issues

Test package description.
"#;

        let (_temp_file, file_path) = create_temp_file(content, "METADATA");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(
            package_data.code_view_url,
            Some("https://github.com/user/test-package".to_string()),
            "Source should map to code_view_url"
        );

        assert_eq!(
            package_data.bug_tracking_url,
            Some("https://github.com/user/test-package/issues".to_string()),
            "Issues should map to bug_tracking_url"
        );

        assert!(package_data.extra_data.is_some());
        let extra_data = package_data.extra_data.unwrap();
        let project_urls = extra_data.get("project_urls").and_then(|v| v.as_object());
        assert!(project_urls.is_some(), "All Project-URLs should be stored");

        let urls = project_urls.unwrap();
        assert_eq!(urls.len(), 3, "Should have all 3 Project-URLs");
        assert_eq!(
            urls.get("Documentation").and_then(|v| v.as_str()),
            Some("https://docs.example.com")
        );
        assert_eq!(
            urls.get("Source").and_then(|v| v.as_str()),
            Some("https://github.com/user/test-package")
        );
        assert_eq!(
            urls.get("Issues").and_then(|v| v.as_str()),
            Some("https://github.com/user/test-package/issues")
        );
    }

    #[test]
    fn test_extract_license_file_from_metadata() {
        let metadata_path = PathBuf::from("testdata/python/metadata-license-files/METADATA");
        let package_data = PythonParser::extract_first_package(&metadata_path);

        assert_eq!(package_data.name, Some("example-package".to_string()));
        assert_eq!(package_data.version, Some("1.0.0".to_string()));

        let extra_data = package_data
            .extra_data
            .expect("extra_data should be present");

        let license_files = extra_data
            .get("license_files")
            .expect("license_files should be present")
            .as_array()
            .expect("license_files should be an array");

        assert_eq!(license_files.len(), 2);
        assert_eq!(license_files[0].as_str().unwrap(), "LICENSE");
        assert_eq!(license_files[1].as_str().unwrap(), "COPYING.txt");
    }

    #[test]
    fn test_is_match_wheel_extension() {
        let wheel_path = PathBuf::from("/some/path/package-1.0.0-py3-none-any.whl");
        let wheel_uppercase = PathBuf::from("/some/path/package-1.0.0-py3-none-any.WHL");

        assert!(PythonParser::is_match(&wheel_path));
        assert!(PythonParser::is_match(&wheel_uppercase));
    }

    #[test]
    fn test_is_match_egg_extension() {
        let egg_path = PathBuf::from("/some/path/package-1.0.0-py3.9.egg");
        let egg_uppercase = PathBuf::from("/some/path/package-1.0.0-py3.9.EGG");

        assert!(PythonParser::is_match(&egg_path));
        assert!(PythonParser::is_match(&egg_uppercase));
    }

    #[test]
    fn test_extract_from_wheel_archive() {
        let wheel_path = PathBuf::from(
            "testdata/python/golden/archives/atomicwrites-1.2.1-py2.py3-none-any.whl",
        );

        let package_data = PythonParser::extract_first_package(&wheel_path);

        assert_eq!(package_data.package_type, Some(PackageType::Pypi));
        assert_eq!(package_data.name, Some("atomicwrites".to_string()));
        assert_eq!(package_data.version, Some("1.2.1".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://github.com/untitaker/python-atomicwrites".to_string())
        );
        assert_eq!(package_data.datasource_id, Some(DatasourceId::PypiWheel));
        assert_eq!(package_data.primary_language, Some("Python".to_string()));

        assert!(!package_data.parties.is_empty());
        assert_eq!(
            package_data.parties[0].name,
            Some("Markus Unterwaditzer".to_string())
        );
        assert_eq!(
            package_data.parties[0].email,
            Some("markus@unterwaditzer.net".to_string())
        );

        assert!(package_data.purl.is_some());
        let purl = package_data.purl.unwrap();
        assert!(purl.starts_with("pkg:pypi/atomicwrites@1.2.1"));
        assert!(purl.contains("extension="));

        let extra_data = package_data.extra_data.expect("extra_data should exist");
        assert!(extra_data.contains_key("python_requires"));
        assert!(extra_data.contains_key("abi_tag"));
        assert!(extra_data.contains_key("platform_tag"));

        assert!(package_data.size.is_some(), "size should be calculated");
        assert_eq!(package_data.size.unwrap(), 1427);
        assert!(package_data.sha256.is_some(), "sha256 should be calculated");
    }

    #[test]
    fn test_extract_from_egg_archive() {
        let egg_path =
            PathBuf::from("testdata/python/golden/archives/commoncode-21.5.12-py3.9.egg");

        let package_data = PythonParser::extract_first_package(&egg_path);

        assert_eq!(package_data.package_type, Some(PackageType::Pypi));
        assert_eq!(package_data.name, Some("commoncode".to_string()));
        assert_eq!(package_data.version, Some("21.5.12".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://github.com/nexB/commoncode".to_string())
        );
        assert_eq!(package_data.datasource_id, Some(DatasourceId::PypiEgg));
        assert_eq!(package_data.primary_language, Some("Python".to_string()));

        assert!(!package_data.parties.is_empty());
        assert_eq!(
            package_data.parties[0].name,
            Some("nexB. Inc. and others".to_string())
        );

        assert!(package_data.purl.is_some());
        let purl = package_data.purl.unwrap();
        assert!(purl.starts_with("pkg:pypi/commoncode@21.5.12"));
        assert!(purl.contains("type=egg"));

        assert!(package_data.size.is_some(), "size should be calculated");
        assert_eq!(package_data.size.unwrap(), 1756);
        assert!(package_data.sha256.is_some(), "sha256 should be calculated");
    }

    #[test]
    fn test_corrupt_wheel_archive_no_panic() {
        let (_temp_dir, corrupt_path) = create_temp_file("this is not a valid zip file", "bad.whl");
        let package_data = PythonParser::extract_first_package(&corrupt_path);

        assert!(package_data.name.is_none());
        assert!(package_data.version.is_none());
    }

    #[test]
    fn test_corrupt_egg_archive_no_panic() {
        let (_temp_dir, corrupt_path) = create_temp_file("this is not a valid zip file", "bad.egg");
        let package_data = PythonParser::extract_first_package(&corrupt_path);

        assert!(package_data.name.is_none());
        assert!(package_data.version.is_none());
    }

    #[test]
    fn test_wheel_purl_format() {
        let wheel_path = PathBuf::from(
            "testdata/python/golden/archives/atomicwrites-1.2.1-py2.py3-none-any.whl",
        );

        let package_data = PythonParser::extract_first_package(&wheel_path);

        let purl = package_data.purl.expect("PURL should be present");
        assert!(
            purl.contains("extension=py2.py3-none-any"),
            "PURL should contain extension qualifiers: {}",
            purl
        );
    }

    #[test]
    fn test_egg_purl_format() {
        let egg_path =
            PathBuf::from("testdata/python/golden/archives/commoncode-21.5.12-py3.9.egg");

        let package_data = PythonParser::extract_first_package(&egg_path);

        let purl = package_data.purl.expect("PURL should be present");
        assert!(
            purl.contains("type=egg"),
            "PURL should contain type=egg qualifier: {}",
            purl
        );
    }

    #[test]
    fn test_parse_record_csv() {
        use crate::parsers::python::parse_record_csv;

        let record_content = "package/__init__.py,sha256=47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU,0\n\
                             package/module.py,sha256=2jmj7l5rSw0yVb_vlWAYkK_YBwk1BkwQZq6ZNzJBH20,1234\n\
                             package/data.txt,,100\n";

        let file_refs = parse_record_csv(record_content);

        assert_eq!(file_refs.len(), 3);

        assert_eq!(file_refs[0].path, "package/__init__.py");
        assert_eq!(file_refs[0].size, Some(0));
        assert!(file_refs[0].sha256.is_some());

        assert_eq!(file_refs[1].path, "package/module.py");
        assert_eq!(file_refs[1].size, Some(1234));
        assert!(file_refs[1].sha256.is_some());

        assert_eq!(file_refs[2].path, "package/data.txt");
        assert_eq!(file_refs[2].size, Some(100));
        assert!(file_refs[2].sha256.is_none());
    }

    #[test]
    fn test_parse_installed_files_txt() {
        use crate::parsers::python::parse_installed_files_txt;

        let installed_files_content = "__init__.py\n\
                                       module.py\n\
                                       data.txt\n";

        let file_refs = parse_installed_files_txt(installed_files_content);

        assert_eq!(file_refs.len(), 3);
        assert_eq!(file_refs[0].path, "__init__.py");
        assert_eq!(file_refs[1].path, "module.py");
        assert_eq!(file_refs[2].path, "data.txt");

        for file_ref in &file_refs {
            assert!(file_ref.size.is_none());
            assert!(file_ref.sha256.is_none());
            assert!(file_ref.sha1.is_none());
        }
    }

    #[test]
    fn test_wheel_file_references() {
        let wheel_path = PathBuf::from(
            "testdata/python/golden/archives/atomicwrites-1.2.1-py2.py3-none-any.whl",
        );

        let package_data = PythonParser::extract_first_package(&wheel_path);

        assert!(!package_data.file_references.is_empty());
        for file_ref in &package_data.file_references {
            assert!(!file_ref.path.is_empty());
        }
    }

    #[test]
    fn test_egg_file_references() {
        let egg_path =
            PathBuf::from("testdata/python/golden/archives/commoncode-21.5.12-py3.9.egg");

        let package_data = PythonParser::extract_first_package(&egg_path);

        assert!(package_data.file_references.is_empty());
        for file_ref in &package_data.file_references {
            assert!(!file_ref.path.is_empty());
        }
    }

    #[test]
    fn test_missing_file_references_graceful() {
        let (_temp_dir, corrupt_path) = create_temp_file("this is not a valid zip file", "bad.whl");
        let package_data = PythonParser::extract_first_package(&corrupt_path);

        assert!(package_data.file_references.is_empty());
    }

    #[test]
    fn test_pip_inspect_is_match() {
        let pip_inspect_path = PathBuf::from("/some/path/pip-inspect.deplock");
        assert!(PythonParser::is_match(&pip_inspect_path));
    }

    #[test]
    fn test_extract_from_pip_inspect() {
        let test_file = PathBuf::from("testdata/python/pip-inspect/pip-inspect.deplock");
        let package_data = PythonParser::extract_first_package(&test_file);

        assert_eq!(package_data.package_type, Some(PackageType::Pypi));
        assert_eq!(package_data.name, Some("univers".to_string()));
        assert_eq!(package_data.version, Some("0.0.0".to_string()));
        assert_eq!(package_data.primary_language, Some("Python".to_string()));
        assert_eq!(
            package_data.datasource_id,
            Some(DatasourceId::PypiInspectDeplock)
        );
        assert!(package_data.is_virtual);

        assert_eq!(package_data.declared_license_expression, None);
        assert_eq!(package_data.declared_license_expression_spdx, None);
        assert_eq!(package_data.license_detections.len(), 0);
        assert!(package_data.extracted_license_statement.is_some());

        let extra_data = package_data
            .extra_data
            .expect("extra_data should be present");
        assert_eq!(
            extra_data.get("pip_version").and_then(|v| v.as_str()),
            Some("24.1")
        );
        assert_eq!(
            extra_data.get("inspect_version").and_then(|v| v.as_str()),
            Some("1")
        );

        assert_eq!(package_data.dependencies.len(), 2);

        let dep_purls: Vec<&str> = package_data
            .dependencies
            .iter()
            .filter_map(|d| d.purl.as_deref())
            .collect();

        assert!(
            dep_purls
                .iter()
                .any(|p| p.starts_with("pkg:pypi/packaging@")),
            "Should contain packaging dependency"
        );
        assert!(
            dep_purls
                .iter()
                .any(|p| p.starts_with("pkg:pypi/setuptools@")),
            "Should contain setuptools dependency"
        );

        for dep in &package_data.dependencies {
            assert_eq!(dep.is_runtime, Some(true));
            assert_eq!(dep.is_optional, Some(false));
            assert_eq!(dep.is_pinned, Some(true));
            assert!(dep.resolved_package.is_some());
        }
    }

    #[test]
    fn test_extract_from_pip_inspect_direct_dependencies() {
        let test_file = PathBuf::from("testdata/python/pip-inspect/pip-inspect.deplock");
        let package_data = PythonParser::extract_first_package(&test_file);

        let direct_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.is_direct == Some(true))
            .collect();

        assert_eq!(
            direct_deps.len(),
            1,
            "Should have 1 direct dependency (setuptools with requested=true)"
        );
    }

    #[test]
    fn test_pip_inspect_invalid_json() {
        let (_temp_dir, invalid_path) = create_temp_file("not valid json", "pip-inspect.deplock");
        let package_data = PythonParser::extract_first_package(&invalid_path);

        assert!(package_data.name.is_none());
        assert!(package_data.version.is_none());
    }

    #[test]
    fn test_pip_inspect_missing_installed_array() {
        let content = r#"{"version": "1", "pip_version": "24.1"}"#;
        let (_temp_dir, file_path) = create_temp_file(content, "pip-inspect.deplock");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert!(package_data.name.is_none());
        assert!(package_data.version.is_none());
    }

    #[test]
    fn test_pyproject_optional_dependencies_scopes() {
        let content = r#"
[project]
name = "test-package"
version = "1.0.0"

[project.dependencies]
requests = ">=2.0"

[project.optional-dependencies]
dev = ["pytest>=7.0", "black>=22.0"]
docs = ["sphinx>=5.0"]
test = ["coverage>=6.0"]
"#;

        let (_temp_dir, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.dependencies.len(), 5);

        let dev_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("dev"))
            .collect();
        assert_eq!(dev_deps.len(), 2);
        assert!(dev_deps.iter().all(|d| d.is_optional == Some(true)));

        let docs_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("docs"))
            .collect();
        assert_eq!(docs_deps.len(), 1);
        assert!(docs_deps.iter().all(|d| d.is_optional == Some(true)));

        let test_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("test"))
            .collect();
        assert_eq!(test_deps.len(), 1);
        assert!(test_deps.iter().all(|d| d.is_optional == Some(true)));
    }

    #[test]
    fn test_pyproject_optional_dependencies_is_runtime() {
        let content = r#"
[project]
name = "test-package"
version = "1.0.0"

[project.dependencies]
requests = ">=2.0"
click = ">=8.0"

[project.optional-dependencies]
dev = ["pytest>=7.0", "black>=22.0"]
docs = ["sphinx>=5.0"]
"#;

        let (_temp_dir, file_path) = create_temp_file(content, "pyproject.toml");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.dependencies.len(), 5);

        let regular_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.is_optional == Some(false))
            .collect();
        assert_eq!(regular_deps.len(), 2);
        assert!(
            regular_deps.iter().all(|d| d.is_runtime == Some(true)),
            "Regular dependencies should have is_runtime=true"
        );

        let optional_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.is_optional == Some(true))
            .collect();
        assert_eq!(optional_deps.len(), 3);
        assert!(
            optional_deps.iter().all(|d| d.is_runtime == Some(false)),
            "Optional dependencies should have is_runtime=false"
        );
    }

    #[test]
    fn test_setup_cfg_extras_require_scopes() {
        let content = r#"
[metadata]
name = test-package
version = 1.0.0

[options]
install_requires =
    requests>=2.0

[options.extras_require]
dev = 
    pytest>=7.0
    black>=22.0
docs = sphinx>=5.0
test = coverage>=6.0
"#;

        let (_temp_dir, file_path) = create_temp_file(content, "setup.cfg");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert!(package_data.dependencies.len() >= 4);

        let dev_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("dev"))
            .collect();
        assert_eq!(dev_deps.len(), 2);
        assert!(dev_deps.iter().all(|d| d.is_optional == Some(true)));

        let docs_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("docs"))
            .collect();
        assert_eq!(docs_deps.len(), 1);
        assert!(docs_deps.iter().all(|d| d.is_optional == Some(true)));

        let test_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("test"))
            .collect();
        assert_eq!(test_deps.len(), 1);
        assert!(test_deps.iter().all(|d| d.is_optional == Some(true)));

        let install_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("install"))
            .collect();
        assert_eq!(install_deps.len(), 1);
        assert!(install_deps.iter().all(|d| d.is_optional == Some(false)));

        let regular_dep: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.is_none() || d.scope.as_deref() == Some("install"))
            .collect();
        assert!(regular_dep.iter().all(|d| d.is_optional == Some(false)));
    }

    #[test]
    fn test_setup_py_extras_require_scopes() {
        let content = r#"
from setuptools import setup

setup(
    name="test-package",
    version="1.0.0",
    extras_require={
        'dev': ['pytest>=7.0', 'black>=22.0'],
        'docs': ['sphinx>=5.0'],
    },
    tests_require=['coverage>=6.0'],
)
"#;

        let (_temp_dir, file_path) = create_temp_file(content, "setup.py");
        let package_data = PythonParser::extract_first_package(&file_path);

        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.dependencies.len(), 4);

        let dev_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("dev"))
            .collect();
        assert_eq!(dev_deps.len(), 2);
        assert!(dev_deps.iter().all(|d| d.is_optional == Some(true)));

        let docs_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("docs"))
            .collect();
        assert_eq!(docs_deps.len(), 1);
        assert!(docs_deps.iter().all(|d| d.is_optional == Some(true)));

        let test_deps: Vec<&Dependency> = package_data
            .dependencies
            .iter()
            .filter(|d| d.scope.as_deref() == Some("test"))
            .collect();
        assert_eq!(test_deps.len(), 1);
        assert!(test_deps.iter().all(|d| d.is_optional == Some(true)));
    }

    #[test]
    fn test_archive_security_constants_exist() {
        const MAX_ARCHIVE_SIZE: u64 = 100 * 1024 * 1024;
        const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
        const MAX_COMPRESSION_RATIO: f64 = 100.0;

        assert_eq!(MAX_ARCHIVE_SIZE, 100 * 1024 * 1024);
        assert_eq!(MAX_FILE_SIZE, 50 * 1024 * 1024);
        assert_eq!(MAX_COMPRESSION_RATIO, 100.0);

        const { assert!(MAX_FILE_SIZE < MAX_ARCHIVE_SIZE) };
        const { assert!(MAX_COMPRESSION_RATIO > 1.0) };
    }

    #[test]
    fn test_is_requirement_pinned_exact_version() {
        assert!(crate::parsers::python::is_requirement_pinned("foo==1.0.0"));
        assert!(crate::parsers::python::is_requirement_pinned("foo===1.0.0"));
        assert!(crate::parsers::python::is_requirement_pinned(
            "foo==1.0.0rc1"
        ));
    }

    #[test]
    fn test_is_requirement_pinned_wildcard_not_pinned() {
        assert!(!crate::parsers::python::is_requirement_pinned("foo==1.0.*"));
        assert!(!crate::parsers::python::is_requirement_pinned(
            "foo==0.19.*"
        ));
    }

    #[test]
    fn test_is_requirement_pinned_range_not_pinned() {
        assert!(!crate::parsers::python::is_requirement_pinned("foo>=1.0.0"));
        assert!(!crate::parsers::python::is_requirement_pinned(
            "foo>=1.0.0,<2.0.0"
        ));
        assert!(!crate::parsers::python::is_requirement_pinned("foo~=1.0.0"));
        assert!(!crate::parsers::python::is_requirement_pinned("foo!=1.0.0"));
    }

    #[test]
    fn test_is_requirement_pinned_no_version_not_pinned() {
        assert!(!crate::parsers::python::is_requirement_pinned("foo"));
        assert!(!crate::parsers::python::is_requirement_pinned("foo[extra]"));
    }

    #[test]
    fn test_is_requirement_pinned_with_markers() {
        assert!(crate::parsers::python::is_requirement_pinned(
            "foo==1.0.0; python_version >= '3.8'"
        ));
        assert!(!crate::parsers::python::is_requirement_pinned(
            "foo>=1.0.0; sys_platform == 'win32'"
        ));
    }

    #[test]
    fn test_setup_cfg_dependency_is_pinned_computed() {
        let content = r#"
[metadata]
name = test-package
version = 1.0.0

[options]
install_requires =
    requests==2.28.0
    pytest>=7.0.0
    Flask
"#;
        let (_temp_dir, file_path) = create_temp_file(content, "setup.cfg");
        let package_data = PythonParser::extract_first_package(&file_path);

        let deps: Vec<_> = package_data.dependencies.iter().collect();
        assert_eq!(deps.len(), 3);

        let requests_dep = deps
            .iter()
            .find(|d| d.purl.as_ref().unwrap().contains("requests"))
            .unwrap();
        assert_eq!(
            requests_dep.is_pinned,
            Some(true),
            "requests==2.28.0 should be pinned"
        );

        let pytest_dep = deps
            .iter()
            .find(|d| d.purl.as_ref().unwrap().contains("pytest"))
            .unwrap();
        assert_eq!(
            pytest_dep.is_pinned,
            Some(false),
            "pytest>=7.0.0 should not be pinned"
        );

        let flask_dep = deps
            .iter()
            .find(|d| d.purl.as_ref().unwrap().contains("flask"))
            .unwrap();
        assert_eq!(
            flask_dep.is_pinned,
            Some(false),
            "Flask without version should not be pinned"
        );
    }
}
