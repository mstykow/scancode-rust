# Parser Feature Parity Plan

This document outlines the roadmap for achieving feature parity between the Rust parsers in `scancode-rust` and the original Python implementation in the ScanCode Toolkit reference at `reference/scancode-toolkit/src/packagedcode/`.

**Status**: This is a living document that tracks missing features across our 5 existing parsers.

**Last Updated**: February 2026

---

## Overview

The scancode-rust project currently implements 5 package parsers:
- **npm** (`src/parsers/npm.rs`) - Node.js packages
- **cargo** (`src/parsers/cargo.rs`) - Rust packages
- **maven** (`src/parsers/maven.rs`) - Java/Maven packages
- **python** (`src/parsers/python.rs`) - Python packages
- **npm_lock** (`src/parsers/npm_lock.rs`) - npm lockfiles

This plan systematically identifies and addresses gaps between our implementation and the original Python codebase to ensure comprehensive package metadata extraction.

---

## 1. NPM Parser Enhancements

**Reference**: `reference/scancode-toolkit/src/packagedcode/npm.py`

**Current implementation**: `src/parsers/npm.rs`

### 1.1 Additional Lockfile Support

#### 1.1.1 npm-shrinkwrap.json Support

- Add `NpmShrinkwrapHandler` that recognizes `npm-shrinkwrap.json` and `.npm-shrinkwrap.json`
- Reuse existing `package-lock.json` parsing logic (format is identical)
- Add `is_match()` pattern for shrinkwrap files
- Test with shrinkwrap examples from Python testdata

#### 1.1.2 yarn.lock Support (v1 and v2)

**Yarn v1 format:**

- Implement parser for Yarn v1 custom YAML-like format
- Parse dependency blocks with format: `package@version:\n  version "x.y.z"`
- Handle resolution field and dependency mappings
- Extract checksums from `resolved` field
- Support multiple package names aliasing same version (`pkg1@^1.0.0, pkg2@~1.0.0:`)

**Yarn v2 format:**

- Implement parser for Yarn v2 YAML format with `__metadata` section
- Parse `version`, `resolution`, `dependencies`, `peerDependencies`
- Handle Yarn v2 descriptor format (`workspace:*`, `npm:x.y.z`)
- Extract `checksum` field (different format from v1)

**Implementation:**

- Create test file: `src/parsers/yarn_lock.rs`
- Detect version from file structure (v1 has no `__metadata`, v2 has it)
- Share common dependency extraction logic where possible

#### 1.1.3 pnpm Lockfile Support (pnpm-lock.yaml, pnpm-shrinkwrap.yaml, pnpm-workspace.yaml)

**pnpm-lock.yaml and pnpm-shrinkwrap.yaml:**

- Implement pnpm lockfile parser (supports versions 5, 6, 9)
- Parse `lockfileVersion` to determine format
- For v5/6: parse `packages` with resolution strings as keys
- For v9: parse flat structure with different key format
- Extract `resolution`, `dependencies`, `devDependencies`, `peerDependencies`
- Handle pnpm-specific fields: `hasBin`, `requiresBuild`, `optional`, `dev`
- Recognize both `pnpm-lock.yaml` and legacy `pnpm-shrinkwrap.yaml` filenames

**pnpm-workspace.yaml:**

- Parse workspace configuration file
- Extract `packages` array with glob patterns
- Store in `extra_data` or use for workspace resolution context

### 1.2 Additional Dependency Scopes

Extract additional dependency types from package.json:

- **peerDependencies**: Extract from `peerDependencies` object, mark with `is_runtime: true` and custom scope marker
- **optionalDependencies**: Extract from `optionalDependencies`, mark with `is_optional: true` and `is_runtime: true`
- **bundledDependencies**: Extract from `bundledDependencies` or `bundleDependencies` (both valid spellings), parse array of package names (no versions), create dependencies with special bundled flag
- **resolutions**: Extract Yarn `resolutions` field (version overrides), store in `extra_data` or as special dependency type

### 1.3 Additional Metadata Fields

Extract additional metadata fields from package.json:

- **description**: Extract `description` string, add to `PackageData.description` field (may need to add this field to the struct)
- **keywords**: Extract `keywords` array, store as comma-separated string or in `extra_data`
- **engines**: Extract `engines` object (node, npm versions), store in `extra_data` as JSON string
- **packageManager**: Extract `packageManager` field (e.g., "pnpm@8.6.0"), store in `extra_data`
- **workspaces**: Extract `workspaces` array or object, parse glob patterns for workspace packages, store in `extra_data`
- **private**: Extract `private` boolean flag, store in `extra_data`

### 1.4 Additional URLs

#### 1.4.1 bugs URL

- Extract `bugs` field (string or object with `url` property)
- Add `bug_tracking_url` field to `PackageData` or store in `extra_data`

### 1.5 Distribution Metadata

Extract distribution metadata from the `dist` object:

- **dist.integrity**: Extract checksum (SHA-512 base64), parse and add to checksums array in `PackageData`
- **dist.tarball**: Extract download URL, add to `download_url` field in `PackageData`

### 1.6 Dependency Metadata

Parse metadata objects that provide additional information about dependencies:

- **peerDependenciesMeta**: Extract object, mark peer dependencies as optional based on this metadata
- **dependenciesMeta**: Extract object (pnpm-specific), apply `injected` and `optional` flags to dependencies

### 1.7 Workspace Features

Implement npm/Yarn workspace support:

- **workspace: protocol resolution**: Detect `workspace:*`, `workspace:^`, `workspace:~` in dependency versions, resolve to actual package versions from workspace, mark dependencies as workspace references
- **Glob pattern matching**: Parse glob patterns from `workspaces` array, use `globset` crate for pattern matching, identify workspace package locations

### 1.8 Advanced Features

#### 1.8.1 npm: Alias Support

- Parse `npm:package@version` alias format in dependencies
- Extract actual package name and version
- Create PURL with actual package info

#### 1.8.2 Registry URL Generation

- Generate npm registry URLs: `https://registry.npmjs.org/{package}`
- Add API URL: `https://registry.npmjs.org/{package}/{version}`
- Add to `PackageData` URLs

---

## 2. Cargo Parser Enhancements

**Reference**: `reference/scancode-toolkit/src/packagedcode/cargo.py`

**Current implementation**: `src/parsers/cargo.rs`

### 2.1 Cargo.lock Support

Implement Cargo.lock lockfile parser:

- **Parser implementation**: Create parser for TOML-based `Cargo.lock` format, parse `[[package]]` array entries, extract `name`, `version`, `source`, `dependencies`, `checksum`
- **Source types**: Handle different source formats: `registry+https://`, `git+https://`
- **Dependency extraction**: Parse `dependencies` array with format `"serde 1.0.0"` or `"serde"`, generate PURLs for all transitive dependencies, mark all as `is_pinned: true`
- **Checksum extraction**: Extract `checksum` field (SHA-256 hash), add to package checksums array

### 2.2 Additional Metadata Fields

Extract additional package metadata from Cargo.toml:

- **description**: Extract `package.description`, add to `PackageData.description`
- **keywords**: Extract `package.keywords` array, store as comma-separated string
- **categories**: Extract `package.categories` array, merge with keywords or store separately in `extra_data`
- **rust-version**: Extract `package.rust-version` (MSRV - Minimum Supported Rust Version), store in `extra_data`
- **edition**: Extract `package.edition` (2015, 2018, 2021, 2024), store in `extra_data`

### 2.3 Additional URLs

#### 2.3.1 documentation URL

- Extract `package.documentation` field
- Add to `PackageData` as separate field or in `extra_data`

### 2.4 License Files

#### 2.4.1 license-file Field

- Extract `package.license-file` path
- Store in `extra_data` or attempt to read actual file
- If file can be read, use for license detection

### 2.5 Additional Dependency Sections

#### 2.5.1 build-dependencies Support

- Extract `[build-dependencies]` table
- Mark with special scope: `is_runtime: false`, add build flag
- Parse both string and table formats

### 2.6 Workspace Features

Implement Cargo workspace support with field and dependency inheritance:

- **Workspace detection**: Parse `[workspace]` table, extract `members` array (glob patterns) and `exclude` array
- **Package field inheritance**: Parse `workspace.package` table with inheritable fields, when member has `version.workspace = true` inherit from workspace, apply to: `version`, `authors`, `license`, `repository`, `homepage`, `documentation`, `edition`, `rust-version`
- **Dependency inheritance**: Parse `workspace.dependencies` table, when dependency has `workspace = true` inherit version/features from workspace, resolve workspace dependencies for member packages

### 2.7 Dependency Attributes

Parse additional dependency attributes from table format:

- **optional**: Extract `optional` flag from dependency table, mark dependency with `is_optional: true`
- **workspace**: Detect `workspace = true` in dependencies, resolve to actual version from workspace dependencies table
- **features**: Extract `features` array from dependency table, store in `extra_data` or dependency metadata

---

## 3. Maven Parser Enhancements

**Reference**: `reference/scancode-toolkit/src/packagedcode/maven.py`

**Current implementation**: `src/parsers/maven.rs`

### 3.1 Additional File Support

Implement parsers for additional Maven-related files:

- **pom.properties**: Implement parser for Java properties format, look for `pom.properties` alongside `pom.xml`, extract `groupId`, `artifactId`, `version`, merge properties into main POM data
- **MANIFEST.MF**: Parse JAR `META-INF/MANIFEST.MF` files, extract `Bundle-SymbolicName`, `Bundle-Version`, `Implementation-Title`, etc., can assemble with POM data when both present

### 3.2 Organization Metadata

#### 3.2.1 Organization Name and URL

- Extract `organization/name` element
- Extract `organization/url` element
- Add to `PackageData` parties or `extra_data`

### 3.3 People Metadata

Extract developers and contributors from POM:

- **Developers**: Parse `developers/developer` elements, extract for each: `id`, `name`, `email`, `url`, `organization`, `organizationUrl`, `roles/role`, `timezone`, `properties`, create `Party` objects with all fields, add to `PackageData.parties`
- **Contributors**: Parse `contributors/contributor` elements with same structure as developers, create separate `Party` objects, distinguish from developers with role or flag

### 3.4 SCM Metadata

#### 3.4.1 SCM Information Extraction

- Parse `scm` element
- Extract: `connection`, `developerConnection`, `url`, `tag`
- Normalize VCS URLs (git://, svn://, etc.)
- Store in `extra_data` or dedicated field

### 3.5 Issue Management

#### 3.5.1 Issue Tracker Extraction

- Parse `issueManagement` element
- Extract: `system` (JIRA, GitHub, etc.), `url`
- Add bug tracking URL to `PackageData`

### 3.6 CI Configuration

#### 3.6.1 CI System Extraction

- Parse `ciManagement` element
- Extract: `system` (Jenkins, Travis, etc.), `url`
- Store in `extra_data`

### 3.7 Distribution Management

Extract distribution management configuration:

- **Repository configuration**: Parse `distributionManagement/repository` (release repo) and `distributionManagement/snapshotRepository` (snapshot repo), extract `id`, `name`, `url`, `layout`
- **Site configuration**: Parse `distributionManagement/site`, extract `id`, `name`, `url`
- **Download URL**: Parse `distributionManagement/downloadUrl`, add to `PackageData.download_url`

### 3.8 Repository Definitions

Extract repository and plugin repository configurations:

- **Repositories**: Parse `repositories/repository` elements, extract `id`, `name`, `url`, `layout`, `releases`, `snapshots`, store in `extra_data`
- **Plugin repositories**: Parse `pluginRepositories/pluginRepository` elements with same structure, store separately in `extra_data`

### 3.9 Multi-Module Support

#### 3.9.1 Modules Extraction

- Parse `modules/module` elements (list of subdirectories)
- Store module paths in `extra_data`
- Can be used to discover related POMs

### 3.10 Mailing Lists

#### 3.10.1 Mailing List Extraction

- Parse `mailingLists/mailingList` elements
- Extract: `name`, `subscribe`, `unsubscribe`, `post`, `archive`
- Store in `extra_data`

### 3.11 Dependency Management

Implement dependency version management and inheritance:

- **dependencyManagement parsing**: Parse `dependencyManagement/dependencies/dependency` elements, extract version constraints for managed dependencies
- **Version inheritance**: When dependency has no version, look up in `dependencyManagement` and apply inherited version, handle both direct and parent POM dependency management

### 3.12 Property Resolution

Implement Maven property resolution system:

- **Property extraction**: Parse `properties` element (all child elements), build map of property name → value, include standard properties: `project.version`, `project.groupId`, etc.
- **Property substitution**: Replace `${propertyName}` references in all fields, support nested properties: `${outer.${inner}}`
- **Expression evaluation**: Handle substring operations `${property.substring(8)}` and other Maven expressions like `.replace()`, limit complexity to avoid full expression parser (focus on common patterns)

### 3.13 Parent POM Support

Implement parent POM reference and inheritance:

- **Parent reference extraction**: Parse `parent` element, extract `groupId`, `artifactId`, `version`, `relativePath`, store parent coordinates in `extra_data`
- **Field inheritance**: Inherit `groupId` and `version` from parent if missing in current POM, inherit URLs from parent (with `artifactId` appended to path)
- **Note**: Full parent POM loading out of scope (requires file system access to resolve and load parent POM files)

### 3.14 Source Package Support

#### 3.14.1 Source Classifier PURL

- Generate additional PURL with `classifier=sources`
- Add to dependencies or package references
- Maven Central convention for source JARs

### 3.15 License Mapping Extension

#### 3.15.1 Expanded SPDX Mapping

- Current: 4 licenses mapped (Apache-2.0, MIT, GPL-3.0, BSD-3-Clause)
- Add mappings for: EPL-1.0, EPL-2.0, LGPL-2.1, LGPL-3.0, CDDL-1.0, MPL-2.0
- Add mappings for common variations: "Apache License, Version 2.0"
- Create lookup table similar to Python implementation

---

## 4. Python Parser Enhancements

**Reference**: `reference/scancode-toolkit/src/packagedcode/pypi.py`

**Current implementation**: `src/parsers/python.rs`

### 4.1 Additional Manifest Files

Implement parsers for additional Python package formats:

- **PKG-INFO and METADATA**: Implement RFC 822 email header parser for both formats (PKG-INFO in `.egg-info/`, METADATA in `.dist-info/`), extract metadata fields (Name, Version, Summary, Home-page, Author, etc.), handle multi-line fields (continuation with indentation), METADATA is newer version with dynamic fields and multiple Project-URL entries
- **setup.cfg**: Implement INI-style parser, parse `[metadata]` section for package info, parse `[options]` section for dependencies, parse `[options.extras_require]` for optional dependencies, map setup.cfg fields to PackageData

### 4.2 Archive Parsing

Implement archive format support for Python packages:

- **Wheel archives (.whl)**: Detect `.whl` files (zip archives), extract and parse METADATA from `.dist-info/` directory, extract RECORD file for file listing with checksums, generate PURL from wheel filename format: `{name}-{version}-{python}-{abi}-{platform}.whl`
- **Egg archives (.egg)**: Detect `.egg` files (zip archives), extract and parse PKG-INFO from `.egg-info/` directory, extract `installed-files.txt` for file listing, generate PURL from egg filename
- **Source distributions**: Detect `.tar.gz`, `.tar.bz2`, `.zip` source distributions, look for `PKG-INFO` in root or package directory, extract and parse setup.py if PKG-INFO missing

### 4.3 Lockfile Support

Implement Python lockfile format parsers:

- **poetry.lock**: Implement TOML parser, parse `[[package]]` array entries, extract name/version/description/category/optional/dependencies, handle different dependency types (main, dev), parse `[metadata]` section for lock version
- **Pipfile**: Implement TOML parser, parse `[packages]` and `[dev-packages]` sections, extract version specifiers (can be strings or tables), parse `[requires]` for Python version, parse `[source]` for custom package indexes
- **Pipfile.lock**: Implement JSON parser, parse `default` and `develop` dependency sections, extract version/hashes/markers/index, mark as pinned dependencies

### 4.4 pip-inspect Format

#### 4.4.1 pip-inspect.deplock Parser

- Implement parser for `pip inspect` JSON output
- Parse `installed` array of packages
- Extract metadata, dependencies, and files from inspection data

### 4.5 Additional Metadata Fields

Extract additional metadata from PKG-INFO/METADATA files:

- **Summary/Description**: Extract `Summary` field (single-line) and `Description` field (multi-line payload), handle legacy `DESCRIPTION.rst` file references, map to appropriate PackageData field
- **Download-URL**: Extract `Download-URL` metadata field, add to `PackageData.download_url`
- **Requires-Python**: Extract `Requires-Python` (e.g., ">=3.8"), store in `extra_data` as `python_requires`
- **Classifiers**: Extract `Classifier` fields (can be multiple), filter license classifiers and extract to licenses, filter non-license classifiers and store as keywords, parse Trove classifier format

### 4.6 Project URLs

#### 4.6.1 Project-URL Extraction

- Parse multiple `Project-URL` fields (format: "Label, URL")
- Recognize types: Homepage, Documentation, Source, Bug Tracker, Changelog, etc.
- Map recognized types to appropriate PackageData fields
- Store others in `extra_data`

### 4.7 License Files

#### 4.7.1 License-File Field

- Extract `License-File` metadata field (can be multiple)
- Store file paths in `extra_data`
- Consider reading actual license files for detection

### 4.8 File References

Extract file listings and checksums from installed packages:

- **RECORD files**: Parse `.dist-info/RECORD` CSV file, extract file path/hash algorithm/hash value/size, store checksums for installed files, add to PackageData file references
- **installed-files.txt**: Parse `.egg-info/installed-files.txt`, extract list of installed file paths, store in PackageData

### 4.9 Advanced setup.py Parsing

Improve setup.py parsing accuracy:

- **AST-based parser**: Replace regex-based extraction with AST parsing, use Python AST parsing (via py_literal crate or embedded Python), parse `setup()` function call arguments, extract name/version/description/dependencies/extras_require/etc.
- **Version detection**: Scan Python files for `__version__ = "x.y.z"` assignments, use when setup.py doesn't contain version, look in common locations: `__init__.py`, `_version.py`

### 4.10 Advanced Dependency Parsing

Implement complete PEP 508 requirement specification parser:

- **Full PEP 508 support**: Implement parser for format `package[extra1,extra2]>=1.0,<2.0; python_version>='3.8' and platform_system=='Linux'`, extract package name/extras/version specifiers/markers
- **Marker evaluation**: Parse environment markers (`python_version`, `platform_system`, `sys_platform`, etc.), extract Python version and OS/platform requirements, store in dependency metadata
- **Extras handling**: Parse `extra == 'name'` markers, use extra name as dependency scope, link extras to optional dependency groups

### 4.11 Dependency Scopes

Implement proper dependency scope detection:

- **tests scope**: Recognize test dependencies from setup.py `tests_require`, mark with `is_runtime: false` and test scope
- **setup scope**: Recognize setup dependencies from pyproject.toml `build-system.requires`, mark with `is_runtime: false` and build scope
- **Extra-based scopes**: Create scope per extra name (e.g., "extra:dev", "extra:test"), extract from `extras_require` or `optional-dependencies`

### 4.12 requirements.txt Features

Support advanced requirements.txt features:

- **Editable installs**: Parse `-e` or `--editable` flags, extract package reference (path or VCS URL), mark as editable in dependency metadata
- **VCS URLs**: Parse VCS URLs (`git+https://...`, `git+ssh://...`, `svn+https://...`), extract VCS type/URL/revision/tag/branch, generate PURL with VCS qualifier
- **Nested includes**: Parse `-r` or `--requirement` includes, recursively read included requirements files, handle relative paths
- **Constraints files**: Parse `-c` or `--constraint` references, extract version constraints from constraint files

---

## 5. npm_lock Parser Enhancements

**Reference**: `reference/scancode-toolkit/src/packagedcode/npm.py` (lockfile handlers)

**Current implementation**: `src/parsers/npm_lock.rs`

### 5.1 Minor Enhancements

Small improvements to npm_lock parser:

- **License extraction**: If `packages` entries contain `license` field, extract it and add to `LicenseDetection` for dependency packages (not common in lockfiles but supported in spec)
- **is_direct flag correction**: Current implementation marks all dependencies as `is_direct: false` in v2+ format; fix by parsing root `dependencies` vs `devDependencies` to identify direct deps, match package paths to direct dependency names, mark direct dependencies with `is_direct: true`
- **Peer dependencies metadata**: Extract `peerDependencies` from package entries (if present), add peer dependency relationships, mark with appropriate scope/flags

---

## Implementation Strategy

### Development Order

1. **npm.rs** (highest priority, most complex)
   - Start with additional lockfiles (Yarn, pnpm) - new parsers
   - Then additional fields and scopes - extend existing
   - Finally workspace and advanced features

2. **python.rs** (second priority, high usage)
   - Start with PKG-INFO/METADATA (most common)
   - Add lockfile support (Poetry, Pipfile)
   - Improve setup.py parsing
   - Add PEP 508 parser

3. **cargo.rs** (medium priority)
   - Add Cargo.lock (most important missing feature)
   - Add additional fields
   - Add workspace support

4. **maven.rs** (medium priority)
   - Add people, SCM, issue management (most useful)
   - Add property resolution (most complex)
   - Add parent POM support
   - Expand license mapping

5. **npm_lock.rs** (low priority, mostly complete)
   - Small fixes for is_direct flag
   - Add license extraction if needed

### Testing Approach

- For each feature, create test files in `testdata/<ecosystem>/`
- Reference Python test data at `reference/scancode-toolkit/tests/packagedcode/data/`
- Write unit tests in `*_test.rs` files
- Ensure output matches Python implementation where possible
- Document intentional deviations

### Code Quality

- Run `cargo clippy` after each feature implementation
- Run `cargo fmt` before commits
- Update `AGENTS.md` if adding new patterns/conventions
- Keep functions focused and well-documented
- Use proper error handling (`Result<T, E>`, not `unwrap()`)

### Dependencies

May need to add:

- `globset` - for workspace glob pattern matching
- `py_literal` - for Python AST parsing (or use external Python interpreter)
- Additional TOML/YAML/JSON parser features
- `csv` crate - for RECORD file parsing

---

## Progress Tracking

To track progress on this plan, mark completed sections with checkboxes:

- [ ] npm.rs - Additional lockfiles
- [ ] npm.rs - Additional dependency scopes
- [ ] npm.rs - Additional metadata fields
- [ ] npm.rs - Workspace features
- [ ] cargo.rs - Cargo.lock support
- [ ] cargo.rs - Additional metadata
- [ ] cargo.rs - Workspace support
- [ ] maven.rs - Additional files and people
- [ ] maven.rs - Property resolution
- [ ] maven.rs - Parent POM support
- [ ] python.rs - PKG-INFO/METADATA
- [ ] python.rs - Lockfile support
- [ ] python.rs - PEP 508 parser
- [ ] python.rs - Advanced features
- [ ] npm_lock.rs - Minor enhancements

---

## Contributing

When implementing features from this plan:

1. Read the relevant section in this document
2. Review the original Python implementation in `reference/scancode-toolkit/`
3. Create tests before implementing (TDD approach recommended)
4. Implement the feature following Rust best practices
5. Run tests and quality checks (`cargo test`, `cargo clippy`, `cargo fmt`)
6. Update this document to mark the feature as complete
7. Create a PR with clear description of what was implemented

For questions or clarifications, refer to `AGENTS.md` for coding guidelines and project conventions.
