// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use super::PackageParser;
use super::ivy::{IvyDependenciesPropertiesParser, IvyXmlParser};
use crate::models::{DatasourceId, PackageType};

#[test]
fn test_is_match() {
    assert!(IvyXmlParser::is_match(&PathBuf::from("project/ivy.xml")));
    assert!(IvyXmlParser::is_match(&PathBuf::from(
        "/home/user/app/ivy.xml"
    )));
    assert!(!IvyXmlParser::is_match(&PathBuf::from("ivy.xml.bak")));
    assert!(!IvyXmlParser::is_match(&PathBuf::from("pom.xml")));
    assert!(!IvyXmlParser::is_match(&PathBuf::from("ivyconfig.xml")));
}

#[test]
fn test_is_match_dependencies_properties() {
    assert!(IvyDependenciesPropertiesParser::is_match(&PathBuf::from(
        "dependencies.properties"
    )));
    assert!(IvyDependenciesPropertiesParser::is_match(&PathBuf::from(
        "project/devel-dependencies.properties"
    )));
    assert!(!IvyDependenciesPropertiesParser::is_match(&PathBuf::from(
        "project.properties"
    )));
    assert!(!IvyDependenciesPropertiesParser::is_match(&PathBuf::from(
        "dependencies.properties.bak"
    )));
}

#[test]
fn test_parses_info_and_dependencies() {
    let path = PathBuf::from("testdata/ivy-golden/basic/ivy.xml");
    let packages = IvyXmlParser::extract_packages(&path);

    assert_eq!(packages.len(), 1);
    let pkg = &packages[0];
    assert_eq!(pkg.package_type, Some(PackageType::Ivy));
    assert_eq!(pkg.datasource_id, Some(DatasourceId::AntIvyXml));
    assert_eq!(pkg.namespace.as_deref(), Some("org.apache.example"));
    assert_eq!(pkg.name.as_deref(), Some("example-core"));
    assert_eq!(pkg.version.as_deref(), Some("4.5.6"));
    assert_eq!(
        pkg.purl.as_deref(),
        Some("pkg:ivy/org.apache.example/example-core@4.5.6")
    );

    assert_eq!(pkg.dependencies.len(), 3);

    let commons = &pkg.dependencies[0];
    assert_eq!(
        commons.purl.as_deref(),
        Some("pkg:ivy/commons-lang/commons-lang")
    );
    assert_eq!(commons.extracted_requirement.as_deref(), Some("2.6"));
    assert_eq!(commons.scope.as_deref(), Some("compile"));
    assert_eq!(commons.is_direct, Some(true));

    let junit = &pkg.dependencies[1];
    assert_eq!(junit.scope.as_deref(), Some("test"));
    assert_eq!(junit.extracted_requirement.as_deref(), Some("4.13.2"));

    // Dependency without org falls back to a name-only ivy purl.
    let local = &pkg.dependencies[2];
    assert_eq!(local.purl.as_deref(), Some("pkg:ivy/local-only"));
    assert_eq!(local.extracted_requirement.as_deref(), Some("1.0"));
}

#[test]
fn test_parses_dependencies_properties_maven_coordinates() {
    let path = PathBuf::from("testdata/ivy-golden/dependencies/dependencies.properties");
    let package_data = IvyDependenciesPropertiesParser::extract_first_package(&path);

    assert_eq!(package_data.package_type, Some(PackageType::Maven));
    assert_eq!(
        package_data.datasource_id,
        Some(DatasourceId::AntIvyDependenciesProperties)
    );
    assert_eq!(package_data.primary_language.as_deref(), Some("Java"));
    assert_eq!(package_data.dependencies.len(), 4);

    let jaxrs = package_data
        .dependencies
        .iter()
        .find(|dependency| {
            dependency.purl.as_deref() == Some("pkg:maven/javax.ws.rs/javax.ws.rs-api@2.1")
        })
        .expect("value-side GAV should be parsed");
    assert_eq!(jaxrs.extracted_requirement.as_deref(), Some("2.1"));
    assert_eq!(jaxrs.is_pinned, Some(true));
    assert_eq!(jaxrs.is_direct, Some(true));
    assert_eq!(jaxrs.is_runtime, None);
    assert_eq!(jaxrs.is_optional, None);
    assert_eq!(
        jaxrs
            .resolved_package
            .as_ref()
            .map(|pkg| pkg.namespace.as_str()),
        Some("javax.ws.rs")
    );
    assert_eq!(
        jaxrs
            .extra_data
            .as_ref()
            .and_then(|extra| extra.get("property_name"))
            .and_then(|value| value.as_str()),
        Some("javax.ws.rs-api")
    );
    assert_eq!(
        jaxrs
            .extra_data
            .as_ref()
            .and_then(|extra| extra.get("coordinate_format"))
            .and_then(|value| value.as_str()),
        Some("value_gav")
    );

    let slf4j = package_data
        .dependencies
        .iter()
        .find(|dependency| {
            dependency.purl.as_deref() == Some("pkg:maven/org.slf4j/slf4j-api@2.0.13")
        })
        .expect("key-side GA plus version should be parsed");
    assert_eq!(slf4j.extracted_requirement.as_deref(), Some("2.0.13"));
    assert_eq!(
        slf4j
            .extra_data
            .as_ref()
            .and_then(|extra| extra.get("coordinate_format"))
            .and_then(|value| value.as_str()),
        Some("key_ga")
    );
}

#[test]
fn test_dependencies_properties_prefix_becomes_scope() {
    use std::io::Write;

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("devel-dependencies.properties");
    let mut file = std::fs::File::create(&path).expect("create");
    writeln!(file, "junit:junit=4.13.2").expect("write");
    drop(file);

    let package_data = IvyDependenciesPropertiesParser::extract_first_package(&path);
    assert_eq!(package_data.dependencies.len(), 1);
    assert_eq!(
        package_data.dependencies[0].purl.as_deref(),
        Some("pkg:maven/junit/junit@4.13.2")
    );
    assert_eq!(package_data.dependencies[0].scope.as_deref(), Some("devel"));
}

#[test]
fn test_dependencies_properties_comment_backslash_does_not_continue() {
    use std::io::Write;

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("dependencies.properties");
    let mut file = std::fs::File::create(&path).expect("create");
    writeln!(file, "# issue note \\").expect("write");
    writeln!(file, "com.example:kept=1.2.3").expect("write");
    drop(file);

    let package_data = IvyDependenciesPropertiesParser::extract_first_package(&path);
    assert_eq!(package_data.dependencies.len(), 1);
    assert_eq!(
        package_data.dependencies[0].purl.as_deref(),
        Some("pkg:maven/com.example/kept@1.2.3")
    );
}

#[test]
fn test_dependencies_properties_strips_inline_comment_from_version() {
    use std::io::Write;

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("dependencies.properties");
    let mut file = std::fs::File::create(&path).expect("create");
    writeln!(
        file,
        "io.dropwizard.metrics5:metrics-core = 5.0.0-rc16 # bad version"
    )
    .expect("write");
    drop(file);

    let package_data = IvyDependenciesPropertiesParser::extract_first_package(&path);
    assert_eq!(package_data.dependencies.len(), 1);
    assert_eq!(
        package_data.dependencies[0].purl.as_deref(),
        Some("pkg:maven/io.dropwizard.metrics5/metrics-core@5.0.0-rc16")
    );
    assert_eq!(
        package_data.dependencies[0]
            .extracted_requirement
            .as_deref(),
        Some("5.0.0-rc16")
    );
}

#[test]
fn test_parses_info_without_dependencies() {
    let path = PathBuf::from("testdata/ivy-golden/no-deps/ivy.xml");
    let packages = IvyXmlParser::extract_packages(&path);

    assert_eq!(packages.len(), 1);
    let pkg = &packages[0];
    assert_eq!(pkg.namespace.as_deref(), Some("com.example"));
    assert_eq!(pkg.name.as_deref(), Some("lonely"));
    assert_eq!(pkg.version.as_deref(), Some("0.1.0"));
    assert!(pkg.dependencies.is_empty());
}

#[test]
fn test_malformed_xml_falls_back_to_default() {
    use std::io::Write;
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ivy.xml");
    let mut file = std::fs::File::create(&path).expect("create");
    file.write_all(b"<ivy-module><info organisation=")
        .expect("write");
    drop(file);

    let packages = IvyXmlParser::extract_packages(&path);
    assert_eq!(packages.len(), 1);
    // datasource_id is set even on the error path.
    assert_eq!(packages[0].datasource_id, Some(DatasourceId::AntIvyXml));
}

#[test]
fn test_xml_entities_are_decoded() {
    use std::io::Write;
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ivy.xml");
    let mut file = std::fs::File::create(&path).expect("create");
    file.write_all(
        br#"<ivy-module version="2.0">
    <info organisation="org.example&amp;co" module="a&amp;b" revision="1.0"/>
</ivy-module>"#,
    )
    .expect("write");
    drop(file);

    let packages = IvyXmlParser::extract_packages(&path);
    let pkg = &packages[0];
    assert_eq!(pkg.namespace.as_deref(), Some("org.example&co"));
    assert_eq!(pkg.name.as_deref(), Some("a&b"));
}

#[test]
fn test_ivy_purls_encode_components_and_keep_the_revision() {
    use std::io::Write;
    use std::str::FromStr;

    // `organisation`, `module` and `rev` are raw XML attributes. Hand-formatting
    // them lost data on re-parse: a space truncated the PURL and took the
    // revision with it, so the module silently lost its version.
    let dir = std::env::temp_dir().join(format!(
        "provenant-ivy-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("ivy.xml");
    let mut file = std::fs::File::create(&path).expect("create ivy.xml");
    write!(
        file,
        r#"<ivy-module version="2.0">
  <info organisation="org with space" module="mod?x" revision="1.0"/>
  <dependencies>
    <dependency org="dep org" name="dep?name" rev="2.0"/>
  </dependencies>
</ivy-module>"#
    )
    .expect("write ivy.xml");

    let package_data = IvyXmlParser::extract_first_package(&path);

    let purl = package_data.purl.as_deref().expect("package purl");
    assert_eq!(purl, "pkg:ivy/org%20with%20space/mod%3Fx@1.0");
    let parsed = packageurl::PackageUrl::from_str(purl).expect("purl should parse");
    assert_eq!(parsed.namespace(), Some("org with space"));
    assert_eq!(parsed.name(), "mod?x");
    // The revision is the part that used to disappear.
    assert_eq!(parsed.version(), Some("1.0"));
    assert_eq!(parsed.to_string(), purl);

    let dependency = package_data
        .dependencies
        .first()
        .expect("one dependency expected");
    assert_eq!(
        dependency.purl.as_deref(),
        Some("pkg:ivy/dep%20org/dep%3Fname")
    );

    std::fs::remove_dir_all(&dir).ok();
}
