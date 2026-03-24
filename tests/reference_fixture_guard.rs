use std::fs;
use std::path::{Path, PathBuf};

fn is_runtime_reference_fixture_usage(line: &str, forbidden: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return false;
    }

    let constructors = [
        "PathBuf::from(",
        "Path::new(",
        "read_to_string(",
        "read(",
        "copy(",
        "copy2(",
    ];

    line.contains(forbidden) && constructors.iter().any(|ctor| line.contains(ctor))
}

fn rust_files_under(root: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files_under(&path, files);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
}

#[test]
fn tests_do_not_read_runtime_fixtures_from_reference_tree() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let forbidden = ["reference", "scancode-toolkit", "tests"].join("/");
    let mut files = Vec::new();
    rust_files_under(&repo_root.join("src"), &mut files);
    rust_files_under(&repo_root.join("tests"), &mut files);

    let mut violations = Vec::new();
    for file in files {
        if file.file_name().and_then(|name| name.to_str()) == Some("reference_fixture_guard.rs") {
            continue;
        }
        let content = fs::read_to_string(&file).expect("rust source should be readable");
        for (line_no, line) in content.lines().enumerate() {
            if is_runtime_reference_fixture_usage(line, &forbidden) {
                violations.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    line_no + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "test/runtime fixture references to reference/scancode-toolkit are forbidden:\n{}",
        violations.join("\n")
    );
}
