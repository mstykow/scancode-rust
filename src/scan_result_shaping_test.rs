use super::*;
use crate::models::{Author, Copyright, FileInfo, FileType, OutputEmail, OutputURL};
use glob::Pattern;
use std::collections::HashSet;
use std::path::Path;

fn file(path: &str) -> FileInfo {
    FileInfo::new(
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .extension()
            .and_then(|n| n.to_str())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default(),
        path.to_string(),
        FileType::File,
        None,
        1,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

fn dir(path: &str) -> FileInfo {
    FileInfo::new(
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        String::new(),
        path.to_string(),
        FileType::Directory,
        None,
        0,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn include_filter_keeps_matching_files_and_parent_dirs() {
    let mut files = vec![
        dir("project"),
        dir("project/src"),
        file("project/src/main.rs"),
        file("project/README.md"),
    ];
    let include_patterns = vec![Pattern::new("*.rs").expect("valid pattern")];

    apply_include_filter(&mut files, &include_patterns);

    let paths: HashSet<_> = files.into_iter().map(|f| f.path).collect();
    assert!(paths.contains("project/src/main.rs"));
    assert!(paths.contains("project/src"));
    assert!(paths.contains("project"));
    assert!(!paths.contains("project/README.md"));
}

#[test]
fn only_findings_keeps_file_with_findings_and_parent_dirs() {
    let mut files = vec![dir("project"), file("project/a.txt"), file("project/b.txt")];
    files[2].copyrights = vec![Copyright {
        copyright: "Copyright Example".to_string(),
        start_line: 1,
        end_line: 1,
    }];

    apply_only_findings_filter(&mut files);

    let paths: HashSet<_> = files.into_iter().map(|f| f.path).collect();
    assert!(paths.contains("project"));
    assert!(paths.contains("project/b.txt"));
    assert!(!paths.contains("project/a.txt"));
}

#[test]
fn filter_redundant_clues_dedupes_exact_duplicates() {
    let mut files = vec![file("project/a.txt")];
    files[0].authors = vec![
        Author {
            author: "Jane".to_string(),
            start_line: 2,
            end_line: 2,
        },
        Author {
            author: "Jane".to_string(),
            start_line: 2,
            end_line: 2,
        },
    ];
    files[0].emails = vec![
        OutputEmail {
            email: "a@example.com".to_string(),
            start_line: 3,
            end_line: 3,
        },
        OutputEmail {
            email: "a@example.com".to_string(),
            start_line: 3,
            end_line: 3,
        },
    ];
    files[0].urls = vec![
        OutputURL {
            url: "https://example.com".to_string(),
            start_line: 4,
            end_line: 4,
        },
        OutputURL {
            url: "https://example.com".to_string(),
            start_line: 4,
            end_line: 4,
        },
    ];

    filter_redundant_clues(&mut files);

    assert_eq!(files[0].authors.len(), 1);
    assert_eq!(files[0].emails.len(), 1);
    assert_eq!(files[0].urls.len(), 1);
}

#[test]
fn normalize_paths_strip_root_removes_scan_root_prefix() {
    let mut files = vec![file("project/src/main.rs")];
    normalize_paths(&mut files, "project", true, false);
    assert_eq!(files[0].path, "src/main.rs");
}

#[test]
fn mark_source_sets_directory_flags_at_threshold() {
    let mut files = vec![
        dir("project"),
        dir("project/src"),
        file("project/src/a.rs"),
        file("project/src/b.rs"),
        file("project/src/c.txt"),
    ];
    files[2].programming_language = Some("Rust".to_string());
    files[3].programming_language = Some("Rust".to_string());

    apply_mark_source(&mut files);

    let src = files
        .iter()
        .find(|f| f.path == "project/src")
        .expect("src directory exists");
    assert_eq!(src.is_source, None);
    assert_eq!(src.source_count, None);

    files[4].programming_language = Some("Rust".to_string());
    apply_mark_source(&mut files);

    let src_after = files
        .iter()
        .find(|f| f.path == "project/src")
        .expect("src directory exists");
    assert_eq!(src_after.is_source, Some(true));
    assert_eq!(src_after.source_count, Some(3));
}

#[test]
fn mark_source_ignores_go_test_only_files_for_directory_threshold() {
    let mut files = vec![
        dir("module"),
        file("module/main.go"),
        file("module/helper.go"),
        file("module/helper_test.go"),
    ];
    files[1].programming_language = Some("Go".to_string());
    files[2].programming_language = Some("Go".to_string());
    files[3].programming_language = Some("Go".to_string());
    files[3].is_source = Some(false);

    apply_mark_source(&mut files);

    let module_dir = files
        .iter()
        .find(|f| f.path == "module")
        .expect("module dir exists");
    assert_eq!(module_dir.is_source, Some(true));
    assert_eq!(module_dir.source_count, Some(2));
}
