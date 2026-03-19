//! Generate the embedded license loader artifact.
//!
//! This binary loads rules and licenses from the reference ScanCode data directory,
//! serializes them to MessagePack, compresses with zstd, and writes to the output path.
//!
//! Usage:
//!   cargo run --bin generate-license-loader-artifact -- [OPTIONS]
//!
//! Options:
//!   --output <PATH>    Output path (default: resources/license_detection/license_index_loader.msgpack.zst)
//!   --rules <PATH>     Rules directory (default: reference/scancode-toolkit/src/licensedcode/data/rules)
//!   --licenses <PATH>  Licenses directory (default: reference/scancode-toolkit/src/licensedcode/data/licenses)
//!   --check            Verify existing artifact matches regenerated output

use anyhow::{Context, Result};
use scancode_rust::license_detection::models::{LoadedLicense, LoadedRule};
use scancode_rust::license_detection::rules::loader::{
    load_loaded_licenses_from_directory, load_loaded_rules_from_directory,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Current schema version for the loader artifact.
/// Increment this when the serialized types change incompatibly.
const SCHEMA_VERSION: u32 = 1;

/// Wrapper struct for the loader artifact.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoaderArtifact {
    /// Schema version for compatibility checking.
    pub schema_version: u32,
    /// Loaded rules (all, including deprecated).
    pub rules: Vec<LoadedRule>,
    /// Loaded licenses (all, including deprecated).
    pub licenses: Vec<LoadedLicense>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mut output_path =
        PathBuf::from("resources/license_detection/license_index_loader.msgpack.zst");
    let mut rules_dir = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
    let mut licenses_dir =
        PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
    let mut check_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                i += 1;
                output_path = PathBuf::from(args.get(i).context("--output requires a path")?);
            }
            "--rules" => {
                i += 1;
                rules_dir = PathBuf::from(args.get(i).context("--rules requires a path")?);
            }
            "--licenses" => {
                i += 1;
                licenses_dir = PathBuf::from(args.get(i).context("--licenses requires a path")?);
            }
            "--check" => {
                check_mode = true;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    println!("Loading rules from: {}", rules_dir.display());
    println!("Loading licenses from: {}", licenses_dir.display());

    let mut loaded_rules = load_loaded_rules_from_directory(&rules_dir)
        .with_context(|| format!("Failed to load rules from {}", rules_dir.display()))?;
    let mut loaded_licenses = load_loaded_licenses_from_directory(&licenses_dir)
        .with_context(|| format!("Failed to load licenses from {}", licenses_dir.display()))?;

    println!("Loaded {} rules", loaded_rules.len());
    println!("Loaded {} licenses", loaded_licenses.len());

    sort_deterministically(&mut loaded_rules, &mut loaded_licenses);

    let artifact = LoaderArtifact {
        schema_version: SCHEMA_VERSION,
        rules: loaded_rules,
        licenses: loaded_licenses,
    };

    let compressed = serialize_and_compress(&artifact)?;

    if check_mode {
        check_artifact(&output_path, &compressed)?;
    } else {
        write_artifact(&output_path, &compressed)?;
    }

    Ok(())
}

fn sort_deterministically(rules: &mut [LoadedRule], licenses: &mut [LoadedLicense]) {
    rules.sort_by(|a, b| a.identifier.cmp(&b.identifier));
    licenses.sort_by(|a, b| a.key.cmp(&b.key));
}

fn serialize_and_compress(artifact: &LoaderArtifact) -> Result<Vec<u8>> {
    let msgpack = rmp_serde::to_vec(artifact).context("Failed to serialize to MessagePack")?;
    println!("MessagePack size: {} bytes", msgpack.len());

    let compressed = zstd::encode_all(&msgpack[..], 0).context("Failed to compress with zstd")?;
    println!("Compressed size: {} bytes", compressed.len());

    Ok(compressed)
}

fn write_artifact(output_path: &Path, compressed: &[u8]) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    fs::write(output_path, compressed)
        .with_context(|| format!("Failed to write to {}", output_path.display()))?;

    println!("Wrote artifact to: {}", output_path.display());
    Ok(())
}

fn check_artifact(output_path: &Path, expected: &[u8]) -> Result<()> {
    let existing = fs::read(output_path).with_context(|| {
        format!(
            "Failed to read existing artifact from {}",
            output_path.display()
        )
    })?;

    if existing == expected {
        println!("Artifact is up to date: {}", output_path.display());
        Ok(())
    } else {
        eprintln!("Artifact is out of date: {}", output_path.display());
        eprintln!("Run: cargo run --bin generate-license-loader-artifact");
        std::process::exit(1);
    }
}
