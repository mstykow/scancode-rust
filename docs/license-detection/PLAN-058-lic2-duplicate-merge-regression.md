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

## Fix Required

### Implementation

Add preprocessing for source files in Rust:

1. **Add `remove_verbatim_cr_lf_tab_chars()` function** to `src/license_detection/query.rs` or new module:
   ```rust
   fn remove_verbatim_escape_sequences(s: &str) -> String {
       s.replace("\\r", " ")
        .replace("\\n", " ")
        .replace("\\t", " ")
   }
   ```

2. **Detect source files** and apply preprocessing before tokenization:
   - Similar to Python's `is_source()` function
   - Apply to files with source code extensions (`.c`, `.cpp`, `.h`, `.java`, etc.)

3. **Apply in `Query::with_options()`** before the tokenization loop:
   ```rust
   let processed_text = if is_source_file {
       remove_verbatim_escape_sequences(text)
   } else {
       text.to_string()
   };
   ```

### Affected Files

| File | Change |
|------|--------|
| `src/license_detection/query.rs` | Add preprocessing function and source file detection |
| `src/license_detection/tokenize.rs` | Optionally add helper function |

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
