# Debian Parser: Beyond-Parity Improvements

## Summary

The Debian `.deb` parser in scancode-rust implements missing direct-archive behavior beyond the Python reference implementation:

- **✨ New Feature**: direct `.deb` archive introspection from `control.tar.gz` and `control.tar.xz`
- **🔍 Enhanced Extraction**: direct `.deb` scans now also read package-matching `/usr/share/doc/<pkg>/copyright` metadata from `data.tar.gz` and `data.tar.xz`
- **🐛 Bug Fix**: Debian DEP-5 top-level `Files: *` license declarations now emit a parser-level primary `license_detection` with preserved header casing and absolute line numbers
- **🐛 Bug Fix**: Debian DEP-5 `License:` header paragraphs now emit detections in file order with preserved header casing and absolute line numbers, including standalone bottom license paragraphs
- **🐛 Bug Fix**: Debian DEP-5 header-paragraph licenses can now become the primary detection when the `Files: *` paragraph has no usable `License:` field
- **🔍 Enhanced Extraction**: installed Debian package metadata from `status` / `status.d` now integrates matching `info/*.list` and `info/*.md5sums` sidecars during rootfs/container scans
- **⚡ Performance**: Debian copyright paragraph parsing now avoids the extra paragraph reparse/rescan cycle on large DEP-5 files

## Improvement: .deb Archive Introspection (New Feature)

### Python Implementation (TODO)

**Location**: `reference/scancode-toolkit/src/packagedcode/debian.py`

**Comment** (line 97-105):

```python
class DebianDebHandler(DatafileHandler):
    datasource_id = 'debian_deb'
    path_patterns = ('*.deb',)
    default_package_type = 'deb'
    description = 'Debian binary package archive'
    documentation_url = 'https://wiki.debian.org/Packaging'

    @classmethod
    def assemble(cls, package_data, resource, codebase, package_adder):
        # TODO: introspect archive
        pass
```

**Current Python Behavior**: The `.deb` parser exists but only handles the archive **after** extraction. It does not introspect the archive directly, so direct `.deb` scans miss both control metadata and embedded package copyright metadata.

### Our Rust Implementation (Complete)

**Location**: `src/parsers/debian.rs`

**Implementation**: direct `.deb` archive extraction and metadata parsing:

```rust
pub fn extract_package_data(path: &Path) -> PackageData {
    // 1. Open .deb as AR archive
    let deb_file = File::open(path)?;
    let mut archive = Archive::new(deb_file);

    // 2. Find control.tar.gz (or control.tar.xz)
    let control_tar = archive
        .entries()?
        .find(|entry| {
            entry.header()
                .identifier()
                .starts_with(b"control.tar")
        })?;

    // 3. Extract control.tar.gz to temp directory
    let temp_dir = TempDir::new()?;
    let decoder = GzDecoder::new(control_tar);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(temp_dir.path())?;

    // 4. Parse control file
    let control_path = temp_dir.path().join("control");
    let content = fs::read_to_string(control_path)?;

    // 5. Extract metadata using RFC822 parser
    let fields = parse_rfc822(&content);

    PackageData {
        name: fields.get("Package").cloned(),
        version: fields.get("Version").cloned(),
        description: fields.get("Description").cloned(),
        homepage_url: fields.get("Homepage").cloned(),
        dependencies: parse_dependencies(fields.get("Depends")),
        // ... other fields from control file
    }
}
```

### Additional embedded copyright support

After parsing `control.tar.*`, Rust now also inspects `data.tar.gz` / `data.tar.xz` for package copyright metadata under paths like:

```text
./usr/share/doc/<package>/copyright
```

If a path matches the current package name, that file is parsed with the existing Debian copyright parser and its extracted license statement / copyright-holder parties are merged back onto the `.deb` package.

### Architecture

**.deb File Structure**:

```text
example.deb (AR archive)
├── debian-binary          # Version number (e.g., "2.0\n")
├── control.tar.gz         # Package metadata (what we extract)
│   ├── control            # Main metadata file (RFC822 format)
│   ├── md5sums            # File checksums
│   ├── preinst            # Pre-installation script
│   ├── postinst           # Post-installation script
│   └── ...                # Other control files
└── data.tar.xz            # Actual files to install
    ├── usr/bin/example
    └── ...
```

**We extract**:

- `control.tar.gz` or `control.tar.xz` → parse `control` file → populate PackageData
- package-matching copyright file from `data.tar.gz` or `data.tar.xz` → parse Debian copyright metadata → merge license/copyright details onto the same package

### Example Output

**Before (Python)**:

```json
{
  "datasource_id": "debian_deb",
  "type": "deb"
  // No actual metadata extraction
}
```

**After (Rust)**:

```json
{
  "name": "apt",
  "version": "2.0.2",
  "namespace": "debian",
  "description": "commandline package manager\n This package provides commandline tools for searching and\n managing as well as querying information about packages\n as a low-level access to all features of the libapt-pkg library.",
  "homepage_url": "https://wiki.debian.org/Apt",
  "dependencies": [
    {
      "purl": "pkg:deb/debian/libc6",
      "extracted_requirement": "libc6 >= 2.15",
      "scope": "dependencies",
      "is_runtime": true
    },
    {
      "purl": "pkg:deb/debian/libapt-pkg6.0",
      "extracted_requirement": "libapt-pkg6.0 >= 2.0.2",
      "scope": "dependencies",
      "is_runtime": true
    }
  ],
  "sha256": "e3b5c4a...", // Archive checksum
  "size": 1234567
}
```

### Verification

**Test Cases**: `.deb` archive introspection

```rust
#[test]
fn test_extract_deb_archive_with_control_tar_xz() {
    // synthetic .deb with control.tar.xz
}

#[test]
fn test_extract_deb_archive_collects_embedded_copyright_metadata() {
    // synthetic .deb with data.tar.gz usr/share/doc/<pkg>/copyright
}

#[test]
fn test_extract_deb_archive_collects_embedded_copyright_from_data_tar_xz() {
    // synthetic .deb with data.tar.xz usr/share/doc/<pkg>/copyright
}

#[test]
fn test_debian_deb_introspection() {
    let result = DebianDebParser::extract_package_data(
        Path::new("testdata/debian/apt_2.0.2_amd64.deb")
    );

    assert_eq!(result.name, Some("apt".to_string()));
    assert_eq!(result.version, Some("2.0.2".to_string()));
    assert_eq!(result.namespace, Some("debian".to_string()));

    // Verify dependencies were extracted
    let deps = result.dependencies;
    assert!(deps.len() > 0, "Should extract dependencies from control file");

    // Verify dependency with version constraint
    let libc_dep = deps.iter()
        .find(|d| d.purl.as_ref().unwrap().contains("libc6"))
        .expect("Should find libc6 dependency");

    assert!(
        libc_dep.extracted_requirement
            .as_ref()
            .unwrap()
            .contains(">="),
        "Should parse version constraint"
    );
}
```

**Result**: ✅ Direct `.deb` control metadata extraction, xz control support, and package-matching embedded copyright metadata merging

## Improvement: Primary DEP-5 License Detection

**Area**: `debian/copyright` DEP-5 parsing  
**Issue**: local `#183`, upstream `aboutcode-org/scancode-toolkit#3424`

Rust now emits a parser-level primary `license_detection` for the top `Files: *` `License:` paragraph in DEP-5 Debian copyright files.

What this specifically adds:

- a non-empty `license_detections` entry for the primary Debian copyright license
- preserved `matched_text` casing from the original `License:` header line
- absolute file `start_line` / `end_line` instead of paragraph-relative numbering for that primary detection

This is intentionally narrow:

- it covers the primary `Files: *` paragraph only
- it does not claim full paragraph-by-paragraph Debian copyright detection parity
- it does not close the rest of the Debian copyright issue cluster on its own

Proof point:

- the `bsdutils` upstream Debian copyright fixture now produces a primary detection with matched text `License: GPL-2+` at absolute line `47`

## Improvement: DEP-5 License Header Paragraph Detections

**Area**: `debian/copyright` DEP-5 paragraph detection  
**Issues**: local `#176`, `#178`, `#179`, `#180`

Rust now emits parser-level detections for **all** DEP-5 `License:` header paragraphs, not just the primary `Files: *` paragraph.

What this specifically adds:

- detection order now follows file order
- `matched_text` preserves the original `License:` header casing from the file
- line numbers are absolute file line numbers, not paragraph-relative offsets
- standalone bottom `License:` paragraphs are no longer dropped when they have their own paragraph in the file

This is still intentionally narrow:

- it is based on explicit `License:` header lines
- it does not yet do full paragraph-body license text detection parity
- it does not close the remaining Debian copyright issue `#181`

Proof points:

- local fixture `testdata/debian/copyright/copyright` now emits ordered detections at lines `11`, `15`, `19`, `23`, and `25`
- upstream `clamav` fixture now includes the bottom standalone `License: Zlib` paragraph at absolute line `732`

## Improvement: Header-Paragraph Primary Fallback

**Area**: `debian/copyright` DEP-5 primary detection fallback  
**Issue**: local `#181`, upstream `aboutcode-org/scancode-toolkit#2977`

Rust now falls back to the DEP-5 header paragraph license as the primary detection when the `Files: *` paragraph has no usable `License:` field.

What this specifically adds:

- nameless/blank `Files: *` license fields no longer leave the package without any primary detection
- the header-paragraph `License:` line is preserved as the primary `matched_text`
- the primary line number remains absolute in the source file

This stays intentionally narrow:

- it handles the header-paragraph fallback only
- it does not attempt full paragraph-body license-text parsing parity
- it does not broaden into Debian control/container work

Proof point:

- upstream crafted fixture `test_license_nameless` now promotes `License: LGPL-3+ or GPL-2+` from line `8` as the primary detection when the later `Files: *` paragraph has a blank `License:` field

## Improvement: Installed Debian Metadata Integration

**Area**: installed package metadata from `var/lib/dpkg/status`, `status.d`, and `info/*`  
**Issues**: local `#185`, `#186`

Rust now integrates Debian installed-package sidecar metadata into the assembled installed package view.

What this specifically adds:

- installed packages parsed from `var/lib/dpkg/status`
- distroless/container-style installed packages parsed from `var/lib/dpkg/status.d/*`
- sidecar file references from matching `var/lib/dpkg/info/*.list`
- sidecar checksum/file-reference details from matching `var/lib/dpkg/info/*.md5sums`
- safe matching for Ubuntu-family namespaces and multiarch sidecar filenames

This means installed Debian packages in rootfs/container scans can now:

- keep their dependency info from the control-like status file
- attach actual installed files back to the assembled package
- attach md5-backed file references from matching sidecars

This is still intentionally narrow:

- it does not attempt performance work (`#177`)
- it does not broaden into non-Debian container inventory
- it is explicitly about Debian installed metadata integration, not generic scanner refactors

Proof points:

- unit resolver coverage for regular `status` + `info/*.list` / `*.md5sums`
- explicit Ubuntu namespace matching regression
- explicit multiarch isolation regression
- scan-level `status.d` regression proving dependency retention and installed-file assignment together

## Improvement: Debian Copyright Parsing Hot-Path Optimization

**Area**: `debian/copyright` parser performance  
**Issue**: local `#177`, upstream `aboutcode-org/scancode-toolkit#2642`

Rust now avoids the most obvious repeated work inside DEP-5 paragraph processing.

Before this slice, each Debian copyright paragraph was:

1. accumulated as raw lines,
2. joined into a new string,
3. reparsed through the generic RFC822 paragraph parser,
4. cloned again for later rescanning, and
5. rescanned to rediscover the `License:` header line.

Now the Debian parser finalizes each paragraph in one local pass:

- build the paragraph headers directly,
- capture the first `License:` header line while walking those lines,
- keep the same parser-visible semantics for primary/other detections,
- and avoid the extra per-paragraph reparse/rescan cycle.

This is intentionally narrow:

- it is a Debian-local optimization only,
- it does not refactor the shared `rfc822` parser,
- it does not attempt broader scanner or license-engine performance work,
- and it leaves Debian copyright semantics unchanged.

Proof points:

- existing Debian parser tests and Debian goldens still pass unchanged
- an ignored large-fixture performance probe now exercises the real `bsdutils` and `clamav` DEP-5 files repeatedly for local before/after measurement

## Implementation Details

### Archive Format Handling

**.deb files are AR archives** containing compressed tarballs:

1. **Outer layer**: AR archive (Unix archiver format, like static libraries `.a`)
2. **Control layer**: `control.tar.gz` or `control.tar.xz` (metadata)
3. **Data layer**: `data.tar.xz` or `data.tar.gz` (installed files)

**We use**:

- `ar` crate for AR archive reading
- `flate2` for gzip decompression
- `liblzma` for xz decompression
- `tar` crate for tarball extraction
- `tempfile` for temporary directory

### Control File Format

The `control` file uses **RFC822 format** (same as Debian package databases):

```text
Package: apt
Version: 2.0.2
Architecture: amd64
Depends: libc6 (>= 2.15), libapt-pkg6.0 (>= 2.0.2)
Description: commandline package manager
 This package provides commandline tools for searching and
 managing as well as querying information about packages
 as a low-level access to all features of the libapt-pkg library.
Homepage: https://wiki.debian.org/Apt
```

**Key features**:

- Colon-separated key-value pairs
- Multi-line values indented with space
- Comma-separated lists (dependencies)
- Version constraints in parentheses: `package (>= version)`

### Dependency Parsing

Debian dependencies use complex syntax:

```text
Depends: pkg1 (>= 1.0), pkg2 | pkg3, pkg4 [amd64]
```

**Operators**:

- `>=` - Greater than or equal
- `<=` - Less than or equal
- `=` - Exactly equal
- `>>` - Strictly greater
- `<<` - Strictly less

**Features**:

- **Alternatives**: `pkg2 | pkg3` (either package satisfies)
- **Architecture-specific**: `pkg4 [amd64]` (only on amd64)

### Security

Archive extraction includes safety checks:

- **Size limits**: Max 1GB uncompressed
- **Compression ratio**: Max 100:1 (zip bomb protection)
- **Path traversal**: Block `../` in tar entries
- **Temp directory cleanup**: Automatic via `TempDir`

## Impact

### SBOM Completeness

**Critical for**: Scanning Debian/Ubuntu systems, Docker images based on Debian.

Without .deb introspection:

- ❌ Must extract .deb files externally before scanning
- ❌ Can't scan .deb files directly
- ❌ Incomplete SBOM for package files

With .deb introspection:

- ✅ Scan .deb files directly
- ✅ No external extraction needed
- ✅ Complete metadata from archive

### Use Cases Enabled

1. **Package Repository Scanning**: Scan entire Debian/Ubuntu repos
2. **Docker Image Analysis**: Extract metadata from .deb layers
3. **Offline Analysis**: Analyze .deb files without installation
4. **Supply Chain Security**: Verify package contents match metadata

## Testing

### Unit Tests

- `test_debian_deb_introspection()` - Verifies control file extraction
- `test_debian_deb_dependencies()` - Tests dependency parsing
- `test_debian_deb_multiline_description()` - Handles RFC822 multi-line values

### Golden Tests

- Multiple passing tests for Debian .deb files
- Validates against Python reference (where Python has control files pre-extracted)

### Test Data

- Real .deb packages: `testdata/debian/*.deb`
- Covers: apt, dpkg, coreutils, systemd

## Python vs Rust: Why Rust Can Do This

### Python Challenge

Python's `debian_inspector` library:

- Primarily designed for extracted control files
- AR archive support is limited
- TODO comment suggests this was planned but not implemented

**Comment from Python code** (line 99):

> "TODO: introspect archive"

### Rust Advantage

Rust ecosystem provides excellent archive handling:

- `ar` crate for AR archives (used in linkers, well-tested)
- `tar` crate for tarballs (production-ready)
- `flate2` for gzip/zlib (bindings to C library)
- Memory safety prevents common extraction vulnerabilities

**We leverage these libraries to deliver a complete feature.**

## Related Parsers

Our Debian parser suite includes multiple parsers (all complete):

1. **DebianControlParser** - Control files (manifests)
2. **DebianDebParser** - .deb archives (THIS IMPROVEMENT)
3. **DebianCopyrightParser** - DEP-5 copyright files
4. **DebianDscParser** - Source package metadata
5. **DebianInstalledParser** - dpkg status database

**This improvement completes the `.deb` archive parser in that suite.**

## References

### Python Reference Issues

- TODO: Archive introspection not implemented

### Debian Documentation

- [Debian Binary Package Format](https://wiki.debian.org/Packaging)
- [Control File Format](https://www.debian.org/doc/debian-policy/ch-controlfields.html)
- [AR Archive Format](<https://en.wikipedia.org/wiki/Ar_(Unix)>)

### Rust Crates Used

- [`ar`](https://docs.rs/ar/) - AR archive reader
- [`tar`](https://docs.rs/tar/) - Tarball extraction
- [`flate2`](https://docs.rs/flate2/) - Gzip decompression
- [`tempfile`](https://docs.rs/tempfile/) - Temporary directories

### Our Implementation

## Status

- ✅ **Archive introspection**: Complete, validated, production-ready
- ✅ **Control file parsing**: RFC822 format handled correctly
- ✅ **Dependency extraction**: Version constraints and alternatives supported
- ✅ **Security**: Archive safety checks implemented
- ✅ **Documentation**: Complete
