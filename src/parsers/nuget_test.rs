#[cfg(test)]
mod tests {
    use super::super::PackageParser;
    use super::super::nuget::{
        NupkgParser, NuspecParser, PackagesConfigParser, PackagesLockParser, infer_party_type,
        parse_license_element,
    };
    use crate::models::DatasourceId;
    use crate::models::PackageType;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_license_element() {
        assert_eq!(
            parse_license_element(Some("expression"), Some("MIT")),
            Some("MIT".to_string())
        );
        assert_eq!(
            parse_license_element(Some("file"), Some("LICENSE.txt")),
            Some("file:LICENSE.txt".to_string())
        );
        assert_eq!(
            parse_license_element(None, Some("Apache-2.0")),
            Some("Apache-2.0".to_string())
        );
        assert_eq!(
            parse_license_element(Some("expression"), Some("(MIT OR Apache-2.0)")),
            Some("(MIT OR Apache-2.0)".to_string())
        );
        assert_eq!(parse_license_element(None, None), None);
        assert_eq!(parse_license_element(Some("unknown"), Some("text")), None);
    }

    #[test]
    fn test_nuspec_license_expression() {
        let xml = r#"<?xml version="1.0"?>
        <package>
          <metadata>
            <id>TestPackage</id>
            <version>1.0.0</version>
            <license type="expression">MIT</license>
          </metadata>
        </package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        assert_eq!(
            package_data.extracted_license_statement,
            Some("MIT".to_string())
        );
    }

    #[test]
    fn test_nuspec_license_file_reference() {
        let xml = r#"<?xml version="1.0"?>
        <package>
          <metadata>
            <id>TestPackage</id>
            <version>1.0.0</version>
            <license type="file">LICENSE.txt</license>
          </metadata>
        </package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        assert_eq!(
            package_data.extracted_license_statement,
            Some("file:LICENSE.txt".to_string())
        );
    }

    #[test]
    fn test_nuspec_license_plain_text() {
        let xml = r#"<?xml version="1.0"?>
        <package>
          <metadata>
            <id>TestPackage</id>
            <version>1.0.0</version>
            <license>Apache-2.0</license>
          </metadata>
        </package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        assert_eq!(
            package_data.extracted_license_statement,
            Some("Apache-2.0".to_string())
        );
    }

    #[test]
    fn test_nuspec_license_expression_complex() {
        let xml = r#"<?xml version="1.0"?>
        <package>
          <metadata>
            <id>TestPackage</id>
            <version>1.0.0</version>
            <license type="expression">(MIT OR Apache-2.0)</license>
          </metadata>
        </package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        assert_eq!(
            package_data.extracted_license_statement,
            Some("(MIT OR Apache-2.0)".to_string())
        );
    }

    #[test]
    fn test_nuspec_license_expression_with_version() {
        let xml = r#"<?xml version="1.0"?>
        <package>
          <metadata>
            <id>TestPackage</id>
            <version>1.0.0</version>
            <license type="expression">Apache-2.0</license>
          </metadata>
        </package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        assert_eq!(
            package_data.extracted_license_statement,
            Some("Apache-2.0".to_string())
        );
    }

    #[test]
    fn test_infer_party_type_organization() {
        assert_eq!(infer_party_type("Microsoft"), "organization");
        assert_eq!(infer_party_type("Twitter, Inc."), "organization");
        assert_eq!(
            infer_party_type("Castle Project Contributors"),
            "organization"
        );
        assert_eq!(infer_party_type("Google LLC"), "organization");
        assert_eq!(
            infer_party_type("Apache Software Foundation"),
            "organization"
        );
        assert_eq!(infer_party_type("Red Hat, Inc."), "organization");
        assert_eq!(infer_party_type("JetBrains s.r.o."), "organization");
    }

    #[test]
    fn test_infer_party_type_person() {
        assert_eq!(infer_party_type("James Newton-King"), "person");
        assert_eq!(
            infer_party_type("Sam Saffron,Marc Gravell,Nick Craver"),
            "person"
        );
        assert_eq!(infer_party_type("John Doe"), "person");
        assert_eq!(infer_party_type("Jane Smith"), "person");
    }

    #[test]
    fn test_nuspec_party_types_populated() {
        let xml = r#"<?xml version="1.0"?>
        <package>
          <metadata>
            <id>TestPackage</id>
            <version>1.0.0</version>
            <authors>Twitter, Inc.</authors>
            <owners>bootstrap</owners>
          </metadata>
        </package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        let parties = &package_data.parties;

        assert_eq!(parties.len(), 2);
        assert_eq!(parties[0].r#type, Some("organization".to_string()));
        assert_eq!(parties[0].role, Some("author".to_string()));
        assert_eq!(parties[1].r#type, Some("person".to_string()));
        assert_eq!(parties[1].role, Some("owner".to_string()));
    }

    #[test]
    fn test_packages_config_is_match() {
        assert!(PackagesConfigParser::is_match(&PathBuf::from(
            "packages.config"
        )));
        assert!(!PackagesConfigParser::is_match(&PathBuf::from("other.xml")));
        assert!(!PackagesConfigParser::is_match(&PathBuf::from(
            "packages.config.bak"
        )));
    }

    #[test]
    fn test_nuspec_is_match() {
        assert!(NuspecParser::is_match(&PathBuf::from("example.nuspec")));
        assert!(NuspecParser::is_match(&PathBuf::from("MyPackage.nuspec")));
        assert!(!NuspecParser::is_match(&PathBuf::from("example.xml")));
        assert!(!NuspecParser::is_match(&PathBuf::from("example.nupkg")));
    }

    #[test]
    fn test_packages_config_simple() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<packages>
  <package id="Newtonsoft.Json" version="13.0.1" targetFramework="net472" />
  <package id="NUnit" version="3.13.2" targetFramework="net472" />
</packages>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = PackagesConfigParser::extract_first_package(path);

        assert_eq!(package_data.package_type, Some(PackageType::Nuget));
        assert_eq!(
            package_data.datasource_id,
            Some(DatasourceId::NugetPackagesConfig)
        );

        let deps = &package_data.dependencies;
        assert_eq!(deps.len(), 2);

        let dep1 = &deps[0];
        assert_eq!(dep1.purl, Some("pkg:nuget/Newtonsoft.Json".to_string()));
        assert_eq!(dep1.extracted_requirement, Some("13.0.1".to_string()));
        assert_eq!(dep1.scope, Some("net472".to_string()));
        assert_eq!(dep1.is_pinned, Some(true));
        assert_eq!(dep1.is_direct, Some(true));

        let dep2 = &deps[1];
        assert_eq!(dep2.purl, Some("pkg:nuget/NUnit".to_string()));
        assert_eq!(dep2.extracted_requirement, Some("3.13.2".to_string()));
    }

    #[test]
    fn test_packages_config_no_target_framework() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<packages>
  <package id="jQuery" version="3.6.0" />
</packages>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = PackagesConfigParser::extract_first_package(path);

        assert_eq!(package_data.dependencies.len(), 1);
        assert_eq!(package_data.dependencies[0].scope, None);
    }

    #[test]
    fn test_nuspec_basic() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2010/07/nuspec.xsd">
  <metadata>
    <id>Newtonsoft.Json</id>
    <version>13.0.1</version>
    <description>Json.NET is a popular high-performance JSON framework for .NET</description>
    <authors>James Newton-King</authors>
    <owners>James Newton-King</owners>
    <license>MIT</license>
    <projectUrl>https://www.newtonsoft.com/json</projectUrl>
    <copyright>Copyright © James Newton-King 2008</copyright>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        assert_eq!(package_data.name, Some("Newtonsoft.Json".to_string()));
        assert_eq!(package_data.version, Some("13.0.1".to_string()));
        assert!(package_data.description.is_some());
        assert_eq!(
            package_data.homepage_url,
            Some("https://www.newtonsoft.com/json".to_string())
        );
        assert_eq!(
            package_data.copyright,
            Some("Copyright © James Newton-King 2008".to_string())
        );

        let parties = &package_data.parties;
        assert_eq!(parties.len(), 2);
        assert_eq!(parties[0].name, Some("James Newton-King".to_string()));
        assert_eq!(parties[0].role, Some("author".to_string()));
        assert_eq!(parties[1].role, Some("owner".to_string()));
    }

    #[test]
    fn test_nuspec_with_dependencies() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<package>
  <metadata>
    <id>MyPackage</id>
    <version>1.0.0</version>
    <dependencies>
      <dependency id="Newtonsoft.Json" version="13.0.1" />
      <dependency id="NUnit" version="[3.0,4.0)" />
    </dependencies>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        let deps = &package_data.dependencies;

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].purl, Some("pkg:nuget/Newtonsoft.Json".to_string()));
        assert_eq!(deps[0].extracted_requirement, Some("13.0.1".to_string()));
        assert_eq!(deps[0].scope, Some("dependency".to_string()));
        assert_eq!(deps[0].is_pinned, Some(false));

        assert_eq!(deps[1].purl, Some("pkg:nuget/NUnit".to_string()));
        assert_eq!(deps[1].extracted_requirement, Some("[3.0,4.0)".to_string()));
    }

    #[test]
    fn test_nuspec_with_dependency_groups() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<package>
  <metadata>
    <id>MyPackage</id>
    <version>1.0.0</version>
    <dependencies>
      <group targetFramework="net45">
        <dependency id="Newtonsoft.Json" version="12.0.0" />
      </group>
      <group targetFramework="netstandard2.0">
        <dependency id="Newtonsoft.Json" version="13.0.1" />
        <dependency id="System.Text.Json" version="6.0.0" />
      </group>
    </dependencies>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);
        let deps = &package_data.dependencies;

        assert_eq!(deps.len(), 3);

        // First group (net45)
        assert_eq!(deps[0].purl, Some("pkg:nuget/Newtonsoft.Json".to_string()));
        assert_eq!(deps[0].extracted_requirement, Some("12.0.0".to_string()));
        let extra = deps[0].extra_data.as_ref().unwrap();
        assert_eq!(extra["framework"], "net45");

        // Second group (netstandard2.0)
        assert_eq!(deps[1].purl, Some("pkg:nuget/Newtonsoft.Json".to_string()));
        let extra = deps[1].extra_data.as_ref().unwrap();
        assert_eq!(extra["framework"], "netstandard2.0");

        assert_eq!(deps[2].purl, Some("pkg:nuget/System.Text.Json".to_string()));
        let extra = deps[2].extra_data.as_ref().unwrap();
        assert_eq!(extra["framework"], "netstandard2.0");
    }

    #[test]
    fn test_nuspec_with_repository() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<package>
  <metadata>
    <id>MyPackage</id>
    <version>1.0.0</version>
    <repository type="git" url="https://github.com/user/repo.git" />
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        assert_eq!(
            package_data.vcs_url,
            Some("git+https://github.com/user/repo.git".to_string())
        );
    }

    #[test]
    fn test_nuspec_license_url_fallback() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<package>
  <metadata>
    <id>OldPackage</id>
    <version>1.0.0</version>
    <licenseUrl>https://opensource.org/licenses/MIT</licenseUrl>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        assert_eq!(
            package_data.extracted_license_statement,
            Some("https://opensource.org/licenses/MIT".to_string())
        );
    }

    #[test]
    fn test_nuspec_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<package>
  <metadata>
    <id>Broken</id>
    <version>1.0.0
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        // Should return default package data on error
        assert_eq!(package_data.package_type, Some(PackageType::Nuget));
        assert_eq!(package_data.datasource_id, Some(DatasourceId::NugetNuspec));
        assert!(package_data.name.is_none());
    }

    #[test]
    fn test_packages_config_empty() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<packages>
</packages>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = PackagesConfigParser::extract_first_package(path);

        assert_eq!(package_data.dependencies.len(), 0);
    }

    #[test]
    fn test_nuspec_repository_urls() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<package>
  <metadata>
    <id>TestPackage</id>
    <version>2.5.0</version>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        assert_eq!(
            package_data.repository_homepage_url,
            Some("https://www.nuget.org/packages/TestPackage/2.5.0".to_string())
        );
        assert_eq!(
            package_data.repository_download_url,
            Some("https://www.nuget.org/api/v2/package/TestPackage/2.5.0".to_string())
        );
        assert_eq!(
            package_data.api_data_url,
            Some("https://api.nuget.org/v3/registration3/testpackage/2.5.0.json".to_string())
        );
    }

    #[test]
    fn test_packages_lock_is_match() {
        assert!(PackagesLockParser::is_match(&PathBuf::from(
            "packages.lock.json"
        )));
        assert!(PackagesLockParser::is_match(&PathBuf::from(
            "MyProject.packages.lock.json"
        )));
        assert!(!PackagesLockParser::is_match(&PathBuf::from(
            "package.json"
        )));
    }

    #[test]
    fn test_nupkg_is_match() {
        assert!(NupkgParser::is_match(&PathBuf::from("Example.nupkg")));
        assert!(NupkgParser::is_match(&PathBuf::from(
            "Newtonsoft.Json.13.0.1.nupkg"
        )));
        assert!(!NupkgParser::is_match(&PathBuf::from("example.zip")));
    }

    #[test]
    fn test_packages_lock_simple() {
        let json = r#"{
  "version": 1,
  "dependencies": {
    "net5.0": {
      "Newtonsoft.Json": {
        "type": "Direct",
        "requested": "[13.0.1, )",
        "resolved": "13.0.1",
        "contentHash": "sha512-ppPFpBcvxdsfUonNcvITKqLl3bqxWbDCZIzDWHzjpdAHRFfZe0Dw9HmA0+za13IdyrgJwpkDTDA9fHaxOrt20A=="
      }
    }
  }
}"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = PackagesLockParser::extract_first_package(path);

        assert_eq!(package_data.package_type, Some(PackageType::Nuget));
        assert_eq!(
            package_data.datasource_id,
            Some(DatasourceId::NugetPackagesLock)
        );

        let deps = &package_data.dependencies;
        assert_eq!(deps.len(), 1);

        let dep = &deps[0];
        assert_eq!(
            dep.purl,
            Some("pkg:nuget/Newtonsoft.Json@13.0.1".to_string())
        );
        assert_eq!(dep.extracted_requirement, Some("[13.0.1, )".to_string()));
        assert_eq!(dep.scope, Some("net5.0".to_string()));
        assert_eq!(dep.is_direct, Some(true));
        assert_eq!(dep.is_pinned, Some(true));
    }

    #[test]
    fn test_packages_lock_multiple_frameworks() {
        let json = r#"{
  "version": 1,
  "dependencies": {
    "net5.0": {
      "Newtonsoft.Json": {
        "type": "Direct",
        "resolved": "13.0.1"
      }
    },
    "netstandard2.0": {
      "System.Text.Json": {
        "type": "Transitive",
        "resolved": "6.0.0"
      }
    }
  }
}"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = PackagesLockParser::extract_first_package(path);
        let deps = &package_data.dependencies;

        assert_eq!(deps.len(), 2);

        assert_eq!(deps[0].scope, Some("net5.0".to_string()));
        assert_eq!(deps[0].is_direct, Some(true));

        assert_eq!(deps[1].scope, Some("netstandard2.0".to_string()));
        assert_eq!(deps[1].is_direct, Some(false));
    }

    #[test]
    fn test_packages_lock_malformed() {
        let json = r#"{"this is": "not valid json"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = PackagesLockParser::extract_first_package(path);

        assert_eq!(package_data.package_type, Some(PackageType::Nuget));
        assert!(package_data.dependencies.is_empty());
    }

    #[test]
    fn test_nuspec_description_building() {
        let xml = r#"<?xml version="1.0"?>
<package xmlns="http://schemas.microsoft.com/packaging/2010/07/nuspec.xsd">
  <metadata>
    <id>TestPackage</id>
    <version>1.0.0</version>
    <title>Test Package Title</title>
    <summary>A short summary of the package</summary>
    <description>This is the full description of the package with more details.</description>
    <authors>Test Author</authors>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        assert_eq!(package_data.name, Some("TestPackage".to_string()));
        assert!(package_data.description.is_some());

        let desc = package_data.description.unwrap();
        assert!(
            desc.contains("Test Package Title"),
            "Description should contain title"
        );
        assert!(
            desc.contains("A short summary"),
            "Description should contain summary"
        );
        assert!(
            desc.contains("full description"),
            "Description should contain description"
        );

        let lines: Vec<&str> = desc.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "Description should have 3 lines (title, summary, description)"
        );
        assert_eq!(lines[0], "Test Package Title");
        assert_eq!(lines[1], "A short summary of the package");
        assert_eq!(
            lines[2],
            "This is the full description of the package with more details."
        );
    }

    #[test]
    fn test_nuspec_description_summary_only() {
        let xml = r#"<?xml version="1.0"?>
<package xmlns="http://schemas.microsoft.com/packaging/2010/07/nuspec.xsd">
  <metadata>
    <id>TestPackage</id>
    <version>1.0.0</version>
    <summary>Just a summary</summary>
    <authors>Test Author</authors>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        assert_eq!(package_data.description, Some("Just a summary".to_string()));
    }

    #[test]
    fn test_nuspec_description_title_same_as_name() {
        let xml = r#"<?xml version="1.0"?>
<package xmlns="http://schemas.microsoft.com/packaging/2010/07/nuspec.xsd">
  <metadata>
    <id>TestPackage</id>
    <version>1.0.0</version>
    <title>TestPackage</title>
    <description>Description text</description>
    <authors>Test Author</authors>
  </metadata>
</package>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        let path = temp_file.path();

        let package_data = NuspecParser::extract_first_package(path);

        assert_eq!(
            package_data.description,
            Some("Description text".to_string())
        );
        assert!(
            !package_data
                .description
                .as_ref()
                .unwrap()
                .contains("TestPackage\n"),
            "Title should not be prepended when it matches the package name"
        );
    }
}
