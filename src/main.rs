use askalono::Store;
use chrono::Utc;
use clap::Parser;
use glob::Pattern;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::to_string_pretty;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crate::cli::Cli;
use crate::models::{ExtraData, Header, Output, SystemEnvironment, SCANCODE_OUTPUT_FORMAT_VERSION};
use crate::scanner::{count, process};

mod cli;
mod models;
mod scanner;
mod utils;

fn main() -> std::io::Result<()> {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let start_time = Utc::now();

    let exclude_patterns = compile_exclude_patterns(&cli.exclude);
    println!("Exclusion patterns: {:?}", cli.exclude);

    let store = load_license_database()?;

    let (total_files, total_dirs, excluded_count) =
        count(&cli.dir_path, cli.max_depth, &exclude_patterns)?;
    println!(
        "Found {} files in {} directories ({} items excluded)",
        total_files, total_dirs, excluded_count
    );

    let progress_bar = create_progress_bar(total_files);
    let scan_result = process(
        &cli.dir_path,
        cli.max_depth,
        Arc::clone(&progress_bar),
        &exclude_patterns,
        &store,
    )?;
    progress_bar.finish_with_message("Scan complete!");

    let end_time = Utc::now();
    let output = create_output(start_time, end_time, scan_result, total_dirs);
    write_output(&cli.output_file, &output)?;

    println!("JSON output written to {}", cli.output_file);
    Ok(())
}

fn compile_exclude_patterns(patterns: &[String]) -> Vec<Pattern> {
    patterns
        .iter()
        .filter_map(|pattern| Pattern::new(pattern).ok())
        .collect()
}

fn load_license_database() -> Result<Store, Box<dyn Error>> {
    println!("Loading SPDX data, this may take a while...");
    let mut store = Store::new();

    // TODO: Make this configurable via CLI
    let license_path = std::env::var("LICENSE_DATA_PATH").unwrap_or_else(|_| {
        "/Users/maximstykow/Documents/license-list-data/json/details".to_string()
    });

    store.load_spdx(Path::new(&license_path), false)?;
    Ok(store)
}

fn create_progress_bar(total_files: usize) -> Arc<ProgressBar> {
    let progress_bar = ProgressBar::new(total_files as u64);
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

    Output {
        headers: vec![Header {
            start_timestamp: start_time.to_rfc3339(),
            end_timestamp: end_time.to_rfc3339(),
            duration,
            extra_data,
            errors: Vec::new(), // TODO: implement
            output_format_version: SCANCODE_OUTPUT_FORMAT_VERSION.to_string(),
        }],
        files: scan_result.files,
        license_references: Vec::new(), // TODO: implement
        license_rule_references: Vec::new(), // TODO: implement
    }
}

fn write_output(output_file: &str, output: &Output) -> std::io::Result<()> {
    let json_output = match to_string_pretty(output) {
        Ok(json) => json,
        Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::Other, err)),
    };
    let mut file = File::create(output_file)?;
    file.write_all(json_output.as_bytes())?;
    Ok(())
}
