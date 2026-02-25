use askalono::ScanStrategy;
use chrono::Utc;
use clap::Parser;
use glob::Pattern;
use include_dir::{Dir, include_dir};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::{Value, from_str, to_string_pretty};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use crate::askalono::{Store, TextData};
use crate::cli::Cli;
use crate::models::{ExtraData, Header, Output, SCANCODE_OUTPUT_FORMAT_VERSION, SystemEnvironment};
use crate::scanner::{count, process};

mod askalono;
mod assembly;
mod cli;
#[allow(dead_code, unused_imports)]
mod copyright;
mod models;
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

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let start_time = Utc::now();

    let exclude_patterns = compile_exclude_patterns(&cli.exclude);
    println!("Exclusion patterns: {:?}", cli.exclude);

    let store = load_license_database()?;
    let strategy = ScanStrategy::new(&store)
        .optimize(true)
        .confidence_threshold(LICENSE_DETECTION_THRESHOLD);

    let (total_files, total_dirs, excluded_count) =
        count(&cli.dir_path, cli.max_depth, &exclude_patterns)?;
    println!(
        "Found {} files in {} directories ({} items excluded)",
        total_files, total_dirs, excluded_count
    );

    let progress_bar = create_progress_bar(total_files);
    let mut scan_result = process(
        &cli.dir_path,
        cli.max_depth,
        Arc::clone(&progress_bar),
        &exclude_patterns,
        &strategy,
    )?;
    progress_bar.finish_with_message("Scan complete!");

    let assembly_result = if cli.no_assemble {
        assembly::AssemblyResult {
            packages: Vec::new(),
            dependencies: Vec::new(),
        }
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
    );
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

// Embed the license files into the binary
const LICENSES_DIR: Dir = include_dir!("resources/licenses/json/details");

fn load_license_database() -> Result<Store, Box<dyn Error>> {
    println!("Loading SPDX data, this may take a while...");
    let mut store = Store::new();

    for file in LICENSES_DIR.files() {
        let string_content = file.contents_utf8().ok_or("Failed to read file as UTF-8")?;
        let value: Value = from_str(string_content)?;

        if value["isDeprecatedLicenseId"].as_bool().unwrap_or(false) {
            continue;
        }

        let name = value["licenseId"]
            .as_str()
            .ok_or("Missing license ID")?
            .to_string();
        let text = value["licenseText"]
            .as_str()
            .ok_or("Missing license text")?;

        store.add_license(name, TextData::new(text));
    }

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
    assembly_result: assembly::AssemblyResult,
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
        license_references: Vec::new(),      // TODO: implement
        license_rule_references: Vec::new(), // TODO: implement
    }
}

fn write_output(output_file: &str, output: &Output) -> std::io::Result<()> {
    let json_output = match to_string_pretty(output) {
        Ok(json) => json,
        Err(err) => return Err(std::io::Error::other(err)),
    };
    let mut file = File::create(output_file)?;
    file.write_all(json_output.as_bytes())?;
    Ok(())
}
