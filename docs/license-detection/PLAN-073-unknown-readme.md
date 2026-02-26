# PLAN-073: Unknown Detection - README.md

## Status: INVESTIGATING

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/README.md`

| Expected | Actual |
|----------|--------|
| `["unknown-license-reference", "unknown-license-reference", "unknown-license-reference"]` (3) | `["unknown-license-reference", "unknown-license-reference", "unknown", "unknown", "unknown-license-reference"]` (5) |

**Issue**: Extra "unknown" detections and wrong count.

## Investigation Steps

1. Compare Python vs Rust output for this file
2. Identify where extra "unknown" matches are created
3. Determine why detection count differs
