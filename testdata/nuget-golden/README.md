# NuGet Parser Golden Test Suite

## Test Status

**Currently Passing:** 12/12 tests

- ✅ Legacy `.nuspec` fixtures remain covered
- ✅ Modern `.nuspec` coverage now includes file-based license metadata (`Fizzler`)
- ✅ Legacy `project.json` and PackageReference `.csproj` fixtures now have parser goldens
- ✅ `.deps.json` runtime dependency graph fixtures now have parser golden coverage
- ✅ Standalone `Directory.Packages.props` CPM parsing now has parser golden coverage
- ✅ Standalone `Directory.Build.props` bounded property-source parsing now has parser golden coverage

**Why parser-only**: NuGet parser goldens verify manifest extraction only. License detection fields (`declared_license_expression*`, `license_detections`) are intentionally validated elsewhere because this suite compares `PackageData` parser output, not full scan-time license analysis.

**Architecture Details**: See `docs/ARCHITECTURE.md` and `docs/adr/` for the extraction vs detection separation of concerns

## Test Coverage

Legacy `.nuspec` parser goldens:

1. **bootstrap** - MIT license via GitHub URL
2. **castle-core** - Apache-2.0 license via URL
3. **entity-framework** - Microsoft EULA via go.microsoft.com URL
4. **jquery-ui** - jQuery license via jquery.org URL
5. **aspnet-mvc** - Microsoft Web Platform EULA
6. **net-http** - Microsoft reference license URL

Modern/additional manifest parser goldens:

7. **fizzler** - modern `.nuspec` with `<license type="file">...` and repository commit metadata
8. **legacy-project-json** - legacy `project.json` direct and framework-specific dependencies
9. **package-reference** - PackageReference `.csproj` metadata and dependencies
10. **deps-json** - `.deps.json` runtime-target dependency graph with direct, transitive, and compile-only entries
11. **central-package-management** - standalone `Directory.Packages.props` with central versions and CPM flags
12. **directory-build-props** - bounded `Directory.Build.props` property-source parsing with parent import metadata

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
- ✅ PackageReference and legacy `project.json` manifests now have parser-golden coverage

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

- **Legacy** (pre-4.9): `<licenseUrl>` - still covered by the original six fixtures
- **Modern** (4.9+): `<license type="expression">MIT</license>` - covered by unit regressions
- **Modern file-based**: `<license type="file">COPYING.txt</license>` - covered by the `Fizzler` parser golden and `.nupkg` unit regression
