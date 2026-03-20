use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use provenant::license_detection::models::{LoadedLicense, LoadedRule};
use provenant::license_detection::rules::{
    load_loaded_licenses_from_directory, load_loaded_rules_from_directory,
};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(
    name = "generate-license-loader-artifact",
    about = "Generate the embedded license loader artifact from ScanCode rules and licenses"
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

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoaderArtifact {
    pub schema_version: u32,
    pub rules: Vec<LoadedRule>,
    pub licenses: Vec<LoadedLicense>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output_path = args.output.unwrap_or_else(|| {
        PathBuf::from("resources/license_detection/license_index_loader.msgpack.zst")
    });
    let rules_dir = args.rules.unwrap_or_else(|| {
        PathBuf::from("resources/scancode-licenses/src/licensedcode/data/rules")
    });
    let licenses_dir = args.licenses.unwrap_or_else(|| {
        PathBuf::from("resources/scancode-licenses/src/licensedcode/data/licenses")
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

    let artifact = LoaderArtifact {
        schema_version: SCHEMA_VERSION,
        rules: loaded_rules,
        licenses: loaded_licenses,
    };

    let msgpack = rmp_serde::to_vec(&artifact).context("Failed to serialize to MessagePack")?;
    println!("MessagePack size: {} bytes", msgpack.len());

    let compressed = zstd::encode_all(&msgpack[..], 0).context("Failed to compress with zstd")?;
    println!("Compressed size: {} bytes", compressed.len());

    if args.check {
        let existing = fs::read(&output_path).with_context(|| {
            format!(
                "Failed to read existing artifact from {}",
                output_path.display()
            )
        })?;

        if existing == compressed {
            println!("Artifact is up to date: {}", output_path.display());
        } else {
            eprintln!("Artifact is out of date: {}", output_path.display());
            eprintln!(
                "Run: cargo run --manifest-path xtask/Cargo.toml --bin generate-license-loader-artifact"
            );
            std::process::exit(1);
        }
    } else {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        fs::write(&output_path, &compressed)
            .with_context(|| format!("Failed to write to {}", output_path.display()))?;

        println!("Wrote artifact to: {}", output_path.display());
    }

    Ok(())
}
