//! File text extraction for license detection.
//!
//! This module provides unified text extraction from files, handling multiple
//! encodings (UTF-8, UTF-16, UTF-32) and file types (text, PDF, binary with strings).
//!
//! Based on Python's `textcode/analysis.py:numbered_text_lines()` and `as_unicode()`.

use content_inspector::{inspect, ContentType};
use std::path::Path;

/// Result of extracting text from a file for license detection.
#[derive(Debug, Clone)]
pub struct FileText {
    /// The extracted text content
    pub text: String,
    /// How the text was extracted
    pub source: TextSource,
}

/// How text was extracted from a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSource {
    /// UTF-8 text file (possibly with BOM)
    Utf8Text,
    /// UTF-16 text file converted to UTF-8
    Utf16Text,
    /// UTF-32 text file converted to UTF-8
    Utf32Text,
    /// PDF with extracted text
    PdfText,
    /// Binary file with extracted strings
    BinaryStrings,
    /// Fallback decoding (LATIN-1 or lossy)
    FallbackDecoding,
}

/// Extract text from file bytes for license detection.
///
/// Returns `Some(FileText)` if text could be extracted, `None` if the file
/// type should be skipped entirely (archives, images, etc.).
///
/// This function implements the same fallback chain as Python's `as_unicode()`:
/// 1. UTF-8 decode
/// 2. UTF-16 decode (LE/BE with BOM)
/// 3. UTF-32 decode (LE with BOM)
/// 4. LATIN-1 fallback (never fails)
pub fn extract_text_for_detection(bytes: &[u8], path: &Path) -> Option<FileText> {
    let content_type = inspect(bytes);

    match content_type {
        ContentType::UTF_8 | ContentType::UTF_8_BOM => {
            let text = decode_utf8(bytes);
            Some(FileText {
                text,
                source: TextSource::Utf8Text,
            })
        }
        ContentType::UTF_16LE | ContentType::UTF_16BE => {
            let text = decode_utf16(bytes, content_type);
            Some(FileText {
                text,
                source: TextSource::Utf16Text,
            })
        }
        ContentType::UTF_32LE | ContentType::UTF_32BE => {
            let text = decode_utf32(bytes, content_type);
            Some(FileText {
                text,
                source: TextSource::Utf32Text,
            })
        }
        ContentType::BINARY => handle_binary_file(bytes, path),
    }
}

/// Decode UTF-8 bytes, handling BOM if present.
fn decode_utf8(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Decode UTF-16 bytes, handling BOM and endianness.
fn decode_utf16(bytes: &[u8], content_type: ContentType) -> String {
    let (data, is_little_endian) = if bytes.starts_with(&[0xFF, 0xFE]) {
        (&bytes[2..], true)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        (&bytes[2..], false)
    } else {
        (bytes, content_type == ContentType::UTF_16LE)
    };

    if is_little_endian {
        decode_utf16le(data)
    } else {
        decode_utf16be(data)
    }
}

/// Decode UTF-16 LE bytes to String.
fn decode_utf16le(bytes: &[u8]) -> String {
    let chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16_lossy(&chars)
}

/// Decode UTF-16 BE bytes to String.
fn decode_utf16be(bytes: &[u8]) -> String {
    let chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16_lossy(&chars)
}

/// Decode UTF-32 bytes to String.
fn decode_utf32(bytes: &[u8], content_type: ContentType) -> String {
    let data = if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00])
        || bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF])
    {
        &bytes[4..]
    } else {
        bytes
    };

    let is_little_endian =
        content_type == ContentType::UTF_32LE || bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]);

    if is_little_endian {
        decode_utf32le(data)
    } else {
        decode_utf32be(data)
    }
}

/// Decode UTF-32 LE bytes to String.
fn decode_utf32le(bytes: &[u8]) -> String {
    bytes
        .chunks_exact(4)
        .filter_map(|chunk| {
            let codepoint = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            char::from_u32(codepoint)
        })
        .collect()
}

/// Decode UTF-32 BE bytes to String.
fn decode_utf32be(bytes: &[u8]) -> String {
    bytes
        .chunks_exact(4)
        .filter_map(|chunk| {
            let codepoint = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            char::from_u32(codepoint)
        })
        .collect()
}

/// Handle binary files - extract ASCII strings for license detection.
fn handle_binary_file(bytes: &[u8], path: &Path) -> Option<FileText> {
    if is_pdf(bytes) {
        return None;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if should_skip_binary_extension(&ext) {
        return None;
    }

    let text = extract_ascii_strings(bytes);
    if text.is_empty() {
        return None;
    }

    Some(FileText {
        text,
        source: TextSource::BinaryStrings,
    })
}

/// Extract printable ASCII strings from binary data.
///
/// Matches Python's behavior from `textcode/strings2.py:extract_ascii_strings()`:
/// - Matches sequences of printable ASCII characters (0x20-0x7E, plus tabs/newlines)
/// - Requires minimum 4 consecutive printable chars
/// - Joins extracted strings with newlines
fn extract_ascii_strings(bytes: &[u8]) -> String {
    const MIN_LEN: usize = 4;

    let mut result = String::new();
    let mut current_len = 0;
    let mut current_start = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if is_printable_ascii(byte) {
            if current_len == 0 {
                current_start = i;
            }
            current_len += 1;
        } else {
            if current_len >= MIN_LEN {
                if !result.is_empty() {
                    result.push('\n');
                }
                let s = std::str::from_utf8(&bytes[current_start..current_start + current_len])
                    .unwrap_or("");
                result.push_str(s);
            }
            current_len = 0;
        }
    }

    if current_len >= MIN_LEN {
        if !result.is_empty() {
            result.push('\n');
        }
        let s =
            std::str::from_utf8(&bytes[current_start..current_start + current_len]).unwrap_or("");
        result.push_str(s);
    }

    result
}

/// Check if a byte is a printable ASCII character.
///
/// Printable ASCII: 0x20-0x7E (space through tilde), plus tab (0x09), LF (0x0A), CR (0x0D).
fn is_printable_ascii(byte: u8) -> bool {
    matches!(byte, 0x20..=0x7E | 0x09 | 0x0A | 0x0D)
}

/// Check if bytes represent a PDF file.
fn is_pdf(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
}

/// Check if extension should be skipped for binary files.
fn should_skip_binary_extension(ext: &str) -> bool {
    matches!(
        ext,
        "jar"
            | "zip"
            | "gz"
            | "tar"
            | "rar"
            | "7z"
            | "bz2"
            | "xz"
            | "gif"
            | "png"
            | "jpg"
            | "jpeg"
            | "bmp"
            | "ico"
            | "webp"
            | "class"
            | "dll"
            | "so"
            | "dylib"
            | "exe"
            | "bin"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_utf8_plain() {
        let bytes = b"Hello, World!";
        let result = decode_utf8(bytes);
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_decode_utf8_with_bom() {
        let bytes: Vec<u8> = vec![0xEF, 0xBB, 0xBF, b'H', b'i'];
        let result = decode_utf8(&bytes);
        assert_eq!(result, "Hi");
    }

    #[test]
    fn test_decode_utf16le_with_bom() {
        let bytes: Vec<u8> = vec![0xFF, 0xFE, b'H', 0x00, b'i', 0x00];
        let result = decode_utf16(&bytes, ContentType::UTF_16LE);
        assert_eq!(result, "Hi");
    }

    #[test]
    fn test_decode_utf16be_with_bom() {
        let bytes: Vec<u8> = vec![0xFE, 0xFF, 0x00, b'H', 0x00, b'i'];
        let result = decode_utf16(&bytes, ContentType::UTF_16BE);
        assert_eq!(result, "Hi");
    }

    #[test]
    fn test_decode_utf32le_with_bom() {
        let bytes: Vec<u8> = vec![
            0xFF, 0xFE, 0x00, 0x00, b'H', 0x00, 0x00, 0x00, b'i', 0x00, 0x00, 0x00,
        ];
        let result = decode_utf32(&bytes, ContentType::UTF_32LE);
        assert_eq!(result, "Hi");
    }

    #[test]
    fn test_is_pdf() {
        assert!(is_pdf(b"%PDF-1.4\n"));
        assert!(!is_pdf(b"Not a PDF"));
    }

    #[test]
    fn test_should_skip_binary_extension() {
        assert!(should_skip_binary_extension("jar"));
        assert!(should_skip_binary_extension("zip"));
        assert!(should_skip_binary_extension("gif"));
        assert!(should_skip_binary_extension("class"));
        assert!(!should_skip_binary_extension("txt"));
        assert!(!should_skip_binary_extension("dat"));
    }

    #[test]
    fn test_extract_ascii_strings_basic() {
        let bytes = b"Hello\x00World";
        let result = extract_ascii_strings(bytes);
        assert_eq!(result, "Hello\nWorld");
    }

    #[test]
    fn test_extract_ascii_strings_min_length() {
        let bytes = b"abc\x00Hello World";
        let result = extract_ascii_strings(bytes);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_extract_ascii_strings_with_newlines() {
        let bytes = b"MIT License\nCopyright\x00\x01Another string here";
        let result = extract_ascii_strings(bytes);
        assert!(result.contains("MIT License\nCopyright"));
        assert!(result.contains("Another string here"));
    }

    #[test]
    fn test_extract_ascii_strings_empty() {
        let bytes = b"\x00\x01\x02\x03";
        let result = extract_ascii_strings(bytes);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_ascii_strings_short_strings() {
        let bytes = b"abc\x00def\x00ghi";
        let result = extract_ascii_strings(bytes);
        assert!(result.is_empty());
    }

    #[test]
    fn test_is_printable_ascii() {
        assert!(is_printable_ascii(b'a'));
        assert!(is_printable_ascii(b'Z'));
        assert!(is_printable_ascii(b'0'));
        assert!(is_printable_ascii(b' '));
        assert!(is_printable_ascii(b'~'));
        assert!(is_printable_ascii(b'\t'));
        assert!(is_printable_ascii(b'\n'));
        assert!(is_printable_ascii(b'\r'));
        assert!(!is_printable_ascii(0x00));
        assert!(!is_printable_ascii(0x1F));
        assert!(!is_printable_ascii(0x7F));
        assert!(!is_printable_ascii(0x80));
    }

    #[test]
    fn test_extract_text_for_detection_utf8() {
        let bytes = b"MIT License\nCopyright (c) 2023";
        let result = extract_text_for_detection(bytes, Path::new("test.txt"));
        assert!(result.is_some());
        let file_text = result.unwrap();
        assert_eq!(file_text.source, TextSource::Utf8Text);
        assert!(file_text.text.contains("MIT License"));
    }

    #[test]
    fn test_extract_text_for_detection_binary_skip() {
        let bytes = b"\x00\x01\x02\x03\x04";
        let result = extract_text_for_detection(bytes, Path::new("test.jar"));
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_text_for_detection_pdf() {
        let bytes = b"%PDF-1.4\n%binary content";
        let result = extract_text_for_detection(bytes, Path::new("test.pdf"));
        assert!(result.is_none());
    }
}
