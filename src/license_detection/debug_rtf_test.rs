#[cfg(test)]
mod debug_tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn debug_gpl_eula_rtf() {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            eprintln!("Reference data not available");
            return;
        }

        let engine = LicenseDetectionEngine::new(&data_path).expect("Failed to create engine");

        let rtf_bytes = fs::read("testdata/license-golden/datadriven/lic1/gpl_eula.rtf")
            .expect("Failed to read RTF file");

        let rtf_text = String::from_utf8_lossy(&rtf_bytes);
        eprintln!("RTF raw text (first 500 chars):");
        eprintln!("{}", &rtf_text[..rtf_text.len().min(500)]);
        eprintln!("\n---\n");

        let detections = engine.detect(&rtf_text).expect("Detection failed");

        eprintln!("Detections: {}", detections.len());
        for (i, d) in detections.iter().enumerate() {
            eprintln!("Detection {}: {:?}", i, d.license_expression);
            for m in &d.matches {
                eprintln!(
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
}
