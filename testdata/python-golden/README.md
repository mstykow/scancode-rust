# Python Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Test Status

**Currently Passing:** 2/2 tests

- ✅ `test_golden_metadata` - Passes (PKG-INFO/METADATA files)
- ✅ `test_golden_setup_cfg` - Passes (setup.cfg file)

## Test Coverage

### Passing Tests

1. **metadata** - PKG-INFO/METADATA parser
2. **setup_cfg** - setup.cfg parser

## Test Data

Test files validate Python package metadata extraction from various manifest formats (PKG-INFO, METADATA, setup.cfg, pyproject.toml).
