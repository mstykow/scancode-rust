# Python ScanCode Assembly Logic - Detailed Implementation Reference

## Complete Assembler List

| Ecosystem | File | Handler Class | Pattern | Multi-File | Workspace |
|-----------|------|---------------|---------|-----------|-----------|
| npm | npm.py | BaseNpmHandler | Sibling-merge | ✓ | ✗ |
| cargo | cargo.py | CargoBaseHandler | Sibling-merge | ✓ | ✓ |
| maven | maven.py | MavenBasePackageHandler | Nested sibling | ✓ | ✗ |
| cocoapods | cocoapods.py | BasePodHandler | Sibling-merge | ✓ | ✗ |
| phpcomposer | phpcomposer.py | BasePhpComposerHandler | Sibling-merge | ✓ | ✗ |
| rubygems | rubygems.py | BaseGemProjectHandler | Multi-file | ✓ | ✗ |
| golang | golang.py | BaseGoModuleHandler | Sibling-merge | ✓ | ✗ |
| pubspec | pubspec.py | BaseDartPubspecHandler | Sibling-merge | ✓ | ✗ |
| swift | swift.py | BaseSwiftDatafileHandler | Multi-file | ✓ | ✗ |
| conda | conda.py | CondaBaseHandler | Directory-based | ✓ | ✗ |
| debian | debian.py | DebianDebPackageHandler | Archive-based | ✓ | ✗ |
| alpine | alpine.py | AlpineApkArchiveHandler | Archive+DB | ✓ | ✗ |
| rpm | rpm.py | BaseRpmInstalledDatabaseHandler | Database | ✗ | ✗ |
| pypi | pypi.py | BaseExtractedPythonLayout | Multi-format | ✓ | ✗ |
| chef | chef.py | BaseChefMetadataHandler | Sibling-merge | ✓ | ✗ |
| conan | conan.py | ConanFileHandler | Sibling-merge | ✓ | ✗ |
| debian_copyright | debian_copyright.py | BaseDebianCopyrightFileHandler | Standalone | ✗ | ✗ |
| about | about.py | AboutFileHandler | Standalone | ✗ | ✗ |
| build | build.py | AutotoolsConfigureHandler | Non-assembling | ✗ | ✗ |

## Detailed Assembly Logic by Ecosystem

### npm (npm.py)

**Files Involved:**

- Primary: `package.json`
- Lockfiles: `package-lock.json`, `npm-shrinkwrap.json`, `.package-lock.json`

**Assembly Logic:**

```text
if resource.name == 'package.json':
    package_resource = resource
elif resource.name in lockfile_names:
    # Find sibling package.json
    siblings = resource.siblings(codebase)
    package_resource = find_by_name(siblings, 'package.json')

if package_resource:
    # Create package from package.json
    # Walk tree, skip node_modules
    # Assign resources to package
    # For each lockfile, merge dependencies
else:
    # No package.json: yield dependencies only
```

**Key Features:**

- Handles missing package.json (lockfile-only case)
- Skips node_modules directory
- Merges multiple dependency types (dependencies, devDependencies, etc.)
- Supports scoped packages (@namespace/name)

---

### cargo (cargo.py)

**Files Involved:**

- Primary: `Cargo.toml`
- Lockfile: `Cargo.lock`
- Workspace: `Cargo.toml` (with [workspace] section)

**Assembly Logic:**

```text
workspace = package_data.extra_data.get('workspace', {})
workspace_members = workspace.get('members', [])

if workspace_members:
    for member_path in workspace_members:
        # Find Cargo.toml in member directory
        # Create package for each member
        # Copy workspace-level license data to members

# Merge Cargo.lock dependencies
```

**Key Features:**

- Workspace support with multiple packages
- Glob pattern support for workspace members
- Workspace-level license inheritance
- Handles workspace package data

---

### maven (maven.py)

**Files Involved:**

- Primary: `pom.xml` (in `META-INF/maven/**/`)
- Secondary: `MANIFEST.MF` (in `META-INF/`)

**Assembly Logic:**

```text
if is_pom_xml:
    # Create package from pom.xml
    # Find sibling MANIFEST.MF
    # Update package with MANIFEST.MF data
elif is_manifest:
    # Find sibling pom.xml
    # Create package from pom.xml
    # Update with MANIFEST.MF data
```

**Key Features:**

- Nested directory structure (JAR-specific)
- Order-dependent: pom.xml first, then MANIFEST.MF
- Handles both source and compiled JAR packages

---

### cocoapods (cocoapods.py)

**Files Involved:**

- Primary: `.podspec` or `Podfile`
- Lockfile: `Podfile.lock`

**Assembly Logic:**

```text
if resource.name == '*.podspec':
    # Find sibling Podfile.lock
    # Merge dependency data
    # Handle spec repositories and checksums
```

**Key Features:**

- Podspec JSON support
- Dependency repository tracking
- Checksum verification data
- External source handling

---

### phpcomposer (phpcomposer.py)

**Files Involved:**

- Primary: `composer.json`
- Lockfile: `composer.lock`

**Assembly Logic:**

```text
if resource.name == 'composer.json':
    # Find sibling composer.lock
    # Merge locked versions
    # Create single package with combined dependencies
```

**Key Features:**

- Simple sibling-merge pattern
- Locked version pinning
- Namespace support

---

### rubygems (rubygems.py)

**Files Involved:**

- Primary: `.gemspec`
- Lockfile: `Gemfile.lock`
- Archive: `*.gem` (tar archive)
- Extracted: `metadata.gz-extract`

**Assembly Logic:**

```text
# Multiple handler classes:
# 1. GemArchiveHandler: Parse .gem files
# 2. GemMetadataArchiveExtractedHandler: Handle extracted metadata
# 3. BaseGemProjectHandler: Handle .gemspec + Gemfile.lock

if resource.name == '*.gemspec':
    # Find sibling Gemfile.lock
    # Merge dependencies
```

**Key Features:**

- Multiple gem formats (archive, extracted, project)
- Gemfile.lock dependency merging
- Metadata extraction from archives

---

### golang (golang.py)

**Files Involved:**

- Primary: `go.mod`
- Lockfile: `go.sum`

**Assembly Logic:**

```text
# Always use go.mod first, then go.sum
if resource.name == 'go.mod':
    # Find sibling go.sum
    # Merge checksum data
```

**Key Features:**

- Checksum verification
- Module path handling
- Indirect dependency tracking

---

### pubspec (pubspec.py)

**Files Involved:**

- Primary: `pubspec.yaml`
- Lockfile: `pubspec.lock`

**Assembly Logic:**

```text
if resource.name == 'pubspec.yaml':
    # Find sibling pubspec.lock
    # Merge locked versions
```

**Key Features:**

- YAML format
- Locked version pinning
- Dart/Flutter specific

---

### swift (swift.py)

**Files Involved:**

- Primary: `Package.swift` (or `Package.swift.json`)
- Resolved: `Package.resolved`
- Dependency graph: `swift-show-dependencies.deplock`

**Assembly Logic:**

```text
# Combine multiple sources:
# 1. Package.swift manifest
# 2. Package.resolved (locked versions)
# 3. swift-show-dependencies.deplock (dependency graph)
```

**Key Features:**

- Multiple manifest formats
- Resolved dependency tracking
- DepLock format support
- Dependency graph information

---

### conda (conda.py)

**Files Involved:**

- Metadata: `conda-meta/*.json`
- Environment: `environment.yaml` or `conda.yaml`

**Assembly Logic:**

```text
# Directory-based assembly
for json_file in conda-meta/:
    # Parse package metadata
    # Find related environment.yaml
    # Merge environment data
```

**Key Features:**

- Directory-based scanning
- Multiple metadata files per package
- Environment specification merging
- Installation structure awareness

---

### debian (debian.py)

**Files Involved:**

- Package: `*.deb` (binary package)
- Source metadata: `*.debian.tar.xz` or `*.debian.tar.gz`
- Source code: `*.orig.tar.xz` or `*.orig.tar.gz`

**Assembly Logic:**

```text
# Archive extraction and merging
if resource.name == '*.deb':
    # Extract .deb archive
    # Parse control files
    # Find related .debian.tar.xz
    # Merge metadata
```

**Key Features:**

- Archive extraction
- Control file parsing
- Source package handling
- Debian-specific metadata

---

### alpine (alpine.py)

**Files Involved:**

- Package: `*.apk` (tar gzipped)
- Database: `lib/apk/db/installed`
- Build script: `APKBUILD`

**Assembly Logic:**

```text
# Multiple handler classes:
# 1. AlpineApkArchiveHandler: Parse .apk archives
# 2. AlpineInstalledDatabaseHandler: Parse installed DB
# 3. AlpineApkbuildHandler: Parse APKBUILD scripts
```

**Key Features:**

- Archive parsing
- Installed database integration
- Build script analysis
- Alpine-specific format

---

### rpm (rpm.py)

**Files Involved:**

- Database: `usr/lib/sysimage/rpm/Packages.db` (NDB)
- Database: `usr/lib/sysimage/rpm/Packages.sqlite` (SQLite)

**Assembly Logic:**

```text
# Database-based (no multi-file assembly)
# Parse RPM database directly
# Extract package metadata from database
```

**Key Features:**

- NDB database format (recent SUSE)
- SQLite database format (RHEL/CentOS/Fedora)
- System package database parsing
- No multi-file assembly

---

### pypi (pypi.py)

**Files Involved:**

- Metadata: `PKG-INFO` (in EGG-INFO or egg-info)
- Build: `setup.py`, `setup.cfg`
- Modern: `pyproject.toml`

**Assembly Logic:**

```text
# Multiple handler classes for different formats:
# 1. PythonEggPkgInfoFile: EGG-INFO/PKG-INFO
# 2. PythonEditableInstallationPkgInfoFile: .egg-info/PKG-INFO
# 3. BaseExtractedPythonLayout: setup.py/setup.cfg/pyproject.toml

# Merge data from multiple sources
```

**Key Features:**

- Multiple metadata formats
- Editable installation support
- Setup.py/setup.cfg parsing
- pyproject.toml support
- Egg-info extraction

---

### chef (chef.py)

**Files Involved:**

- Primary: `metadata.rb`
- Secondary: `metadata.json`

**Assembly Logic:**

```text
if resource.name == 'metadata.rb':
    # Find sibling metadata.json
    # Merge metadata from both files
```

**Key Features:**

- Ruby DSL parsing
- JSON metadata merging
- Cookbook-specific format

---

### conan (conan.py)

**Files Involved:**

- Primary: `conanfile.py`
- Data: `conandata.yml`

**Assembly Logic:**

```text
if resource.name == 'conanfile.py':
    # Find sibling conandata.yml
    # Merge external source data
```

**Key Features:**

- Python-based recipe parsing
- External source tracking
- C++ package manager specific

---

### debian_copyright (debian_copyright.py)

**Files Involved:**

- Copyright: `debian/copyright` (machine-readable format)

**Assembly Logic:**

```text
# Standalone parsing (no multi-file assembly)
# Parse Debian copyright file
# Extract license and copyright information
```

**Key Features:**

- Machine-readable format
- Can be in source or installed package
- Debian-specific copyright format

---

### about (about.py)

**Files Involved:**

- Metadata: `*.ABOUT`

**Assembly Logic:**

```text
# Standalone parsing (no multi-file assembly)
# Parse AboutCode ABOUT file
```

**Key Features:**

- AboutCode toolkit format
- Standalone metadata files

---

### build (build.py)

**Files Involved:**

- Build scripts: `configure`, `BUILD`, etc.

**Assembly Logic:**

```text
# Non-assembling (uses NonAssemblableDatafileHandler)
# Parse build scripts for informational data only
# No package creation
```

**Key Features:**

- Informational only
- No assembly logic
- Build system detection

---

## Assembly Helper Methods

### assemble_from_many()

```python
@classmethod
def assemble_from_many(
    cls,
    pkgdata_resources,  # List of (PackageData, Resource) tuples
    codebase,
    package_adder=add_to_package,
    ignore_name_check=False,
    parent_resource=None,
):
    """
    Combine multiple PackageData into single Package.
    
    Order matters:
    - First item creates Package
    - Subsequent items update it
    - Packages yielded before Dependencies
    """
```

### assemble_from_many_datafiles()

```python
@classmethod
def assemble_from_many_datafiles(
    cls,
    datafile_names,  # List of filenames to find
    resource,        # Starting resource
    codebase,
    package_adder=add_to_package,
):
    """
    Find multiple datafiles and assemble them.
    """
```

### assemble_from_many_datafiles_in_directory()

```python
@classmethod
def assemble_from_many_datafiles_in_directory(
    cls,
    datafile_names,  # List of filenames to find
    resource,        # Directory resource
    codebase,
    package_adder=add_to_package,
):
    """
    Find multiple datafiles in directory and assemble them.
    """
```

---

## Scope Terminology by Ecosystem

### npm

- `dependencies` - Runtime dependencies
- `devDependencies` - Development-only
- `peerDependencies` - Peer dependencies
- `optionalDependencies` - Optional runtime
- `bundledDependencies` - Bundled with package

### cargo

- `dependencies` - Runtime dependencies
- `dev-dependencies` - Development-only
- `build-dependencies` - Build-time only

### maven

- `compile` - Compile and runtime (default)
- `test` - Test-time only
- `provided` - Provided by runtime
- `runtime` - Runtime only
- `system` - System-provided

### python (pypi)

- `None` - Runtime dependencies
- `<extra_name>` - Optional dependency groups
- `dev` - Development dependencies (Poetry)

### golang

- `direct` - Direct dependencies
- `indirect` - Transitive dependencies

### ruby (gems)

- `runtime` - Runtime dependencies
- `development` - Development-only

---

## Key Implementation Patterns

### Pattern: Sibling Finding

```python
# Find sibling files in same directory
siblings = resource.siblings(codebase)
lockfile = [r for r in siblings if r.name == 'package-lock.json']
```

### Pattern: Parent Tree Assignment

```python
# Assign package to parent directory tree
parent = resource.parent(codebase)
cls.assign_package_to_resources(package, parent, codebase, package_adder)
```

### Pattern: Conditional Package Creation

```python
# Only create package if PURL exists
if package_data.purl:
    package = Package.from_package_data(package_data, datafile_path)
    yield package
else:
    # No package, only yield dependencies
    package_uid = None
```

### Pattern: Dependency Merging

```python
# Merge dependencies from multiple sources
dependencies = []
for datafile_data in [manifest_data, lockfile_data]:
    dependencies.extend(datafile_data.dependencies)
```

---

## Testing Considerations

### Golden Tests

- Compare Python ScanCode output with Rust implementation
- Test data in `testdata/<ecosystem>/`
- Verify package creation, dependencies, and resource assignment

### Edge Cases

- Missing lockfiles (manifest-only)
- Missing manifests (lockfile-only)
- Workspace configurations
- Nested directory structures
- Archive extraction
- Database parsing

### Scope Preservation

- Verify native scope terminology is preserved
- Test scope-specific dependency handling
- Validate optional/runtime/dev distinctions
