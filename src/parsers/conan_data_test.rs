#[cfg(test)]
mod tests {
    use super::super::PackageParser;
    use super::super::conan_data::*;
    use crate::models::DatasourceId;
    use std::path::PathBuf;

    #[test]
    fn test_is_match() {
        assert!(ConanDataParser::is_match(&PathBuf::from(
            "/path/to/conandata.yml"
        )));
        assert!(ConanDataParser::is_match(&PathBuf::from(
            "some/dir/conandata.yml"
        )));
        assert!(!ConanDataParser::is_match(&PathBuf::from("conanfile.py")));
        assert!(!ConanDataParser::is_match(&PathBuf::from("conandata.yaml")));
        assert!(!ConanDataParser::is_match(&PathBuf::from("package.json")));
    }

    #[test]
    fn test_parse_basic_conandata() {
        let content = r#"
sources:
  "1.0.0":
    url: "https://example.com/package-1.0.0.tar.gz"
    sha256: "abc123def456"
  "2.0.0":
    url: "https://example.com/package-2.0.0.tar.gz"
    sha256: "def456abc789"
"#;
        let packages = parse_conandata_yml(content);
        assert_eq!(packages.len(), 2);

        // Check first package
        let pkg1 = packages
            .iter()
            .find(|p| p.version.as_deref() == Some("1.0.0"));
        assert!(pkg1.is_some());
        let pkg1 = pkg1.unwrap();
        assert_eq!(pkg1.package_type.as_deref(), Some("conan"));
        assert_eq!(pkg1.primary_language.as_deref(), Some("C++"));
        assert_eq!(
            pkg1.download_url.as_deref(),
            Some("https://example.com/package-1.0.0.tar.gz")
        );
        assert_eq!(pkg1.sha256.as_deref(), Some("abc123def456"));
        assert_eq!(pkg1.datasource_id, Some(DatasourceId::ConanConanDataYml));

        // Check second package
        let pkg2 = packages
            .iter()
            .find(|p| p.version.as_deref() == Some("2.0.0"));
        assert!(pkg2.is_some());
        let pkg2 = pkg2.unwrap();
        assert_eq!(
            pkg2.download_url.as_deref(),
            Some("https://example.com/package-2.0.0.tar.gz")
        );
        assert_eq!(pkg2.sha256.as_deref(), Some("def456abc789"));
    }

    #[test]
    fn test_parse_multiple_urls() {
        let content = r#"
sources:
  "1.5.0":
    url:
      - "https://mirror1.com/package-1.5.0.tar.gz"
      - "https://mirror2.com/package-1.5.0.tar.gz"
    sha256: "xyz789"
"#;
        let packages = parse_conandata_yml(content);
        assert_eq!(packages.len(), 1);

        let pkg = &packages[0];
        // Should use first URL from list
        assert_eq!(
            pkg.download_url.as_deref(),
            Some("https://mirror1.com/package-1.5.0.tar.gz")
        );
        assert_eq!(pkg.sha256.as_deref(), Some("xyz789"));
    }

    #[test]
    fn test_parse_missing_fields() {
        let content = r#"
sources:
  "3.0.0":
    url: "https://example.com/package-3.0.0.tar.gz"
  "4.0.0":
    sha256: "onlyhash"
"#;
        let packages = parse_conandata_yml(content);
        assert_eq!(packages.len(), 2);

        let pkg1 = packages
            .iter()
            .find(|p| p.version.as_deref() == Some("3.0.0"));
        assert!(pkg1.is_some());
        let pkg1 = pkg1.unwrap();
        assert_eq!(
            pkg1.download_url.as_deref(),
            Some("https://example.com/package-3.0.0.tar.gz")
        );
        assert_eq!(pkg1.sha256, None);

        let pkg2 = packages
            .iter()
            .find(|p| p.version.as_deref() == Some("4.0.0"));
        assert!(pkg2.is_some());
        let pkg2 = pkg2.unwrap();
        assert_eq!(pkg2.download_url, None);
        assert_eq!(pkg2.sha256.as_deref(), Some("onlyhash"));
    }

    #[test]
    fn test_parse_empty_sources() {
        let content = r#"
sources: {}
"#;
        let packages = parse_conandata_yml(content);
        // Should return default package when sources is empty
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].package_type.as_deref(), Some("conan"));
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let content = "this is not valid yaml: [[[";
        let packages = parse_conandata_yml(content);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].package_type.as_deref(), Some("conan"));
    }

    #[test]
    fn test_parse_with_patches() {
        let content = r#"
sources:
  "1.12.0":
    url: "https://github.com/libcpr/cpr/archive/refs/tags/1.12.0.tar.gz"
    sha256: "f64b501de66e163d6a278fbb6a95f395ee873b7a66c905dd785eae107266a709"
patches:
  "1.12.0":
    - patch_file: "patches/008-1.12.0-remove-warning-flags.patch"
      patch_description: "disable warning flags and warning as error"
      patch_type: "portability"
    - patch_file: "patches/009-1.12.0-windows-msvc-runtime.patch"
      patch_description: "dont hardcode value of CMAKE_MSVC_RUNTIME_LIBRARY"
      patch_type: "conan"
"#;
        let packages = parse_conandata_yml(content);
        assert_eq!(packages.len(), 1);

        let pkg = &packages[0];
        assert_eq!(pkg.version.as_deref(), Some("1.12.0"));
        assert_eq!(
            pkg.download_url.as_deref(),
            Some("https://github.com/libcpr/cpr/archive/refs/tags/1.12.0.tar.gz")
        );

        assert!(pkg.extra_data.is_some());
        let extra = pkg.extra_data.as_ref().unwrap();
        assert!(extra.contains_key("patches"));

        let patches = extra.get("patches").unwrap();
        assert!(patches.is_array());
        let patches_array = patches.as_array().unwrap();
        assert_eq!(patches_array.len(), 2);

        let patch1 = &patches_array[0];
        assert_eq!(
            patch1.get("patch_file").and_then(|v| v.as_str()),
            Some("patches/008-1.12.0-remove-warning-flags.patch")
        );
        assert_eq!(
            patch1.get("patch_description").and_then(|v| v.as_str()),
            Some("disable warning flags and warning as error")
        );
        assert_eq!(
            patch1.get("patch_type").and_then(|v| v.as_str()),
            Some("portability")
        );
    }

    #[test]
    fn test_parse_with_mirror_urls() {
        let content = r#"
sources:
  "1.0.0":
    url:
      - "https://mirror1.com/package.tar.gz"
      - "https://mirror2.com/package.tar.gz"
      - "https://mirror3.com/package.tar.gz"
    sha256: "abc123"
"#;
        let packages = parse_conandata_yml(content);
        assert_eq!(packages.len(), 1);

        let pkg = &packages[0];
        assert_eq!(
            pkg.download_url.as_deref(),
            Some("https://mirror1.com/package.tar.gz")
        );

        assert!(pkg.extra_data.is_some());
        let extra = pkg.extra_data.as_ref().unwrap();
        assert!(extra.contains_key("mirror_urls"));

        let mirrors = extra.get("mirror_urls").unwrap();
        assert!(mirrors.is_array());
        let mirrors_array = mirrors.as_array().unwrap();
        assert_eq!(mirrors_array.len(), 3);
    }

    #[test]
    fn test_parse_patches_without_matching_source() {
        let content = r#"
sources:
  "1.0.0":
    url: "https://example.com/package.tar.gz"
patches:
  "2.0.0":
    - patch_file: "some.patch"
"#;
        let packages = parse_conandata_yml(content);
        assert_eq!(packages.len(), 1);

        let pkg = &packages[0];
        assert_eq!(pkg.version.as_deref(), Some("1.0.0"));
        assert!(
            pkg.extra_data.is_none() || !pkg.extra_data.as_ref().unwrap().contains_key("patches")
        );
    }
}
