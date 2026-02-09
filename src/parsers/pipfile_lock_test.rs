#[cfg(test)]
mod tests {
    use crate::parsers::{PackageParser, PipfileLockParser};
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_parse_pipfile_lock_golden() {
        let test_file = PathBuf::from("testdata/python/golden/pipfile_lock/Pipfile.lock");
        let expected_file =
            PathBuf::from("testdata/python/golden/pipfile_lock/Pipfile.lock-expected.json");

        let package_data = PipfileLockParser::extract_first_package(&test_file);

        assert_eq!(package_data.dependencies.len(), 9);
        assert_eq!(
            package_data.sha256.as_deref(),
            Some("813f8e1b624fd42eee7d681228d7aca1fce209e1d60bf21c3eb33a73f7268d57")
        );
        assert!(
            package_data
                .dependencies
                .iter()
                .all(|dep| dep.scope.as_deref() != Some("develop"))
        );

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }

    #[test]
    fn test_pipfile_lock_with_develop_dependencies() {
        use std::fs;
        use tempfile::tempdir;

        let content = r#"{
    "_meta": {
        "hash": {"sha256": "test-hash"},
        "pipfile-spec": 6
    },
    "default": {
        "requests": {
            "hashes": ["sha256:abc123"],
            "version": "==2.28.0"
        }
    },
    "develop": {
        "pytest": {
            "hashes": ["sha256:def456"],
            "version": "==7.2.0"
        },
        "black": {
            "hashes": ["sha256:ghi789"],
            "version": "==23.1.0"
        }
    }
}"#;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("Pipfile.lock");
        fs::write(&file_path, content).unwrap();

        let package_data = PipfileLockParser::extract_first_package(&file_path);

        assert_eq!(package_data.dependencies.len(), 3);

        let default_deps: Vec<_> = package_data
            .dependencies
            .iter()
            .filter(|dep| dep.scope.as_deref() == Some("install"))
            .collect();
        assert_eq!(default_deps.len(), 1);
        assert_eq!(default_deps[0].is_runtime, Some(true));

        let develop_deps: Vec<_> = package_data
            .dependencies
            .iter()
            .filter(|dep| dep.scope.as_deref() == Some("develop"))
            .collect();
        assert_eq!(develop_deps.len(), 2);
        for dep in develop_deps {
            assert_eq!(dep.scope, Some("develop".to_string()));
            assert_eq!(dep.is_runtime, Some(false));
        }
    }
}
