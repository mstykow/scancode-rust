# NuGet Parser Improvements

## Summary

Rust now goes beyond the released Python ScanCode NuGet support in five concrete ways:

1. parses additional NuGet and Visual Studio manifests (`project.json`, `project.lock.json`, and PackageReference project files)
2. parses `.deps.json` runtime dependency graphs from built .NET outputs
3. preserves modern nuspec license hints (`license_type`, `license_file`) instead of collapsing everything to deprecated `licenseUrl` fallbacks
4. reads archive-backed license file contents from `.nupkg` files when the nuspec points at a packaged license file
5. parses standalone NuGet Central Package Management files (`Directory.Packages.props`)

## Python Status

- Released ScanCode handles `.nuspec`, `.nupkg`, and `packages.lock.json`, but not `project.json`, `project.lock.json`, PackageReference project files, standalone `Directory.Packages.props`, or `.deps.json` runtime graphs.
- Upstream enhancement issues explicitly ask for these extra manifests and modern nuspec/license improvements.
- Python also keeps NuGet party `type` empty and does not extract packaged license file contents from `.nupkg` archives.

## Rust Improvements

### Extra manifest support

- `project.json` now extracts package metadata plus direct and framework-specific dependencies.
- `project.lock.json` now extracts dependency groups from `projectFileDependencyGroups`.
- PackageReference `.csproj`, `.vbproj`, and `.fsproj` files now extract package metadata and `<PackageReference>` dependencies.
- `Directory.Packages.props` now extracts central `PackageVersion` declarations as dependency metadata, including `Condition` and central-package-management feature flags.
- `.deps.json` now extracts runtime-target-aware resolved dependency graphs from built .NET outputs.

### Standalone CPM parsing (no resolution overclaim)

Rust now parses `Directory.Packages.props` as a standalone NuGet metadata surface.

This first CPM slice intentionally:

- extracts `<PackageVersion Include="..." Version="..." />` and `<PackageVersion Update="..." Version="..." />`
- preserves `Condition` metadata on central package-version entries
- preserves central flags such as `ManagePackageVersionsCentrally`, `CentralPackageTransitivePinningEnabled`, and `CentralPackageVersionOverrideEnabled`
- does **not** attempt to backfill versionless `PackageReference` entries from ancestor props files
- does **not** evaluate parent imports or full MSBuild semantics

That keeps the parser truthful: it adds CPM file visibility now without pretending that full central-version resolution already works.

### Modern nuspec metadata

- NuGet author/owner parties now record `type = person`.
- Nuspec `<license type="expression">...` and `<license type="file">...` now preserve their modern type hints in `extra_data`.
- File-based nuspec licenses keep the real file reference instead of falling back to deprecated `licenseUrl` placeholders.
- Repository `branch` and `commit` attributes are preserved in `extra_data` when present.

### Archive-backed license extraction

- When a `.nupkg` nuspec declares `<license type="file">LICENSE.txt</license>`, Rust now reads that packaged file and stores its contents as the extracted license statement.
- This gives downstream license analysis a real license text source instead of only the placeholder filename.

## Validation

- `cargo test nuget --lib`
- `cargo test --features golden-tests nuget_golden --lib`
- `cargo test --features golden-tests test_assembly_nuget_basic --lib`

## Related Issues

- #157, #159, #162, #163, #165, #215, #216
- #340 standalone `Directory.Packages.props` parser support
