# PLAN-055: Expression Normalization

## Status: NOT IMPLEMENTED

## Summary

Python has expression normalization/simplification logic that Rust lacks. Complex expressions may be simplified differently.

---

## Problem Statement

**Example**: Python normalizes `lgpl-2.1 WITH exception OR cpl-1.0 WITH exception` to `lzma-sdk-2006` (based on SPDX mapping).

**Rust**: Does not have this normalization layer.

---

## Impact

- ~30+ tests with complex expressions
- Expression outputs may differ from Python

---

## Implementation

**Location**: `src/license_detection/expression.rs`, `src/license_detection/spdx_mapping.rs`

Requires investigation of Python's expression simplification logic in the `license-expression` library.

---

## Priority: MEDIUM

Complex feature requiring significant investigation.

---

## Reference

- PLAN-029 section 2.6
