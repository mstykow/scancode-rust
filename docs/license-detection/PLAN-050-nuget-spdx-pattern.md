# PLAN-050: Add NuGet SPDX Pattern Detection

## Status: NOT IMPLEMENTED

## Summary

Rust only checks for `["spdx", "license", "identifier"]` pattern when detecting SPDX license identifiers from URLs. It misses NuGet's `["licenses", "nuget", "org"]` pattern, causing NuGet SPDX URLs like `https://licenses.nuget.org/MIT` to not be detected.

---

## Problem Statement

**Python** (query.py:255-264):

```python
spdxid = [dic_get(u'spdx'), dic_get(u'license'), dic_get(u'identifier')]
nuget_spdx_id = [dic_get(u'licenses'), dic_get(u'nuget'), dic_get(u'org')]
self.spdx_lid_token_ids = [x for x in [spdxid, nuget_spdx_id] if x != [None, None, None]]
```

**Rust** (query.rs:371-374):

```rust
let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
    || first_three == ["spdx", "licence", "identifier"];
// Missing: NuGet pattern check!
```

---

## Impact

NuGet SPDX URLs like `https://licenses.nuget.org/MIT` won't be detected as license identifiers.

Affected test: `nuget/nuget_test_url_155.txt` (mentioned in PLAN-023)

---

## Implementation

**Location**: `src/license_detection/query.rs:371-374`

Add NuGet pattern to the SPDX detection:

```rust
let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
    || first_three == ["spdx", "licence", "identifier"]
    || first_three == ["licenses", "nuget", "org"];  // NuGet pattern
```

Also check if there are other callers that need updating for the token ID extraction.

---

## Priority: MEDIUM

Affects NuGet package license detection but is a relatively narrow use case.

---

## Verification

1. Run NuGet-related golden tests
2. Verify `nuget_test_url_155.txt` now produces expected output
3. Run full golden test suite to check for regressions

---

## Reference

- PLAN-048: P3 - Original finding
- PLAN-023: Mentions `nuget/nuget_test_url_155.txt` failing
