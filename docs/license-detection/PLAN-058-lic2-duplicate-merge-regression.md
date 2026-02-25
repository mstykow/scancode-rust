# PLAN-058: Lic2 Regression - Duplicate License Detections Merged

## Status: ROOT CAUSE IDENTIFIED

## Problem Statement

The CDDL fix caused 3 new regressions in lic2 tests where duplicate license detections are incorrectly merged into a single detection.

### Golden Test Changes

| Test Set | Baseline | Current | Change |
|----------|----------|---------|--------|
| lic1 | 240 passed, 51 failed | 241 passed, 50 failed | **+1** |
| lic2 | 802 passed, 51 failed | 799 passed, 54 failed | **-3** |
| external | 2169 passed, 398 failed | 2176 passed, 391 failed | **+7** |

### New Lic2 Failures

| Test File | Expected | Actual | Issue |
|-----------|----------|--------|-------|
| `1908-bzip2/bzip2.106.c` | `["bzip2-libbzip-2010", "bzip2-libbzip-2010"]` | `["bzip2-libbzip-2010"]` | Under-merge |
| `apache-2.0_and_apache-2.0.txt` | `["apache-2.0", "apache-2.0"]` | `["apache-2.0"]` | Under-merge |
| `aladdin-md5_and_not_rsa-md5.txt` | `["zlib", "zlib"]` | `["zlib"]` | Under-merge |

---

## Root Cause Analysis

### Initial Hypothesis (INCORRECT)

The `qcontains()` fix for mixed `qspan_positions` modes was suspected of causing two separate license detections to be incorrectly merged.

### Actual Root Cause

**The second license match is never created in the first place** due to a missing preprocessing step for source files.

#### Technical Details

**The Issue**: Source files like `bzip2.106.c` contain C string literals with escape sequences:
```c
"   This program is free software; you can redistribute it and/or modify\n"
"   it under the terms set out in the LICENSE file..."
```

The `\n` in the source code is a **literal backslash-n** (two characters: `\` and `n`), not an actual newline.

**Python Behavior** (`textcode/analysis.py:298-303`):
```python
def remove_verbatim_cr_lf_tab_chars(s):
    """Replace literal \n, \r, \t with spaces."""
    return s.replace('\\r', ' ').replace('\\n', ' ').replace('\\t', ' ')

def unicode_text_lines(location, decrlf=False):
    lines = _unicode_text_lines(location)
    if decrlf:  # True for .c files
        return map(remove_verbatim_cr_lf_tab_chars, lines)  # <-- PREPROCESSING
```

For `.c` files, Python calls `remove_verbatim_cr_lf_tab_chars()` which replaces:
- `modify\n` → `modify ` (backslash-n becomes space)
- Then tokenized as `["modify"]` (no "n" token)

**Rust Behavior** (MISSING preprocessing):
- `modify\n` tokenizes to `["modify", "n"]` (backslash stripped, "n" kept)
- Rule text has actual newlines, so `modify\nit` tokenizes to `["modify", "it"]`
- Token mismatch: query has `"n"` where rule expects `"it"`

**Consequence**: The Aho-Corasick automaton cannot match the second rule because the token sequence differs:
- Query tokens at position 84-96: `[...modify, n, it, ...]`
- Rule tokens: `[...modify, it, ...]`
- Match breaks at position 96: query has token `8579` ("n") but rule expects `7054` ("it")

#### Evidence

1. **Investigation test** (`src/license_detection/duplicate_merge_investigation_test.rs`):
   - Token 8579 = "n"
   - Token 7054 = "it"
   - Match fails at offset 12 in the token sequence

2. **Python reference** (`reference/scancode-toolkit/src/textcode/analysis.py`):
   - `is_source()` identifies `.c` files
   - `unicode_text_lines(decrlf=True)` applies preprocessing
   - `remove_verbatim_cr_lf_tab_chars()` replaces `\n` with space

3. **Rust implementation** (`src/license_detection/query.rs`):
   - No preprocessing for literal escape sequences
   - Lines are tokenized directly without `remove_verbatim_cr_lf_tab_chars()`

---

## Investigation Findings

### 1. Where to Add Preprocessing

**Primary location: `src/license_detection/query.rs` in `Query::with_options()`**

The preprocessing should happen in the tokenization loop (line 331 onwards), where text is processed line-by-line:

```rust
for line in text.lines() {
    let line_trimmed = line.trim();
    // PREPROCESSING GOES HERE for source files
    for token in tokenize_without_stopwords(line_trimmed) {
        ...
    }
}
```

**Architecture consideration**: The `LicenseDetectionEngine::detect()` method currently receives only text content, not file path info. To enable file-type-aware preprocessing:

**Option A (Recommended)**: Add preprocessing in scanner before calling detect:
- `src/scanner/process.rs:187` - `engine.detect(&text_content)` becomes `engine.detect_with_options(&text_content, path)`
- Simpler, keeps file path context available

**Option B**: Add `is_source` parameter to `Query::new()` and propagate through call chain
- Requires updating `LicenseDetectionEngine::detect()` signature
- More invasive change

### 2. Which File Types Need Preprocessing

Python's `is_source()` function (lines 351-423) identifies source files by extension:

**Core source extensions** (most likely to have string literals with escape sequences):
- C/C++: `.c`, `.c++`, `.cc`, `.cpp`, `.cxx`, `.h`, `.hh`, `.hpp`, `.hxx`
- Java: `.java`
- JavaScript/TypeScript: `.js`, `.jsx`, `.ts`, `.jsp`
- Python: `.py`
- Rust: `.rs`
- Go: `.go`
- Ruby: `.rb`, `.ruby`
- PHP: `.php`
- Shell: `.sh`, `.ksh`, `.csh`, `.bat`
- Others: `.cs`, `.swift`, `.kt`, `.scala`, `.rs`, `.pl`, `.lua`, `.m`, `.f`, `.f90`, `.pas`, `.ada`, `.adb`, `.el`, `.clj`, `.hs`, `.nim`, `.d`, `.s`, `.asm`

**Rust implementation** should define a constant with these extensions:

```rust
const SOURCE_EXTENSIONS: &[&str] = &[
    ".c", ".c++", ".cc", ".cpp", ".cxx", ".h", ".hh", ".hpp", ".hxx",
    ".java", ".js", ".jsx", ".ts", ".jsp", ".py", ".rs", ".go", ".rb",
    // ... (full list from Python)
];

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SOURCE_EXTENSIONS.contains(&format!(".{}", ext).as_str()))
        .unwrap_or(false)
}
```

### 3. What Strings to Replace

**Python's `remove_verbatim_cr_lf_tab_chars()` (lines 298-303)**:
```python
def remove_verbatim_cr_lf_tab_chars(s):
    """Return a string replacing by a space any verbatim but escaped line endings
    and tabs (such as a literal \n or \r \t).
    """
    return s.replace('\\r', ' ').replace('\\n', ' ').replace('\\t', ' ')
```

**Rust implementation**:
```rust
fn remove_verbatim_escape_sequences(s: &str) -> String {
    s.replace("\\r", " ")
     .replace("\\n", " ")
     .replace("\\t", " ")
}
```

**Important**: These are **literal backslash + character** pairs, NOT actual escape sequences:
- `"\\n"` in Rust source = the two-character sequence `\n` (backslash followed by 'n')
- This matches what appears in C string literals like `"modify\n"` in source code

### 4. Investigation Test Results

Running `cargo test duplicate_merge_investigation --lib -- --nocapture`:

**bzip2.106.c** (confirmed root cause):
- Token 8579 = "n" (from literal `\n` in C string)
- Token 7054 = "it" (expected token from rule)
- Query has `["modify", "n", "it", ...]` where rule expects `["modify", "it", ...]`
- Match fails at offset 12: `query=8579, rule=7054, pos=96`

**aladdin-md5_and_not_rsa-md5.txt** (DIFFERENT ROOT CAUSE - needs investigation):
- File has `.txt` extension but contains C code with CRLF line endings
- Python would NOT apply preprocessing (not a source extension)
- File contains zlib-like license at lines 4-18
- Rust matches lines 4-18 (one match), but Python expects 2 zlib matches
- Second match location unclear - second comment block (lines 25-52) is a changelog, not license text
- May need to investigate Python's actual output to understand expected behavior
- Possible CRLF handling difference

**apache-2.0_and_apache-2.0.txt** (DIFFERENT ROOT CAUSE - needs investigation):
- Contains XML/maven file with CRLF line endings
- Apache license in XML comment at lines 2-16
- License metadata at lines 63-69
- Rust matches lines 63-69 (one match), Python expects 2 apache-2.0 matches
- Python likely matches BOTH the comment section (lines 2-16) AND the metadata section
- Rust may not be matching the XML comment properly
- May be XML/HTML comment handling issue, not escape sequence issue

### 5. Implementation Approach

**Step 1**: Add preprocessing function to `src/license_detection/query.rs`:
```rust
/// Replace literal escape sequences (\n, \r, \t) with spaces.
/// These appear in source code string literals and should be treated as
/// whitespace for license matching purposes.
fn remove_verbatim_escape_sequences(s: &str) -> String {
    s.replace("\\r", " ")
     .replace("\\n", " ")
     .replace("\\t", " ")
}
```

**Step 2**: Add source file detection (choose location based on Option A or B above):
```rust
/// Source code extensions that may contain escape sequences in string literals.
/// From Python: reference/scancode-toolkit/src/textcode/analysis.py:351-423
const SOURCE_EXTENSIONS: &[&str] = &[
    ".c", ".c++", ".cc", ".cpp", ".cxx", ".h", ".hh", ".hpp", ".hxx",
    ".java", ".js", ".jsx", ".ts", ".jsp", ".py", ".rs", ".go", ".rb",
    ".php", ".pl", ".sh", ".cs", ".swift", ".kt", ".scala", // ... full list
];

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_with_dot = format!(".{}", ext.to_lowercase());
            SOURCE_EXTENSIONS.contains(&ext_with_dot.as_str())
        })
        .unwrap_or(false)
}
```

**Step 3**: Apply preprocessing in detection pipeline:

**If Option A (recommended)**, modify `src/scanner/process.rs`:
```rust
fn extract_license_information(
    file_info_builder: &mut FileInfoBuilder,
    text_content: String,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
    path: &Path,  // Add path parameter
) -> Result<(), Error> {
    let Some(engine) = license_engine else {
        return Ok(());
    };
    
    let processed_text = if is_source_file(path) {
        text_content.lines()
            .map(remove_verbatim_escape_sequences)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        text_content
    };
    
    match engine.detect(&processed_text) {
        // ...
    }
}
```

**If Option B**, modify `src/license_detection/query.rs`:
```rust
pub fn with_options(
    text: &str,
    index: &LicenseIndex,
    _line_threshold: usize,
    is_source: bool,  // Add parameter
) -> Result<Self, anyhow::Error> {
    let processed_text = if is_source {
        text.lines()
            .map(|line| remove_verbatim_escape_sequences(line))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        text.to_string()
    };
    
    // Continue with processed_text...
}
```

---

## Fix Required

### Implementation Summary

1. Add `remove_verbatim_escape_sequences()` function to `src/license_detection/query.rs`
2. Add `SOURCE_EXTENSIONS` constant and `is_source_file()` function
3. Apply preprocessing in the scanner (`process.rs`) before calling `engine.detect()`
4. Process line-by-line to preserve line number tracking for match positions

### Affected Files

| File | Change |
|------|--------|
| `src/license_detection/query.rs` | Add preprocessing function |
| `src/scanner/process.rs` | Add source file detection, apply preprocessing before detect() |

### Reference Implementation

Python: `reference/scancode-toolkit/src/textcode/analysis.py`
- Lines 298-303: `remove_verbatim_cr_lf_tab_chars()`
- Lines 321-333: `unicode_text_lines(decrlf=...)`
- Lines 351-423: `is_source()` extension list

---

## Success Criteria

1. lic2: 802+ passed (restore baseline)
2. lic1: 241+ passed (keep CDDL improvement)
3. external: 2176+ passed (keep improvement)
4. `bzip2.106.c` produces 2 bzip2-libbzip-2010 detections
5. `apache-2.0_and_apache-2.0.txt` produces 2 apache-2.0 detections
6. `aladdin-md5_and_not_rsa-md5.txt` produces 2 zlib detections

---

## Related Plans

- PLAN-056: CDDL Rule Selection Investigation (original issue)
- PLAN-057: CDDL Fix Regression Cleanup (surround merge fix)
