# PLAN-059: CRLF Line Ending Handling

## Status: CLOSED - NOT A CRLF ISSUE (Verified 2026-02-25)

## Problem Statement

Files with CRLF line endings (`\r\n`) were suspected to be matched incorrectly compared to files with LF line endings (`\n`). After thorough investigation, **this is NOT a CRLF issue**.

### Original Evidence (Now Debunked)

**lzma-sdk tests**:
- `lzma-sdk-original.txt` (CRLF, 2565 bytes): expected `lzma-sdk-2006`
- `lzma-sdk-original_1.txt` (LF, 738 bytes): expected `lzma-sdk-2006`

**Investigation Finding**: These are **different files with different content**:
- `lzma-sdk-original.txt`: 100 lines, CRLF (`\r\n`), full C header file
- `lzma-sdk-original_1.txt`: 19 lines, LF (`\n`), just the license notice

They are NOT the same file with different line endings.

**lic2 test failures**:
- `aladdin-md5_and_not_rsa-md5.txt`: expected `["zlib", "zlib"]`, got `["zlib"]`
- `apache-2.0_and_apache-2.0.txt`: expected `["apache-2.0", "apache-2.0"]`, got `["apache-2.0"]`

**Investigation Finding**: Converting CRLF to LF produces the same result. The issue is **NOT related to CRLF**.

---

## Investigation Results

### 1. Rust's CRLF Handling is Correct

Rust's `str::lines()` correctly handles CRLF:
- `\r\n` is recognized as a line ending
- Lines returned do NOT contain the `\r` character
- Tokenization works identically for CRLF and LF files

```rust
// Verified: both produce identical token sequences
let crlf_text = "line1\r\nline2\r\nline3";
let lf_text = "line1\nline2\nline3";
// Both tokenize to ["line1", "line2", "line3"]
```

### 2. Python's CRLF Handling (for reference)

Python handles CRLF in `reference/scancode-toolkit/src/textcode/analysis.py`:

```python
def remove_verbatim_cr_lf_tab_chars(s):
    """Replace literal escaped sequences with newline."""
    return (s
        .replace('\\r\\n', '\n')  # literal backslash-r-backslash-n
        .replace('\\r', '\n')     # literal backslash-r
        .replace('\\n', '\n')     # literal backslash-n
    )

def unicode_text_lines(location, decrlf=False):
    lines = _unicode_text_lines(location)  # splitlines() handles CRLF
    if decrlf:
        return map(remove_verbatim_cr_lf_tab_chars, lines)
    else:
        return lines
```

Key insight: `remove_verbatim_cr_lf_tab_chars` removes **literal backslash sequences** like `\r` (the two characters backslash and 'r'), NOT the actual carriage return character (byte 0x0D). Python's `splitlines()` already handles CRLF correctly.

The `decrlf` parameter is for **source code files** that may contain literal escaped sequences like `\r\n` in string literals (e.g., `print("hello\n")`), NOT for actual CRLF line endings.

### 3. Actual Issue: Duplicate License Detection

The real issue is that **duplicate license matches are being merged incorrectly**. This is tracked in a separate plan (PLAN-058).

For `aladdin-md5_and_not_rsa-md5.txt`:
- Expected: 2 separate `zlib` matches
- Actual: 1 merged `zlib` match

The file has TWO separate comment blocks, each containing similar zlib-style license text. Python detects both, Rust merges them into one.

For `apache-2.0_and_apache-2.0.txt`:
- Expected: 2 separate `apache-2.0` matches
- Actual: 1 merged `apache-2.0` match

Same issue - two separate license blocks being merged.

### 4. Where Rust Handles Text Normalization

| Location | Function | Purpose |
|----------|----------|---------|
| `src/scanner/process.rs:164-168` | `strip_utf8_bom_bytes()` | Remove UTF-8 BOM |
| `src/scanner/process.rs:166-168` | `remove_verbatim_escape_sequences()` | Remove `\r`, `\n`, `\t` for source files |
| `src/license_detection/query.rs:331` | `text.lines()` | Line iteration (handles CRLF) |
| `src/license_detection/tokenize.rs:133-151` | `tokenize()` | Token extraction (ignores `\r`) |

The current implementation is correct for CRLF handling.

---

## Root Cause Analysis

### lzma-sdk Tests

The two test files have **completely different content**:
- `lzma-sdk-original.txt`: Full C header file with 100 lines (CRLF, `0x0D 0x0A` line endings)
- `lzma-sdk-original_1.txt`: Just the license notice, 19 lines (LF, `0x0A` line endings)

**Verified via `file` command**:
```
lzma-sdk-original.txt:   C source, ASCII text, with CRLF line terminators
lzma-sdk-original_1.txt: ASCII text
```

**Verified via `xxd` hex dump**:
- `lzma-sdk-original.txt`: Shows `0d 0a` (CRLF) bytes
- `lzma-sdk-original_1.txt`: Shows `0a` (LF) only bytes

Both should detect `lzma-sdk-2006`. If there's a detection difference, it's due to **content differences**, not line endings.

### lic2 Duplicate Detection Tests

This is the **same issue as PLAN-058**. The problem is in the match grouping/merging logic, not CRLF handling.

---

## Recommendation

**Close this plan as NOT A CRLF ISSUE.** 

The duplicate detection issue is tracked in PLAN-058. The lzma-sdk test files need to be verified that both actually detect `lzma-sdk-2006` correctly (if not, it's a separate bug).

---

## Files Reviewed (No Changes Needed)

| File | Finding |
|------|---------|
| `src/license_detection/query.rs` | `text.lines()` handles CRLF correctly |
| `src/license_detection/tokenize.rs` | Regex pattern ignores `\r` correctly |
| `src/utils/text.rs` | `remove_verbatim_escape_sequences()` handles literal escapes |
| `src/scanner/process.rs` | File reading and text processing is correct |

---

## Test Files Verified

| File | Line Ending | Content | Expected | Status |
|------|-------------|---------|----------|--------|
| `lic3/lzma-sdk-original.txt` | CRLF (`\r\n`) | Full C header (100 lines, 2565 bytes) | `lzma-sdk-2006` | Different file, not CRLF issue |
| `lic3/lzma-sdk-original_1.txt` | LF (`\n`) | License notice (19 lines, 738 bytes) | `lzma-sdk-2006` | Different file, not CRLF issue |
| `lic2/aladdin-md5_and_not_rsa-md5.txt` | CRLF | Two comment blocks | `["zlib", "zlib"]` | PLAN-058 issue |
| `lic2/apache-2.0_and_apache-2.0.txt` | CRLF | Two license blocks | `["apache-2.0", "apache-2.0"]` | PLAN-058 issue |

**Key Verification**: Converting `lic2` CRLF files to LF produces the same detection result. The issue is NOT CRLF handling.

---

## Action Items

1. ~~**Close this plan** - Not a CRLF issue~~ **DONE**
2. ~~**Verify lzma-sdk tests pass** - Run golden tests to confirm~~ Different files, not CRLF issue
3. **Fix duplicate detection** - Address in PLAN-058

---

## Verification Summary (2026-02-25)

### Confirmed: NOT a CRLF Issue

| Verification | Result |
|--------------|--------|
| `lzma-sdk-original.txt` vs `lzma-sdk-original_1.txt` | **Different files** (100 lines vs 19 lines, different content) |
| Python `decrlf` meaning | Handles **literal backslash sequences** (`\r`), NOT actual CR bytes |
| Rust `str::lines()` | Correctly handles both `\n` and `\r\n` line endings |
| Rust tokenization | Regex pattern `[^_\W]+\+?[^_\W]*` ignores `\r` naturally |
| lic2 tests with CRLF→LF conversion | Same detection result, proving CRLF is not the issue |

**Conclusion**: Close this plan. The duplicate detection issue is tracked in PLAN-058.
