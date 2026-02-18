use std::fs;
use std::path::PathBuf;

fn main() {
    let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");

    let engine = scancode_rust::license_detection::LicenseDetectionEngine::new(&data_path)
        .expect("Failed to create engine");

    let rtf_bytes = fs::read("testdata/license-golden/datadriven/lic1/gpl_eula.rtf")
        .expect("Failed to read RTF file");

    let rtf_text = String::from_utf8_lossy(&rtf_bytes);
    println!("RTF raw text (first 500 chars):");
    println!("{}", &rtf_text[..rtf_text.len().min(500)]);
    println!("\n---\n");

    let detections = engine.detect(&rtf_text).expect("Detection failed");

    println!("Detections: {}", detections.len());
    for (i, d) in detections.iter().enumerate() {
        println!("Detection {}: {:?}", i, d.license_expression);
        for m in &d.matches {
            println!(
                "  Match: {} lines {}-{} score={} len={} rule_id={}",
                m.license_expression,
                m.start_line,
                m.end_line,
                m.score,
                m.matched_length,
                m.rule_identifier
            );
        }
    }
}
