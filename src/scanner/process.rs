use crate::models::{FileInfo, FileType};
use crate::scanner::ProcessResult;
use crate::utils::file::{get_creation_date, is_path_excluded};
use crate::utils::hash::{calculate_md5, calculate_sha1, calculate_sha256};
use crate::utils::language::detect_language;
use crate::askalono::{Store, TextData};
use content_inspector::{ContentType, inspect};
use glob::Pattern;
use indicatif::ProgressBar;
use mime_guess::from_path;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

// License detection threshold - scores above this value are considered a match
const LICENSE_DETECTION_THRESHOLD: f32 = 0.9;

pub fn process<P: AsRef<Path>>(
    path: P,
    max_depth: usize,
    progress_bar: Arc<ProgressBar>,
    exclude_patterns: &[Pattern],
    store: &Store,
) -> std::io::Result<ProcessResult> {
    let path = path.as_ref();

    if is_path_excluded(path, exclude_patterns) {
        return Ok(ProcessResult {
            files: Vec::new(),
            excluded_count: 1,
        });
    }

    let mut all_files = Vec::new();
    let mut total_excluded = 0;

    // Read directory entries and group by exclusion status and type
    let entries: Vec<_> = fs::read_dir(path)?.filter_map(Result::ok).collect();

    let mut file_entries = Vec::new();
    let mut dir_entries = Vec::new();

    for entry in entries {
        let path = entry.path();

        // Check exclusion only once per path
        if is_path_excluded(&path, exclude_patterns) {
            total_excluded += 1;
            continue;
        }

        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => file_entries.push((path, metadata)),
            Ok(metadata) if path.is_dir() => dir_entries.push((path, metadata)),
            _ => continue,
        }
    }

    // Process files in parallel
    all_files.append(
        &mut file_entries
            .par_iter()
            .map(|(path, metadata)| {
                let file_entry = process_file(path, metadata, store);
                progress_bar.inc(1);
                file_entry
            })
            .collect(),
    );

    // Process directories
    for (path, metadata) in dir_entries {
        all_files.push(process_directory(&path, &metadata));

        if max_depth > 0 {
            match process(
                &path,
                max_depth - 1,
                progress_bar.clone(),
                exclude_patterns,
                store,
            ) {
                Ok(mut result) => {
                    all_files.append(&mut result.files);
                    total_excluded += result.excluded_count;
                }
                Err(e) => eprintln!("Error processing directory {}: {}", path.display(), e),
            }
        }
    }

    Ok(ProcessResult {
        files: all_files,
        excluded_count: total_excluded,
    })
}

fn process_file(path: &Path, metadata: &fs::Metadata, store: &Store) -> FileInfo {
    let size = metadata.len();
    let mut scan_errors = Vec::new();

    let (sha1, md5, sha256, programming_language, license_result) =
        match read_file_data(path, size, store) {
            Ok((sha1, md5, sha256, lang, license)) => {
                (Some(sha1), Some(md5), Some(sha256), Some(lang), license)
            }
            Err(e) => {
                scan_errors.push(e.to_string());
                (None, None, None, None, None)
            }
        };

    FileInfo {
        name: path.file_name().unwrap().to_string_lossy().to_string(),
        base_name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        extension: path
            .extension()
            .map_or("".to_string(), |ext| format!(".{}", ext.to_string_lossy())),
        path: path.to_string_lossy().to_string(),
        file_type: FileType::File,
        mime_type: Some(
            from_path(path)
                .first_or_octet_stream()
                .essence_str()
                .to_string(),
        ),
        size,
        date: get_creation_date(metadata),
        sha1,
        md5,
        sha256,
        programming_language,
        license_expression: license_result,
        copyrights: Vec::new(), // TODO: implement
        license_detections: Vec::new(), // TODO: implement
        urls: Vec::new(), // TODO: implement
        scan_errors,
    }
}

fn read_file_data(
    path: &Path,
    size: u64,
    store: &Store,
) -> std::io::Result<(String, String, String, String, Option<String>)> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::with_capacity(size as usize);
    file.read_to_end(&mut buffer)?;

    // Convert Vec<u8> to String only if it's valid UTF-8
    let text_content = if inspect(&buffer) == ContentType::UTF_8 {
        String::from_utf8_lossy(&buffer).into_owned()
    } else {
        String::new() // Empty string for binary files
    };

    // Analyze license with the text content
    let license_result = if !text_content.is_empty() {
        let result = store.analyze(&TextData::from(text_content.as_str()));
        if result.score > LICENSE_DETECTION_THRESHOLD {
            Some(result.name.to_owned())
        } else {
            None
        }
    } else {
        None
    };

    let language = detect_language(path, &buffer);

    Ok((
        calculate_sha1(&buffer),
        calculate_md5(&buffer),
        calculate_sha256(&buffer),
        language,
        license_result,
    ))
}

fn process_directory(path: &Path, metadata: &fs::Metadata) -> FileInfo {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let base_name = name.clone(); // For directories, base_name is the same as name

    FileInfo {
        name,
        base_name,
        extension: "".to_string(),
        path: path.to_string_lossy().to_string(),
        file_type: FileType::Directory,
        mime_type: None,
        size: 0,
        date: get_creation_date(metadata),
        sha1: None,
        md5: None,
        sha256: None,
        programming_language: None,
        license_expression: None,
        copyrights: Vec::new(), // TODO: implement
        license_detections: Vec::new(), // TODO: implement
        urls: Vec::new(), // TODO: implement
        scan_errors: Vec::new(),
    }
}
