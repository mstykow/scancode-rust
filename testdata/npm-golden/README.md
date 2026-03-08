# NPM Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility. These tests validate that our npm parser extracts metadata in the same format as the reference implementation.

## Test Status

**Currently Passing:** 11/11 tests

- ✅ `test_golden_basic` - Basic package.json parsing
- ✅ `test_golden_bundled_deps` - Bundled dependencies handling
- ✅ `test_golden_authors_list_strings` - Author array extraction
- ✅ `test_golden_authors_list_dicts` - Author dictionary arrays and raw license statement extraction
- ✅ `test_golden_double_license` - Multiple declared-license forms preserved at parser level
- ✅ `test_golden_express_jwt` - Manifest dependency PURLs match parser-only reference semantics
- ✅ `test_golden_from_npmjs` - Registry metadata parity, including dist hashes and VCS revision handling
- ✅ `test_golden_chartist` - Manifest dependencies remain unversioned and unpinned
- ✅ `test_golden_dist` - Dependency ordering and scoped PURLs match expected output
- ✅ `test_golden_electron` - Fixture-backed parser-only golden coverage restored

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
4. **authors_list_dicts** - Author object arrays and placeholder metadata normalization
5. **double_license** - Multiple declared-license forms preserved without scanner output
6. **express_jwt** - Alias and requirement parity for manifest dependencies
7. **from_npmjs** - Registry tarball/VCS metadata and installed-manifest parity
8. **chartist** - Scoped dependency ordering and unversioned dependency PURLs
9. **dist** - Dist metadata, dependency order preservation, and scoped PURL encoding
10. **electron** - Parser-only fixture coverage for Electron metadata

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
