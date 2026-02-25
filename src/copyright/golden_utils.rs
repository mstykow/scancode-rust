use std::fs;
use std::io;
use std::path::Path;

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

    let decoded = crate::utils::file::decode_bytes_to_string(&bytes);
    if !decoded.is_empty() {
        return Ok(decoded);
    }

    let allow_binary_strings = matches!(ext.as_deref(), Some("dll") | Some("exe"));
    if allow_binary_strings {
        return Ok(extract_printable_strings(&bytes));
    }

    Ok(String::new())
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
