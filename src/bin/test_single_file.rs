use scancode_rust::license_detection::LicenseDetectionEngine;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin test_single_file -- <file>");
        std::process::exit(1);
    }

    let test_file = PathBuf::from(&args[1]);

    // Initialize engine
    let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    println!("Loading license data from {:?}...", data_path);

    let start = Instant::now();
    let engine = LicenseDetectionEngine::new(&data_path).expect("Failed to create engine");
    println!("Engine initialized in {:?}\n", start.elapsed());

    // Read file
    let content = std::fs::read(&test_file).expect("Failed to read file");
    println!("File: {}", test_file.display());
    println!("Size: {} bytes\n", content.len());

    // Extract text
    let start = Instant::now();
    let text = scancode_rust::utils::file_text::extract_text_for_detection(&content, &test_file);
    let text = text.map(|ft| ft.text);
    println!("Text extraction: {:?}", start.elapsed());

    let text = match text {
        Some(t) => t,
        None => {
            println!("No text extracted (binary file)");
            return;
        }
    };

    println!("Text length: {} chars\n", text.len());

    // Run detection
    let start = Instant::now();
    let matches = engine
        .detect_matches(&text, false)
        .expect("Detection failed");
    let elapsed = start.elapsed();

    println!("Detection time: {:?}", elapsed);
    println!("Matches found: {}\n", matches.len());

    for m in &matches {
        println!(
            "  - {} (lines {}-{}, score {:.2})",
            m.license_expression, m.start_line, m.end_line, m.score
        );
    }
}
