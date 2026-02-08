//! Generate `.expected.json` files for golden tests.
//!
//! This utility runs a parser on a test input file and generates the expected
//! output JSON file that golden tests compare against.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin generate-test-expected <parser_type> <input_file> <output_file>
//! cargo run --bin generate-test-expected --list
//! ```
//!
//! # Example
//!
//! ```bash
//! cargo run --bin generate-test-expected DebianDebParser \
//!   testdata/debian/deb/adduser.deb \
//!   testdata/debian/deb/adduser.deb.expected.json
//! ```
//!
//! # Auto-Discovery
//!
//! This tool automatically discovers ALL parsers registered in `src/parsers/mod.rs`
//! via the `define_parsers!` macro. No manual maintenance required!
//!
//! When you add a new parser to `define_parsers!`, it automatically becomes available here.

use scancode_rust::parsers;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 2 && (args[1] == "--list" || args[1] == "-l") {
        print_available_parsers();
        return;
    }

    if args.len() != 4 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let parser_type = &args[1];
    let input_file = PathBuf::from(&args[2]);
    let output_file = PathBuf::from(&args[3]);

    let package_data = match parsers::parse_by_type_name(parser_type, &input_file) {
        Some(data) => data,
        None => {
            eprintln!("❌ Unknown parser type: {}\n", parser_type);
            eprintln!("Run with --list to see all available parser types\n");
            print_usage(&args[0]);
            std::process::exit(1);
        }
    };

    let json = serde_json::to_string_pretty(&vec![package_data]).unwrap();
    fs::write(&output_file, json).unwrap();
    println!("✅ Generated: {}", output_file.display());
}

fn print_usage(program_name: &str) {
    eprintln!(
        "Usage: {} <parser_type> <input_file> <output_file>",
        program_name
    );
    eprintln!("       {} --list", program_name);
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <parser_type>   Parser struct name (e.g., NpmParser, DebianDebParser)");
    eprintln!("  <input_file>    Path to package manifest file");
    eprintln!("  <output_file>   Path to write expected JSON output");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --list, -l      List all available parser types");
    eprintln!();
    eprintln!("Example:");
    eprintln!("  {} DebianDebParser \\", program_name);
    eprintln!("    testdata/debian/deb/adduser.deb \\");
    eprintln!("    testdata/debian/deb/adduser.deb.expected.json");
}

fn print_available_parsers() {
    println!("Available parser types:");
    println!();

    let mut parsers = parsers::list_parser_types();
    parsers.sort();

    for (i, parser) in parsers.iter().enumerate() {
        println!("  {:<3} {}", format!("{}.", i + 1), parser);
    }

    println!();
    println!("Total: {} parsers", parsers.len());
}
