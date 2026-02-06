# NPM Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility. These tests validate that our npm parser extracts metadata in the same format as the reference implementation.

## Test Status

**Currently Passing:** 3/10 tests (7 tests require license detection engine integration)

- ✅ `test_golden_basic` - Basic package.json parsing
- ✅ `test_golden_bundled_deps` - Bundled dependencies handling  
- ✅ `test_golden_authors_list_strings` - Author array extraction
- 🔄 7 tests ignored - Require license detection engine (see below)

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

**Tests expecting these fields are ignored** until the license detection engine is integrated.

## Test Suite Coverage

### Passing Tests (Parser-Level Validation)

1. **basic** - Minimal package.json, validates core field extraction
2. **bundledDeps** - Bundled dependencies handling
3. **authors_list_strings** - Author array extraction and party information

### Ignored Tests (Require License Engine)

Tests that validate parser functionality but are currently ignored because they expect license detection engine fields:

1. **authors_list_dicts** - Duplicate license detection objects (engine behavior)
2. **double_license** - License identifier and advanced matching
3. **express_jwt** - Full license scanning integration
4. **from_npmjs** - Real-world package with complex licenses
5. **casepath** - Advanced license detection
6. **chartist** - Multiple license scenarios
7. **dist** - Advanced metadata fields
8. **electron** - Empty test directory (no files)

## When to Unignore Tests

The 7 ignored tests should be re-enabled once:

1. License detection engine is integrated into the scanning pipeline
2. License file scanning (not just package.json extraction) is implemented
3. License identifier generation is functional
4. Rule matching and scoring systems are in place

Until then, the parser-only comparison function (`compare_package_data_parser_only` in `src/test_utils.rs`) skips these engine-specific fields.

## Adding New Golden Tests

Focus on parser-specific functionality:

- Edge cases in version constraint parsing
- Unusual dependency configurations
- Different party information formats
- Repository URL normalization
- Workspace dependency resolution

Avoid tests that primarily validate license detection engine behavior.
