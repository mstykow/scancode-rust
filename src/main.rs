use anyhow::{Result, anyhow};
use askalono::ScanStrategy;
use chrono::Utc;
use clap::Parser;
use glob::Pattern;
use include_dir::{Dir, include_dir};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use serde_json::{Value, from_str};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::askalono::{Store, TextData};
use crate::cli::Cli;
use crate::models::{ExtraData, Header, Output, SCANCODE_OUTPUT_FORMAT_VERSION, SystemEnvironment};
use crate::output::{OutputWriteConfig, write_output_file};
use crate::scanner::{TextDetectionOptions, count, process, process_with_options};

mod askalono;
mod assembly;
mod cli;
#[allow(dead_code, unused_imports)]
mod copyright;
mod finder;
mod models;
mod output;
mod parsers;
mod scanner;
mod utils;

#[cfg(test)]
mod test_utils;

const LICENSE_DETECTION_THRESHOLD: f32 = 0.9;

fn main() -> std::io::Result<()> {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let start_time = Utc::now();

    if cli.from_json && (cli.email || cli.url) {
        return Err(anyhow!(
            "When using --from-json, file scan options like --email/--url are not allowed"
        ));
    }

    let exclude_patterns = compile_exclude_patterns(&cli.exclude);
    let include_patterns = compile_include_patterns(&cli.include);

    if !cli.quiet {
        eprintln!("Exclusion patterns: {:?}", cli.exclude);
    }

    let (
        mut scan_result,
        total_dirs,
        preloaded_assembly,
        preloaded_license_references,
        preloaded_license_rule_references,
    ) = if cli.from_json {
        let mut loaded = load_scan_from_json(&cli.dir_path)?;
        loaded.excluded_count = 0;
        let directories_count = loaded
            .files
            .iter()
            .filter(|f| f.file_type == crate::models::FileType::Directory)
            .count();
        (
            scanner::ProcessResult {
                files: loaded.files,
                excluded_count: loaded.excluded_count,
            },
            directories_count,
            assembly::AssemblyResult {
                packages: loaded.packages,
                dependencies: loaded.dependencies,
            },
            loaded.license_references,
            loaded.license_rule_references,
        )
    } else {
        let store = load_license_database(cli.quiet)?;
        let strategy = ScanStrategy::new(&store)
            .optimize(true)
            .confidence_threshold(LICENSE_DETECTION_THRESHOLD);

        let (total_files, total_dirs, excluded_count) =
            count(&cli.dir_path, cli.max_depth, &exclude_patterns)?;

        if !cli.quiet {
            eprintln!(
                "Found {} files in {} directories ({} items excluded)",
                total_files, total_dirs, excluded_count
            );
        }

        let progress_bar = create_progress_bar(total_files, cli.quiet || cli.verbose);
        let text_options = TextDetectionOptions {
            detect_emails: cli.email,
            detect_urls: cli.url,
            max_emails: cli.max_email,
            max_urls: cli.max_url,
            timeout_seconds: cli.timeout,
            verbose_paths: cli.verbose && !cli.quiet,
        };

        let thread_count = resolve_thread_count(cli.processes);
        let mut result = if text_options.detect_emails || text_options.detect_urls {
            run_with_thread_pool(thread_count, || {
                process_with_options(
                    &cli.dir_path,
                    cli.max_depth,
                    Arc::clone(&progress_bar),
                    &exclude_patterns,
                    &strategy,
                    &text_options,
                )
            })?
        } else {
            run_with_thread_pool(thread_count, || {
                process(
                    &cli.dir_path,
                    cli.max_depth,
                    Arc::clone(&progress_bar),
                    &exclude_patterns,
                    &strategy,
                )
            })?
        };

        result.excluded_count = excluded_count;
        if !progress_bar.is_hidden() {
            progress_bar.finish_with_message("Scan complete!");
        } else {
            progress_bar.finish_and_clear();
        }

        (
            result,
            total_dirs,
            assembly::AssemblyResult {
                packages: Vec::new(),
                dependencies: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
    };

    if cli.filter_clues {
        filter_redundant_clues(&mut scan_result.files);
    }

    if !include_patterns.is_empty() {
        apply_include_filter(&mut scan_result.files, &include_patterns);
    }

    if cli.only_findings {
        apply_only_findings_filter(&mut scan_result.files);
    }

    if cli.strip_root || cli.full_root {
        normalize_paths(
            &mut scan_result.files,
            &cli.dir_path,
            cli.strip_root,
            cli.full_root,
        );
    }

    if cli.mark_source {
        apply_mark_source(&mut scan_result.files);
    }

    let assembly_result = if cli.no_assemble {
        assembly::AssemblyResult {
            packages: Vec::new(),
            dependencies: Vec::new(),
        }
    } else if cli.from_json
        && (!preloaded_assembly.packages.is_empty() || !preloaded_assembly.dependencies.is_empty())
    {
        preloaded_assembly
    } else {
        assembly::assemble(&mut scan_result.files)
    };

    let end_time = Utc::now();
    let output = create_output(
        start_time,
        end_time,
        scan_result,
        total_dirs,
        assembly_result,
        preloaded_license_references,
        preloaded_license_rule_references,
    );
    for target in cli.output_targets() {
        let output_config = OutputWriteConfig {
            format: target.format,
            custom_template: target.custom_template.clone(),
            scanned_path: Some(cli.dir_path.clone()),
        };

        write_output_file(&target.file, &output, &output_config)?;
        if !cli.quiet {
            eprintln!("{:?} output written to {}", target.format, target.file);
        }
    }

    Ok(())
}

fn compile_exclude_patterns(patterns: &[String]) -> Vec<Pattern> {
    patterns
        .iter()
        .filter_map(|pattern| Pattern::new(pattern).ok())
        .collect()
}

fn compile_include_patterns(patterns: &[String]) -> Vec<Pattern> {
    patterns
        .iter()
        .filter_map(|pattern| Pattern::new(pattern).ok())
        .collect()
}

fn resolve_thread_count(processes: i32) -> usize {
    if processes <= 0 {
        1
    } else {
        processes as usize
    }
}

fn run_with_thread_pool<T, F>(threads: usize, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send,
    T: Send,
{
    if threads <= 1 {
        return f();
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()?;
    pool.install(f)
}

fn matches_patterns(path: &str, patterns: &[Pattern]) -> bool {
    if patterns.is_empty() {
        return true;
    }

    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    patterns
        .iter()
        .any(|pattern| pattern.matches(path) || pattern.matches(file_name))
}

fn apply_include_filter(files: &mut Vec<crate::models::FileInfo>, include_patterns: &[Pattern]) {
    let mut explicitly_included_files = HashSet::new();
    let mut explicitly_included_dirs = Vec::<String>::new();

    for entry in files.iter() {
        if matches_patterns(&entry.path, include_patterns) {
            match entry.file_type {
                crate::models::FileType::File => {
                    explicitly_included_files.insert(entry.path.clone());
                }
                crate::models::FileType::Directory => {
                    explicitly_included_dirs.push(entry.path.clone());
                }
            }
        }
    }

    let mut kept_file_paths = HashSet::new();
    for entry in files.iter() {
        if entry.file_type != crate::models::FileType::File {
            continue;
        }

        let explicitly = explicitly_included_files.contains(&entry.path);
        let under_included_dir = explicitly_included_dirs
            .iter()
            .any(|dir| Path::new(&entry.path).starts_with(Path::new(dir)));

        if explicitly || under_included_dir {
            kept_file_paths.insert(entry.path.clone());
        }
    }

    files.retain(|entry| match entry.file_type {
        crate::models::FileType::File => kept_file_paths.contains(&entry.path),
        crate::models::FileType::Directory => {
            explicitly_included_dirs.contains(&entry.path)
                || kept_file_paths
                    .iter()
                    .any(|path| Path::new(path).starts_with(Path::new(&entry.path)))
        }
    });
}

fn has_findings(file: &crate::models::FileInfo) -> bool {
    file.license_expression.is_some()
        || !file.license_detections.is_empty()
        || !file.copyrights.is_empty()
        || !file.holders.is_empty()
        || !file.authors.is_empty()
        || !file.emails.is_empty()
        || !file.urls.is_empty()
        || !file.package_data.is_empty()
        || !file.scan_errors.is_empty()
}

fn apply_only_findings_filter(files: &mut Vec<crate::models::FileInfo>) {
    let kept_file_paths: HashSet<String> = files
        .iter()
        .filter(|entry| entry.file_type == crate::models::FileType::File && has_findings(entry))
        .map(|entry| entry.path.clone())
        .collect();

    files.retain(|entry| match entry.file_type {
        crate::models::FileType::File => kept_file_paths.contains(&entry.path),
        crate::models::FileType::Directory => kept_file_paths
            .iter()
            .any(|path| Path::new(path).starts_with(Path::new(&entry.path))),
    });
}

fn dedupe_vec_by_key<T, K, F>(items: &mut Vec<T>, mut key_fn: F)
where
    K: std::hash::Hash + Eq,
    F: FnMut(&T) -> K,
{
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(key_fn(item)));
}

fn filter_redundant_clues(files: &mut [crate::models::FileInfo]) {
    for file in files.iter_mut() {
        dedupe_vec_by_key(&mut file.copyrights, |c| {
            (c.copyright.clone(), c.start_line, c.end_line)
        });
        dedupe_vec_by_key(&mut file.holders, |h| {
            (h.holder.clone(), h.start_line, h.end_line)
        });
        dedupe_vec_by_key(&mut file.authors, |a| {
            (a.author.clone(), a.start_line, a.end_line)
        });
        dedupe_vec_by_key(&mut file.emails, |e| {
            (e.email.clone(), e.start_line, e.end_line)
        });
        dedupe_vec_by_key(&mut file.urls, |u| {
            (u.url.clone(), u.start_line, u.end_line)
        });
    }
}

fn normalize_paths(
    files: &mut [crate::models::FileInfo],
    scan_root: &str,
    strip_root: bool,
    full_root: bool,
) {
    for entry in files.iter_mut() {
        let current_path = PathBuf::from(&entry.path);

        if full_root {
            let absolute_candidate = if current_path.is_absolute() {
                current_path.clone()
            } else {
                env::current_dir()
                    .map(|cwd| cwd.join(&current_path))
                    .unwrap_or(current_path.clone())
            };
            let absolute = absolute_candidate
                .canonicalize()
                .unwrap_or(absolute_candidate);
            entry.path = absolute.to_string_lossy().to_string();
            continue;
        }

        if strip_root && let Some(stripped) = strip_root_prefix(&current_path, Path::new(scan_root))
        {
            entry.path = stripped.to_string_lossy().to_string();
        }
    }
}

fn strip_root_prefix(path: &Path, root: &Path) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(root)
        && !stripped.as_os_str().is_empty()
    {
        return Some(stripped.to_path_buf());
    }

    let canonical_path = path.canonicalize().ok()?;
    let canonical_root = root.canonicalize().ok()?;
    let stripped = canonical_path.strip_prefix(canonical_root).ok()?;
    if stripped.as_os_str().is_empty() {
        None
    } else {
        Some(stripped.to_path_buf())
    }
}

#[derive(Deserialize)]
struct JsonScanInput {
    #[serde(default)]
    files: Vec<crate::models::FileInfo>,
    #[serde(default)]
    packages: Vec<crate::models::Package>,
    #[serde(default)]
    dependencies: Vec<crate::models::TopLevelDependency>,
    #[serde(default)]
    license_references: Vec<crate::models::LicenseReference>,
    #[serde(default)]
    license_rule_references: Vec<crate::models::LicenseRuleReference>,
    #[serde(default)]
    excluded_count: usize,
}

fn load_scan_from_json(path: &str) -> Result<JsonScanInput> {
    let input_path = Path::new(path);
    if !input_path.is_file() {
        return Err(anyhow!("--from-json input must be a valid file: {}", path));
    }

    let content = fs::read_to_string(input_path)?;
    let parsed: JsonScanInput = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Input JSON scan file is not valid JSON: {path}: {e}"))?;

    Ok(parsed)
}

fn apply_mark_source(files: &mut [crate::models::FileInfo]) {
    let mut index_by_path = HashMap::<String, usize>::new();
    for (idx, entry) in files.iter().enumerate() {
        index_by_path.insert(entry.path.clone(), idx);
    }

    for entry in files.iter_mut() {
        if entry.file_type == crate::models::FileType::File {
            entry.is_source = Some(entry.programming_language.is_some());
            entry.source_count = None;
        }
    }

    let mut dir_paths = files
        .iter()
        .filter(|entry| entry.file_type == crate::models::FileType::Directory)
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    dir_paths.sort_by_key(|path| usize::MAX - Path::new(path).components().count());

    let mut direct_file_count = HashMap::<String, usize>::new();
    let mut direct_source_file_count = HashMap::<String, usize>::new();
    let mut child_dirs = HashMap::<String, Vec<String>>::new();

    for entry in files.iter() {
        if let Some(parent) = Path::new(&entry.path).parent().and_then(|p| p.to_str()) {
            let parent_key = parent.to_string();
            if entry.file_type == crate::models::FileType::File {
                *direct_file_count.entry(parent_key.clone()).or_insert(0) += 1;
                if entry.is_source.unwrap_or(false) {
                    *direct_source_file_count.entry(parent_key).or_insert(0) += 1;
                }
            } else {
                child_dirs
                    .entry(parent_key)
                    .or_default()
                    .push(entry.path.clone());
            }
        }
    }

    let mut descendant_file_count = HashMap::<String, usize>::new();
    let mut descendant_source_count = HashMap::<String, usize>::new();

    for dir_path in dir_paths {
        let mut total_files = *direct_file_count.get(&dir_path).unwrap_or(&0);
        let mut source_files = *direct_source_file_count.get(&dir_path).unwrap_or(&0);

        if let Some(children) = child_dirs.get(&dir_path) {
            for child in children {
                total_files += descendant_file_count.get(child).copied().unwrap_or(0);
                source_files += descendant_source_count.get(child).copied().unwrap_or(0);
            }
        }

        let qualifies = total_files > 0 && (source_files as f64 / total_files as f64) >= 0.9;

        if let Some(idx) = index_by_path.get(&dir_path)
            && let Some(entry) = files.get_mut(*idx)
        {
            if qualifies && source_files > 0 {
                entry.is_source = Some(true);
                entry.source_count = Some(source_files);
            } else {
                entry.is_source = None;
                entry.source_count = None;
            }
        }

        descendant_file_count.insert(dir_path.clone(), total_files);
        descendant_source_count.insert(dir_path, if qualifies { source_files } else { 0 });
    }
}

// Embed the license files into the binary
const LICENSES_DIR: Dir = include_dir!("resources/licenses/json/details");

fn load_license_database(quiet: bool) -> Result<Store> {
    if !quiet {
        eprintln!("Loading SPDX data, this may take a while...");
    }
    let mut store = Store::new();

    for file in LICENSES_DIR.files() {
        let string_content = file
            .contents_utf8()
            .ok_or_else(|| anyhow!("Failed to read file as UTF-8"))?;
        let value: Value = from_str(string_content)?;

        if value["isDeprecatedLicenseId"].as_bool().unwrap_or(false) {
            continue;
        }

        let name = value["licenseId"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing license ID"))?
            .to_string();
        let text = value["licenseText"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing license text"))?;

        store.add_license(name, TextData::new(text));
    }

    Ok(store)
}

fn create_progress_bar(total_files: usize, hidden: bool) -> Arc<ProgressBar> {
    let progress_bar = if hidden {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(total_files as u64)
    };
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files processed ({eta})")
            .expect("Failed to create progress bar style")
            .progress_chars("#>-"),
    );
    Arc::new(progress_bar)
}

fn create_output(
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>,
    scan_result: scanner::ProcessResult,
    total_dirs: usize,
    assembly_result: assembly::AssemblyResult,
    license_references: Vec<crate::models::LicenseReference>,
    license_rule_references: Vec<crate::models::LicenseRuleReference>,
) -> Output {
    let duration = (end_time - start_time).num_nanoseconds().unwrap_or(0) as f64 / 1_000_000_000.0;

    let extra_data = ExtraData {
        files_count: scan_result.files.len(),
        directories_count: total_dirs,
        excluded_count: scan_result.excluded_count,
        system_environment: SystemEnvironment {
            operating_system: sys_info::os_type().ok(),
            cpu_architecture: env::consts::ARCH.to_string(),
            platform: format!(
                "{}-{}-{}",
                sys_info::os_type().unwrap_or_else(|_| "unknown".to_string()),
                sys_info::os_release().unwrap_or_else(|_| "unknown".to_string()),
                env::consts::ARCH
            ),
            rust_version: rustc_version_runtime::version().to_string(),
        },
    };

    // Collect all scan errors from individual files
    let errors: Vec<String> = scan_result
        .files
        .iter()
        .filter_map(|file| {
            if file.scan_errors.is_empty() {
                None
            } else {
                Some(
                    file.scan_errors
                        .iter()
                        .map(|error| format!("{}: {}", file.path, error))
                        .collect::<Vec<String>>(),
                )
            }
        })
        .flatten()
        .collect();

    Output {
        headers: vec![Header {
            start_timestamp: start_time.to_rfc3339(),
            end_timestamp: end_time.to_rfc3339(),
            duration,
            extra_data,
            errors,
            output_format_version: SCANCODE_OUTPUT_FORMAT_VERSION.to_string(),
        }],
        packages: assembly_result.packages,
        dependencies: assembly_result.dependencies,
        files: scan_result.files,
        license_references,
        license_rule_references,
    }
}

#[cfg(test)]
mod main_test;
