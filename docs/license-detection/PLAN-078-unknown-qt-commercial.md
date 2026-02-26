# PLAN-078: Unknown Detection - qt.commercial.txt

## Status: INVESTIGATING

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/qt.commercial.txt`

| Expected | Actual |
|----------|--------|
| 20 detections | 32 detections |

**Issue**: Extra "unknown" detections, order mismatch.

## Investigation Steps

1. Compare detection ordering
2. Check if commercial-license matches are correct
3. Identify source of extra unknown matches
