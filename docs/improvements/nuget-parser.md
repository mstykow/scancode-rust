# NuGet Parser Improvements

## Summary

Rust now goes beyond the released Python ScanCode NuGet support in four concrete ways:

1. parses additional NuGet and Visual Studio manifests (`project.json`, `project.lock.json`, and PackageReference project files)
2. parses `.deps.json` runtime dependency graphs from built .NET outputs
3. preserves modern nuspec license hints (`license_type`, `license_file`) instead of collapsing everything to deprecated `licenseUrl` fallbacks
4. reads archive-backed license file contents from `.nupkg` files when the nuspec points at a packaged license file

## Python Status

- Released ScanCode handles `.nuspec`, `.nupkg`, and `packages.lock.json`, but not `project.json`, `project.lock.json`, PackageReference project files, or `.deps.json` runtime graphs.
- Upstream enhancement issues explicitly ask for these extra manifests and modern nuspec/license improvements.
- Python also keeps NuGet party `type` empty and does not extract packaged license file contents from `.nupkg` archives.

## Rust Improvements

### Extra manifest support

- `project.json` now extracts package metadata plus direct and framework-specific dependencies.
- `project.lock.json` now extracts dependency groups from `projectFileDependencyGroups`.
- PackageReference `.csproj`, `.vbproj`, and `.fsproj` files now extract package metadata and `<PackageReference>` dependencies.
- `.deps.json` now extracts runtime-target-aware resolved dependency graphs from built .NET outputs.

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
