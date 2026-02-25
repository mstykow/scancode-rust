# PLAN-050: Add NuGet SPDX Pattern Detection

## Status: IMPLEMENTED ✓

**Commit**: Added NuGet SPDX pattern `["licenses", "nuget", "org"]` to all three SPDX prefix checks in `query.rs`.

## Summary

Rust only checks for `["spdx", "license", "identifier"]` pattern when detecting SPDX license identifiers from URLs. It misses NuGet's `["licenses", "nuget", "org"]` pattern, causing NuGet SPDX URLs like `https://licenses.nuget.org/MIT` to not be detected.

---

## Problem Statement

**Python** (query.py:255-264, 492-497):

```python
# Build token ID lists for SPDX patterns
spdxid = [dic_get(u'spdx'), dic_get(u'license'), dic_get(u'identifier')]
nuget_spdx_id = [dic_get(u'licenses'), dic_get(u'nuget'), dic_get(u'org')]
self.spdx_lid_token_ids = [x for x in [spdxid, nuget_spdx_id] if x != [None, None, None]]

# Later, when checking lines:
if line_tokens[:3] in spdx_lid_token_ids:
    spdx_start_offset = 0
elif line_tokens[1:4] in spdx_lid_token_ids:
    spdx_start_offset = 1
elif line_tokens[2:5] in spdx_lid_token_ids:
    spdx_start_offset = 2
```

**Rust** (query.rs:371-398):

```rust
let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
    || first_three == ["spdx", "licence", "identifier"];
// Missing: NuGet pattern check!
```

---

## Impact

NuGet SPDX URLs like `https://licenses.nuget.org/MIT` won't be detected as SPDX license identifiers during query tokenization. This affects:

1. License detection from NuGet package metadata (nuspec files with `<licenseUrl>https://licenses.nuget.org/MIT</licenseUrl>`)
2. Golden tests for license expressions containing NuGet URLs
3. Feature parity with Python ScanCode Toolkit

---

## Root Cause Analysis

The Rust implementation uses **string comparison** instead of token IDs:

| Approach | Python | Rust |
|----------|--------|------|
| Method | Token ID lookup from dictionary | Direct string comparison |
| Patterns checked | `[spdxid, nuget_spdx_id]` | `["spdx", "license", "identifier"]` only |
| NuGet pattern | ✅ Included via token IDs | ❌ Missing |

The Rust approach is valid and simpler (avoids dictionary lookups), but the NuGet pattern was never added.

---

## Implementation

### Location: `src/license_detection/query.rs:371-398`

The fix requires adding the NuGet pattern `["licenses", "nuget", "org"]` to all three SPDX prefix checks:

1. **First three tokens** (line 374-375)
2. **Second three tokens** (line 385-386)
3. **Third three tokens** (line 396-397)

### Code Change

```rust
// Before (line 374-375):
let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
    || first_three == ["spdx", "licence", "identifier"];

// After:
let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
    || first_three == ["spdx", "licence", "identifier"]
    || first_three == ["licenses", "nuget", "org"];  // NuGet pattern
```

Apply the same pattern to `is_spdx_second` (lines 385-386) and `is_spdx_third` (lines 396-397).

### Complete Diff

```diff
--- a/src/license_detection/query.rs
+++ b/src/license_detection/query.rs
@@ -371,17 +371,20 @@ impl<'a> Query<'a> {
             let spdx_start_offset = if tokens_lower.len() >= 3 {
                 let first_three: Vec<&str> =
                     tokens_lower.iter().take(3).map(|s| s.as_str()).collect();
                 let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
-                    || first_three == ["spdx", "licence", "identifier"];
+                    || first_three == ["spdx", "licence", "identifier"]
+                    || first_three == ["licenses", "nuget", "org"];
                 if is_spdx_prefix {
                     Some(0)
                 } else if tokens_lower.len() >= 4 {
                     let second_three: Vec<&str> = tokens_lower
                         .iter()
                         .skip(1)
                         .take(3)
                         .map(|s| s.as_str())
                         .collect();
                     let is_spdx_second = second_three == ["spdx", "license", "identifier"]
-                        || second_three == ["spdx", "licence", "identifier"];
+                        || second_three == ["spdx", "licence", "identifier"]
+                        || second_three == ["licenses", "nuget", "org"];
                     if is_spdx_second {
                         Some(1)
                     } else if tokens_lower.len() >= 5 {
@@ -393,7 +396,8 @@ impl<'a> Query<'a> {
                         .take(3)
                         .map(|s| s.as_str())
                         .collect();
                         let is_spdx_third = third_three == ["spdx", "license", "identifier"]
-                            || third_three == ["spdx", "licence", "identifier"];
+                            || third_three == ["spdx", "licence", "identifier"]
+                            || third_three == ["licenses", "nuget", "org"];
                         if is_spdx_third { Some(2) } else { None }
                     } else {
                         None
```

---

## Testing Strategy

Following `docs/TESTING_STRATEGY.md`, this fix requires:

### 1. Unit Tests (Layer 1)

Add new unit tests in `src/license_detection/query.rs`:

```rust
#[test]
fn test_query_tracks_spdx_lines_with_nuget_url() {
    let mut index = create_query_test_index();
    let _ = index.dictionary.get_or_assign("licenses");
    let _ = index.dictionary.get_or_assign("nuget");
    let _ = index.dictionary.get_or_assign("org");
    let _ = index.dictionary.get_or_assign("mit");

    let text = "https://licenses.nuget.org/MIT";
    let query = Query::new(text, &index).unwrap();

    assert_eq!(query.spdx_lines.len(), 1, "Should track 1 NuGet SPDX line");

    let (spdx_text, start, end) = &query.spdx_lines[0];
    assert!(*start <= *end, "Token positions should be valid");
    assert!(
        spdx_text.to_lowercase().contains("licenses.nuget.org"),
        "SPDX text should contain NuGet URL"
    );
}

#[test]
fn test_query_tracks_spdx_lines_with_nuget_url_and_prefix() {
    let mut index = create_query_test_index();
    let _ = index.dictionary.get_or_assign("licenses");
    let _ = index.dictionary.get_or_assign("nuget");
    let _ = index.dictionary.get_or_assign("org");
    let _ = index.dictionary.get_or_assign("mit");

    // Test with comment prefix (offset 1)
    let text = "// https://licenses.nuget.org/MIT";
    let query = Query::new(text, &index).unwrap();

    assert_eq!(query.spdx_lines.len(), 1, "Should track 1 NuGet SPDX line");
}

#[test]
fn test_query_tracks_spdx_lines_with_nuget_complex_expression() {
    let mut index = create_query_test_index();
    let _ = index.dictionary.get_or_assign("licenses");
    let _ = index.dictionary.get_or_assign("nuget");
    let _ = index.dictionary.get_or_assign("org");

    // Test with complex license expression
    let text = "https://licenses.nuget.org/(LGPL-2.0-only WITH FLTK-exception OR Apache-2.0)";
    let query = Query::new(text, &index).unwrap();

    assert_eq!(query.spdx_lines.len(), 1, "Should track 1 NuGet SPDX line");
}

#[test]
fn test_query_tracks_spdx_lines_mixed_patterns() {
    let mut index = create_query_test_index();
    let _ = index.dictionary.get_or_assign("spdx");
    let _ = index.dictionary.get_or_assign("license");
    let _ = index.dictionary.get_or_assign("identifier");
    let _ = index.dictionary.get_or_assign("licenses");
    let _ = index.dictionary.get_or_assign("nuget");
    let _ = index.dictionary.get_or_assign("org");
    let _ = index.dictionary.get_or_assign("mit");

    let text = "SPDX-License-Identifier: MIT\nhttps://licenses.nuget.org/Apache-2.0";
    let query = Query::new(text, &index).unwrap();

    assert_eq!(query.spdx_lines.len(), 2, "Should track both SPDX patterns");
}
```

### 2. Existing Tests to Verify

The following existing tests should continue to pass:

- `test_split_spdx_lid_nuget` in `src/license_detection/spdx_lid.rs:379`
- `test_extract_spdx_expressions_nuget_url` in `src/license_detection/spdx_lid.rs:520`

### 3. Golden Tests (Layer 2)

No direct golden test updates needed - the license detection pipeline should automatically pass NuGet URLs through once the query layer is fixed. Note: The existing NuGet golden test data in `testdata/nuget-golden/` doesn't contain `licenses.nuget.org` URLs, so these tests won't directly validate this fix.

Run the NuGet golden tests to verify no regressions:

```bash
cargo test nuget_golden
```

### 4. Integration Verification

Test with a real NuGet URL:

```bash
echo "https://licenses.nuget.org/MIT" > /tmp/test_nuget.txt
cargo run -- /tmp/test_nuget.txt -o /tmp/output.json
# Verify output.json contains mit license expression
```

---

## Test Cases Summary

| Test | Purpose | Location |
|------|---------|----------|
| `test_query_tracks_spdx_lines_with_nuget_url` | Basic NuGet URL detection | `query.rs` |
| `test_query_tracks_spdx_lines_with_nuget_url_and_prefix` | NuGet URL with comment prefix | `query.rs` |
| `test_query_tracks_spdx_lines_with_nuget_complex_expression` | Complex license expression | `query.rs` |
| `test_query_tracks_spdx_lines_mixed_patterns` | Both SPDX and NuGet patterns | `query.rs` |
| Existing `test_split_spdx_lid_nuget` | Verify split_spdx_lid still works | `spdx_lid.rs` |
| Existing `test_extract_spdx_expressions_nuget_url` | Verify extraction still works | `spdx_lid.rs` |

---

## Priority: MEDIUM

Affects NuGet package license detection but is a relatively narrow use case. The fix is straightforward and low-risk.

---

## Verification Checklist

- [ ] Add NuGet pattern to all three SPDX prefix checks in `query.rs`
- [ ] Add 4 new unit tests for NuGet pattern detection
- [ ] Run `cargo test` - all tests pass
- [ ] Run `cargo clippy` - no warnings
- [ ] Run `cargo fmt` - code formatted
- [ ] Test with NuGet URL input produces correct license detection

---

## Related

- **PLAN-048**: P3 - Original finding
- **Python reference**: `reference/scancode-toolkit/src/licensedcode/query.py:255-264, 492-497`
- **Python reference**: `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py:398-421`
- **Rust implementation**: `src/license_detection/spdx_lid.rs:40-42` (NuGet regex already exists)
- **Testing strategy**: `docs/TESTING_STRATEGY.md`

---

## Notes

The `spdx_lid.rs` module already has the NuGet pattern implemented correctly in `NUGET_SPDX_PATTERN` regex (line 40-42). The issue is only in `query.rs` where the SPDX line detection during tokenization misses the NuGet pattern. The two modules work together:

1. `query.rs` detects which lines contain SPDX identifiers → populates `spdx_lines`
2. `spdx_lid.rs` extracts and processes the license expressions from those lines

Both need to recognize NuGet URLs, but currently only `spdx_lid.rs` does.
