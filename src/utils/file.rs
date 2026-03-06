use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use chrono::{TimeZone, Utc};
use glob::Pattern;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractedTextKind {
    None,
    Decoded,
    Pdf,
    BinaryStrings,
}

/// Get the creation date of a file or directory as an RFC3339 string.
pub fn get_creation_date(metadata: &fs::Metadata) -> Option<String> {
    metadata.created().ok().map(|time: std::time::SystemTime| {
        let seconds_since_epoch = time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Utc.timestamp_opt(seconds_since_epoch, 0)
            .single()
            .unwrap_or_else(Utc::now)
            .to_rfc3339()
    })
}

/// Check if a path should be excluded based on a list of glob patterns.
pub fn is_path_excluded(path: &Path, exclude_patterns: &[Pattern]) -> bool {
    let path_str = path.to_string_lossy();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();

    for pattern in exclude_patterns {
        // Match against full path
        if pattern.matches(&path_str) {
            return true;
        }

        // Match against just the file/directory name
        if pattern.matches(&file_name) {
            return true;
        }
    }

    false
}

/// Decode a byte buffer to a String, trying UTF-8 first, then Latin-1.
///
/// Latin-1 (ISO-8859-1) maps bytes 0x00-0xFF directly to Unicode U+0000-U+00FF,
/// so it can decode any byte sequence. This matches Python ScanCode's use of
/// `UnicodeDammit` which auto-detects encoding with Latin-1 as fallback.
pub fn decode_bytes_to_string(bytes: &[u8]) -> String {
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            let bytes = e.into_bytes();
            // Binary heuristic: >10% control chars (0x00-0x08, 0x0E-0x1F) means binary.
            let control_count = bytes
                .iter()
                .filter(|&&b| b < 0x09 || (b > 0x0D && b < 0x20))
                .count();
            if control_count > bytes.len() / 10 {
                return String::new();
            }
            bytes.iter().map(|&b| b as char).collect()
        }
    }
}

pub fn extract_text_for_detection(path: &Path, bytes: &[u8]) -> (String, ExtractedTextKind) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    if matches!(ext.as_deref(), Some("pdf")) {
        let text = extract_pdf_text(bytes);
        return if text.is_empty() {
            (String::new(), ExtractedTextKind::None)
        } else {
            (text, ExtractedTextKind::Pdf)
        };
    }

    let decoded = decode_bytes_to_string(bytes);
    if !decoded.is_empty() {
        return (decoded, ExtractedTextKind::Decoded);
    }

    if matches!(ext.as_deref(), Some("dll") | Some("exe")) {
        let text = extract_printable_strings(bytes);
        return if text.is_empty() {
            (String::new(), ExtractedTextKind::None)
        } else {
            (text, ExtractedTextKind::BinaryStrings)
        };
    }

    (String::new(), ExtractedTextKind::None)
}

fn extract_pdf_text(bytes: &[u8]) -> String {
    if bytes.len() < 5 || &bytes[..5] != b"%PDF-" {
        return String::new();
    }

    let extracted = catch_unwind(AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem_by_pages(bytes)
    }));
    match extracted {
        Ok(Ok(pages)) => {
            let Some(text) = pages.into_iter().next() else {
                return String::new();
            };
            let normalized = text.replace(['\r', '\u{0c}'], "\n");
            if normalized.trim().is_empty() {
                String::new()
            } else {
                normalized
            }
        }
        Ok(Err(_)) | Err(_) => String::new(),
    }
}

pub fn extract_printable_strings(bytes: &[u8]) -> String {
    const MIN_LEN: usize = 4;
    const MAX_OUTPUT_BYTES: usize = 2_000_000;

    fn is_printable_ascii(b: u8) -> bool {
        matches!(b, 0x20..=0x7E)
    }

    let mut out = String::new();
    let mut run: Vec<u8> = Vec::new();

    let flush_run = |out: &mut String, run: &mut Vec<u8>| {
        if run.len() >= MIN_LEN {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&String::from_utf8_lossy(run));
        }
        run.clear();
    };

    for &b in bytes {
        if is_printable_ascii(b) {
            run.push(b);
        } else {
            flush_run(&mut out, &mut run);
            if out.len() >= MAX_OUTPUT_BYTES {
                return out;
            }
        }
    }
    flush_run(&mut out, &mut run);
    if out.len() >= MAX_OUTPUT_BYTES {
        return out;
    }

    for start in 0..=1 {
        run.clear();
        let mut i = start;
        while i + 1 < bytes.len() {
            let b0 = bytes[i];
            let b1 = bytes[i + 1];
            let (ch, zero) = if start == 0 { (b0, b1) } else { (b1, b0) };
            if is_printable_ascii(ch) && zero == 0 {
                run.push(ch);
            } else {
                flush_run(&mut out, &mut run);
                if out.len() >= MAX_OUTPUT_BYTES {
                    return out;
                }
            }
            i += 2;
        }
        flush_run(&mut out, &mut run);
        if out.len() >= MAX_OUTPUT_BYTES {
            return out;
        }
    }

    out
}
