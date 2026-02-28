# PLAN-0015: filter_dupes Regression Analysis

## Status: Investigation Complete

## Executive Summary

The `filter_dupes()` fix is **correct** - it aligns with Python's implementation. The regressions it introduces reveal **pre-existing issues** in other parts of the pipeline that were previously masked.

## Background

### The Fix
Added `filter_dupes()` to `src/license_detection/seq_match.rs` to deduplicate candidates by grouping them and keeping only the best from each group. This matches Python's behavior.

### Impact
- **Fixed**: 14 tests (including npruntime.h)
- **New failures**: 15 tests
- **Net**: -1 regression

## Root Cause Analysis

### Primary Issue: matched_length Precision Loss

**Location**: `src/license_detection/seq_match.rs:254`

**Problem**: The `DupeGroupKey.matched_length` uses integer rounding, losing precision compared to Python's 1-decimal-place rounding.

| License | matched_length | Python rounded | Rust rounded | Same Group? |
|---------|---------------|----------------|--------------|-------------|
| x11-dec1 | 138 | 6.9 | 7 | - |
| cmu-uc | 133 | 6.7 | 7 | **YES (wrong)** |

In Python, these are DIFFERENT groups (6.9 ≠ 6.7), so both candidates survive.
In Rust, they're the SAME group (7 = 7), so only one survives.

**Affected tests**:
- `MIT-CMU-style.txt` - Expected: x11-dec1, Actual: cmu-uc

**Fix**: Store the 1-decimal-place rounded value:
```rust
// Current (wrong):
matched_length: (candidate.score_vec_rounded.matched_length * 20.0).round() as i32,

// Should be:
matched_length: (candidate.score_vec_rounded.matched_length * 10.0).round() as i32,  // 69, 67
```

### Secondary Issues (Uncovered, Not Caused by filter_dupes)

#### 1. Source Map File Processing Issue (ar-ER.js.map)

**Location**: File preprocessing before license detection

**Problem**: Source map files (`.js.map`, `.css.map`) are JSON files containing `sourcesContent` arrays with the actual source code. The license text in these files uses escaped newlines (`\n` as literal backslash-n), which are NOT being unescaped before tokenization.

**Root Cause Analysis**:

1. **What Python does** (correct):
   - `textcode/analysis.py:js_map_sources_lines()` parses JSON and extracts `sourcesContent`
   - The JSON parser automatically unescapes `\n` to actual newlines
   - Tokenization sees: `"can be\n * found"` → `["can", "be", "found"]`
   - This matches `mit_129.RULE` perfectly

2. **What Rust does** (incorrect):
   - The scanner reads the raw JSON file content directly
   - The tokenizer sees literal `\n` (backslash + 'n')
   - Tokenization sees: `"can be\\n * found"` → `["can", "be", "n", "found"]`
   - The extra "n" token breaks the match for `mit_129.RULE`
   - Instead, shorter rules (`mit_131.RULE`, `mit_132.RULE`) match separately

**Evidence**:
```
MIT_129 tokens (25): ["use", "of", ..., "can", "be", "found", ...]
Query tokens (from raw JSON): [..., "can", "be", "n", "found", ...]
                                             ^-- extra "n" from \n
```

**Affected tests**:
- `ar-ER.js.map` - Expected 1 "mit" (mit_129.RULE), Actual 2 "mit" (mit_131.RULE + mit_132.RULE)

**Fix Required**: Implement source map file preprocessing similar to Python's `js_map_sources_lines()`:
1. Detect `.js.map` and `.css.map` files
2. Parse JSON and extract `sourcesContent` array
3. Concatenate sourcesContent entries (already unescaped by JSON parser)
4. Feed the extracted content to license detection

#### 2. Missing License Reference Detection

**Location**: `src/license_detection/seq_match.rs` or `aho_match.rs`

**Problem**: Text like `"Re-licensed mDNSResponder daemon source code under Apache License, Version 2.0"` (changelog entries) isn't being detected.

**Affected tests**:
- `DNSDigest.c` - Expected 3 apache-2.0, Actual 2

#### 3. Dual-License Header Detection Issue

**Location**: `src/license_detection/seq_match.rs` or detection pipeline

**Problem**: Dual-license headers like MPL/GPL aren't being fully matched. Only short tags like `MODULE_LICENSE("Dual MPL/GPL")` are detected.

**Affected tests**:
- `sa11xx_base.c` - Expected 2 "mpl-1.1 OR gpl-2.0", Actual 1

#### 4. License Expression Combination Issue

**Location**: `src/license_detection/detection.rs`

**Problem**: When multiple overlapping matches are detected, the expression combination logic creates incorrect expressions like `lgpl-2.0-plus WITH wxwindows-exception-3.1 AND wxwindows-exception-3.1`.

**Affected tests**:
- `lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt` - Expected 1 expression, Actual 5

### Non-Issues (Already Working)

#### git.mk
- Expected: fsfap-no-warranty-disclaimer
- Status: **PASSING** - correctly detected after filter_dupes

#### lgpl-2.1_14.txt
- Expected: lgpl-2.1
- Status: **PASSING** - correctly detected after filter_dupes

## Test Case Analysis

| Test | Expected | Actual | Root Cause | Priority |
|------|----------|--------|------------|----------|
| MIT-CMU-style.txt | x11-dec1 | cmu-uc | matched_length precision loss | High |
| ar-ER.js.map | 1 mit | 2 mit | Source map preprocessing missing | High |
| DNSDigest.c | 3 apache-2.0 | 2 apache-2.0 | License reference detection | Medium |
| sa11xx_base.c | 2 mpl/gpl | 1 mpl/gpl | Dual-license detection | Medium |
| lgpl-2.0-plus_wxwindows | 1 expr | 5 exprs | Expression combination | Medium |
| MIT.t21 | proprietary | mit | Needs investigation | Low |
| bsd.f | bsd-simplified | bsd-new | Needs investigation | Low |

## ar-ER.js.map Detailed Analysis

### The File Format

`ar-ER.js.map` is a JavaScript source map (JSON format) with this structure:
```json
{
  "version": 3,
  "file": "ar-ER.js",
  "sources": ["../../../../../packages/common/locales/extra/ar-ER.ts"],
  "sourcesContent": ["/**\n * @license\n * Copyright Google Inc. All Rights Reserved.\n *\n * Use of this source code is governed by an MIT-style license that can be\n * found in the LICENSE file at https://angular.io/license\n */\n..."],
  "mappings": "..."
}
```

### The License Text

The actual license notice is embedded in `sourcesContent[0]`:
```
/**
 * @license
 * Copyright Google Inc. All Rights Reserved.
 *
 * Use of this source code is governed by an MIT-style license that can be
 * found in the LICENSE file at https://angular.io/license
 */
```

### The Problem

In the JSON file, the newlines are represented as `\n` (backslash-n). When the JSON is parsed, these become actual newlines. But when scanning the raw file content without JSON parsing, we see literal `\n` characters.

**Python's Approach** (`textcode/analysis.py:js_map_sources_lines`):
```python
with io.open(location, encoding='utf-8') as jsm:
    content = json.load(jsm)  # JSON parser unescapes \n → actual newline
    sources = content.get('sourcesContent', [])
    for entry in sources:
        for line in entry.splitlines():  # Splits on actual newlines
            yield l
```

**Current Rust Approach**:
- Reads raw file content
- Tokenizes directly without JSON parsing
- Sees `\n` as two tokens: `\` (filtered) and `n` (kept as token)

### Token Sequence Comparison

| Position | MIT_129 Token | Raw JSON Token | Match? |
|----------|---------------|----------------|--------|
| 0-11 | use...license | use...license | ✓ |
| 12 | that | that | ✓ |
| 13 | can | can | ✓ |
| 14 | be | be | ✓ |
| 15 | **found** | **n** | ✗ BREAK |
| 16 | in | found | ✗ |
| ... | ... | ... | ✗ |

The extra `n` token at position 15 breaks the match, causing MIT_129 to never match. Instead, shorter rules MIT_131 (tokens 0-11) and MIT_132 (URL at end) match separately.

### Why Two Matches Instead of One

Since MIT_129 doesn't match, the Aho-Corasick matcher finds:
1. `mit_131.RULE`: "Use of this source code is governed by an MIT-style license" (tokens 29-41)
2. `mit_132.RULE`: "https://angular.io/license" (tokens 51-55)

These are at non-overlapping positions, so neither `filter_contained_matches` nor `filter_overlapping_matches` filters them out.

### The Fix

Add source map file preprocessing in the scanner:

```rust
// In src/scanner/mod.rs or a new src/utils/sourcemap.rs

/// Extract source content from a source map file.
/// Returns the concatenated sourcesContent if the file is a valid source map.
pub fn extract_sourcemap_content(content: &str) -> Option<String> {
    // Try to parse as JSON
    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    
    // Check for source map structure
    if json.get("version").is_none() || json.get("sourcesContent").is_none() {
        return None;
    }
    
    // Extract and concatenate sourcesContent
    let sources = json.get("sourcesContent")?.as_array()?;
    let combined: String = sources
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    
    Some(combined)
}
```

Then in the license detection pipeline:
1. Check if file extension is `.js.map` or `.css.map`
2. If so, extract sourcesContent and use that for license detection
3. Otherwise, use the raw file content

## Recommended Fix Order

### Phase 1: Fix the precision issue (High Priority)

**File**: `src/license_detection/seq_match.rs:254`

Change the `DupeGroupKey.matched_length` calculation to use 1-decimal precision:
```rust
matched_length: (candidate.score_vec_rounded.matched_length * 10.0).round() as i32,
```

This should fix `MIT-CMU-style.txt` and may fix other cases.

### Phase 2: Fix overlapping match filtering (Medium Priority)

**File**: `src/license_detection/match_refine.rs`

Investigate why matches at identical line boundaries both survive filtering.

### Phase 3: Investigate remaining issues (Lower Priority)

- License reference detection for changelog entries
- Dual-license header detection
- Expression combination for WITH expressions

## Code Locations

| Component | File | Lines |
|-----------|------|-------|
| filter_dupes | `src/license_detection/seq_match.rs` | 130-180 |
| DupeGroupKey | `src/license_detection/seq_match.rs` | 27-35 |
| matched_length calculation | `src/license_detection/seq_match.rs` | 254 |
| Overlapping match filter | `src/license_detection/match_refine.rs` | filter_contained_matches, filter_overlapping_matches |
| Expression combination | `src/license_detection/detection.rs` | determine_license_expression |

## References

- Python filter_dupes: `reference/scancode-toolkit/src/licensedcode/match_set.py:467-485`
- Python ScoresVector: `reference/scancode-toolkit/src/licensedcode/match_set.py:440`
