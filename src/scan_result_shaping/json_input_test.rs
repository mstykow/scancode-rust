use super::*;
use crate::scan_result_shaping::test_fixtures::json_file;
use serde_json::json;
use std::fs;

#[test]
fn load_scan_from_json_reads_files_and_metadata_sections() {
    let temp_path = std::env::temp_dir().join("provenant-from-json-test.json");
    let content = json!({
        "files": [
            {
                "name": "main.rs",
                "base_name": "main",
                "extension": ".rs",
                "path": "src/main.rs",
                "type": "file",
                "size": 10,
                "programming_language": "Rust"
            }
        ],
        "packages": [],
        "dependencies": [],
        "license_references": [
            {"name":"MIT","short_name":"MIT","spdx_license_key":"MIT","text":"..."}
        ],
        "license_rule_references": []
    });
    fs::write(&temp_path, content.to_string()).expect("write json fixture");

    let parsed = load_scan_from_json(temp_path.to_str().expect("utf-8 path"))
        .expect("from-json loading should succeed");

    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].path, "src/main.rs");
    assert_eq!(parsed.license_references.len(), 1);

    let _ = fs::remove_file(temp_path);
}

#[test]
fn normalize_loaded_json_scan_applies_strip_root_per_loaded_input() {
    let mut loaded = JsonScanInput {
        files: vec![
            json_file("archive/root", crate::models::FileType::Directory),
            json_file("archive/root/src/main.rs", crate::models::FileType::File),
        ],
        packages: vec![],
        dependencies: vec![],
        license_references: vec![],
        license_rule_references: vec![],
        excluded_count: 0,
    };

    normalize_loaded_json_scan(&mut loaded, true, false);

    let paths: Vec<_> = loaded.files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, vec!["root", "src/main.rs"]);
}

#[test]
fn normalize_loaded_json_scan_trims_full_root_display_without_absolutizing() {
    let mut loaded = JsonScanInput {
        files: vec![json_file(
            "/tmp/archive/root/src/main.rs",
            crate::models::FileType::File,
        )],
        packages: vec![],
        dependencies: vec![],
        license_references: vec![],
        license_rule_references: vec![],
        excluded_count: 0,
    };

    normalize_loaded_json_scan(&mut loaded, false, true);

    assert_eq!(loaded.files[0].path, "tmp/archive/root/src/main.rs");
}
