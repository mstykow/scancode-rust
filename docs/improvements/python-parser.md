# Python Parser: Manifest Metadata, Dunder Fallbacks, and PyPI JSON

**Area**: Python package metadata extraction  
**Files**: `src/parsers/python.rs`, `src/parsers/python_test.rs`, `src/parsers/python_golden_test.rs`  
**Upstream Context**: `aboutcode-org/scancode-toolkit#2912`, `#2263`, `#2267`, `#2599`, `#3918`, `#3968`, `#1545`

## Summary

**🐛 Bug Fix + ✨ New Feature + 🔍 Enhanced Extraction**: Rust now extracts richer Python manifest metadata, resolves imported sibling dunder metadata for setup.py, preserves PKG-INFO license-file references, and supports saved `pypi.json` API payloads.

## What changed

### 1. Richer `setup.cfg` metadata

Rust `setup.cfg` parsing now extracts more of the metadata already present in real package manifests:

- `description`
- maintainer name/email
- keyword lists
- `python_requires`
- `project_urls`
- mapped issue-tracker URLs

### 2. setup.py `project_urls` from `OrderedDict`

Rust setup.py parsing now understands `project_urls=OrderedDict([...])` in addition to plain dict literals.

### 3. Imported sibling dunder metadata fallback for setup.py

When setup.py leaves values unresolved after AST parsing, Rust now performs a narrow fallback against imported sibling Python modules for plain:

- `__version__`
- `__author__`
- `__license__`

### 4. Private package classifier support

Rust now recognizes the classifier `Private :: Do Not Upload` and maps it to `is_private = true`.

### 5. PKG-INFO / METADATA license file references

Rust already preserved `License-File` headers in `extra_data.license_files`.
It now also exposes them as structured `file_references`.

### 5a. Installed metadata sidecar collection and assignment

Rust now also uses sibling installed metadata sidecars in the narrow installed-layout cases that matter for scan-time package/file assignment:

- sibling `RECORD` next to `METADATA`
- sibling `installed-files.txt` next to `PKG-INFO`

And when those metadata files live under `site-packages/` or `dist-packages/`, Rust now resolves the referenced paths back onto scanned files and assigns those files to the assembled Python package.

This stays intentionally narrow:

- it is driven by explicit file references,
- it supports installed Python metadata layouts,
- and it does **not** broaden into generic whole-tree Python file harvesting.

### 6. Saved PyPI JSON support

Rust now supports parsing saved `pypi.json` payloads and extracts core package metadata, project URLs, artifact download data, and private-package classifier state.

This support is intentionally scoped to the exact local filename `pypi.json`.

## Verified issue relevance

### Implemented as real remaining gaps

- `#136` richer `setup.cfg` metadata
- `#138` unresolved computed-version style setup.py cases (narrow sibling-dunder fallback)
- `#139` dunder metadata from sibling Python source files
- `#140` setup.py `project_urls` with non-plain-dict forms like `OrderedDict`
- `#143` qualified/mapped setup.cfg project URLs
- `#147` PKG-INFO / METADATA license-file signal preservation via `file_references`
- `#148` private-package classifier support
- `#209` PyPI JSON parse support
- `#150` installed Python metadata file-to-package assignment in scans

### Verified as already covered locally or audited as nonblocking

- `#141`
- `#142`
- `#145`
- `#146`
- `#210`
- `#212`
- `#213`

### Umbrella / partially overlapping issues

- `#144`
- `#149`

These were treated as umbrella quality/collection issues rather than as evidence of one remaining single parser defect after the local audit.

## Coverage

Coverage includes:

- focused red-to-green unit tests for richer `setup.cfg` extraction
- setup.py `OrderedDict` URL regression coverage
- imported-module dunder metadata fallback coverage
- private classifier coverage
- PKG-INFO / METADATA license-file reference coverage
- standalone metadata sibling `RECORD` / `installed-files.txt` coverage
- scan-level installed Python file assignment coverage for `.dist-info` and `.egg-info`
- a new pyproject parser golden
- the existing metadata and setup.cfg parser goldens
- the broader Python parser test suite

## References

- Local issues: `#136`, `#138`, `#139`, `#140`, `#143`, `#147`, `#148`, `#209`
- Reference implementation: `reference/scancode-toolkit/src/packagedcode/pypi.py`
- Current local golden surface: `src/parsers/python_golden_test.rs`
