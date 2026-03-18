# Arch Parser: Source Metadata, Binary Metadata, and Split-Package Support

## Summary

Rust now parses the main Arch Linux metadata surfaces that were previously missing:

- `.SRCINFO`
- legacy `.AURINFO`
- standalone `.PKGINFO`

This goes beyond the current Python reference state, where upstream work is focused on `.SRCINFO` only and does not yet cover Arch `.PKGINFO` or legacy `.AURINFO` handling.

## Python Reference Status

The current upstream Arch work is an open `.SRCINFO` parser PR only.

That upstream work:

- parses `pkgbase` + `pkgname` sections
- extracts basic metadata and dependency fields
- handles architecture-suffixed dependency keys

But it still leaves important gaps for Provenant users:

- no `.PKGINFO` parser
- no legacy `.AURINFO` compatibility
- no local guarantee that split-package merge semantics preserve both pkgbase and pkgname repeated fields together

## Rust Improvements

### 1. Full Arch metadata surface coverage

Rust now supports both the source and package metadata layers that matter for Arch scans:

- `.SRCINFO` for current source package metadata
- `.AURINFO` as a legacy alias of the same source metadata family
- `.PKGINFO` for installed/binary package metadata

This means Arch package metadata is no longer limited to only one side of the packaging workflow.

### 2. Correct `alpm` package identity and purls

Rust introduces real `alpm` package typing and emits Arch package purls in the registered purl ecosystem instead of treating Arch data as generic distro metadata.

The implementation uses:

- package type `alpm`
- namespace `arch`
- `arch=` qualifiers when a single architecture is known for a package identity

### 3. Split-package inheritance with repeated-field preservation

`.SRCINFO` supports a `pkgbase` section plus one or more `pkgname` sections.

Rust now:

- inherits shared `pkgbase` metadata into each produced package
- preserves repeated base fields such as `makedepends`, `source`, and checksum families
- lets package-specific scalar fields such as `pkgdesc` override base values where appropriate
- keeps package-specific repeated fields additive instead of clobbering shared base arrays

That is a safer model for split packages than a naive `pkgbase.copy(); pkg.update(...)` strategy.

### 4. Architecture-specific dependency scope preservation

Rust preserves architecture-targeted dependency keys directly in dependency scope fields, such as:

- `depends_x86_64`
- `depends_aarch64`

This keeps the original Arch metadata semantics visible instead of flattening them away.

### 5. `.PKGINFO` dependency and relation coverage

Rust now extracts `.PKGINFO` identity and package metadata including:

- `pkgname`, `pkgbase`, `pkgver`, `pkgdesc`, `url`, `arch`
- `packager`
- runtime/build/check/optional dependency families
- `provides`, `conflict`, `replaces`
- build metadata such as `builddate` and `size`

This gives Arch package metadata parity on the installed/binary side that upstream does not currently provide.

## Primary Areas Affected

- Arch source package metadata
- Arch installed/binary package metadata
- Arch split-package handling
- Arch dependency scope fidelity
- `alpm` purl generation

## Verification

This improvement is covered by:

- unit tests for `.SRCINFO`, split packages, arch-specific dependencies, legacy `.AURINFO`, and `.PKGINFO`
- parser golden coverage for a basic `.SRCINFO` case
- parser golden coverage for a basic `.PKGINFO` case
