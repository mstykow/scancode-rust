use super::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_match_cargo_lock() {
        assert!(CargoLockParser::is_match(&PathBuf::from("Cargo.lock")));
        assert!(CargoLockParser::is_match(&PathBuf::from(
            "/path/to/project/Cargo.lock"
        )));
    }

    #[test]
    fn test_is_not_match_cargo_toml() {
        assert!(!CargoLockParser::is_match(&PathBuf::from("Cargo.toml")));
        assert!(!CargoLockParser::is_match(&PathBuf::from(
            "package-lock.json"
        )));
    }

    #[test]
    fn test_extract_from_real_cargo_lock() {
        let lock_path = PathBuf::from("Cargo.lock");
        let package_data = CargoLockParser::extract_first_package(&lock_path);

        assert_eq!(package_data.package_type, Some("cargo".to_string()));
        // The first package is alphabetically first, not the root
        assert!(package_data.name.is_some());
        assert!(package_data.version.is_some());
        assert!(!package_data.dependencies.is_empty());
    }

    #[test]
    fn test_extract_from_testdata() {
        let lock_path = PathBuf::from("testdata/cargo/Cargo-lock-basic.lock");
        let package_data = CargoLockParser::extract_first_package(&lock_path);

        assert_eq!(package_data.package_type, Some("cargo".to_string()));
        assert_eq!(package_data.name, Some("test-project".to_string()));
        assert_eq!(package_data.version, Some("0.1.0".to_string()));
        assert!(package_data.sha256.is_some());
        assert!(!package_data.dependencies.is_empty());

        assert_eq!(
            package_data.purl,
            Some("pkg:cargo/test-project@0.1.0".to_string())
        );
        assert_eq!(
            package_data.api_data_url,
            Some("https://crates.io/api/v1/crates/test-project/0.1.0".to_string())
        );
    }

    #[test]
    fn test_extract_dependencies() {
        let lock_path = PathBuf::from("testdata/cargo/Cargo-lock-deps.lock");
        let package_data = CargoLockParser::extract_first_package(&lock_path);

        let deps = package_data.dependencies;
        assert!(!deps.is_empty());

        let serde_dep = deps
            .iter()
            .find(|d| d.purl.as_ref().is_some_and(|p| p.contains("serde")));
        assert!(serde_dep.is_some());

        if let Some(dep) = serde_dep {
            assert_eq!(dep.is_pinned, Some(true));
            assert_eq!(dep.is_runtime, Some(true));
            assert_eq!(dep.scope, Some("dependencies".to_string()));
        }
    }

    #[test]
    fn test_datasource_id() {
        let lock_path = PathBuf::from("testdata/cargo/Cargo-lock-basic.lock");
        let package_data = CargoLockParser::extract_first_package(&lock_path);

        assert_eq!(package_data.datasource_id, Some("cargo_lock".to_string()));
    }

    #[test]
    fn test_is_direct_flag() {
        let lock_path = PathBuf::from("testdata/cargo/Cargo-lock-deps.lock");
        let package_data = CargoLockParser::extract_first_package(&lock_path);

        let deps = package_data.dependencies;
        assert!(!deps.is_empty());

        let direct_deps: Vec<_> = deps.iter().filter(|d| d.is_direct == Some(true)).collect();
        let transitive_deps: Vec<_> = deps.iter().filter(|d| d.is_direct == Some(false)).collect();

        assert!(
            !direct_deps.is_empty(),
            "Should have at least one direct dependency"
        );
        assert!(
            !transitive_deps.is_empty(),
            "Should have at least one transitive dependency"
        );
    }

    #[test]
    fn test_cargo_lock_runtime_dependencies_only() {
        // Cargo.lock only contains resolved runtime dependencies by design.
        // Dev dependencies and build dependencies are NOT included in the lockfile.
        //
        // This is intentional Cargo behavior, not a parser limitation:
        // - Dev dependencies are only used during `cargo test` and `cargo bench`
        // - Build dependencies are only used during build scripts
        // - Neither affect the final binary or library
        //
        // Therefore, all dependencies in Cargo.lock have scope="dependencies"
        // and is_runtime=true. This test documents and verifies this behavior.

        let lock_path = PathBuf::from("testdata/cargo/Cargo-lock-deps.lock");
        let package_data = CargoLockParser::extract_first_package(&lock_path);

        let deps = package_data.dependencies;
        assert!(!deps.is_empty());

        // Verify all dependencies are runtime dependencies
        for dep in &deps {
            assert_eq!(
                dep.scope,
                Some("dependencies".to_string()),
                "All Cargo.lock dependencies should have scope='dependencies'"
            );
            assert_eq!(
                dep.is_runtime,
                Some(true),
                "All Cargo.lock dependencies should be runtime dependencies"
            );
        }

        // Verify no dev or build dependencies exist
        let non_runtime_deps: Vec<_> = deps
            .iter()
            .filter(|d| {
                d.scope
                    .as_ref()
                    .is_some_and(|s| s.contains("dev") || s.contains("build"))
            })
            .collect();

        assert!(
            non_runtime_deps.is_empty(),
            "Cargo.lock should not contain dev or build dependencies"
        );
    }
}
