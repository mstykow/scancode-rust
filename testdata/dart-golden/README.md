# Dart/Pub Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Coverage Status

These parser-only goldens are active and cover:

- representative `pubspec.yaml` metadata extraction
- `publish_to`, executables, environment handling, and manifest dependency descriptors
- lockfile direct/dev/transitive classification and path-source preservation

## Test Data

Test files sourced from Python ScanCode reference:

- `reference/scancode-toolkit/tests/packagedcode/data/pubspec/`
