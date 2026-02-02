#[cfg(test)]
mod tests {
    use crate::parsers::{CargoParser, PackageParser};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper function to create a temporary Cargo.toml file with the given content
    fn create_temp_cargo_toml(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cargo_path = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_path, content).expect("Failed to write Cargo.toml");

        (temp_dir, cargo_path)
    }

    #[test]
    fn test_is_match() {
        let valid_path = PathBuf::from("/some/path/Cargo.toml");
        let invalid_path = PathBuf::from("/some/path/not_cargo.toml");

        assert!(CargoParser::is_match(&valid_path));
        assert!(!CargoParser::is_match(&invalid_path));
    }

    #[test]
    fn test_extract_from_testdata() {
        let cargo_path = PathBuf::from("testdata/cargo/Cargo.toml");
        let package_data = CargoParser::extract_package_data(&cargo_path);

        assert_eq!(package_data.package_type, Some("cargo".to_string()));
        assert_eq!(package_data.name, Some("test-cargo".to_string()));
        assert_eq!(package_data.version, Some("1.2.3".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://example.com".to_string())
        );
        assert_eq!(
            package_data.download_url,
            Some("https://github.com/example/test-cargo".to_string())
        );

        // Check license detection
        assert_eq!(package_data.license_detections.len(), 1);
        assert_eq!(
            package_data.license_detections[0].license_expression,
            "MIT OR Apache-2.0"
        );

        // Check purl
        assert_eq!(
            package_data.purl,
            Some("pkg:cargo/test-cargo@1.2.3".to_string())
        );

        // Check authors extraction
        assert_eq!(package_data.parties.len(), 1);
        assert_eq!(package_data.parties[0].email, "test@example.com");

        // Check dependencies via PURLs
        assert_eq!(package_data.dependencies.len(), 3);
        let purls: Vec<&str> = package_data
            .dependencies
            .iter()
            .filter_map(|d| d.purl.as_deref())
            .collect();
        assert!(purls.iter().any(|p| p.contains("serde")));
        assert!(purls.iter().any(|p| p.contains("tokio")));
        assert!(purls.iter().any(|p| p.contains("mockito")));
    }

    #[test]
    fn test_extract_basic_package_info() {
        let content = r#"
[package]
name = "test-package"
version = "0.1.0"
license = "MIT"
repository = "https://github.com/user/test-package"
homepage = "https://example.com"
authors = ["Test User <test@example.com>"]
        "#;

        let (_temp_file, cargo_path) = create_temp_cargo_toml(content);
        let package_data = CargoParser::extract_package_data(&cargo_path);

        assert_eq!(package_data.package_type, Some("cargo".to_string()));
        assert_eq!(package_data.name, Some("test-package".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://example.com".to_string())
        );
        assert_eq!(
            package_data.download_url,
            Some("https://github.com/user/test-package".to_string())
        );

        // Check license detection
        assert_eq!(package_data.license_detections.len(), 1);
        assert_eq!(package_data.license_detections[0].license_expression, "MIT");

        // Check purl
        assert_eq!(
            package_data.purl,
            Some("pkg:cargo/test-package@0.1.0".to_string())
        );

        // Check authors extraction
        assert_eq!(package_data.parties.len(), 1);
        assert_eq!(package_data.parties[0].email, "test@example.com");
    }

    #[test]
    fn test_extract_dependencies() {
        let content = r#"
[package]
name = "test-package"
version = "0.1.0"
license = "MIT"

[dependencies]
serde = "1.0"
log = { version = "0.4", features = ["std"] }

[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
"#;

        let (_temp_file, cargo_path) = create_temp_cargo_toml(content);
        let package_data = CargoParser::extract_package_data(&cargo_path);

        // We should have 3 dependencies in total (2 regular, 1 dev)
        assert_eq!(package_data.dependencies.len(), 3);

        // Find the regular dependency "serde"
        let serde_dep = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_ref().unwrap().contains("serde"))
            .expect("Should find serde dependency");

        assert_eq!(serde_dep.purl, Some("pkg:cargo/serde@1.0".to_string()));
        assert_eq!(serde_dep.is_optional, Some(false));

        // Find the dev dependency "tokio"
        let tokio_dep = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_ref().unwrap().contains("tokio"))
            .expect("Should find tokio dependency");

        assert_eq!(tokio_dep.purl, Some("pkg:cargo/tokio@1.0".to_string()));
        assert_eq!(tokio_dep.is_optional, Some(true));
    }

    #[test]
    fn test_extract_with_complex_dependencies() {
        let content = r#"
[package]
name = "complex-package"
version = "0.2.0"
license = "Apache-2.0"

[dependencies]
regex = "1.5.4"
serde = { version = "1.0.136", features = ["derive"] }
reqwest = { version = "0.11", optional = true }

[dev-dependencies]
mockito = "0.31.0"
"#;

        let (_temp_file, cargo_path) = create_temp_cargo_toml(content);
        let package_data = CargoParser::extract_package_data(&cargo_path);

        // Check we have all dependencies extracted (3 regular + 1 dev)
        assert_eq!(package_data.dependencies.len(), 4);

        // Verify regex dependency
        let regex_dep = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_ref().unwrap().contains("regex"))
            .expect("Should find regex dependency");

        assert_eq!(regex_dep.purl, Some("pkg:cargo/regex@1.5.4".to_string()));

        // Verify serde dependency with specific version
        let serde_dep = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_ref().unwrap().contains("serde"))
            .expect("Should find serde dependency");

        assert_eq!(serde_dep.purl, Some("pkg:cargo/serde@1.0.136".to_string()));
    }

    #[test]
    fn test_empty_or_invalid_cargo_toml() {
        // Test with empty content
        let content = "";
        let (_temp_file, cargo_path) = create_temp_cargo_toml(content);
        let package_data = CargoParser::extract_package_data(&cargo_path);

        // Should return default/empty package data
        assert_eq!(package_data.name, None);
        assert_eq!(package_data.version, None);
        assert!(package_data.dependencies.is_empty());

        // Test with invalid TOML
        let content = "this is not valid TOML";
        let (_temp_file, cargo_path) = create_temp_cargo_toml(content);
        let package_data = CargoParser::extract_package_data(&cargo_path);

        // Should return default/empty package data
        assert_eq!(package_data.name, None);
        assert_eq!(package_data.version, None);
        assert!(package_data.dependencies.is_empty());
    }
}
