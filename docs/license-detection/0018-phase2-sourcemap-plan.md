# Phase 2: Source Map File Processing Implementation Plan

**Status:** Ready for Implementation  
**Created:** 2026-03-01  
**Estimated Tests Fixed:** ~2  
**Priority:** High  
**Complexity:** Medium

---

## Executive Summary

Source map files (`.js.map`, `.css.map`) contain embedded source code in JSON format. The license text within uses escaped newlines (`\n`), which must be properly unescaped before tokenization. Currently, Rust processes the raw JSON string, causing tokenization mismatches that lead to duplicate detections.

---

## Problem Statement

### Current Behavior

When Rust processes `ar-ER.js.map`:

1. Reads raw JSON file content as UTF-8 text
2. Tokenizer sees literal `\n` (backslash + 'n') in the JSON string
3. Tokenization produces extra tokens that break license matching
4. Multiple shorter rules match instead of one complete match

**Example:**
```
ar-ER.js.map expected: ["mit"]
ar-ER.js.map actual:   ["mit", "mit"]
```

### Root Cause

The `sourcesContent` array in source map JSON contains license text like:
```json
"sourcesContent": ["Use of this source code is governed by an MIT-style license\nthat can be found..."]
```

The JSON string `"...license\nthat..."` represents actual text with a newline between "license" and "that". When processed correctly:
- JSON parser unescapes `\n` → actual newline character
- Tokenizer sees: `["license", "that"]`

When processed incorrectly (current Rust):
- Raw text contains literal backslash-n: `"...license\nthat..."`
- Tokenizer may see extra tokens or different token boundaries

### Python Reference Implementation

**File:** `reference/scancode-toolkit/src/textcode/analysis.py:223-247`

```python
def js_map_sources_lines(location):
    """
    Yield unicode text lines from the js.map or css.map file at `location`.
    """
    with io.open(location, encoding='utf-8') as jsm:
        content = json.load(jsm)
        sources = content.get('sourcesContent', [])
        for entry in sources:
            entry = replace_verbatim_cr_lf_chars(entry)
            for line in entry.splitlines():
                l = remove_verbatim_cr_lf_tab_chars(line)
                yield l
```

**Key operations:**
1. Parse JSON (automatically unescapes `\n` to actual newlines)
2. Get `sourcesContent` array
3. For each entry, call `replace_verbatim_cr_lf_chars()` to convert remaining escaped sequences
4. Split into lines and yield

**`replace_verbatim_cr_lf_chars()` (line 306-318):**
```python
def replace_verbatim_cr_lf_chars(s):
    return (s
        .replace('\\r\\n', '\n')
        .replace('\\r', '\n')
        .replace('\\n', '\n')
    )
```

**Important: What Each Layer Handles**

1. **JSON parsing** (`json.load()`): Unescapes JSON escape sequences
   - JSON string `"hello\nworld"` becomes `hello` + newline + `world` in Python string
   - JSON string `"hello\\nworld"` (double-escaped) becomes `hello\nworld` (backslash-n literal)

2. **`replace_verbatim_cr_lf_chars()`**: Handles source code that contained literal `\n` text
   - After JSON parsing, if source had literal `\n`, it becomes backslash-n in the string
   - This function converts those to actual newlines for proper tokenization

---

## Implementation Plan

### Step 1: Create Source Map Module

**Create:** `src/utils/sourcemap.rs`

```rust
//! Source map file processing for license detection.
//!
//! Source map files (.js.map, .css.map) are JSON files containing embedded
//! source code in a `sourcesContent` array. This module extracts that content
//! for license detection.

use std::path::Path;

/// Check if a file is a source map file based on extension.
pub fn is_sourcemap(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_lower = ext.to_lowercase();
            ext_lower == "map" && 
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with(".js.map") || name.ends_with(".css.map"))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Extract source content from a source map JSON file.
///
/// Parses the JSON and extracts all entries from `sourcesContent`,
/// combining them with newlines for license detection.
///
/// Returns `Some(combined_text)` if successfully parsed with content.
/// Returns `None` if JSON parsing fails or no sourcesContent exists.
pub fn extract_sourcemap_content(json_text: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(json_text).ok()?;
    let sources = json.get("sourcesContent")?.as_array()?;
    
    let combined: String = sources
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| replace_verbatim_cr_lf_chars(s))
        .collect::<Vec<_>>()
        .join("\n");
    
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}

/// Replace verbatim escaped CR/LF characters with actual newlines.
///
/// This matches Python's `replace_verbatim_cr_lf_chars()` behavior exactly:
/// - Double-escaped (e.g., source had literal `\r` that was escaped again):
///   - `\\r\\n` (backslash-backslash-r-backslash-backslash-n) → newline
///   - `\\r` (backslash-backslash-r) → newline
///   - `\\n` (backslash-backslash-n) → newline
/// - Single-escaped (e.g., JSON-escaped newlines):
///   - `\r\n` (backslash-r-backslash-n) → newline
///   - `\r` (backslash-r) → newline
///   - `\n` (backslash-n) → newline
fn replace_verbatim_cr_lf_chars(s: &str) -> String {
    s.replace("\\\\r\\\\n", "\n")
     .replace("\\r\\n", "\n")
     .replace("\\\\r", "\n")
     .replace("\\\\n", "\n")
     .replace("\\r", "\n")
     .replace("\\n", "\n")
}
```

### Step 2: Update `src/utils/mod.rs`

**Add module declaration:**

```rust
pub mod sourcemap;
```

### Step 3: Update `src/scanner/process.rs`

**Modify `extract_information_from_content()` to handle source maps:**

```rust
fn extract_information_from_content(
    file_info_builder: &mut FileInfoBuilder,
    path: &Path,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<(), Error> {
    let buffer = fs::read(path)?;

    file_info_builder
        .sha1(Some(calculate_sha1(&buffer)))
        .md5(Some(calculate_md5(&buffer)))
        .sha256(Some(calculate_sha256(&buffer)))
        .programming_language(Some(detect_language(path, &buffer)));

    if let Some(package_data) = try_parse_file(path) {
        file_info_builder.package_data(package_data);
        Ok(())
    } else if let Some(file_text) = extract_text_for_detection(&buffer, path) {
        let mut text_content = file_text.text;
        
        // Handle source map files specially
        if crate::utils::sourcemap::is_sourcemap(path) {
            if let Some(sourcemap_content) = crate::utils::sourcemap::extract_sourcemap_content(&text_content) {
                text_content = sourcemap_content;
            }
        } else if is_source(path) {
            text_content = remove_verbatim_escape_sequences(&text_content);
        }
        
        extract_license_information(
            file_info_builder,
            text_content,
            license_engine,
            include_text,
        )
    } else {
        Ok(())
    }
}
```

### Step 4: Add Tests

**Create:** `src/utils/sourcemap_test.rs`

```rust
//! Tests for source map processing.

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_sourcemap_js_map() {
        assert!(is_sourcemap(&PathBuf::from("app.js.map")));
        assert!(is_sourcemap(&PathBuf::from("APP.JS.MAP")));
    }

    #[test]
    fn test_is_sourcemap_css_map() {
        assert!(is_sourcemap(&PathBuf::from("style.css.map")));
        assert!(is_sourcemap(&PathBuf::from("STYLE.CSS.MAP")));
    }

    #[test]
    fn test_is_sourcemap_not_map() {
        assert!(!is_sourcemap(&PathBuf::from("app.js")));
        assert!(!is_sourcemap(&PathBuf::from("data.json")));
        assert!(!is_sourcemap(&PathBuf::from("other.map"))); // Not .js.map or .css.map
    }

    #[test]
    fn test_extract_sourcemap_content_basic() {
        let json = r#"{"version":3,"sourcesContent":["hello\nworld"]}"#;
        let result = extract_sourcemap_content(json);
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("hello"));
        assert!(content.contains("world"));
    }

    #[test]
    fn test_extract_sourcemap_content_mit_license() {
        let json = r#"{"version":3,"sourcesContent":["Use of this source code is governed by an MIT-style license\nthat can be found in the LICENSE file"]}"#;
        let result = extract_sourcemap_content(json);
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("MIT-style license"));
        assert!(content.contains("LICENSE file"));
        // Verify newline was unescaped
        assert!(content.contains("\n"));
    }

    #[test]
    fn test_extract_sourcemap_content_multiple_entries() {
        let json = r#"{"version":3,"sourcesContent":["first\nfile","second\nfile"]}"#;
        let result = extract_sourcemap_content(json);
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("first"));
        assert!(content.contains("second"));
    }

    #[test]
    fn test_extract_sourcemap_content_no_sources() {
        let json = r#"{"version":3,"sources":[]}"#;
        let result = extract_sourcemap_content(json);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_sourcemap_content_invalid_json() {
        let json = r#"not valid json"#;
        let result = extract_sourcemap_content(json);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_sourcemap_content_null_entries() {
        // Some source maps have null in sourcesContent
        let json = r#"{"version":3,"sourcesContent":[null,"actual\ncontent"]}"#;
        let result = extract_sourcemap_content(json);
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("actual"));
    }

    #[test]
    fn test_replace_verbatim_cr_lf_chars() {
        // Single-escaped (JSON-escaped, which JSON parser already unescaped)
        assert_eq!(replace_verbatim_cr_lf_chars("a\\nb"), "a\nb");
        assert_eq!(replace_verbatim_cr_lf_chars("a\\rb"), "a\nb");
        assert_eq!(replace_verbatim_cr_lf_chars("a\\r\\nb"), "a\nb");
        // Double-escaped (source had literal backslash-r/n that got escaped again)
        assert_eq!(replace_verbatim_cr_lf_chars("a\\\\nb"), "a\nb");
        assert_eq!(replace_verbatim_cr_lf_chars("a\\\\rb"), "a\nb");
        assert_eq!(replace_verbatim_cr_lf_chars("a\\\\r\\\\nb"), "a\nb");
    }

    #[test]
    fn test_ar_er_js_map_detection() {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic2/ar-ER.js.map");
        if !path.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }
        
        let text = std::fs::read_to_string(&path).expect("Failed to read file");
        let result = extract_sourcemap_content(&text);
        assert!(result.is_some(), "Should extract content from ar-ER.js.map");
        
        let content = result.unwrap();
        assert!(content.contains("MIT-style license"), "Should contain MIT license text");
    }
}
```

### Step 5: Add Golden Test Verification

**Add to existing golden test or create specific test:**

```rust
#[test]
fn test_sourcemap_ar_er_golden() {
    // This test verifies the fix for ar-ER.js.map
    let engine = create_engine_from_reference().expect("Engine creation failed");
    
    let path = PathBuf::from("testdata/license-golden/datadriven/lic2/ar-ER.js.map");
    if !path.exists() {
        eprintln!("Skipping test: file not found");
        return;
    }
    
    let raw_text = std::fs::read_to_string(&path).expect("Failed to read file");
    let content = extract_sourcemap_content(&raw_text)
        .expect("Should extract sourcemap content");
    
    let detections = engine.detect(&content, false).expect("Detection failed");
    
    let expressions: Vec<_> = detections
        .iter()
        .filter_map(|d| d.license_expression.as_ref())
        .collect();
    
    // Should detect exactly one MIT license
    assert_eq!(expressions.len(), 1, "Expected exactly 1 detection, got {:?}", expressions);
    assert!(expressions[0].contains("mit"), "Expected MIT, got {:?}", expressions);
}
```

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/utils/sourcemap.rs` | Create | Source map extraction module |
| `src/utils/sourcemap_test.rs` | Create | Unit tests for source map module |
| `src/utils/mod.rs` | Modify | Add `pub mod sourcemap;` |
| `src/scanner/process.rs` | Modify | Add source map handling in `extract_information_from_content()` |

---

## Integration Points

### Where to Hook Into Pipeline

```
File read (process.rs:extract_information_from_content)
    ↓
Check if sourcemap file (NEW: is_sourcemap)
    ↓
If sourcemap:
    Parse JSON → extract sourcesContent (NEW: extract_sourcemap_content)
    ↓
Else if source file:
    Apply remove_verbatim_escape_sequences
    ↓
License detection (existing pipeline)
```

### Key Considerations

1. **Order matters:** Source map extraction must happen BEFORE the source file escape processing, because we want the JSON-parsed content, not the raw JSON string.

2. **Fallback behavior:** If JSON parsing fails, fall back to raw text processing. This matches Python's try/except pattern.

3. **Null handling:** Source maps can have `null` entries in `sourcesContent`. Filter these out.

4. **Multiple entries:** `sourcesContent` is an array. Join entries with newlines.

---

## Testing Strategy

### Unit Tests

1. **`is_sourcemap()`** - Extension detection
   - `.js.map` files → true
   - `.css.map` files → true
   - Other `.map` files → false
   - Non-map files → false

2. **`extract_sourcemap_content()`** - JSON extraction
   - Valid JSON with sourcesContent → returns content
   - Invalid JSON → returns None
   - Missing sourcesContent → returns None
   - Null entries in array → skipped
   - Multiple entries → joined with newlines

3. **`replace_verbatim_cr_lf_chars()`** - Escape replacement
   - `\n` → newline
   - `\r` → newline
   - `\r\n` → single newline

### Integration Tests

1. **End-to-end detection:**
   ```bash
   cargo test ar_er_debug_test --lib -- --nocapture
   ```

2. **Golden test verification:**
   ```bash
   cargo test --release -q --lib license_detection::golden_test
   ```

### Manual Verification

```bash
# Before fix
cargo run -- testdata/license-golden/datadriven/lic2/ar-ER.js.map -o before.json

# After fix
cargo run -- testdata/license-golden/datadriven/lic2/ar-ER.js.map -o after.json

# Compare - should show single MIT detection
```

---

## Expected Outcomes

### Tests Fixed

| Test File | Before | After |
|-----------|--------|-------|
| `ar-ER.js.map` | `["mit", "mit"]` | `["mit"]` |

### No Regressions

All existing golden tests should continue to pass. Source map processing is additive and only affects `.js.map` and `.css.map` files.

---

## Risks and Mitigations

### Low Risk

This is an isolated change that:
- Only affects source map files (identified by extension)
- Falls back to raw text if JSON parsing fails
- Does not modify existing detection logic

### Potential Issues

1. **Large source maps:** Very large `sourcesContent` could cause memory issues.
   - **Mitigation:** This matches Python behavior; acceptable for now.

2. **Non-standard source maps:** Some tools may generate non-standard JSON.
   - **Mitigation:** Fall back to raw text processing if parsing fails.

3. **CSS map files have no test coverage:** No `.css.map` test files exist in testdata.
   - **Mitigation:** The implementation handles both `.js.map` and `.css.map` identically. CSS maps work the same way (JSON with `sourcesContent`). Unit tests verify the extension matching.

---

## References

- **Python Implementation:** `reference/scancode-toolkit/src/textcode/analysis.py:223-247`
- **Source Map Spec:** https://docs.google.com/document/d/1U1RGAehQwRypUTovF1KRlpiOFze0b-_2gc6fAH0KY0k
- **Roadmap:** `docs/license-detection/0016-feature-parity-roadmap.md`
- **Debug Investigation:** `src/license_detection/investigation/ar_er_debug_test.rs`
