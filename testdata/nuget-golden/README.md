# NuGet Parser Golden Test Suite

## Coverage Summary

This fixture set covers legacy and modern `.nuspec` manifests, legacy `project.json`, PackageReference `.csproj` files, `.deps.json` runtime dependency graphs, and standalone central-package-management metadata.

**Why parser-only**: NuGet parser goldens verify manifest extraction only. License detection fields (`declared_license_expression*`, `license_detections`) are intentionally validated elsewhere because this suite compares `PackageData` parser output, not full scan-time license analysis.

**Architecture Details**: See `docs/ARCHITECTURE.md` and `docs/adr/` for the extraction vs detection separation of concerns

## Fixture Coverage

Coverage spans legacy URL-based license metadata, modern file-based NuGet license metadata, legacy and PackageReference project manifests, runtime-target dependency graphs, and standalone central-package-management inputs.

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
- ✅ party `type` now records `person` for NuGet author/owner data
- ✅ modern NuGet license metadata preserves `license_type`/`license_file` in parser `extra_data`
- PackageReference and legacy `project.json` manifests are covered in the parser-fixture set.

### Note on `datasource_id` spelling

The `datasource_id` value `"nuget_nupsec"` matches the Python ScanCode reference exactly.
This is a known typo in the original, but we preserve it for compatibility. A comment
in the `DatasourceId` enum documents this.

## Parser Implementation

**What Parser Extracts** (✅ Complete for current fixtures):

- Package metadata (name, version, description, parties)
- Dependencies with framework targeting
- Raw license URLs/text → `extracted_license_statement`
- Modern NuGet license metadata hints (`license_type`, `license_file`) via `extra_data`
- Copyright and holder information
- Repository and API URLs
- Legacy `project.json` and PackageReference project metadata/dependencies
- Standalone `Directory.Packages.props` package versions and CPM flags
- Standalone `Directory.Build.props` bounded property maps and supported import metadata
- `.deps.json` runtime-target metadata, direct/transitive dependency edges, and compile-only flags

**What Parser Does NOT Do** (by design):

- License detection → separate detection engine (see plan doc)

## NuGet License Format Evolution

- **Legacy** (pre-4.9): `<licenseUrl>`
- **Modern** (4.9+): `<license type="expression">MIT</license>`
- **Modern file-based**: `<license type="file">COPYING.txt</license>`
