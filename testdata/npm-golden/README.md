# NPM Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility. These tests validate that our npm parser extracts metadata in the same format as the reference implementation.

## Test Status

**Currently Passing:** 5/10 tests

- ✅ `test_golden_basic` - Basic package.json parsing
- ✅ `test_golden_bundled_deps` - Bundled dependencies handling
- ✅ `test_golden_authors_list_strings` - Author array extraction
- ✅ `test_golden_authors_list_dicts` - Author dictionary arrays and raw license statement extraction
- ✅ `test_golden_double_license` - Multiple declared-license forms preserved at parser level
- 🔄 5 tests still ignored - See current blockers below

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

## Test Suite Coverage

### Passing Tests (Parser-Level Validation)

1. **basic** - Minimal package.json, validates core field extraction
2. **bundledDeps** - Bundled dependencies handling
3. **authors_list_strings** - Author array extraction and party information

### Still-Ignored Tests

These tests remain ignored because they currently fail for reasons outside the parser-only license-field skips:

1. **express_jwt** - Dependency PURL/requirement parity gap
2. **from_npmjs** - Expected `sha1` metadata is still missing from actual output
3. **chartist** - Dependency pinnedness classification mismatch
4. **dist** - Dependency ordering/PURL output mismatch
5. **electron** - Fixture files are missing

## When to Unignore Tests

The remaining ignored tests should be re-enabled once their specific blockers are resolved:

1. Parser/golden parity mismatches are fixed for the affected fixtures
2. Missing fixture data is added for `electron` if that case should remain in the suite

License-engine integration is still relevant for richer package/license coverage, but it is no longer the only reason a npm golden test may be ignored.

## Adding New Golden Tests

Focus on parser-specific functionality:

- Edge cases in version constraint parsing
- Unusual dependency configurations
- Different party information formats
- Repository URL normalization
- Workspace dependency resolution

Avoid tests that primarily validate license detection engine behavior.
