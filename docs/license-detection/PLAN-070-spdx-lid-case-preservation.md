# PLAN-070: SPDX-LID Case Preservation

## Status: RESOLVED

## Problem Statement

`test_spdx_lid_match_simple` expected `license_expression_spdx` to be "MIT" but got "mit".

## Root Cause

The `spdx_lid_match()` function was using the resolved expression (lowercased from lookup) instead of preserving the original case from input text.

## Fix

Changed to use original `spdx_expression` for `license_expression_spdx`:

```rust
// Before:
license_expression_spdx: resolved_expression.clone(),  // "mit" (lowercased)

// After:
license_expression_spdx: spdx_expression.clone(),  // "MIT" (original case)
```

## Python Reference

Python's behavior:
- `license_expression` uses lowercase ScanCode keys (e.g., "mit")
- `license_expression_spdx` uses proper SPDX casing from License DB (e.g., "MIT")

The License DB (`mit.LICENSE`) contains:
```yaml
key: mit
spdx_license_key: MIT
```

## Files Changed

- `src/license_detection/spdx_lid.rs:273-276`

## Tests Fixed

- `test_spdx_lid_match_simple`
