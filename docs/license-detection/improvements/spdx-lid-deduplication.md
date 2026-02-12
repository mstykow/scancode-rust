# SPDX-LID Matcher Deduplication Fix

**Date**: 2026-02-12
**Component**: `src/license_detection/spdx_lid.rs`
**Type**: Bug Fix

## Problem

The SPDX-LID matcher was creating thousands of duplicate matches for a single SPDX identifier.

When detecting `SPDX-License-Identifier: MIT`, the original implementation returned one match for **every rule** in the index with `license_expression: "mit"` — resulting in thousands of identical matches, each with a different `rule_identifier`.

## Root Cause

The `find_matching_rules()` function returned `Vec<usize>` containing ALL rules matching the SPDX key:

```rust
fn find_matching_rules(index: &LicenseIndex, spdx_key: &str) -> Vec<usize> {
    let normalized_spdx = normalize_spdx_key(spdx_key);
    index.rules_by_rid
        .iter()
        .enumerate()
        .filter(|(_, rule)| normalize_spdx_key(&rule.license_expression) == normalized_spdx)
        .map(|(rid, _)| rid)
        .collect()
}
```

This was incorrect because the ScanCode Python implementation only returns **one match per SPDX identifier**, selecting the rule with the highest relevance.

## Solution

Changed `find_matching_rules()` to `find_best_matching_rule()`:

```rust
fn find_best_matching_rule(index: &LicenseIndex, spdx_key: &str) -> Option<usize> {
    let normalized_spdx = normalize_spdx_key(spdx_key);

    let mut best_rid: Option<usize> = None;
    let mut best_relevance: u8 = 0;

    for (rid, rule) in index.rules_by_rid.iter().enumerate() {
        let license_expr = normalize_spdx_key(&rule.license_expression);

        if license_expr == normalized_spdx && rule.relevance > best_relevance {
            best_relevance = rule.relevance;
            best_rid = Some(rid);
        }
    }

    // Fallback to first match if no high-relevance rule found
    best_rid.or_else(|| {
        for (rid, rule) in index.rules_by_rid.iter().enumerate() {
            let license_expr = normalize_spdx_key(&rule.license_expression);
            if license_expr == normalized_spdx {
                return Some(rid);
            }
        }
        None
    })
}
```

This returns a single rule ID (the best match by relevance) instead of all matching rules.

## Impact

**Before**: `SPDX-License-Identifier: MIT` → ~1000+ matches
**After**: `SPDX-License-Identifier: MIT` → 1 match with `matcher: "1-spdx-id"`

The other matches in the output (Aho-Corasick, hash) are expected behavior from the multi-strategy matching pipeline.

## Python Reference

The Python implementation in `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py` creates one `LicenseMatch` per SPDX identifier occurrence, selecting the appropriate rule based on the license expression.
