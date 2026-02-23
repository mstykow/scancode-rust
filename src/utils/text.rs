const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

const UTF8_BOM_CHAR: char = '\u{FEFF}';

pub fn strip_utf8_bom_bytes(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(UTF8_BOM) {
        &bytes[3..]
    } else {
        bytes
    }
}

pub fn strip_utf8_bom_str(s: &str) -> &str {
    s.strip_prefix(UTF8_BOM_CHAR).unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_utf8_bom_bytes_with_bom() {
        let bytes = vec![0xEF, 0xBB, 0xBF, b't', b'e', b's', b't'];
        let stripped = strip_utf8_bom_bytes(&bytes);
        assert_eq!(stripped, b"test");
    }

    #[test]
    fn test_strip_utf8_bom_bytes_without_bom() {
        let bytes = b"test";
        assert_eq!(strip_utf8_bom_bytes(bytes), b"test");
    }

    #[test]
    fn test_strip_utf8_bom_bytes_empty() {
        let bytes: &[u8] = &[];
        assert_eq!(strip_utf8_bom_bytes(bytes), bytes);
    }

    #[test]
    fn test_strip_utf8_bom_bytes_only_bom() {
        let bytes: &[u8] = &[0xEF, 0xBB, 0xBF];
        assert!(strip_utf8_bom_bytes(bytes).is_empty());
    }

    #[test]
    fn test_strip_utf8_bom_str_with_bom() {
        let s = "\u{FEFF}Hello World";
        assert_eq!(strip_utf8_bom_str(s), "Hello World");
    }

    #[test]
    fn test_strip_utf8_bom_str_without_bom() {
        let s = "Hello World";
        assert_eq!(strip_utf8_bom_str(s), "Hello World");
    }

    #[test]
    fn test_strip_utf8_bom_str_empty() {
        let s = "";
        assert_eq!(strip_utf8_bom_str(s), "");
    }

    #[test]
    fn test_strip_utf8_bom_str_only_bom() {
        let s = "\u{FEFF}";
        assert_eq!(strip_utf8_bom_str(s), "");
    }

    #[test]
    fn test_bom_character_is_not_whitespace() {
        let s = "\u{FEFF}Hello";
        assert_ne!(s.trim(), "Hello");
        assert_eq!(strip_utf8_bom_str(s), "Hello");
    }
}
