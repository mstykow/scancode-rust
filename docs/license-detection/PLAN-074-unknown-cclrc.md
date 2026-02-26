# PLAN-074: Unknown Detection - cclrc.txt

## Status: INVESTIGATING

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/cclrc.txt`

| Expected | Actual |
|----------|--------|
| `["cclrc"]` (1) | `[]` (0) |

**Issue**: cclrc license not detected at all.

## Investigation Steps

1. Check if cclrc rule exists in license index
2. Compare Python vs Rust matching
3. Determine why no detection occurs
