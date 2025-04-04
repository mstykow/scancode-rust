use crate::askalono::{ScanStrategy, TextData};
use crate::models::{FileInfo, FileInfoBuilder, FileType, LicenseDetection, Match};
use crate::scanner::ProcessResult;
use crate::utils::file::{get_creation_date, is_path_excluded};
use crate::utils::hash::{calculate_md5, calculate_sha1, calculate_sha256};
use crate::utils::language::detect_language;
use anyhow::Error;
use content_inspector::{ContentType, inspect};
use glob::Pattern;
use indicatif::ProgressBar;
use mime_guess::from_path;
use rayon::prelude::*;
use std::fs::{self};
use std::path::Path;
use std::sync::Arc;

// License detection threshold - scores above this value are considered a match

pub fn process<P: AsRef<Path>>(
    path: P,
    max_depth: usize,
    progress_bar: Arc<ProgressBar>,
    exclude_patterns: &[Pattern],
    scan_strategy: &ScanStrategy,
) -> Result<ProcessResult, Error> {
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
                let file_entry = process_file(path, metadata, scan_strategy);
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
                scan_strategy,
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

fn process_file(path: &Path, metadata: &fs::Metadata, scan_strategy: &ScanStrategy) -> FileInfo {
    let mut scan_errors: Vec<String> = vec![];

    let mut file_info_builder = FileInfoBuilder::default();
    file_info_builder
        .size(metadata.len())
        .date(get_creation_date(metadata));
    add_path_information(&mut file_info_builder, path);
    if let Err(e) = extract_information_from_content(&mut file_info_builder, path, scan_strategy) {
        scan_errors.push(e.to_string());
    };
    file_info_builder.scan_errors(scan_errors);

    return file_info_builder
        .build()
        .expect("FileInformationBuild not completely initialized");
}

fn add_path_information(file_info_builder: &mut FileInfoBuilder, path: &Path) -> () {
    file_info_builder
        .name(path.file_name().unwrap().to_string_lossy().to_string())
        .base_name(
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        )
        .extension(
            path.extension()
                .map_or("".to_string(), |ext| format!(".{}", ext.to_string_lossy())),
        )
        .path(path.to_string_lossy().to_string())
        .file_type(FileType::File)
        .mime_type(Some(
            from_path(path)
                .first_or_octet_stream()
                .essence_str()
                .to_string(),
        ));
}

fn extract_information_from_content(
    file_info_builder: &mut FileInfoBuilder,
    path: &Path,
    scan_strategy: &ScanStrategy,
) -> Result<(), Error> {
    let buffer = fs::read(path)?;

    file_info_builder
        .sha1(Some(calculate_sha1(&buffer)))
        .md5(Some(calculate_md5(&buffer)))
        .sha256(Some(calculate_sha256(&buffer)))
        .programming_language(Some(detect_language(path, &buffer)));

    // Convert Vec<u8> to String only if it's valid UTF-8
    if inspect(&buffer) == ContentType::UTF_8 {
        extract_license_information(
            file_info_builder,
            String::from_utf8_lossy(&buffer).into_owned(),
            scan_strategy,
        )?;
        return Ok(());
    } else {
        return Ok(());
    };
}

fn extract_license_information(
    file_info_builder: &mut FileInfoBuilder,
    text_content: String,
    scan_strategy: &ScanStrategy,
) -> Result<(), Error> {
    // Analyze license with the text content
    if text_content.is_empty() {
        return Ok(());
    }

    let license_result = scan_strategy.scan(&TextData::from(text_content.as_str()))?;
    let license_expr = license_result
        .license
        .and_then(|x| Some(x.name.to_string()));

    let license_detections = license_result
        .containing
        .iter()
        .map(|detection| LicenseDetection {
            license_expression: detection.license.name.to_string(),
            matches: vec![Match {
                score: detection.score as f64,
                start_line: detection.line_range.0,
                end_line: detection.line_range.1,
                license_expression: detection.license.name.to_string(),
                matched_text: None, //TODO
                rule_identifier: "".to_string(),
            }],
        })
        .collect::<Vec<_>>();

    file_info_builder
        .license_expression(license_expr)
        .license_detections(license_detections);

    Ok(())
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
        package_data: Vec::new(), // TODO: implement
        license_expression: None,
        copyrights: Vec::new(),         // TODO: implement
        license_detections: Vec::new(), // TODO: implement
        urls: Vec::new(),               // TODO: implement
        scan_errors: Vec::new(),
    }
}
