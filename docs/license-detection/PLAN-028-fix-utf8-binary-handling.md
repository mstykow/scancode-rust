# PLAN-028: Fix UTF-8/Binary File Handling in Golden Tests

**Date**: 2026-02-20  
**Status**: Completed  
**Priority**: P5 (Infrastructure, 16 failures)  
**Related**: PLAN-023 Pattern E

## Executive Summary

Golden tests fail with "stream did not contain valid UTF-8" for binary and non-UTF-8 encoded files. The current Rust implementation uses `fs::read_to_string()` which fails on non-UTF-8 content. Python ScanCode handles this gracefully with a layered approach: file type detection → text extraction → encoding fallbacks.

## Problem Statement

### Current Behavior (Rust)

**File**: `src/license_detection/golden_test.rs:109-116`

```rust
fn run(&self, engine: &LicenseDetectionEngine) -> Result<(), String> {
    let text = fs::read_to_string(&self.test_file).map_err(|e| {
        format!(
            "Failed to read test file {}: {}",
            self.test_file.display(),
            e
        )
    })?;
```

This fails for:
- Binary files (`.class`, `.pdf`, `.gif`, `.jar`)
- Text files with non-UTF-8 encoding (ISO-8859, Latin-1)
- Files with mixed/invalid encoding

### Failing Test Files

| File | Type | Expected Behavior |
|------|------|-------------------|
| `flt9.gif` | GIF image | Empty detections (`license_expressions: []`) |
| `do-not_detect-licenses-in-archive.jar` | Java archive | Empty detections |
| `ecl-1.0.txt` | ISO-8859 text | `["ecl-1.0"]` (license detection) |
| `NamespaceNode.class` | Java bytecode | Empty detections |

---

## Python ScanCode Approach

### File Reading Pipeline

Python uses `textcode/analysis.py:numbered_text_lines()`:

```
File Path
    ↓
typecode.get_type(location) → FileType detection
    ↓
if not T.contains_text → return empty iterator
    ↓
if T.is_pdf → pdf.get_text_lines() (pdfminer extraction)
    ↓
if T.is_binary → strings.strings_from_file() (ASCII extraction)
    ↓
if T.is_text → unicode_text_lines() with encoding fallbacks
```

### Key Python Functions

1. **`typecode.get_type(location)`** - Detects file type using magic bytes
   - Returns `FileType` object with `is_binary`, `contains_text`, `is_pdf` flags

2. **`unicode_text_lines()`** (`textcode/analysis.py:251-284`) - Handles text files with encoding fallbacks:
   ```python
   try:
       s = line.decode('UTF-8')
   except UnicodeDecodeError:
       try:
           s = line.decode('LATIN-1')  # Never fails
       except:
           enc = chardet.detect(line)['encoding']
           s = str(line, enc)
   ```

3. **`strings_from_file()`** (`textcode/strings.py:36-50`) - Extracts printable ASCII from binaries:
   - Minimum 4 consecutive printable ASCII characters
   - Also extracts UTF-16-LE "wide" strings (Windows binaries)

---

## Current Rust Code Patterns

### Existing Pattern 1: Scanner File Processing

**File**: `src/scanner/process.rs:160-169`

```rust
use content_inspector::{ContentType, inspect};

// ...in process_file()...
if let Some(package_data) = try_parse_file(path) {
    file_info_builder.package_data(package_data);
    Ok(())
} else if inspect(&buffer) == ContentType::UTF_8 {
    extract_license_information(
        file_info_builder,
        String::from_utf8_lossy(&buffer).into_owned(),
        license_engine,
        include_text,
    )
} else {
    Ok(())  // Skip non-UTF-8 files (binaries)
}
```

### Existing Pattern 2: Debug RTF Test

**File**: `src/license_detection/debug_rtf_test.rs:17-25`

```rust
let rtf_bytes = fs::read("testdata/license-golden/datadriven/lic1/gpl_eula.rtf")
    .expect("Failed to read RTF file");

let rtf_text = String::from_utf8_lossy(&rtf_bytes);
// ... uses rtf_text for detection
```

### Existing Dependency

The `content_inspector` crate is already a dependency:
```
content_inspector v0.2.4
└── memchr v2.8.0
```

---

## Recommended Rust Approach

### Strategy: Leverage Existing Patterns

**Phase 1: Quick Fix (Immediate)**
- Use `fs::read()` + `String::from_utf8_lossy()` for golden tests
- This matches the existing `debug_rtf_test.rs` pattern
- Enables ISO-8859 files to be read (lossy conversion preserves most text)

**Phase 2: Binary Detection (Recommended)**
- Use existing `content_inspector` crate to detect binary files
- Skip license detection for true binaries (archives, images)
- Consistent with scanner's production behavior

### Phase 1 + 2 Implementation

**File**: `src/license_detection/golden_test.rs`

#### Step 1: Add helper function (insert after line 80)

```rust
use content_inspector::{ContentType, inspect};

impl LicenseGoldenTest {
    /// Read file content, handling non-UTF-8 and binary files gracefully.
    /// Returns None for files that should be skipped (true binaries).
    fn read_test_file_content(&self) -> Result<Option<String>, String> {
        let bytes = fs::read(&self.test_file).map_err(|e| {
            format!(
                "Failed to read test file {}: {}",
                self.test_file.display(),
                e
            )
        })?;

        // Check content type using the same crate as scanner/process.rs
        let content_type = inspect(&bytes);
        
        // Skip detection for true binaries (archives, images, etc.)
        // This matches Python's `if not T.contains_text: return empty iterator`
        if matches!(
            content_type,
            ContentType::BINARY |
            ContentType::UTF_16_LE |
            ContentType::UTF_16_BE |
            ContentType::UTF_32_LE |
            ContentType::UTF_32_BE
        ) {
            // Additional extension-based check for compressed archives
            let ext = self.test_file.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            
            if matches!(ext, "jar" | "zip" | "gz" | "tar" | "gif" | "png" | "jpg" | "jpeg" | "class") {
                return Ok(None);  // Signal to skip detection
            }
        }

        // For text files (UTF-8 or other encodings), use lossy conversion
        // This handles ISO-8859, Latin-1, and mixed encodings
        match String::from_utf8(bytes.clone()) {
            Ok(s) => Ok(Some(s)),
            Err(_) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        }
    }
}
```

#### Step 2: Update `run()` method (replace lines 108-146)

```rust
fn run(&self, engine: &LicenseDetectionEngine) -> Result<(), String> {
    let text = match self.read_test_file_content()? {
        Some(t) => t,
        None => {
            // Binary file - expect empty detections
            let expected: Vec<&str> = self.yaml.license_expressions
                .iter()
                .map(|s| s.as_str())
                .collect();
            
            if !expected.is_empty() {
                return Err(format!(
                    "Binary file {} has unexpected non-empty license_expressions: {:?}",
                    self.name, expected
                ));
            }
            return Ok(());
        }
    };

    let detections = engine.detect(&text).map_err(|e| {
        format!("Detection failed for {}: {:?}", self.test_file.display(), e)
    })?;

    // Flatten matches from all detections to get individual match expressions.
    let actual: Vec<&str> = detections
        .iter()
        .flat_map(|d| d.matches.iter())
        .map(|m| m.license_expression.as_str())
        .collect();

    let expected: Vec<&str> = self.yaml.license_expressions
        .iter()
        .map(|s| s.as_str())
        .collect();

    if actual != expected {
        return Err(format!(
            "license_expressions mismatch for {}:  Expected: {:?}  Actual:   {:?}",
            self.name, expected, actual
        ));
    }

    Ok(())
}
```

---

## Implementation Plan

### Step 1: Add Import

**File**: `src/license_detection/golden_test.rs`  
**Line**: After line 32 (after existing imports)

Add:
```rust
use content_inspector::{ContentType, inspect};
```

### Step 2: Add Helper Method

**File**: `src/license_detection/golden_test.rs`  
**Location**: Inside `impl LicenseGoldenTest` block, after `load()` method (around line 106)

Add the `read_test_file_content()` method shown above.

### Step 3: Update `run()` Method

**File**: `src/license_detection/golden_test.rs`  
**Lines**: 108-146

Replace the entire `run()` method with the updated version shown above.

### Step 4: Run Tests

```bash
# Run golden tests to verify fix
cargo test --lib test_golden_lic1 -- --nocapture 2>&1 | head -50

# Check for UTF-8 errors (should be 0 now)
cargo test --lib test_golden_lic1 2>&1 | grep -c "stream did not contain valid UTF-8"
```

---

## File Type Handling Matrix

| File Type | Extension | Python Behavior | Rust Behavior |
|-----------|-----------|-----------------|---------------|
| UTF-8 text | `.txt`, `.c`, `.py` | Direct decode | `from_utf8()` ✓ |
| ISO-8859/Latin-1 | `.txt` (non-UTF) | Decode Latin-1 | `from_utf8_lossy()` ✓ |
| GIF image | `.gif` | Empty (no text) | Skip via content_inspector ✓ |
| Java archive | `.jar` | Empty (compressed) | Skip via extension check ✓ |
| Java bytecode | `.class` | ASCII strings extraction | Skip via extension check ✓ |
| PDF | `.pdf` | pdfminer extraction | Lossy UTF-8 (may have noise) |

---

## Dependencies

### Existing Dependencies (No Changes Needed)

```toml
[dependencies]
content_inspector = "0.2.4"  # Already in use
```

### Future Optional Dependencies (Phase 3)

For PDF text extraction:
```toml
[dependencies]
lopdf = "0.32"  # Or pdf-extract = "0.7"
```

---

## Test Cases

### Test 1: ISO-8859 Encoded File

**File**: `testdata/license-golden/datadriven/lic1/ecl-1.0.txt`  
**Encoding**: ISO-8859 (verified via `file` command)  
**YAML**: `license_expressions: ["ecl-1.0"]`  
**Current**: UTF-8 decode error  
**After Fix**: Should detect `ecl-1.0` license via lossy conversion

### Test 2: GIF Image

**File**: `testdata/license-golden/datadriven/lic1/flt9.gif`  
**YAML**: `notes: no license should be detected here. This is a gif file!`  
**Expected**: `license_expressions: []` (implicit empty)  
**After Fix**: Skip detection, return Ok for empty expected

### Test 3: JAR Archive

**File**: `testdata/license-golden/datadriven/lic1/do-not_detect-licenses-in-archive.jar`  
**YAML**: `notes: this is a compressed Jar and we should not detect anything in this`  
**Expected**: `license_expressions: []` (implicit empty)  
**After Fix**: Skip detection via extension check

---

## Risks and Mitigations

### Risk: Lossy Conversion May Produce Noise

**Impact**: Invalid UTF-8 sequences become replacement characters (``)  
**Mitigation**: For text files with encoding issues, lossy conversion preserves most legible text. Testing with `ecl-1.0.txt` shows the license text remains recognizable.

### Risk: PDF Files May Miss Extractable Text

**Impact**: Some PDFs contain license text that Python extracts via pdfminer  
**Mitigation**: Phase 1 uses lossy conversion (may extract some text). Phase 3 adds proper PDF extraction.

### Risk: Class Files May Have False Positives

**Impact**: Java bytecode may contain ASCII strings that look like license text  
**Mitigation**: Skip `.class` files via extension check. This matches Python's expectation (tests say "nothing should be detected").

---

## Future Enhancements

1. **Binary string extraction** - Port Python's `strings_from_file()` logic for `.class` files
2. **PDF text extraction** - Integrate `lopdf` crate for proper PDF handling
3. **Encoding detection** - Use `encoding_rs` crate for more accurate non-UTF-8 decoding

---

## Implementation Results

**Date**: 2026-02-20

### Changes Made

1. Added `content_inspector` import to `golden_test.rs`
2. Added `read_test_file_content()` helper method to handle binary/non-UTF-8 files
3. Updated `run()` method to use the new helper with proper binary file handling

### Test Results

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| lic1 | 224 passed, 67 failed | 227 passed, 64 failed | +3 passed |
| lic2 | 775 passed, 78 failed | 777 passed, 76 failed | +2 passed |
| lic3 | 250 passed, 42 failed | 250 passed, 42 failed | 0 |
| lic4 | 285 passed, 65 failed | 287 passed, 63 failed | +2 passed |
| external | 2018 passed, 549 failed | 2035 passed, 532 failed | +17 passed |
| unknown | 3 passed, 7 failed | 3 passed, 7 failed | 0 |

**Total improvement: +24 tests passing**

**Key achievement: Eliminated all "stream did not contain valid UTF-8" errors**

### Files Modified

- `src/license_detection/golden_test.rs` - Added binary/non-UTF-8 file handling

---

## References

- Python file reading: `reference/scancode-toolkit/src/textcode/analysis.py:51-176`
- Python encoding fallback: `reference/scancode-toolkit/src/textcode/analysis.py:251-284`
- Python string extraction: `reference/scancode-toolkit/src/textcode/strings.py:36-50`
- Current Rust scanner pattern: `src/scanner/process.rs:160-169`
- Current Rust RTF debug test: `src/license_detection/debug_rtf_test.rs:17-25`
- content_inspector docs: https://docs.rs/content_inspector/
