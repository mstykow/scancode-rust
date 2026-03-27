use anyhow::{Result, anyhow};
use chrono::Utc;
use clap::Parser;
use glob::Pattern;
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
    apply_include_filter, apply_mark_source, apply_only_findings_filter, filter_redundant_clues,
    normalize_paths,
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

        let cache_config = prepare_cache_for_scan(scan_path, &cli)?;
        let collection_exclude_patterns =
            build_collection_exclude_patterns(Path::new(scan_path), &cache_config, &cli.exclude);

        let collected = collect_paths(scan_path, cli.max_depth, &collection_exclude_patterns);
        let total_files = collected.file_count();
        let total_dirs = collected.directory_count();
        let total_size = collected.total_file_bytes;
        let excluded_count = collected.excluded_count;
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
    let assembly_result = if cli.from_json
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

    if cli.from_json && (cli.strip_root || cli.full_root) {
        return Err(anyhow!(
            "When using --from-json, --strip-root and --full-root are not supported because the original scan root is unavailable"
        ));
    }

    if !cli.from_json && cli.dir_path.is_empty() {
        return Err(anyhow!("Directory path is required for scan operations"));
    }

    if !cli.from_json && cli.dir_path.len() != 1 {
        return Err(anyhow!(
            "Directory scan mode currently supports exactly one input path"
        ));
    }

    if cli.tallies_by_facet && cli.facet.is_empty() {
        return Err(anyhow!(
            "--tallies-by-facet requires at least one --facet <facet>=<pattern> definition"
        ));
    }

    Ok(())
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

fn build_collection_exclude_patterns(
    scan_root: &Path,
    cache_config: &CacheConfig,
    user_patterns: &[String],
) -> Vec<Pattern> {
    let mut patterns = compile_exclude_patterns(user_patterns);
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
    let mut names = vec!["licenses"];
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
