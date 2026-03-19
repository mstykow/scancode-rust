# Python Parser: Manifest Metadata, Installed Provenance, Dunder Fallbacks, and PyPI JSON

## Summary

**🐛 Bug Fix + ✨ New Feature + 🔍 Enhanced Extraction**: Rust now extracts richer Python manifest metadata, resolves a narrow class of imported sibling dunder values for `setup.py`, preserves more installed and source-package provenance, supports saved `pypi.json` payloads, and recovers RFC822 dependency metadata that was previously missing.

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

## Why this matters

- **Better manifest fidelity**: more of the metadata already present in Python manifests becomes visible to downstream tooling
- **Safer metadata recovery**: narrow static fallbacks recover real values without broadening into execution-heavy parsing
- **Richer installed-package provenance**: wheel, cache, and sidecar metadata all contribute to a clearer package story
- **Broader input coverage**: saved API payloads and RFC822 metadata files now produce useful package data instead of partial results

## Coverage

Coverage focuses on the user-visible behaviors above, including richer `setup.cfg` extraction, `OrderedDict` project URLs, imported sibling dunder fallback, private-package classification, installed and source sidecar handling, `WHEEL` and `origin.json` provenance, saved `pypi.json` parsing, and RFC822 dependency recovery.
