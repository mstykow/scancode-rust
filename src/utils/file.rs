use chrono::{TimeZone, Utc};
use glob::Pattern;
use std::fs;
use std::path::Path;

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
