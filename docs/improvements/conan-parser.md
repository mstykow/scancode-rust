# Conan Parser: Improvements Over Python

## Summary

Our Rust implementation improves on the Python reference by:

- ✨ **New Format: conanfile.txt** — Python has no parser for the simple text-based manifest format
- ✨ **New Format: conan.lock** — Python has no parser for the Conan lockfile format
- 🛡️ **Security: conanfile.py** — AST-based parsing (no code execution) vs Python's approach

## Problem in Python Reference

Python ScanCode has a single Conan handler in `packagedcode/conan.py` that only handles `conanfile.py`:

- `ConanFileHandler` — Parses conanfile.py recipe files
- **No support** for `conanfile.txt` (simple dependency specification)
- **No support** for `conan.lock` (resolved dependency graph)

This means Python misses two common Conan file formats entirely.

## Our Solution

We implemented three parsers covering the full Conan ecosystem:

### 1. ConanFilePyParser (conanfile.py)

AST-based parsing using `rustpython-parser` — extracts class attributes and `self.requires()` calls without executing Python code.

Extracts:

- Package identity (name, version, description)
- URLs (homepage, VCS)
- License declarations
- Topics/keywords
- Dependencies from class-level `requires` attribute and `self.requires()` method calls

### 2. ConanfileTxtParser (conanfile.txt) — NEW

Parses the INI-style text format with `[requires]` and `[build_requires]` sections.

```ini
[requires]
zlib/1.2.13
boost/1.82.0

[build_requires]
cmake/3.26.4
```

**Python Output**: *(no parser exists)*

```json
// File not recognized — no output
```

**Rust Output**:

```json
{
  "type": "conan",
  "primary_language": "C++",
  "dependencies": [
    {
      "purl": "pkg:conan/zlib@1.2.13",
      "extracted_requirement": "1.2.13",
      "scope": "install",
      "is_runtime": true,
      "is_pinned": true
    },
    {
      "purl": "pkg:conan/boost@1.82.0",
      "extracted_requirement": "1.82.0",
      "scope": "install",
      "is_runtime": true,
      "is_pinned": true
    },
    {
      "purl": "pkg:conan/cmake@3.26.4",
      "extracted_requirement": "3.26.4",
      "scope": "build",
      "is_runtime": false,
      "is_pinned": true
    }
  ]
}
```

### 3. ConanLockParser (conan.lock) — NEW

Parses the JSON-based lockfile format with `graph_lock.nodes` structure containing resolved dependency references.

**Python Output**: *(no parser exists)*

```json
// File not recognized — no output
```

**Rust Output**:

```json
{
  "type": "conan",
  "primary_language": "C++",
  "dependencies": [
    {
      "purl": "pkg:conan/zlib@1.2.13",
      "extracted_requirement": "1.2.13",
      "scope": "install",
      "is_runtime": true,
      "is_pinned": true
    }
  ]
}
```

## Dependency Scopes

| Section | Scope | is_runtime | Description |
|---------|-------|------------|-------------|
| `[requires]` | `install` | `true` | Runtime dependencies |
| `[build_requires]` | `build` | `false` | Build-time dependencies |

## Conan Reference Format

Dependencies use the Conan reference format: `name/version@user/channel`

- `zlib/1.2.13` — Simple reference (name + version)
- `boost/1.82.0@` — Reference with empty user/channel
- `pkg/[>1.0 <2.0]` — Version range (not pinned)

Only exact versions (without range operators) are marked as `is_pinned: true`.

## Impact

- **SBOM completeness**: C/C++ projects using `conanfile.txt` or `conan.lock` are now recognized
- **CI/CD coverage**: Lockfile scanning enables reproducible dependency auditing
- **Supply chain security**: Two previously invisible dependency declaration formats now produce structured output

## References

### Python Reference

- `reference/scancode-toolkit/src/packagedcode/conan.py` — Only handles conanfile.py

### Conan Documentation

- [conanfile.txt reference](https://docs.conan.io/2/reference/conanfile_txt.html)
- [conan.lock format](https://docs.conan.io/2/reference/commands/lock.html)
- [Conan reference format](https://docs.conan.io/2/reference/conanfile/attributes.html#requires)

## Status

- ✅ **Implementation**: Complete — all three parsers production-ready
- ✅ **Testing**: Unit tests covering all formats and edge cases
- ✅ **Documentation**: Complete
