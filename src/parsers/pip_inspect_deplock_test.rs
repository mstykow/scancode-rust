#[cfg(test)]
mod tests {
    use super::super::PackageParser;
    use super::super::pip_inspect_deplock::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_match() {
        assert!(PipInspectDeplockParser::is_match(&PathBuf::from(
            "/path/to/pip-inspect.deplock"
        )));
        assert!(PipInspectDeplockParser::is_match(&PathBuf::from(
            "some/dir/pip-inspect.deplock"
        )));
        assert!(!PipInspectDeplockParser::is_match(&PathBuf::from(
            "pip-inspect.json"
        )));
        assert!(!PipInspectDeplockParser::is_match(&PathBuf::from(
            "package.json"
        )));
    }

    #[test]
    fn test_parse_basic_pip_inspect() {
        let content = r#"{
  "version": "1",
  "pip_version": "23.0.1",
  "installed": [
    {
      "metadata": {
        "name": "requests",
        "version": "2.31.0",
        "license": "Apache 2.0",
        "description": "Python HTTP for Humans.",
        "keywords": "http,requests"
      },
      "requested": true,
      "direct_url": {
        "url": "file:///path/to/requests"
      }
    },
    {
      "metadata": {
        "name": "urllib3",
        "version": "2.0.0",
        "license": "MIT"
      },
      "requested": false
    }
  ]
}"#;
        let pkg = parse_pip_inspect_deplock(content);

        assert_eq!(pkg.name.as_deref(), Some("requests"));
        assert_eq!(pkg.version.as_deref(), Some("2.31.0"));
        assert_eq!(
            pkg.extracted_license_statement.as_deref(),
            Some("Apache 2.0")
        );
        assert_eq!(pkg.description.as_deref(), Some("Python HTTP for Humans."));
        assert_eq!(pkg.keywords, vec!["http,requests".to_string()]);
        assert_eq!(pkg.package_type.as_deref(), Some("pypi"));
        assert_eq!(pkg.primary_language.as_deref(), Some("Python"));
        assert_eq!(pkg.datasource_id.as_deref(), Some("pypi_inspect_deplock"));
        assert!(pkg.is_virtual);

        // Check extra_data
        assert!(pkg.extra_data.is_some());
        let extra = pkg.extra_data.as_ref().unwrap();
        assert_eq!(
            extra.get("pip_version").and_then(|v| v.as_str()),
            Some("23.0.1")
        );
        assert_eq!(
            extra.get("inspect_version").and_then(|v| v.as_str()),
            Some("1")
        );
    }

    #[test]
    fn test_parse_no_direct_url() {
        let content = r#"{
  "version": "1",
  "installed": [
    {
      "metadata": {
        "name": "my-package",
        "version": "1.0.0",
        "license": "MIT"
      },
      "requested": true
    }
  ]
}"#;
        let pkg = parse_pip_inspect_deplock(content);

        // Should find the requested package even without direct_url
        assert_eq!(pkg.name.as_deref(), Some("my-package"));
        assert_eq!(pkg.version.as_deref(), Some("1.0.0"));
        assert_eq!(pkg.extracted_license_statement.as_deref(), Some("MIT"));
    }

    #[test]
    fn test_parse_no_installed_packages() {
        let content = r#"{
  "version": "1",
  "pip_version": "23.0.1",
  "installed": []
}"#;
        let pkg = parse_pip_inspect_deplock(content);

        // Should return default package
        assert_eq!(pkg.package_type.as_deref(), Some("pypi"));
        assert_eq!(pkg.datasource_id.as_deref(), Some("pypi_inspect_deplock"));
    }

    #[test]
    fn test_parse_invalid_json() {
        let content = "this is not valid json {{{";
        let pkg = parse_pip_inspect_deplock(content);

        // Should return default package on parse error
        assert_eq!(pkg.package_type.as_deref(), Some("pypi"));
        assert_eq!(pkg.datasource_id.as_deref(), Some("pypi_inspect_deplock"));
    }
}
