# NuGet Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility. These tests validate that our NuGet parser extracts metadata in the same format as the reference implementation.

## Test Status

**Currently Passing:** 0/6 tests (6 tests require license detection engine integration)

- 🔄 All 6 tests ignored - Require URL-based license detection (see below)

## Parser vs License Engine Responsibilities

### Parser Responsibility (What We Test)

The NuGet parser extracts and transforms data from .nuspec files:

- Package metadata (name, version, description, summary, title)
- Party information (authors, owners)
- All dependency types with framework targeting
- PURL generation
- Repository and API URLs
- Copyright and holder information
- Declared license extraction (from `<licenseUrl>` elements)

### License Engine Responsibility (Not Tested Here)

These fields come from ScanCode's license detection engine:

- `declared_license_expression` - Requires URL-based license detection
- `declared_license_expression_spdx` - SPDX normalization of detected license
- `license_detections[].identifier` - UUID from license scanner
- `license_detections[].matches[].matched_text` - Matched license text/URL
- `license_detections[].matches[].matcher` - Matching algorithm name
- `license_detections[].matches[].rule_*` - Rule metadata

**All tests are currently ignored** because NuGet packages use `<licenseUrl>` elements instead of embedded SPDX identifiers, requiring URL-based license detection that is not yet implemented.

## When to Unignore Tests

The 6 ignored tests should be re-enabled once:

1. URL-based license detection is integrated
2. License URLs can be matched to SPDX identifiers (e.g., `https://github.com/twbs/bootstrap/blob/master/LICENSE` → `mit`)
3. License detection engine can populate `declared_license_expression` and `declared_license_expression_spdx` from URLs

## Test Suite Coverage

### Ignored Tests (Require License Engine)

All tests require URL-based license detection:

1. **bootstrap** - MIT license via GitHub URL
2. **castle-core** - Apache-2.0 license via URL
3. **entity-framework** - Microsoft EULA via go.microsoft.com URL
4. **jquery-ui** - jQuery license via jquery.org URL
5. **aspnet-mvc** - Microsoft Web Platform EULA
6. **net-http** - Microsoft reference license URL

## Test Data

Test files sourced from Python ScanCode reference:
- `reference/scancode-toolkit/tests/packagedcode/data/nuget/`

Each test includes:
- `.nuspec` file (input)
- `.expected` file (Python ScanCode output)

## Implementation Notes

### Fixed Issues

- ✅ PURL generation for packages
- ✅ `datasource_id` field: Uses `"nuget_nuspec"` (corrected Python's typo `"nuget_nupsec"`)
- ✅ `holder` field population (copied from `copyright`)

### Intentional Divergence from Python Reference

**`datasource_id` typo correction:**
- Python ScanCode: `"nuget_nupsec"` (typo: missing 'e')
- Rust implementation: `"nuget_nuspec"` (correct spelling)
- Golden test expected files updated to match correct spelling
- This improves consistency and correctness without affecting functionality

### Pending Features

- ❌ URL-based license detection
- ❌ `declared_license_expression` from license URLs
- ❌ `declared_license_expression_spdx` from license URLs

## Adding New Golden Tests

When adding tests, consider:

- **With SPDX identifiers**: If `.nuspec` files have `<license type="expression">` elements (NuGet 4.9+), these can pass without license engine
- **With URLs only**: Will require license engine integration

NuGet package format evolution:
- **Legacy** (pre-4.9): `<licenseUrl>` - Requires URL detection
- **Modern** (4.9+): `<license type="expression">MIT</license>` - Direct SPDX

Future golden tests should prioritize modern format packages for better parser-level validation.
