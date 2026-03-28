use anyhow::{Result, anyhow};
use chrono::Utc;
use clap::Parser;
use glob::Pattern;
use regex::Regex;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cache::{CACHE_DIR_ENV_VAR, CacheConfig};
use crate::cli::Cli;
use crate::license_detection::LicenseDetectionEngine;
use crate::output::{OutputWriteConfig, write_output_file};
use crate::post_processing::{
    CreateOutputContext, CreateOutputOptions, build_facet_rules, create_output,
};
use crate::progress::{ProgressMode, ScanProgress};
use crate::scan_result_shaping::{
    apply_ignore_resource_filter, apply_mark_source, apply_only_findings_filter,
    apply_path_selection_filter, build_clue_rule_lookup, filter_redundant_clues,
    filter_redundant_clues_with_rules, normalize_paths, normalize_top_level_output_paths,
    trim_preloaded_assembly_to_files,
};
use crate::scanner::{TextDetectionOptions, collect_paths, process_collected};

mod assembly;
mod cache;
mod cli;
mod copyright;
mod finder;
mod license_detection;
mod models;
mod output;
mod parsers;
mod post_processing;
mod progress;
mod scan_result_shaping;
mod scanner;
mod utils;

#[cfg(test)]
mod test_utils;

fn main() -> std::io::Result<()> {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.show_attribution {
        print!("{}", include_str!("../NOTICE"));
        return Ok(());
    }

    let start_time = Utc::now();
    let progress = Arc::new(ScanProgress::new(progress_mode_from_cli(&cli)));
    progress.set_processes(resolve_thread_count(cli.processes));
    progress.set_scan_names(configured_scan_names(&cli));
    progress.init_logging_bridge();

    validate_scan_option_compatibility(&cli)?;
    let facet_rules = build_facet_rules(&cli.facet)?;

    let include_patterns = compile_include_patterns(&cli.include);
    let ignore_author_patterns = compile_regex_patterns("--ignore-author", &cli.ignore_author)?;
    let ignore_copyright_holder_patterns =
        compile_regex_patterns("--ignore-copyright-holder", &cli.ignore_copyright_holder)?;

    progress.start_discovery();

    let (
        mut scan_result,
        total_dirs,
        mut preloaded_assembly,
        preloaded_license_references,
        preloaded_license_rule_references,
        active_license_engine,
    ) = if cli.from_json {
        let mut merged: Option<JsonScanInput> = None;
        for input_path in &cli.dir_path {
            let mut loaded = load_scan_from_json(input_path)?;
            if cli.strip_root || cli.full_root {
                normalize_loaded_json_scan(&mut loaded, cli.strip_root, cli.full_root);
            }
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
            None,
        )
    } else {
        let (scan_path, native_input_includes) = resolve_native_scan_inputs(&cli.dir_path)?;
        let mut native_include_patterns = cli.include.clone();
        native_include_patterns.extend(native_input_includes);

        let cache_config = prepare_cache_for_scan(&scan_path, &cli)?;
        let collection_exclude_patterns =
            build_collection_exclude_patterns(Path::new(&scan_path), &cache_config);

        let mut collected = collect_paths(&scan_path, cli.max_depth, &collection_exclude_patterns);
        let user_excluded_count = apply_user_path_filters_to_collected(
            &mut collected,
            Path::new(&scan_path),
            &native_include_patterns,
            &cli.exclude,
        );
        let total_files = collected.file_count();
        let total_dirs = collected.directory_count();
        let total_size = collected.total_file_bytes;
        let excluded_count = collected.excluded_count + user_excluded_count;
        for (path, err) in &collected.collection_errors {
            progress.record_runtime_error(path, err);
        }
        progress.finish_discovery(total_files, total_dirs, total_size, excluded_count);
        if !cli.quiet {
            progress.output_written(&format!(
                "Found {} files in {} directories ({} items excluded)",
                total_files, total_dirs, excluded_count
            ));
        }

        let license_engine = if cli.license {
            progress.start_license_detection_engine_creation();
            let engine = init_license_engine(&cli.license_rules_path)?;
            progress.finish_license_detection_engine_creation();
            progress.output_written(&describe_license_engine_source(
                &engine,
                cli.license_rules_path.as_deref(),
            ));
            Some(engine)
        } else {
            None
        };

        let text_options = TextDetectionOptions {
            collect_info: cli.info,
            detect_packages: cli.package,
            detect_copyrights: cli.copyright,
            detect_generated: cli.generated,
            detect_emails: cli.email,
            detect_urls: cli.url,
            max_emails: cli.max_email,
            max_urls: cli.max_url,
            timeout_seconds: cli.timeout,
            scan_cache_dir: Some(cache_config.scan_results_dir()),
        };

        let thread_count = resolve_thread_count(cli.processes);
        progress.start_scan(total_files);
        let mut result = run_with_thread_pool(thread_count, || {
            Ok(process_collected(
                &collected,
                Arc::clone(&progress),
                license_engine.clone(),
                cli.include_text,
                &text_options,
            ))
        })?;

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
            license_engine,
        )
    };

    if cli.filter_clues {
        let clue_rule_lookup = prepare_filter_clue_rule_lookup(
            &scan_result.files,
            active_license_engine.as_deref(),
            cli.license_rules_path.as_deref(),
        )?;
        if let Some(clue_rule_lookup) = clue_rule_lookup.as_ref() {
            filter_redundant_clues_with_rules(&mut scan_result.files, Some(clue_rule_lookup));
        } else {
            filter_redundant_clues(&mut scan_result.files);
        }
    }

    if !ignore_author_patterns.is_empty() || !ignore_copyright_holder_patterns.is_empty() {
        apply_ignore_resource_filter(
            &mut scan_result.files,
            &ignore_copyright_holder_patterns,
            &ignore_author_patterns,
        );
    }

    if cli.from_json && (!include_patterns.is_empty() || !cli.exclude.is_empty()) {
        apply_path_selection_filter(&mut scan_result.files, |file| {
            is_included_path(&file.path, &cli.include, &cli.exclude)
        });
    }

    if cli.only_findings {
        apply_only_findings_filter(&mut scan_result.files);
    }

    if cli.mark_source {
        apply_mark_source(&mut scan_result.files);
    }

    if cli.from_json {
        trim_preloaded_assembly_to_files(
            &scan_result.files,
            &mut preloaded_assembly.packages,
            &mut preloaded_assembly.dependencies,
        );
    }

    let manifests_seen = scan_result
        .files
        .iter()
        .map(|file| file.package_data.len())
        .sum();
    let mut assembly_result = if cli.from_json
        && (!preloaded_assembly.packages.is_empty() || !preloaded_assembly.dependencies.is_empty())
    {
        progress.start_assembly();
        progress.finish_assembly(preloaded_assembly.packages.len(), manifests_seen);
        preloaded_assembly
    } else if cli.no_assemble {
        assembly::AssemblyResult {
            packages: Vec::new(),
            dependencies: Vec::new(),
        }
    } else {
        progress.start_assembly();
        let assembled = assembly::assemble(&mut scan_result.files);
        progress.finish_assembly(assembled.packages.len(), manifests_seen);
        assembled
    };

    if !cli.from_json && (cli.strip_root || cli.full_root) {
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
        normalize_top_level_output_paths(
            &mut assembly_result.packages,
            &mut assembly_result.dependencies,
            root_path,
            cli.strip_root,
        );
    }

    let end_time = Utc::now();

    let output = create_output(
        start_time,
        end_time,
        scan_result,
        CreateOutputContext {
            total_dirs,
            assembly_result,
            license_references: preloaded_license_references,
            license_rule_references: preloaded_license_rule_references,
            options: CreateOutputOptions {
                facet_rules: &facet_rules,
                include_classify: cli.classify,
                include_summary: cli.summary,
                include_license_clarity_score: cli.license_clarity_score,
                include_tallies: cli.tallies,
                include_tallies_of_key_files: cli.tallies_key_files,
                include_tallies_with_details: cli.tallies_with_details,
                include_tallies_by_facet: cli.tallies_by_facet,
                include_generated: cli.generated,
            },
        },
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
    if cli.from_json && (cli.package || cli.copyright || cli.email || cli.url || cli.generated) {
        return Err(anyhow!(
            "When using --from-json, file scan options like --package/--copyright/--email/--url/--generated are not allowed"
        ));
    }

    if !cli.from_json && cli.dir_path.is_empty() {
        return Err(anyhow!("Directory path is required for scan operations"));
    }

    if cli.tallies_by_facet && cli.facet.is_empty() {
        return Err(anyhow!(
            "--tallies-by-facet requires at least one --facet <facet>=<pattern> definition"
        ));
    }

    Ok(())
}

fn resolve_native_scan_inputs(inputs: &[String]) -> Result<(String, Vec<String>)> {
    if inputs.is_empty() {
        return Err(anyhow!("No directory input path provided"));
    }

    if inputs.len() == 1 {
        return Ok((inputs[0].clone(), Vec::new()));
    }

    if inputs.iter().any(|path| Path::new(path).is_absolute()) {
        return Err(anyhow!(
            "Invalid inputs: all input paths must be relative when using multiple inputs"
        ));
    }

    let common_prefix = common_path_prefix(inputs)
        .unwrap_or_else(|| PathBuf::from("."))
        .to_string_lossy()
        .to_string();
    if common_prefix != "." && !Path::new(&common_prefix).is_dir() {
        return Err(anyhow!(
            "Invalid inputs: all input paths must share a common single parent directory"
        ));
    }

    let synthetic_includes = inputs
        .iter()
        .map(|path| path.replace('\\', "/").trim_end_matches('/').to_string())
        .collect();

    Ok((common_prefix, synthetic_includes))
}

fn common_path_prefix(inputs: &[String]) -> Option<PathBuf> {
    let first = inputs.first()?;
    let mut shared_components: Vec<_> = Path::new(first).components().collect();

    for input in &inputs[1..] {
        let components: Vec<_> = Path::new(input).components().collect();
        let shared_len = shared_components
            .iter()
            .zip(components.iter())
            .take_while(|(left, right)| left == right)
            .count();
        shared_components.truncate(shared_len);
        if shared_components.is_empty() {
            break;
        }
    }

    if shared_components.is_empty() {
        None
    } else {
        let mut prefix = PathBuf::new();
        for component in shared_components {
            prefix.push(component.as_os_str());
        }
        Some(prefix)
    }
}

fn normalize_loaded_json_scan(loaded: &mut JsonScanInput, strip_root: bool, full_root: bool) {
    if let Some(scan_root) = derive_json_scan_root(&loaded.files)
        && strip_root
    {
        normalize_paths(&mut loaded.files, &scan_root, true, false);
        normalize_top_level_output_paths(
            &mut loaded.packages,
            &mut loaded.dependencies,
            &scan_root,
            true,
        );
    }

    if full_root {
        trim_loaded_json_full_root_paths(loaded);
    }
}

fn derive_json_scan_root(files: &[crate::models::FileInfo]) -> Option<String> {
    let mut directories: Vec<&str> = files
        .iter()
        .filter(|file| file.file_type == crate::models::FileType::Directory)
        .map(|file| file.path.as_str())
        .collect();
    directories.sort_by_key(|path| (path.matches('/').count(), path.len()));
    if let Some(root_dir) = directories.first() {
        return Some((*root_dir).to_string());
    }

    if files.len() == 1 {
        return files.first().map(|file| file.path.clone());
    }

    let paths: Vec<String> = files.iter().map(|file| file.path.clone()).collect();
    common_path_prefix(&paths).map(|path| path.to_string_lossy().to_string())
}

fn trim_loaded_json_full_root_paths(loaded: &mut JsonScanInput) {
    for file in &mut loaded.files {
        trim_full_root_display_value(&mut file.path);
        for detection_match in &mut file.license_clues {
            if let Some(from_file) = detection_match.from_file.as_mut() {
                trim_full_root_display_value(from_file);
            }
        }
        for detection in &mut file.license_detections {
            for detection_match in &mut detection.matches {
                if let Some(from_file) = detection_match.from_file.as_mut() {
                    trim_full_root_display_value(from_file);
                }
            }
        }
        for package_data in &mut file.package_data {
            for file_reference in &mut package_data.file_references {
                trim_full_root_display_value(&mut file_reference.path);
            }
            for detection in &mut package_data.license_detections {
                for detection_match in &mut detection.matches {
                    if let Some(from_file) = detection_match.from_file.as_mut() {
                        trim_full_root_display_value(from_file);
                    }
                }
            }
            for detection in &mut package_data.other_license_detections {
                for detection_match in &mut detection.matches {
                    if let Some(from_file) = detection_match.from_file.as_mut() {
                        trim_full_root_display_value(from_file);
                    }
                }
            }
        }
    }

    for package in &mut loaded.packages {
        for datafile_path in &mut package.datafile_paths {
            trim_full_root_display_value(datafile_path);
        }
    }
    for dependency in &mut loaded.dependencies {
        trim_full_root_display_value(&mut dependency.datafile_path);
    }
}

fn trim_full_root_display_value(path: &mut String) {
    *path = path.replace('\\', "/").trim_matches('/').to_string();
}

fn prepare_cache_for_scan(scan_path: &str, cli: &Cli) -> Result<CacheConfig> {
    let env_cache_dir = env::var_os(CACHE_DIR_ENV_VAR).map(PathBuf::from);
    let config = CacheConfig::from_overrides(
        Path::new(scan_path),
        cli.cache_dir.as_deref().map(Path::new),
        env_cache_dir.as_deref(),
    );

    if cli.cache_clear {
        config.clear()?;
    }

    config.ensure_dirs()?;
    Ok(config)
}

fn build_collection_exclude_patterns(scan_root: &Path, cache_config: &CacheConfig) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    let cache_root = cache_config.root_dir();

    if let Ok(relative_cache_root) = cache_root.strip_prefix(scan_root)
        && !relative_cache_root.as_os_str().is_empty()
    {
        for path in [cache_root.to_path_buf(), relative_cache_root.to_path_buf()] {
            let normalized = path.to_string_lossy().replace('\\', "/");
            let escaped = Pattern::escape(&normalized);
            for pattern in [escaped.clone(), format!("{escaped}/**")] {
                if let Ok(pattern) = Pattern::new(&pattern) {
                    patterns.push(pattern);
                }
            }
        }
    }

    patterns
}

fn apply_user_path_filters_to_collected(
    collected: &mut crate::scanner::CollectedPaths,
    scan_root: &Path,
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> usize {
    let before_files = collected.files.len();
    let before_dirs = collected.directories.len();
    collected.files.retain(|(path, _)| {
        let relative_path = normalize_scan_relative_path(path, scan_root);
        is_included_path(&relative_path, include_patterns, exclude_patterns)
    });

    let kept_file_paths: std::collections::HashSet<_> = collected
        .files
        .iter()
        .map(|(path, _)| path.clone())
        .collect();
    collected.directories.retain(|(path, _)| {
        let relative_path = normalize_scan_relative_path(path, scan_root);
        is_included_path(&relative_path, include_patterns, exclude_patterns)
            || kept_file_paths
                .iter()
                .any(|file_path| file_path.starts_with(path))
    });

    (before_files - collected.files.len()) + (before_dirs - collected.directories.len())
}

fn normalize_scan_relative_path(path: &Path, scan_root: &Path) -> String {
    path.strip_prefix(scan_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_included_path(path: &str, include_patterns: &[String], exclude_patterns: &[String]) -> bool {
    if path.trim().is_empty() {
        return false;
    }

    let normalized_path = path.replace('\\', "/").to_ascii_lowercase();
    let stripped_path = normalized_path.trim_start_matches(['/', '0']).to_string();

    if !include_patterns.is_empty()
        && !include_patterns
            .iter()
            .filter(|pattern| !pattern.trim().is_empty())
            .any(|pattern| path_matches_scancode_pattern(pattern, &normalized_path, &stripped_path))
    {
        return false;
    }

    !exclude_patterns
        .iter()
        .filter(|pattern| !pattern.trim().is_empty())
        .any(|pattern| path_matches_scancode_pattern(pattern, &normalized_path, &stripped_path))
}

fn path_matches_scancode_pattern(
    pattern: &str,
    normalized_path: &str,
    stripped_path: &str,
) -> bool {
    let normalized_pattern = pattern.trim_start_matches('/').to_ascii_lowercase();
    let Ok(compiled) = Pattern::new(&normalized_pattern) else {
        return false;
    };

    if !normalized_pattern.contains('/') {
        stripped_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .any(|segment| compiled.matches(segment))
    } else {
        matching_path_candidates(normalized_path, stripped_path)
            .iter()
            .any(|candidate| compiled.matches(candidate))
    }
}

fn matching_path_candidates<'a>(normalized_path: &'a str, stripped_path: &'a str) -> Vec<&'a str> {
    let mut candidates = Vec::new();

    for path in [normalized_path, stripped_path] {
        if path.is_empty() {
            continue;
        }

        candidates.push(path);
        let mut current = path;
        while let Some((parent, _)) = current.rsplit_once('/') {
            if parent.is_empty() {
                break;
            }
            candidates.push(parent);
            current = parent;
        }
    }

    candidates
}

fn compile_include_patterns(patterns: &[String]) -> Vec<Pattern> {
    patterns
        .iter()
        .filter_map(|pattern| Pattern::new(pattern).ok())
        .collect()
}

fn compile_regex_patterns(option_name: &str, patterns: &[String]) -> Result<Vec<Regex>> {
    patterns
        .iter()
        .map(|pattern| {
            Regex::new(pattern).map_err(|err| {
                anyhow!("Invalid regex for {option_name} pattern \"{pattern}\": {err}")
            })
        })
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
    let mut names = vec!["licenses"];
    if cli.info {
        names.push("info");
    }
    if cli.package {
        names.push("packages");
    }
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

fn init_license_engine(rules_path: &Option<String>) -> Result<Arc<LicenseDetectionEngine>> {
    match rules_path {
        Some(p) => {
            let path = PathBuf::from(p);
            if !path.exists() {
                return Err(anyhow!("License rules path does not exist: {:?}", path));
            }
            let engine = LicenseDetectionEngine::from_directory(&path)?;
            Ok(Arc::new(engine))
        }
        None => {
            let engine = LicenseDetectionEngine::from_embedded()?;
            Ok(Arc::new(engine))
        }
    }
}

fn prepare_filter_clue_rule_lookup(
    files: &[crate::models::FileInfo],
    active_license_engine: Option<&LicenseDetectionEngine>,
    rules_path: Option<&str>,
) -> Result<Option<crate::scan_result_shaping::ClueRuleLookup>> {
    let needs_rule_lookup = files.iter().any(|file| {
        file.license_detections
            .iter()
            .any(|detection| !detection.matches.is_empty())
    });
    if !needs_rule_lookup {
        return Ok(None);
    }

    if let Some(active_license_engine) = active_license_engine {
        return Ok(Some(build_clue_rule_lookup(active_license_engine.index())));
    }

    let fallback_engine = match rules_path {
        Some(path) => LicenseDetectionEngine::from_directory(Path::new(path))?,
        None => LicenseDetectionEngine::from_embedded()?,
    };
    Ok(Some(build_clue_rule_lookup(fallback_engine.index())))
}

fn describe_license_engine_source(
    engine: &LicenseDetectionEngine,
    rules_path: Option<&str>,
) -> String {
    match rules_path {
        Some(path) => format!(
            "License detection engine initialized with {} rules from {}",
            engine.index().rules_by_rid.len(),
            path
        ),
        None => format!(
            "License detection engine initialized with {} rules from embedded artifact",
            engine.index().rules_by_rid.len()
        ),
    }
}

#[cfg(test)]
mod main_test;
