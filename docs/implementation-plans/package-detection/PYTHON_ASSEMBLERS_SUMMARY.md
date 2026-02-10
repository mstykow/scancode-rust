# Python ScanCode Assembly Logic - Complete Reference

## Overview

Python ScanCode implements assembly logic for 20 package ecosystems. Assembly is the process of combining data from multiple related manifest/lockfile datafiles into a single Package with its dependencies.

## Assembly Framework (models.py)

### Base Class: DatafileHandler

- **assemble()**: Main method that yields Package, Dependency, and Resource objects
- **assign_package_to_resources()**: Associates package to file tree
- **assign_package_to_parent_tree()**: Associates package to parent directory tree
- **assemble_from_many()**: Combines multiple PackageData into single Package
- **assemble_from_many_datafiles()**: Helper for multi-file assembly
- **assemble_from_many_datafiles_in_directory()**: Helper for directory-based assembly

### NonAssemblableDatafileHandler

- Default implementation that does nothing (for parsers without assembly)

## Ecosystems with Assembly Support (20 total)

### 1. **npm** (npm.py)

- **Pattern**: Sibling-merge + parent-child
- **Files**: package.json + lockfiles (package-lock.json, npm-shrinkwrap.json, .package-lock.json)
- **Logic**:
  - If resource is package.json: create package, walk tree (skip node_modules), assign resources
  - If resource is lockfile: find sibling package.json, use it for package creation
  - If no package.json: yield dependencies only from lockfile
- **Assembly Type**: Multi-file (manifest + lockfile)

### 2. **cargo** (cargo.py)

- **Pattern**: Workspace support + sibling-merge
- **Files**: Cargo.toml + Cargo.lock
- **Logic**:
  - Support cargo workspaces with multiple packages
  - Merge workspace-level package data into member packages
  - Handle workspace members via glob patterns
  - Copy license data from workspace to members
- **Assembly Type**: Multi-file (manifest + lockfile + workspace)

### 3. **maven** (maven.py)

- **Pattern**: Nested sibling-merge
- **Files**: pom.xml + MANIFEST.MF (in META-INF/)
- **Logic**:
  - Find pom.xml in META-INF/maven/**/pom.xml
  - Find MANIFEST.MF in META-INF/MANIFEST.MF
  - Create package from pom.xml, update from MANIFEST.MF
  - Order matters: pom.xml first, then MANIFEST.MF
- **Assembly Type**: Multi-file (nested in JAR structure)

### 4. **cocoapods** (cocoapods.py)

- **Pattern**: Sibling-merge
- **Files**: .podspec + Podfile.lock
- **Logic**:
  - Find sibling Podfile.lock when processing .podspec
  - Merge dependency data from lockfile
  - Handle spec repositories and checksums
- **Assembly Type**: Multi-file (manifest + lockfile)

### 5. **phpcomposer** (phpcomposer.py)

- **Pattern**: Sibling-merge
- **Files**: composer.json + composer.lock
- **Logic**:
  - Find sibling composer.lock when processing composer.json
  - Merge locked dependency versions
- **Assembly Type**: Multi-file (manifest + lockfile)

### 6. **rubygems** (rubygems.py)

- **Pattern**: Multi-file + extracted archive
- **Files**: .gemspec + Gemfile.lock + extracted gem metadata
- **Logic**:
  - Handle .gemspec files
  - Handle Gemfile.lock for dependencies
  - Handle extracted gem metadata.gz
  - Multiple handler classes for different gem formats
- **Assembly Type**: Multi-file (manifest + lockfile + archive)

### 7. **golang** (golang.py)

- **Pattern**: Sibling-merge
- **Files**: go.mod + go.sum
- **Logic**:
  - Always use go.mod first, then go.sum
  - Merge checksum data from go.sum
- **Assembly Type**: Multi-file (manifest + lockfile)

### 8. **pubspec** (pubspec.py)

- **Pattern**: Sibling-merge
- **Files**: pubspec.yaml + pubspec.lock
- **Logic**:
  - Find sibling pubspec.lock when processing pubspec.yaml
  - Merge locked dependency versions
- **Assembly Type**: Multi-file (manifest + lockfile)

### 9. **swift** (swift.py)

- **Pattern**: Multi-file merge
- **Files**: Package.swift + Package.resolved + swift-show-dependencies.deplock
- **Logic**:
  - Combine Package.swift manifest with resolved dependencies
  - Handle DepLock format for dependency graphs
- **Assembly Type**: Multi-file (manifest + resolved + dependency graph)

### 10. **conda** (conda.py)

- **Pattern**: Directory-based + multi-file
- **Files**: conda-meta/*.json + environment.yaml
- **Logic**:
  - Scan conda-meta directory for package metadata
  - Merge with environment.yaml if present
  - Handle conda installation structure
- **Assembly Type**: Multi-file (directory-based metadata)

### 11. **debian** (debian.py)

- **Pattern**: Archive extraction + metadata merge
- **Files**: .deb + .debian.tar.xz + .orig.tar.gz
- **Logic**:
  - Extract and parse .deb archives
  - Merge metadata from .debian.tar.xz
  - Handle source package tarballs
- **Assembly Type**: Multi-file (archive-based)

### 12. **alpine** (alpine.py)

- **Pattern**: Archive + database + build script
- **Files**: .apk + installed database + APKBUILD
- **Logic**:
  - Parse .apk archives
  - Merge with installed package database
  - Handle APKBUILD build scripts
- **Assembly Type**: Multi-file (archive + database)

### 13. **rpm** (rpm.py)

- **Pattern**: Database-based
- **Files**: RPM database files (Packages.db, Packages.sqlite)
- **Logic**:
  - Parse RPM database files
  - Extract package metadata from database
- **Assembly Type**: Database (system packages)

### 14. **pypi** (pypi.py)

- **Pattern**: Multi-file + extracted layout
- **Files**: PKG-INFO + setup.py + setup.cfg + pyproject.toml
- **Logic**:
  - Handle multiple Python package metadata formats
  - Merge data from multiple sources
  - Support egg-info and editable installations
- **Assembly Type**: Multi-file (multiple metadata formats)

### 15. **chef** (chef.py)

- **Pattern**: Sibling-merge
- **Files**: metadata.rb + metadata.json
- **Logic**:
  - Find sibling metadata.json when processing metadata.rb
  - Merge metadata from both files
- **Assembly Type**: Multi-file (manifest + JSON metadata)

### 16. **conan** (conan.py)

- **Pattern**: Sibling-merge
- **Files**: conanfile.py + conandata.yml
- **Logic**:
  - Find sibling conandata.yml when processing conanfile.py
  - Merge external source data
- **Assembly Type**: Multi-file (manifest + data file)

### 17. **debian_copyright** (debian_copyright.py)

- **Pattern**: Standalone + merge
- **Files**: debian/copyright (machine-readable format)
- **Logic**:
  - Parse Debian copyright files
  - Can be in source or installed package
- **Assembly Type**: Single-file (copyright metadata)

### 18. **about** (about.py)

- **Pattern**: Standalone
- **Files**: *.ABOUT files
- **Logic**:
  - Parse AboutCode ABOUT files
  - No multi-file assembly
- **Assembly Type**: Single-file

### 19. **build** (build.py)

- **Pattern**: Non-assembling
- **Files**: configure, BUILD, etc.
- **Logic**:
  - Parse build scripts and configuration
  - No assembly (NonAssemblableDatafileHandler)
- **Assembly Type**: None (informational only)

### 20. **phpcomposer** (already listed above)

## Assembly Patterns Summary

### Pattern 1: Sibling-Merge (Most Common)

Used by: npm, cargo, cocoapods, phpcomposer, golang, pubspec, chef, conan

- Find sibling files in same directory
- Merge data from manifest + lockfile
- Create single Package with combined data

### Pattern 2: Nested Sibling-Merge

Used by: maven

- Find nested sibling files in specific directory structure
- Merge data from multiple levels
- Example: META-INF/MANIFEST.MF + META-INF/maven/**/pom.xml

### Pattern 3: Directory-Based

Used by: conda, alpine, debian

- Scan directory for multiple related files
- Merge all files in directory into single Package
- Example: conda-meta/*.json

### Pattern 4: Archive Extraction

Used by: debian, alpine, rubygems

- Extract archive files
- Parse metadata from extracted contents
- Merge with other metadata sources

### Pattern 5: Database-Based

Used by: rpm, alpine (installed database)

- Parse system package databases
- Extract package metadata from database
- No multi-file assembly needed

### Pattern 6: Multi-Format

Used by: pypi, rubygems

- Support multiple metadata file formats
- Merge data from whichever format is present
- Handle different installation layouts

## Key Assembly Concepts

### 1. Package Creation

- Created from primary manifest file (package.json, Cargo.toml, etc.)
- Must have PURL (Package URL) to create Package object
- If no PURL, only yield dependencies

### 2. Dependency Merging

- Lockfiles provide pinned/resolved versions
- Manifest files provide version requirements
- Both are merged into single dependency list

### 3. Resource Assignment

- Assign package to all files in its tree
- Skip certain directories (e.g., node_modules for npm)
- Use parent tree assignment for some ecosystems

### 4. Workspace Support

- Cargo: Multiple packages in single workspace
- Copy workspace-level metadata to members
- Handle glob patterns for member paths

### 5. Scope Handling

- Each ecosystem has native scope terminology
- npm: dependencies, devDependencies, peerDependencies, optionalDependencies
- cargo: dependencies, dev-dependencies, build-dependencies
- Preserved as-is for semantic fidelity

## Ecosystems WITHOUT Assembly Support

These ecosystems have parsers but NO custom assembly logic (use default):

- bower
- cran
- freebsd
- gemfile_lock
- godeps
- haxe
- jar_manifest
- msi
- nevra
- nuget
- opam
- pyrpm
- readme
- spec
- win_pe
- win_reg
- windows

These use the default DatafileHandler.assemble() which:

- Creates package from single datafile only
- Does not combine multiple files
- Assigns package to datafile resource only

## Implementation Notes

### Order Matters

- In assemble_from_many(), order of PackageData items is critical
- First item creates the Package
- Subsequent items update it
- Packages must be yielded before Dependencies

### Sibling Finding

- Uses resource.siblings(codebase) to find related files
- Checks by filename (e.g., 'package.json')
- Requires codebase context

### Parent Tree Assignment

- Some ecosystems assign package to parent directory tree
- Others assign to specific files only
- Affects which files are marked as "for_packages"

### Lockfile Handling

- Lockfiles are optional (manifest can exist alone)
- If lockfile exists, dependencies are merged
- If only lockfile exists, dependencies are yielded without package
