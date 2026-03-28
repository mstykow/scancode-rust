use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use provenant::license_detection::embedded::index::EmbeddedLicenseIndex;
use provenant::license_detection::index::build_index_from_loaded;
use provenant::license_detection::rules::{
    load_loaded_licenses_from_directory, load_loaded_rules_from_directory,
};

#[derive(Parser, Debug)]
#[command(
    name = "generate-index-artifact",
    about = "Generate the full embedded license index from ScanCode rules and licenses"
)]
struct Args {
    #[arg(long, help = "Output path")]
    output: Option<PathBuf>,

    #[arg(long, help = "Rules directory")]
    rules: Option<PathBuf>,

    #[arg(long, help = "Licenses directory")]
    licenses: Option<PathBuf>,

    #[arg(long, help = "Verify existing artifact matches regenerated output")]
    check: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output_path = args
        .output
        .unwrap_or_else(|| PathBuf::from("resources/license_detection/license_index.bincode.zst"));
    let rules_dir = args.rules.unwrap_or_else(|| {
        PathBuf::from(provenant::license_detection::SCANCODE_LICENSES_RULES_PATH)
    });
    let licenses_dir = args.licenses.unwrap_or_else(|| {
        PathBuf::from(provenant::license_detection::SCANCODE_LICENSES_LICENSES_PATH)
    });

    println!("Loading rules from: {}", rules_dir.display());
    println!("Loading licenses from: {}", licenses_dir.display());

    let mut loaded_rules = load_loaded_rules_from_directory(&rules_dir)
        .with_context(|| format!("Failed to load rules from {}", rules_dir.display()))?;
    let mut loaded_licenses = load_loaded_licenses_from_directory(&licenses_dir)
        .with_context(|| format!("Failed to load licenses from {}", licenses_dir.display()))?;

    println!("Loaded {} rules", loaded_rules.len());
    println!("Loaded {} licenses", loaded_licenses.len());

    loaded_rules.sort_by(|a, b| a.identifier.cmp(&b.identifier));
    loaded_licenses.sort_by(|a, b| a.key.cmp(&b.key));

    println!("Building license index...");
    let license_index = build_index_from_loaded(loaded_rules, loaded_licenses, false);

    println!("Converting to embedded format...");
    let embedded_index = EmbeddedLicenseIndex::from(&license_index);

    println!("Serializing...");
    let bytes = embedded_index
        .serialize_to_bytes()
        .context("Failed to serialize embedded license index")?;

    println!("Total artifact size: {} bytes", bytes.len());

    if args.check {
        let existing = fs::read(&output_path).with_context(|| {
            format!(
                "Failed to read existing artifact from {}",
                output_path.display()
            )
        })?;

        if existing == bytes {
            println!("Artifact is up to date: {}", output_path.display());
        } else {
            eprintln!("Artifact is out of date: {}", output_path.display());
            eprintln!(
                "Run: cargo run --manifest-path xtask/Cargo.toml --bin generate-index-artifact"
            );
            std::process::exit(1);
        }
    } else {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        fs::write(&output_path, &bytes)
            .with_context(|| format!("Failed to write to {}", output_path.display()))?;

        println!("Wrote artifact to: {}", output_path.display());
    }

    Ok(())
}
