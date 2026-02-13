use crate::license_detection::LicenseDetectionEngine;
use crate::models::{FileInfo, FileInfoBuilder, FileType, LicenseDetection, Match};
use crate::parsers::try_parse_file;
use crate::scanner::ProcessResult;
use crate::utils::file::{get_creation_date, is_path_excluded};
use crate::utils::hash::{calculate_md5, calculate_sha1, calculate_sha256};
use crate::utils::language::detect_language;
use anyhow::Error;
use content_inspector::{ContentType, inspect};
use glob::Pattern;
use indicatif::ProgressBar;
use log::warn;
use mime_guess::from_path;
use rayon::prelude::*;
use std::fs::{self};
use std::path::Path;
use std::sync::Arc;

pub fn process<P: AsRef<Path>>(
    path: P,
    max_depth: usize,
    progress_bar: Arc<ProgressBar>,
    exclude_patterns: &[Pattern],
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
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
                let file_entry = process_file(path, metadata, license_engine.clone(), include_text);
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
                license_engine.clone(),
                include_text,
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

fn process_file(
    path: &Path,
    metadata: &fs::Metadata,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> FileInfo {
    let mut scan_errors: Vec<String> = vec![];
    let mut file_info_builder = FileInfoBuilder::default();

    if let Err(e) =
        extract_information_from_content(&mut file_info_builder, path, license_engine, include_text)
    {
        scan_errors.push(e.to_string());
    };

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
        ))
        .size(metadata.len())
        .date(get_creation_date(metadata))
        .scan_errors(scan_errors)
        .build()
        .expect("FileInformationBuild not completely initialized")
}

fn extract_information_from_content(
    file_info_builder: &mut FileInfoBuilder,
    path: &Path,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<(), Error> {
    let buffer = fs::read(path)?;

    file_info_builder
        .sha1(Some(calculate_sha1(&buffer)))
        .md5(Some(calculate_md5(&buffer)))
        .sha256(Some(calculate_sha256(&buffer)))
        .programming_language(Some(detect_language(path, &buffer)));

    if let Some(package_data) = try_parse_file(path) {
        file_info_builder.package_data(package_data);
        Ok(())
    } else if inspect(&buffer) == ContentType::UTF_8 {
        extract_license_information(
            file_info_builder,
            String::from_utf8_lossy(&buffer).into_owned(),
            license_engine,
            include_text,
        )
    } else {
        Ok(())
    }
}

fn extract_license_information(
    file_info_builder: &mut FileInfoBuilder,
    text_content: String,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<(), Error> {
    let Some(engine) = license_engine else {
        return Ok(());
    };

    match engine.detect(&text_content) {
        Ok(detections) => {
            let model_detections: Vec<LicenseDetection> = detections
                .into_iter()
                .filter_map(|d| convert_detection_to_model(d, include_text))
                .collect();

            if !model_detections.is_empty() {
                let expressions: Vec<String> = model_detections
                    .iter()
                    .filter(|d| !d.license_expression_spdx.is_empty())
                    .map(|d| d.license_expression_spdx.clone())
                    .collect();

                if !expressions.is_empty() {
                    let combined = crate::utils::spdx::combine_license_expressions(expressions);
                    if let Some(expr) = combined {
                        file_info_builder.license_expression(Some(expr));
                    }
                }
            }

            file_info_builder.license_detections(model_detections);
        }
        Err(e) => {
            warn!("License detection failed: {}", e);
        }
    }

    Ok(())
}

fn convert_detection_to_model(
    detection: crate::license_detection::LicenseDetection,
    include_text: bool,
) -> Option<LicenseDetection> {
    let license_expression = detection.license_expression?;
    let license_expression_spdx = detection.license_expression_spdx.unwrap_or_default();

    let matches: Vec<Match> = detection
        .matches
        .into_iter()
        .map(|m| Match {
            license_expression: m.license_expression,
            license_expression_spdx: m.license_expression_spdx,
            from_file: m.from_file,
            start_line: m.start_line,
            end_line: m.end_line,
            matcher: Some(m.matcher),
            score: m.score as f64,
            matched_length: Some(m.matched_length),
            match_coverage: Some(m.match_coverage as f64),
            rule_relevance: Some(m.rule_relevance as usize),
            rule_identifier: Some(m.rule_identifier),
            rule_url: Some(m.rule_url),
            matched_text: if include_text { m.matched_text } else { None },
        })
        .collect();

    Some(LicenseDetection {
        license_expression,
        license_expression_spdx,
        matches,
        identifier: detection.identifier,
    })
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
        for_packages: Vec::new(),
        scan_errors: Vec::new(),
    }
}
