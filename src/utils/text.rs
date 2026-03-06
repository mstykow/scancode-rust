use std::path::Path;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

const UTF8_BOM_CHAR: char = '\u{FEFF}';

const SOURCE_EXTENSIONS: &[&str] = &[
    ".ada", ".adb", ".asm", ".asp", ".aj", ".bas", ".bat", ".c", ".c++", ".cc", ".clj", ".cob",
    ".cpp", ".cs", ".csh", ".csx", ".cxx", ".d", ".e", ".el", ".f", ".fs", ".f77", ".f90", ".for",
    ".fth", ".ftn", ".go", ".h", ".hh", ".hpp", ".hs", ".html", ".htm", ".hxx", ".java", ".js",
    ".jsx", ".jsp", ".ksh", ".kt", ".lisp", ".lua", ".m", ".m4", ".nim", ".pas", ".php", ".pl",
    ".pp", ".ps1", ".py", ".r", ".rb", ".ruby", ".rs", ".s", ".scala", ".sh", ".swift", ".ts",
    ".vhdl", ".verilog", ".vb", ".groovy", ".po",
];

pub fn is_source(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext_str = ext.to_string_lossy();
            let ext_lower = format!(".{}", ext_str.to_lowercase());
            SOURCE_EXTENSIONS.contains(&ext_lower.as_str())
        })
        .unwrap_or(false)
}

pub fn remove_verbatim_escape_sequences(s: &str) -> String {
    s.replace("\\r", " ")
        .replace("\\n", " ")
        .replace("\\t", " ")
}

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
    use std::path::PathBuf;

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

    #[test]
    fn test_is_source_rust() {
        assert!(is_source(&PathBuf::from("test.rs")));
        assert!(is_source(&PathBuf::from("TEST.RS")));
    }

    #[test]
    fn test_is_source_python() {
        assert!(is_source(&PathBuf::from("script.py")));
    }

    #[test]
    fn test_is_source_javascript() {
        assert!(is_source(&PathBuf::from("app.js")));
    }

    #[test]
    fn test_is_source_c() {
        assert!(is_source(&PathBuf::from("options.c")));
        assert!(is_source(&PathBuf::from("OPTIONS.C")));
    }

    #[test]
    fn test_is_source_not_source() {
        assert!(!is_source(&PathBuf::from("README.md")));
        assert!(!is_source(&PathBuf::from("data.json")));
        assert!(!is_source(&PathBuf::from("config.yaml")));
    }

    #[test]
    fn test_is_source_no_extension() {
        assert!(!is_source(&PathBuf::from("Makefile")));
    }

    #[test]
    fn test_remove_verbatim_escape_sequences_basic() {
        let input = "line1\\nline2\\rline3\\tline4";
        let output = remove_verbatim_escape_sequences(input);
        assert_eq!(output, "line1 line2 line3 line4");
    }

    #[test]
    fn test_remove_verbatim_escape_sequences_only_backslash_n() {
        let input = "hello\\nworld";
        let output = remove_verbatim_escape_sequences(input);
        assert_eq!(output, "hello world");
    }

    #[test]
    fn test_remove_verbatim_escape_sequences_no_escapes() {
        let input = "normal text without escapes";
        let output = remove_verbatim_escape_sequences(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_remove_verbatim_escape_sequences_actual_newline() {
        let input = "line1\nline2";
        let output = remove_verbatim_escape_sequences(input);
        assert_eq!(output, "line1\nline2");
    }

    #[test]
    fn test_remove_verbatim_escape_sequences_multiple() {
        let input = "a\\nb\\nc\\n";
        let output = remove_verbatim_escape_sequences(input);
        assert_eq!(output, "a b c ");
    }

    #[test]
    fn test_remove_verbatim_escape_sequences_options_c_sample() {
        let input = "Try `progname --help' for more information.\\n";
        let output = remove_verbatim_escape_sequences(input);
        assert_eq!(output, "Try `progname --help' for more information. ");
    }

    #[test]
    fn test_is_source_options_c() {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic2/regression/options.c");
        assert!(
            is_source(&path),
            "options.c should be recognized as source file"
        );
    }
}
