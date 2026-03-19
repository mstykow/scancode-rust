# Debian Parser: Beyond-Parity Improvements

## Summary

The Debian parser in Provenant goes beyond the Python reference in four main ways:

- **✨ New Feature**: direct `.deb` archive introspection from `control.tar.*`
- **🔍 Enhanced Extraction**: package-matching copyright metadata can also be recovered from `data.tar.*`
- **🐛 Bug Fix**: DEP-5 primary and header-paragraph license detections now preserve source order, header text, and absolute line numbers more coherently
- **🔍 Enhanced Extraction**: installed Debian package metadata can integrate matching `status`, `status.d`, `info/*.list`, and `info/*.md5sums` sidecars during rootfs and container scans

## Direct `.deb` archive introspection

The Python reference recognizes Debian package archives, but it does not treat the archive itself as the full metadata source. In practice that means direct `.deb` scans can miss control metadata unless the archive has already been unpacked.

Rust reads the Debian archive directly. It extracts package metadata from `control.tar.gz` or `control.tar.xz`, then uses the existing Debian metadata machinery to populate the package record without requiring a separate unpack step.

### Embedded copyright recovery

After reading control metadata, Rust can also inspect `data.tar.gz` or `data.tar.xz` for package-matching copyright files under paths such as:

```text
./usr/share/doc/<package>/copyright
```

When the embedded copyright file matches the current package, the extracted copyright and license information is merged back onto the same package. That keeps archive scans closer to the data users expect from an installed Debian package.

## DEP-5 license detection improvements

Rust improves several Debian copyright behaviors that were previously thin or inconsistent:

- the primary `Files: *` paragraph can emit a parser-level primary detection
- `License:` header paragraphs are emitted in file order rather than being partially dropped or reordered
- preserved `matched_text` comes from the source header instead of being normalized away
- line numbers stay absolute to the original file
- if the top `Files: *` paragraph has no usable `License:` field, a later header paragraph can supply the primary detection

These changes stay intentionally narrow. They improve the package-visible behavior around explicit DEP-5 headers without claiming full paragraph-body license-text parity.

## Installed Debian metadata integration

Rust also improves the installed-package view for Debian and Debian-derived root filesystems.

Package records parsed from `var/lib/dpkg/status` or `var/lib/dpkg/status.d/*` can now integrate matching sidecars from `var/lib/dpkg/info/`, including:

- installed file lists from `*.list`
- checksum-backed file references from `*.md5sums`
- matching behavior that stays safe across Debian-family namespace differences and multiarch filenames

This lets rootfs and container scans keep dependency information from the status database while also attaching the actual installed files that belong to the package.

## Less repeated DEP-5 parsing work

Rust also removes a Debian-local source of repeated DEP-5 work by finalizing each paragraph in one local pass instead of reparsing and rescanning it multiple times.

The important part for users is behavioral stability. The parser keeps the same Debian-facing semantics while doing less repeated work on large copyright files.

## Why this matters

- **Direct archive scans become more useful**: `.deb` files can carry their own package metadata and embedded copyright data
- **Debian copyright output is clearer**: primary and secondary detections line up better with the source file the user actually scanned
- **Rootfs and container results are richer**: installed Debian packages can recover both metadata and file ownership from their sidecars
- **Debian-specific work stays local**: the improvements tighten Debian behavior without requiring broad scanner-wide special cases

## References

- [Debian Binary Packages](https://www.debian.org/doc/debian-policy/ch-binary.html)
- [Debian control file fields](https://www.debian.org/doc/debian-policy/ch-controlfields.html)
