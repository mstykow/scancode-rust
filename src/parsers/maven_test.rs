#[cfg(test)]
mod tests {
    use crate::parsers::{MavenParser, PackageParser};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    // Helper function to create a temporary pom.xml file with the given content
    fn create_temp_pom_xml(content: &str) -> (NamedTempFile, PathBuf) {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        // Rename the file to pom.xml so that is_match works correctly
        let dir = temp_file.path().parent().unwrap();
        let pom_path = dir.join("pom.xml");
        fs::rename(temp_file.path(), &pom_path).expect("Failed to rename temp file");

        (temp_file, pom_path)
    }

    #[test]
    fn test_is_match() {
        let valid_path = PathBuf::from("/some/path/pom.xml");
        let invalid_path = PathBuf::from("/some/path/not_pom.xml");

        assert!(MavenParser::is_match(&valid_path));
        assert!(!MavenParser::is_match(&invalid_path));
    }

    #[test]
    fn test_extract_from_testdata() {
        let pom_path = PathBuf::from("testdata/maven/pom.xml");
        let package_data = MavenParser::extract_package_data(&pom_path);

        assert_eq!(package_data.package_type, Some("maven".to_string()));
        assert_eq!(package_data.namespace, Some("com.example".to_string()));
        assert_eq!(package_data.name, Some("demo-app".to_string()));
        assert_eq!(package_data.version, Some("1.0.0".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://example.com/demo".to_string())
        );

        // Check license detection
        assert_eq!(package_data.license_detections.len(), 1);
        assert_eq!(
            package_data.license_detections[0].license_expression,
            "Apache-2.0"
        );

        // Check purl
        assert_eq!(
            package_data.purl,
            Some("pkg:maven/com.example/demo-app@1.0.0".to_string())
        );

        // Check dependencies
        assert_eq!(package_data.dependencies.len(), 2);
        let purls: Vec<&str> = package_data
            .dependencies
            .iter()
            .filter_map(|d| d.purl.as_deref())
            .collect();
        assert!(purls.contains(&"pkg:maven/junit/junit@4.12"));
        assert!(purls.contains(&"pkg:maven/org.apache.commons/commons-lang3@3.12.0"));
    }

    #[test]
    fn test_extract_basic_package_info() {
        let content = r#"
<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.test</groupId>
    <artifactId>test-project</artifactId>
    <version>1.0.0</version>
    <name>Test Project</name>
    <url>https://test.example.com</url>
    <licenses>
        <license>
            <name>MIT License</name>
        </license>
    </licenses>
</project>
        "#;

        let (_temp_file, pom_path) = create_temp_pom_xml(content);
        let package_data = MavenParser::extract_package_data(&pom_path);

        assert_eq!(package_data.package_type, Some("maven".to_string()));
        assert_eq!(package_data.namespace, Some("com.test".to_string()));
        assert_eq!(package_data.name, Some("test-project".to_string()));
        assert_eq!(package_data.version, Some("1.0.0".to_string()));
        assert_eq!(
            package_data.homepage_url,
            Some("https://test.example.com".to_string())
        );

        // Check license detection
        assert_eq!(package_data.license_detections.len(), 1);
        assert_eq!(package_data.license_detections[0].license_expression, "MIT");

        // Check purl
        assert_eq!(
            package_data.purl,
            Some("pkg:maven/com.test/test-project@1.0.0".to_string())
        );
    }

    #[test]
    fn test_extract_dependencies() {
        let content = r#"
<project>
    <groupId>com.example</groupId>
    <artifactId>test-deps</artifactId>
    <version>1.0.0</version>
    
    <dependencies>
        <dependency>
            <groupId>org.junit</groupId>
            <artifactId>junit</artifactId>
            <version>5.9.2</version>
            <scope>test</scope>
        </dependency>
        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>2.15.2</version>
        </dependency>
    </dependencies>
</project>
        "#;

        let (_temp_file, pom_path) = create_temp_pom_xml(content);
        let package_data = MavenParser::extract_package_data(&pom_path);

        assert_eq!(package_data.dependencies.len(), 2);

        // Verify junit dependency (test scope)
        let junit_dep = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_ref().unwrap().contains("junit"))
            .expect("Should find junit dependency");
        assert_eq!(
            junit_dep.purl,
            Some("pkg:maven/org.junit/junit@5.9.2".to_string())
        );
        assert!(junit_dep.is_optional);

        // Verify jackson dependency
        let jackson_dep = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_ref().unwrap().contains("jackson-databind"))
            .expect("Should find jackson-databind dependency");
        assert_eq!(
            jackson_dep.purl,
            Some("pkg:maven/com.fasterxml.jackson.core/jackson-databind@2.15.2".to_string())
        );
        assert!(!jackson_dep.is_optional);
    }

    #[test]
    fn test_empty_or_invalid_pom_xml() {
        // Test with empty content
        let content = "";
        let (_temp_file, pom_path) = create_temp_pom_xml(content);
        let package_data = MavenParser::extract_package_data(&pom_path);

        // Should return default/empty package data
        assert_eq!(package_data.name, None);
        assert_eq!(package_data.version, None);
        assert!(package_data.dependencies.is_empty());

        // Test with invalid XML
        let content = "this is not valid XML";
        let (_temp_file, pom_path) = create_temp_pom_xml(content);
        let package_data = MavenParser::extract_package_data(&pom_path);

        // Should return default/empty package data
        assert_eq!(package_data.name, None);
        assert_eq!(package_data.version, None);
        assert!(package_data.dependencies.is_empty());
    }
}
