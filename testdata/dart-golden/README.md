# Dart/Pub Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Coverage Status

This fixture set covers representative `pubspec.lock` and `pubspec.yaml` inputs, but parser-only golden activation still depends on the surrounding license-detection integration boundary.

## When to Unignore Tests

These fixtures can move back into active parser-only golden use once:

1. License detection engine is integrated
2. `declared_license_expression` and `declared_license_expression_spdx` are populated

## Test Data

Test files sourced from Python ScanCode reference:

- `reference/scancode-toolkit/tests/packagedcode/data/pubspec/`
