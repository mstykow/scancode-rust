# Python Parser: Manifest Metadata, Installed Provenance, Dunder Fallbacks, and PyPI JSON

## Summary

**🐛 Bug Fix + ✨ New Feature + 🔍 Enhanced Extraction**: Rust now extracts richer Python manifest metadata, resolves a narrow class of imported sibling dunder values for `setup.py`, preserves more installed and source-package provenance, supports saved `pypi.json` payloads, recovers RFC822 dependency metadata that was previously missing, and can parse Python source distribution archives directly.

## What changed

### 1. Richer `setup.cfg` metadata

Rust now extracts more of the metadata that real `setup.cfg` files already carry, including descriptions, maintainer details, keywords, `python_requires`, and project URLs.

### 2. `setup.py` `project_urls` from `OrderedDict`

Rust now handles `project_urls=OrderedDict([...])` in addition to plain dict literals, which closes a common gap in static `setup.py` parsing.

### 3. Imported sibling dunder fallback for `setup.py`

When AST parsing leaves plain dunder metadata unresolved, Rust can perform a narrow fallback against imported sibling Python modules for values such as `__version__`, `__author__`, and `__license__`.

The fallback stays intentionally tight. It does not broaden into general code execution or whole-tree harvesting.

### 4. Private package classifier support

Rust recognizes the classifier `Private :: Do Not Upload` and maps it to `is_private = true`.

### 5. Installed and source metadata sidecars

Rust now treats several adjacent metadata files as part of the same Python package evidence surface:

- `License-File` headers are exposed as structured `file_references`
- sibling `RECORD` and `installed-files.txt` files can feed scan-time file assignment for installed layouts
- sibling `SOURCES.txt` can recover explicit file references for source layouts
- sibling `WHEEL` data enriches installed wheel metadata without creating duplicate package rows
- pip wheel-cache `origin.json` files can preserve source archive provenance and merge with sibling cached wheels when the identities agree

This improves package attribution while staying grounded in explicit metadata rather than generic filesystem guessing.

### 6. Saved PyPI JSON support

Rust can parse saved `pypi.json` payloads and recover core package metadata, project URLs, artifact download information, and private-package classifier state.

The behavior is intentionally scoped to the exact local filename `pypi.json`.

### 7. RFC822 dependency extraction for wheel and source metadata

Rust now extracts dependency information from RFC822-style Python metadata files, including:

- `Requires-Dist`
- extra-scoped requirements expressed through `extra == ...` markers
- sibling `.egg-info/requires.txt` when source-package metadata needs additional dependency evidence

That closes the wheel versus source-package gap for common Python metadata layouts. Extra scopes, simple markers, and pinned requirements are preserved structurally instead of being dropped.

### 8. Direct source distribution archive support

Rust now parses Python source distribution archives directly instead of requiring an unpacked `PKG-INFO` file to already exist on disk.

- common archive formats such as `.tar.gz`, `.tgz`, `.tar.bz2`, `.tar.xz`, and `.zip` are recognized directly
- embedded `PKG-INFO` is parsed inside the archive without extracting or executing project code
- `.egg-info/PKG-INFO` is preferred over a root `PKG-INFO` when both exist, matching the richer dependency-bearing metadata layout
- embedded `.egg-info/requires.txt` and `SOURCES.txt` sidecars can still recover dependency and file-reference data from archive-only scans
- direct archive parsing intentionally reuses the existing `pypi_sdist_pkginfo` datasource because `PKG-INFO` remains the authoritative metadata surface inside the sdist

### 9. Archive hardening for sdist / wheel / egg inputs

- Zip-based Python archives now bind validation to the actual read path instead of validating first and then reopening suspicious metadata entries by name later.
- Relevant zip entries are validated and then read by index with an explicit byte cap, which closes the gap where a suspicious `PKG-INFO` / `METADATA` entry could still be decompressed after preflight validation.
- Tar-based sdist parsing now checks suspicious expansion while walking the archive stream instead of waiting until the end of iteration to reject a high-ratio archive.
- Archive entry paths are normalized and unsafe absolute / parent-traversal paths are ignored for metadata selection.

## Why this matters

- **Better manifest fidelity**: more of the metadata already present in Python manifests becomes visible to downstream tooling
- **Safer metadata recovery**: narrow static fallbacks recover real values without broadening into execution-heavy parsing
- **Richer installed-package provenance**: wheel, cache, and sidecar metadata all contribute to a clearer package story
- **Broader input coverage**: saved API payloads, RFC822 metadata files, and direct sdist archives now produce useful package data instead of partial results
- **Safer archive ingestion**: sdist, wheel, and egg parsing now enforce their archive safety policy where bytes are actually read, reducing exposure to suspicious metadata entries

## Coverage

Coverage focuses on the user-visible behaviors above, including richer `setup.cfg` extraction, `OrderedDict` project URLs, imported sibling dunder fallback, private-package classification, installed and source sidecar handling, direct sdist archive parsing, archive hardening for suspicious zip metadata entries, `WHEEL` and `origin.json` provenance, saved `pypi.json` parsing, and RFC822 dependency recovery.
