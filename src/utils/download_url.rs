use urlencoding::encode;

#[allow(dead_code)]
pub fn npm_download_url(
    namespace: Option<&str>,
    name: &str,
    version: &str,
    registry: Option<&str>,
) -> String {
    let registry = registry.unwrap_or("https://registry.npmjs.org");
    let name_part = if let Some(ns) = namespace {
        format!("{}/{}", ns, name)
    } else {
        name.to_string()
    };
    format!("{}/-/{}-{}.tgz", registry, encode(&name_part), version)
}

#[allow(dead_code)]
pub fn pypi_download_url(name: &str, version: &str) -> String {
    let normalized = name.to_lowercase().replace('-', "_");
    let first_char = normalized.chars().next().unwrap_or('_');
    format!(
        "https://files.pythonhosted.org/packages/source/{}/{}/{}-{}.tar.gz",
        first_char, normalized, normalized, version
    )
}

#[allow(dead_code)]
pub fn maven_download_url(
    group_id: &str,
    artifact_id: &str,
    version: &str,
    classifier: Option<&str>,
    extension: &str,
    repository_url: Option<&str>,
) -> String {
    let repo = repository_url.unwrap_or("https://repo1.maven.org/maven2");
    let group_path = group_id.replace('.', "/");
    let file_name = match classifier {
        Some(c) => format!("{}-{}-{}.{}", artifact_id, version, c, extension),
        None => format!("{}-{}.{}", artifact_id, version, extension),
    };
    format!(
        "{}/{}/{}/{}/{}",
        repo, group_path, artifact_id, version, file_name
    )
}

#[allow(dead_code)]
pub fn cargo_download_url(name: &str, version: &str) -> String {
    format!(
        "https://crates.io/api/v1/crates/{}/{}/download",
        name, version
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_scoped_package_url() {
        let url = npm_download_url(Some("@babel"), "core", "7.0.0", None);
        // The name_part gets URL encoded, so @babel becomes %40babel
        assert!(url.contains("%40babel") || url.contains("@babel"));
        assert!(url.contains("7.0.0.tgz"));
    }

    #[test]
    fn test_npm_unscoped_package_url() {
        let url = npm_download_url(None, "lodash", "4.17.21", None);
        assert_eq!(url, "https://registry.npmjs.org/-/lodash-4.17.21.tgz");
    }

    #[test]
    fn test_pypi_url_normalization() {
        let url = pypi_download_url("my-package", "1.0.0");
        assert!(url.contains("my_package"));
        assert!(url.contains("my_package-1.0.0.tar.gz"));
    }

    #[test]
    fn test_pypi_single_letter_package() {
        let url = pypi_download_url("a", "1.0.0");
        assert!(url.starts_with("https://files.pythonhosted.org/packages/source/a/a/"));
    }

    #[test]
    fn test_maven_url_with_classifier() {
        let url = maven_download_url("org.example", "lib", "1.0", Some("sources"), "jar", None);
        assert!(url.contains("org/example/lib/1.0/lib-1.0-sources.jar"));
    }

    #[test]
    fn test_maven_url_without_classifier() {
        let url = maven_download_url("org.example", "lib", "1.0", None, "jar", None);
        assert!(url.contains("org/example/lib/1.0/lib-1.0.jar"));
    }

    #[test]
    fn test_cargo_url_format() {
        let url = cargo_download_url("serde", "1.0.0");
        assert_eq!(url, "https://crates.io/api/v1/crates/serde/1.0.0/download");
    }
}
