# Go Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Test Status

**Currently Passing:** 0/4 tests (4 tests require license detection engine integration)

- 🔄 All 4 tests ignored - Require license detection engine

## Test Coverage

### Ignored Tests

1. **kingpin-mod** - go.mod from kingpin project
2. **sample-mod** - Sample go.mod
3. **sample2-sum** - go.sum sample 2
4. **sample3-sum** - go.sum sample 3

## When to Unignore Tests

Tests should be re-enabled once:
1. License detection engine is integrated
2. `declared_license_expression` and `declared_license_expression_spdx` are populated

## Test Data

Test files sourced from Python ScanCode reference:
- `reference/scancode-toolkit/tests/packagedcode/data/golang/`
