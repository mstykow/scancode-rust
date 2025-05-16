#[cfg(test)]
mod tests {
    use crate::parsers::{PythonParser, PackageParser};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    // Helper function to create a temporary test file with the given content and name
    fn create_temp_file(content: &str, filename: &str) -> (NamedTempFile, PathBuf) {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write to temp file");
        
        let dir = temp_file.path().parent().unwrap();
        let file_path = dir.join(filename);
        fs::rename(temp_file.path(), &file_path).expect("Failed to rename temp file");
        
        (temp_file, file_path)
    }

    #[test]
    fn test_is_match() {
        let pyproject_path = PathBuf::from("/some/path/pyproject.toml");
        let setup_path = PathBuf::from("/some/path/setup.py");
        let invalid_path = PathBuf::from("/some/path/not_python.txt");
        
        assert!(PythonParser::is_match(&pyproject_path));
        assert!(PythonParser::is_match(&setup_path));
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
        let package_data = PythonParser::extract_package_data(&file_path);
        
        assert_eq!(package_data.package_type, Some("pypi".to_string()));
        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));
        assert_eq!(package_data.homepage_url, Some("https://example.com".to_string()));
        
        // Check license detection
        assert_eq!(package_data.license_detections.len(), 1);
        assert_eq!(package_data.license_detections[0].license_expression, "MIT");
        
        // Check purl
        assert_eq!(package_data.purl, Some("pkg:pypi/test-package@0.1.0".to_string()));
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
        let package_data = PythonParser::extract_package_data(&file_path);
        
        assert_eq!(package_data.package_type, Some("pypi".to_string()));
        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));
        assert_eq!(package_data.homepage_url, Some("https://example.com".to_string()));
        
        // Check license detection
        assert_eq!(package_data.license_detections.len(), 1);
        assert_eq!(package_data.license_detections[0].license_expression, "MIT");
        
        // Check purl
        assert_eq!(package_data.purl, Some("pkg:pypi/test-package@0.1.0".to_string()));
    }
}
