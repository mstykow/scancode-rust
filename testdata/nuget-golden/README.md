# NuGet Parser Golden Test Suite

## Test Status

**Currently Passing:** 0/6 tests

- ❌ All 6 tests FAILING (expected) - Parser follows extract-only pattern (Feb 7, 2026)
- 🔄 Tests will pass once license detection engine is integrated

**Why Failing**: Parser now extracts ONLY `extracted_license_statement` (raw license URLs/text). License detection fields (`declared_license_expression*`, `license_detections`) are intentionally None/empty until the separate detection engine is built.

**Architecture Details**: See `docs/ARCHITECTURE.md` and `docs/adr/` for the extraction vs detection separation of concerns

## Test Coverage

All 6 tests use legacy `<licenseUrl>` elements (pre-NuGet 4.9):

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
- ✅ `datasource_id` field: Uses `"nuget_nupsec"` (matches Python reference value)
- ✅ `holder` field population (copied from `copyright`)

### Note on `datasource_id` spelling

The `datasource_id` value `"nuget_nupsec"` matches the Python ScanCode reference exactly.
This is a known typo in the original, but we preserve it for compatibility. A comment
in the `DatasourceId` enum documents this.

## Parser Implementation

**What Parser Extracts** (✅ Complete):
- Package metadata (name, version, description, parties)
- Dependencies with framework targeting
- Raw license URLs/text → `extracted_license_statement`
- Copyright and holder information
- Repository and API URLs

**What Parser Does NOT Do** (by design):
- License detection → separate detection engine (see plan doc)

## NuGet License Format Evolution

- **Legacy** (pre-4.9): `<licenseUrl>` - All current tests use this format
- **Modern** (4.9+): `<license type="expression">MIT</license>` - Direct SPDX (parser can extract)
