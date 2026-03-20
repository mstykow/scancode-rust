# SPDX-LID Matcher Deduplication Fix

**Date**: 2026-02-12
**Component**: `src/license_detection/spdx_lid/mod.rs`
**Type**: Bug Fix

**Status**: Implemented

## Problem

The SPDX-LID matcher was creating thousands of duplicate matches for a single SPDX identifier.

When detecting `SPDX-License-Identifier: MIT`, the original implementation returned one match for **every rule** in the index with `license_expression: "mit"` — resulting in thousands of identical matches, each with a different `rule_identifier`.

## Historical Root Cause

The original implementation effectively fanned out one SPDX identifier to every
rule carrying the same normalized license expression, so a single SPDX line
could explode into many duplicate matches.

This was incorrect because the ScanCode Python implementation returns one SPDX
match per identifier occurrence, not one per matching rule.

## Current Implementation

The current implementation avoids duplicate fan-out in two places:

1. `src/license_detection/index/builder/mod.rs` builds `rid_by_spdx_key`, a
   single SPDX-key-to-rule mapping used for SPDX lookup.
2. `src/license_detection/spdx_lid/mod.rs` resolves SPDX expressions through
   that map and creates one `LicenseMatch` per SPDX identifier occurrence.

This keeps SPDX matching deterministic and avoids creating duplicate matches for
every rule sharing the same ScanCode expression.

## Impact

**Before**: `SPDX-License-Identifier: MIT` → ~1000+ matches
**After**: `SPDX-License-Identifier: MIT` → 1 match with `matcher: "1-spdx-id"`

The other matches in the output (Aho-Corasick, hash) are expected behavior from
the multi-strategy matching pipeline.

## Python Reference

The Python implementation in `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py` creates one `LicenseMatch` per SPDX identifier occurrence, selecting the appropriate rule based on the license expression.
