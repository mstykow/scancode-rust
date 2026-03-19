# NPM Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility. These tests validate that our npm parser extracts metadata in the same format as the reference implementation.

## Coverage Summary

This fixture set covers baseline `package.json` extraction, bundled dependencies, author-array forms, multiple declared-license representations, manifest dependency semantics, registry metadata, scoped dependency ordering, and parser-only Electron metadata behavior.

## Parser vs License Engine Responsibilities

### Parser Responsibility (What We Test)

The npm parser extracts and transforms data from package.json:

- Package metadata (name, version, description, keywords)
- Party information (authors, contributors, maintainers)
- All dependency types (dependencies, devDependencies, peer, optional, bundled)
- PURL generation with correct encoding
- Repository URLs and VCS information
- Declared license extraction (from `license` and `licenses` fields)
- Extra data (engines, packageManager, workspaces, private flag)
- Download URLs and SHA integrity values

### License Engine Responsibility (Not Tested Here)

These fields come from ScanCode's license detection engine:

- `license_detections[].identifier` - UUID from license scanner
- `license_detections[].matches[].matched_text` - Matched license text
- `license_detections[].matches[].matcher` - Matching algorithm name
- `license_detections[].matches[].matched_length/match_coverage` - Match metrics
- `license_detections[].matches[].rule_*` - Rule metadata

These fields are intentionally skipped by the parser-only comparison helper.

## Fixture Coverage

The fixtures exercise parser-level validation for core package metadata, party extraction, declared-license preservation, manifest dependency requirements, registry tarball and VCS metadata, and ordering-sensitive npm output details.

## Ignore Policy

For npm parser goldens, ignores should be reserved for cases genuinely blocked by the missing license detection engine. Parser-only parity gaps should be fixed and re-enabled in this suite rather than deferred.

## Adding New Golden Tests

Focus on parser-specific functionality:

- Edge cases in version constraint parsing
- Unusual dependency configurations
- Different party information formats
- Repository URL normalization
- Workspace dependency resolution

Avoid tests that primarily validate license detection engine behavior.
