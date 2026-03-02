use anyhow::{Result, anyhow};
use askalono::ScanStrategy;
use chrono::Utc;
use clap::Parser;
use glob::Pattern;
use include_dir::{Dir, include_dir};
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
use crate::progress::{ProgressMode, ScanProgress};
use crate::scanner::{TextDetectionOptions, count_with_size, process, process_with_options};

mod askalono;
mod assembly;
mod cli;
#[allow(dead_code, unused_imports)]
mod copyright;
mod finder;
mod models;
mod output;
mod parsers;
mod progress;
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
    let progress = Arc::new(ScanProgress::new(progress_mode_from_cli(&cli)));
    progress.set_processes(resolve_thread_count(cli.processes));
    progress.set_scan_names(configured_scan_names(&cli));
    progress.init_logging_bridge();

    validate_scan_option_compatibility(&cli)?;

    let exclude_patterns = compile_exclude_patterns(&cli.exclude);
    let include_patterns = compile_include_patterns(&cli.include);

    progress.start_discovery();

    let (
        mut scan_result,
        total_dirs,
        preloaded_assembly,
        preloaded_license_references,
        preloaded_license_rule_references,
    ) = if cli.from_json {
        let mut merged: Option<JsonScanInput> = None;
        for input_path in &cli.dir_path {
            let mut loaded = load_scan_from_json(input_path)?;
            if let Some(acc) = &mut merged {
                acc.files.append(&mut loaded.files);
                acc.packages.append(&mut loaded.packages);
                acc.dependencies.append(&mut loaded.dependencies);
                acc.license_references
                    .append(&mut loaded.license_references);
                acc.license_rule_references
                    .append(&mut loaded.license_rule_references);
                acc.excluded_count += loaded.excluded_count;
            } else {
                merged = Some(loaded);
            }
        }

        let loaded = merged.ok_or_else(|| anyhow!("No input paths provided"))?;
        let directories_count = loaded
            .files
            .iter()
            .filter(|f| f.file_type == crate::models::FileType::Directory)
            .count();
        let files_count = loaded
            .files
            .iter()
            .filter(|f| f.file_type == crate::models::FileType::File)
            .count();
        let size_count = loaded
            .files
            .iter()
            .filter(|f| f.file_type == crate::models::FileType::File)
            .map(|f| f.size)
            .sum();
        progress.finish_discovery(
            files_count,
            directories_count,
            size_count,
            loaded.excluded_count,
        );
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
        let scan_path = cli
            .dir_path
            .first()
            .ok_or_else(|| anyhow!("No directory input path provided"))?;

        let (total_files, total_dirs, excluded_count, total_size) =
            count_with_size(scan_path, cli.max_depth, &exclude_patterns)?;
        progress.finish_discovery(total_files, total_dirs, total_size, excluded_count);
        if !cli.quiet {
            progress.output_written(&format!(
                "Found {} files in {} directories ({} items excluded)",
                total_files, total_dirs, excluded_count
            ));
        }

        progress.start_spdx_load();
        let store = load_license_database()?;
        progress.finish_spdx_load();
        let strategy = ScanStrategy::new(&store)
            .optimize(true)
            .confidence_threshold(LICENSE_DETECTION_THRESHOLD);

        let text_options = TextDetectionOptions {
            detect_copyrights: cli.copyright,
            detect_emails: cli.email,
            detect_urls: cli.url,
            max_emails: cli.max_email,
            max_urls: cli.max_url,
            timeout_seconds: cli.timeout,
        };
        let default_text_options = TextDetectionOptions::default();

        let thread_count = resolve_thread_count(cli.processes);
        progress.start_scan(total_files);
        let mut result = if !text_options.detect_emails
            && !text_options.detect_urls
            && text_options.detect_copyrights == default_text_options.detect_copyrights
            && text_options.timeout_seconds == default_text_options.timeout_seconds
        {
            run_with_thread_pool(thread_count, || {
                process(
                    scan_path,
                    cli.max_depth,
                    Arc::clone(&progress),
                    &exclude_patterns,
                    &strategy,
                )
            })?
        } else {
            run_with_thread_pool(thread_count, || {
                process_with_options(
                    scan_path,
                    cli.max_depth,
                    Arc::clone(&progress),
                    &exclude_patterns,
                    &strategy,
                    &text_options,
                )
            })?
        };

        result.excluded_count = excluded_count;
        progress.finish_scan();

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
        let root_path = cli
            .dir_path
            .first()
            .ok_or_else(|| anyhow!("No input path available for path normalization"))?;
        normalize_paths(
            &mut scan_result.files,
            root_path,
            cli.strip_root,
            cli.full_root,
        );
    }

    if cli.mark_source {
        apply_mark_source(&mut scan_result.files);
    }

    let manifests_seen = scan_result
        .files
        .iter()
        .map(|file| file.package_data.len())
        .sum();
    let assembly_result = if cli.no_assemble {
        assembly::AssemblyResult {
            packages: Vec::new(),
            dependencies: Vec::new(),
        }
    } else if cli.from_json
        && (!preloaded_assembly.packages.is_empty() || !preloaded_assembly.dependencies.is_empty())
    {
        progress.start_assembly();
        progress.finish_assembly(preloaded_assembly.packages.len(), manifests_seen);
        preloaded_assembly
    } else {
        progress.start_assembly();
        let assembled = assembly::assemble(&mut scan_result.files);
        progress.finish_assembly(assembled.packages.len(), manifests_seen);
        assembled
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

    progress.start_output();
    for target in cli.output_targets() {
        let output_config = OutputWriteConfig {
            format: target.format,
            custom_template: target.custom_template.clone(),
            scanned_path: if cli.dir_path.len() == 1 {
                cli.dir_path.first().cloned()
            } else {
                None
            },
        };

        write_output_file(&target.file, &output, &output_config)?;
        progress.output_written(&format!(
            "{:?} output written to {}",
            target.format, target.file
        ));
    }
    progress.finish_output();

    progress.record_final_counts(&output.files);
    progress.display_summary(&start_time.to_rfc3339(), &Utc::now().to_rfc3339());

    Ok(())
}

fn validate_scan_option_compatibility(cli: &Cli) -> Result<()> {
    if cli.from_json && (cli.copyright || cli.email || cli.url) {
        return Err(anyhow!(
            "When using --from-json, file scan options like --copyright/--email/--url are not allowed"
        ));
    }

    if !cli.from_json && cli.dir_path.len() != 1 {
        return Err(anyhow!(
            "Directory scan mode currently supports exactly one input path"
        ));
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
    if processes > 0 {
        processes as usize
    } else if processes == 0 {
        default_parallel_threads()
    } else {
        1
    }
}

fn default_parallel_threads() -> usize {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    if cpus > 1 { cpus - 1 } else { 1 }
}

fn progress_mode_from_cli(cli: &Cli) -> ProgressMode {
    if cli.quiet {
        ProgressMode::Quiet
    } else if cli.verbose {
        ProgressMode::Verbose
    } else {
        ProgressMode::Default
    }
}

fn configured_scan_names(cli: &Cli) -> String {
    let mut names = vec!["licenses", "packages"];
    if cli.copyright {
        names.push("copyrights");
    }
    if cli.email {
        names.push("emails");
    }
    if cli.url {
        names.push("urls");
    }
    names.join(", ")
}

fn run_with_thread_pool<T, F>(threads: usize, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send,
    T: Send,
{
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
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

const LICENSES_DIR: Dir = include_dir!("resources/licenses/json/details");

fn load_license_database() -> Result<Store> {
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
