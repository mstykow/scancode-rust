// Tests for gradle.lockfile parser

use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

use crate::models::DatasourceId;
use crate::parsers::PackageParser;
use crate::parsers::gradle_lock::GradleLockfileParser;

#[test]
fn test_parse_simple_gradle_lockfile() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "com.example:lib1:1.0.0=abc123\ncom.example:lib2:2.0.0=def456"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 2);
    assert_eq!(
        package_data.dependencies[0]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "lib1".to_string()
    );
    assert_eq!(
        package_data.dependencies[0]
            .resolved_package
            .as_ref()
            .unwrap()
            .version,
        "1.0.0".to_string()
    );
    assert_eq!(
        package_data.dependencies[1]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "lib2".to_string()
    );
    assert_eq!(
        package_data.dependencies[1]
            .resolved_package
            .as_ref()
            .unwrap()
            .version,
        "2.0.0".to_string()
    );
}

#[test]
fn test_parse_gradle_lockfile_with_comments() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "# This is a comment\ncom.example:lib1:1.0.0=abc123\n# Another comment\ncom.example:lib2:2.0.0=def456"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 2);
}

#[test]
fn test_parse_gradle_lockfile_with_empty_lines() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "com.example:lib1:1.0.0=abc123\n\ncom.example:lib2:2.0.0=def456\n\n"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 2);
}

#[test]
fn test_parse_gradle_lockfile_complex_group_name() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "org.springframework.boot:spring-boot-starter-web:2.7.0=hash1"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 1);
    let dep = &package_data.dependencies[0];
    assert_eq!(
        dep.resolved_package.as_ref().unwrap().namespace,
        "org.springframework.boot".to_string()
    );
    assert_eq!(
        dep.resolved_package.as_ref().unwrap().name,
        "spring-boot-starter-web".to_string()
    );
    assert_eq!(
        dep.resolved_package.as_ref().unwrap().version,
        "2.7.0".to_string()
    );
}

#[test]
fn test_parse_gradle_lockfile_empty_file() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file).expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 0);
    assert_eq!(package_data.package_type, Some("maven".to_string()));
}

#[test]
fn test_parse_gradle_lockfile_datasource_id() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file, "com.example:lib:1.0.0=hash").expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(
        package_data.datasource_id,
        Some(DatasourceId::GradleLockfile)
    );
}

#[test]
fn test_parse_gradle_lockfile_dependency_flags() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file, "com.example:lib:1.0.0=hash").expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 1);
    let dep = &package_data.dependencies[0];

    assert_eq!(dep.is_pinned, Some(true));
    assert_eq!(dep.is_optional, Some(false));
    assert_eq!(dep.is_runtime, Some(true));
    assert_eq!(
        dep.resolved_package.as_ref().unwrap().package_type,
        "maven".to_string()
    );
}

#[test]
fn test_parse_gradle_lockfile_generates_purl() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file, "com.google.guava:guava:30.1-jre=abc123").expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 1);
    let dep = &package_data.dependencies[0];

    assert!(dep.purl.is_some());
    let purl = dep.purl.as_ref().unwrap();
    assert!(purl.contains("maven"));
    assert!(purl.contains("guava"));
    assert!(purl.contains("30.1-jre"));
}

#[test]
fn test_parse_gradle_lockfile_extra_data() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file, "org.junit.jupiter:junit-jupiter-api:5.8.0=hash123")
        .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 1);
    let dep = &package_data.dependencies[0];

    assert!(dep.extra_data.is_some());
    let extra = dep.extra_data.as_ref().unwrap();

    assert!(extra.get("group").is_some());
    assert_eq!(
        extra.get("group").and_then(|v| v.as_str()),
        Some("org.junit.jupiter")
    );

    assert!(extra.get("artifact").is_some());
    assert_eq!(
        extra.get("artifact").and_then(|v| v.as_str()),
        Some("junit-jupiter-api")
    );

    assert!(extra.get("hash").is_some());
    assert_eq!(extra.get("hash").and_then(|v| v.as_str()), Some("hash123"));
}

#[test]
fn test_parse_gradle_lockfile_multiple_deps_with_different_hashes() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "com.example:lib1:1.0.0=abc123\ncom.example:lib2:2.0.0=def456\ncom.test:lib3:3.0.0=ghi789"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 3);

    for (i, expected_hash) in ["abc123", "def456", "ghi789"].iter().enumerate() {
        let dep = &package_data.dependencies[i];
        let extra = dep.extra_data.as_ref().unwrap();
        assert_eq!(
            extra.get("hash").and_then(|v| v.as_str()),
            Some(*expected_hash)
        );
    }
}

#[test]
fn test_parse_gradle_lockfile_malformed_lines_skipped() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "com.example:lib1:1.0.0=abc123\ninvalid-line-without-colons\ncom.example:lib2:2.0.0=def456"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    // Only valid dependencies should be extracted
    assert_eq!(package_data.dependencies.len(), 2);
    assert_eq!(
        package_data.dependencies[0]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "lib1".to_string()
    );
    assert_eq!(
        package_data.dependencies[1]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "lib2".to_string()
    );
}

#[test]
fn test_parse_gradle_lockfile_versions_with_special_chars() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "com.example:lib1:1.0.0-SNAPSHOT=abc123\ncom.example:lib2:2.0.0-rc.1=def456"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 2);
    assert_eq!(
        package_data.dependencies[0]
            .resolved_package
            .as_ref()
            .unwrap()
            .version,
        "1.0.0-SNAPSHOT".to_string()
    );
    assert_eq!(
        package_data.dependencies[1]
            .resolved_package
            .as_ref()
            .unwrap()
            .version,
        "2.0.0-rc.1".to_string()
    );
}

#[test]
fn test_parse_gradle_lockfile_real_world_example() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    let content = r#"# Gradle lockfile for example project
org.springframework.boot:spring-boot-starter-web:2.7.0=2e4f5b3c8d9a1f2e3a4b5c6d7e8f9a0b
com.google.guava:guava:30.1-jre=1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p
junit:junit:4.13.2=5a6b7c8d9e0f1g2h3i4j5k6l7m8n9o0p
org.mockito:mockito-core:3.12.4=9z8y7x6w5v4u3t2s1r0q9p8o7n6m5l4k3

# Development dependencies
org.hamcrest:hamcrest-core:1.3=3x2w1v0u9t8s7r6q5p4o3n2m1l0k9j8i7"#;
    writeln!(file, "{}", content).expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 5);

    // Verify first dependency
    assert_eq!(
        package_data.dependencies[0]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "spring-boot-starter-web".to_string()
    );
    assert_eq!(
        package_data.dependencies[0]
            .resolved_package
            .as_ref()
            .unwrap()
            .version,
        "2.7.0".to_string()
    );

    // Verify second dependency
    assert_eq!(
        package_data.dependencies[1]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "guava".to_string()
    );

    // Verify all dependencies are pinned
    for dep in &package_data.dependencies {
        assert_eq!(dep.is_pinned, Some(true));
        assert_eq!(
            dep.resolved_package.as_ref().unwrap().package_type,
            "maven".to_string()
        );
    }
}

#[test]
fn test_is_match_recognizes_gradle_lockfile() {
    assert!(GradleLockfileParser::is_match(Path::new("gradle.lockfile")));
    assert!(GradleLockfileParser::is_match(Path::new(
        "/path/to/gradle.lockfile"
    )));
    assert!(GradleLockfileParser::is_match(Path::new(
        "/some/deep/path/gradle.lockfile"
    )));
}

#[test]
fn test_is_match_rejects_similar_names() {
    assert!(!GradleLockfileParser::is_match(Path::new("gradle.lock")));
    assert!(!GradleLockfileParser::is_match(Path::new(
        "gradle-lockfile"
    )));
    assert!(!GradleLockfileParser::is_match(Path::new(
        "gradle.lockfile.bak"
    )));
    assert!(!GradleLockfileParser::is_match(Path::new(
        "my-gradle.lockfile"
    )));
}

#[test]
fn test_package_data_default_values() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file, "com.example:lib:1.0.0=hash").expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    // Root package data should have no name/version
    assert_eq!(package_data.name, None);
    assert_eq!(package_data.version, None);

    // Should have correct package type
    assert_eq!(package_data.package_type, Some("maven".to_string()));

    // Verify empty/default fields
    assert!(package_data.parties.is_empty());
    assert!(package_data.keywords.is_empty());
    assert!(!package_data.is_private);
    assert!(!package_data.is_virtual);
}

#[test]
fn test_parse_gradle_lockfile_with_no_hash() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file, "com.example:lib:1.0.0=").expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 1);
    let dep = &package_data.dependencies[0];
    assert_eq!(
        dep.resolved_package.as_ref().unwrap().name,
        "lib".to_string()
    );
    assert_eq!(
        dep.resolved_package.as_ref().unwrap().version,
        "1.0.0".to_string()
    );
}

#[test]
fn test_parse_gradle_lockfile_whitespace_handling() {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        "  com.example:lib1:1.0.0=abc123  \n\t\ncom.example:lib2:2.0.0=def456\t"
    )
    .expect("Failed to write to temp file");

    let path = file.path();
    let package_data = GradleLockfileParser::extract_first_package(path);

    assert_eq!(package_data.dependencies.len(), 2);
    assert_eq!(
        package_data.dependencies[0]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "lib1".to_string()
    );
    assert_eq!(
        package_data.dependencies[1]
            .resolved_package
            .as_ref()
            .unwrap()
            .name,
        "lib2".to_string()
    );
}
