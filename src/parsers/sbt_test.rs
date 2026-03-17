#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::models::{DatasourceId, PackageType};
    use crate::parsers::{PackageParser, SbtParser};

    fn create_temp_build_sbt(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let build_sbt = temp_dir.path().join("build.sbt");
        fs::write(&build_sbt, content).expect("Failed to write build.sbt");
        (temp_dir, build_sbt)
    }

    #[test]
    fn test_is_match_build_sbt_only() {
        assert!(SbtParser::is_match(&PathBuf::from("/repo/build.sbt")));
        assert!(!SbtParser::is_match(&PathBuf::from("/repo/plugins.sbt")));
        assert!(!SbtParser::is_match(&PathBuf::from(
            "/repo/project/Build.scala"
        )));
        assert!(!SbtParser::is_match(&PathBuf::from("/repo/module.sbt")));
    }

    #[test]
    fn test_extract_literal_metadata_and_dependencies() {
        let content = r#"
val orgName = "com.example"
val projectName = "demo-app"
val projectVersion = "1.2.3"
val catsVersion = "2.10.0"

ThisBuild / organization := orgName
ThisBuild / name := "fallback-name"
ThisBuild / version := projectVersion
ThisBuild / description := "Fallback description"
ThisBuild / organizationHomepage := Some(url("https://example.com/org"))

name := projectName
description := "Demo application"
homepage := Some(url("https://example.com/demo"))
licenses += "Apache-2.0" -> url("https://www.apache.org/licenses/LICENSE-2.0.txt")

libraryDependencies += "org.typelevel" %% "cats-core" % catsVersion
libraryDependencies += "org.scalatest" %% "scalatest" % "3.2.18" % Test
libraryDependencies ++= Seq(
  "javax.servlet" % "javax.servlet-api" % "4.0.1" % "provided",
  unsupportedDependency,
  "org.slf4j" % "slf4j-api" % "2.0.12"
)
        "#;

        let (_temp_dir, path) = create_temp_build_sbt(content);
        let package_data = SbtParser::extract_first_package(&path);

        assert_eq!(package_data.package_type, Some(PackageType::Maven));
        assert_eq!(package_data.primary_language.as_deref(), Some("Scala"));
        assert_eq!(package_data.datasource_id, Some(DatasourceId::SbtBuildSbt));
        assert_eq!(package_data.namespace.as_deref(), Some("com.example"));
        assert_eq!(package_data.name.as_deref(), Some("demo-app"));
        assert_eq!(package_data.version.as_deref(), Some("1.2.3"));
        assert_eq!(
            package_data.description.as_deref(),
            Some("Demo application")
        );
        assert_eq!(
            package_data.homepage_url.as_deref(),
            Some("https://example.com/demo")
        );
        assert_eq!(
            package_data.purl.as_deref(),
            Some("pkg:maven/com.example/demo-app@1.2.3")
        );
        assert_eq!(
            package_data.extracted_license_statement.as_deref(),
            Some(
                "- license:\n    name: Apache-2.0\n    url: https://www.apache.org/licenses/LICENSE-2.0.txt\n"
            )
        );

        assert_eq!(package_data.dependencies.len(), 4);

        let cats = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_deref() == Some("pkg:maven/org.typelevel/cats-core@2.10.0"))
            .expect("cats dependency missing");
        assert_eq!(cats.extracted_requirement.as_deref(), Some("2.10.0"));
        assert_eq!(cats.scope, None);
        assert_eq!(cats.is_runtime, Some(true));
        assert_eq!(cats.is_optional, Some(false));
        let cats_extra = cats.extra_data.as_ref().expect("cats extra_data missing");
        assert_eq!(
            cats_extra
                .get("sbt_cross_version")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            cats_extra
                .get("sbt_operator")
                .and_then(|value| value.as_str()),
            Some("%%")
        );

        let scalatest = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_deref() == Some("pkg:maven/org.scalatest/scalatest@3.2.18"))
            .expect("scalatest dependency missing");
        assert_eq!(scalatest.scope.as_deref(), Some("test"));
        assert_eq!(scalatest.is_runtime, Some(false));
        assert_eq!(scalatest.is_optional, Some(true));

        let servlet = package_data
            .dependencies
            .iter()
            .find(|dep| {
                dep.purl.as_deref() == Some("pkg:maven/javax.servlet/javax.servlet-api@4.0.1")
            })
            .expect("servlet dependency missing");
        assert_eq!(servlet.scope.as_deref(), Some("provided"));
        assert_eq!(servlet.is_runtime, Some(false));
        assert_eq!(servlet.is_optional, Some(false));

        let slf4j = package_data
            .dependencies
            .iter()
            .find(|dep| dep.purl.as_deref() == Some("pkg:maven/org.slf4j/slf4j-api@2.0.12"))
            .expect("slf4j dependency missing");
        assert_eq!(slf4j.scope, None);
        assert_eq!(slf4j.is_runtime, Some(true));
        assert_eq!(slf4j.is_optional, Some(false));
    }

    #[test]
    fn test_skips_unsupported_constructs_and_uses_organization_homepage_fallback() {
        let content = r#"
val desc = "Alias description"
val depVersion = "1.0.0"

ThisBuild / name := "fallback-name"
organization := "com.fallback"
version := "0.1.0"
description := desc
organizationHomepage := Some(url("https://fallback.example.com/org"))
homepage := Some(url(homepageValue))
licenses += License.Apache

lazy val root = project.settings(
  libraryDependencies += "com.nested" % "ignored" % "9.9.9"
)

libraryDependencies += depGroup % "unresolved" % depVersion
libraryDependencies += "org.valid" % "artifact" % depVersion
        "#;

        let (_temp_dir, path) = create_temp_build_sbt(content);
        let package_data = SbtParser::extract_first_package(&path);

        assert_eq!(package_data.namespace.as_deref(), Some("com.fallback"));
        assert_eq!(package_data.name.as_deref(), Some("fallback-name"));
        assert_eq!(package_data.version.as_deref(), Some("0.1.0"));
        assert_eq!(
            package_data.description.as_deref(),
            Some("Alias description")
        );
        assert_eq!(
            package_data.homepage_url.as_deref(),
            Some("https://fallback.example.com/org")
        );
        assert!(package_data.extracted_license_statement.is_none());
        assert_eq!(package_data.dependencies.len(), 1);
        assert_eq!(
            package_data.dependencies[0].purl.as_deref(),
            Some("pkg:maven/org.valid/artifact@1.0.0")
        );
    }
}
