use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

fn binary_path() -> String {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_provenant") {
        return path;
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push(if cfg!(windows) {
        "provenant.exe"
    } else {
        "provenant"
    });
    path.to_string_lossy().to_string()
}

fn create_scan_fixture() -> (TempDir, String) {
    let temp = TempDir::new().expect("failed to create temp dir");
    let scan_dir = temp.path().join("scan");
    fs::create_dir_all(&scan_dir).expect("failed to create scan dir");
    fs::write(scan_dir.join("a.txt"), "hello world\n").expect("failed to write fixture file");
    (temp, scan_dir.to_string_lossy().to_string())
}

#[test]
fn quiet_mode_suppresses_stderr_output() {
    let (temp, scan_dir) = create_scan_fixture();
    let output_file = temp.path().join("out.json");

    let output = Command::new(binary_path())
        .args([
            "--json-pp",
            output_file.to_str().expect("utf8 output path"),
            "--quiet",
            &scan_dir,
        ])
        .output()
        .expect("failed to run provenant");

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "quiet mode should not emit stderr"
    );
}

#[test]
fn default_mode_emits_summary_to_stderr() {
    let (temp, scan_dir) = create_scan_fixture();
    let output_file = temp.path().join("out.json");

    let output = Command::new(binary_path())
        .args([
            "--json-pp",
            output_file.to_str().expect("utf8 output path"),
            &scan_dir,
        ])
        .output()
        .expect("failed to run provenant");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Scanning done."));
}

#[test]
fn verbose_mode_emits_file_by_file_paths() {
    let (temp, scan_dir) = create_scan_fixture();
    let output_file = temp.path().join("out.json");

    let output = Command::new(binary_path())
        .args([
            "--json-pp",
            output_file.to_str().expect("utf8 output path"),
            "--verbose",
            &scan_dir,
        ])
        .output()
        .expect("failed to run provenant");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("a.txt"));
}
