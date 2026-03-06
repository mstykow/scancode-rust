use std::fs;
use std::io;
use std::path::Path;

use crate::utils::file::{decode_bytes_to_string, extract_printable_strings};

pub fn canonicalize_golden_value(s: &str) -> String {
    let s = s
        .replace(". ,", ".,")
        .replace(") ,", "),")
        .replace(" ,", ",");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn read_input_content(input_path: &Path) -> io::Result<String> {
    let bytes = fs::read(input_path)?;
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    let skip_entirely = matches!(ext.as_deref(), Some("mp4") | Some("rgb"));
    if skip_entirely {
        return Ok(String::new());
    }

    let decoded = decode_bytes_to_string(&bytes);
    if !decoded.is_empty() {
        return Ok(decoded);
    }

    let allow_binary_strings = matches!(ext.as_deref(), Some("dll") | Some("exe"));
    if allow_binary_strings {
        return Ok(extract_printable_strings(&bytes));
    }

    Ok(String::new())
}
