# CPAN Parser: Improvements Over Python

## Summary

Our Rust implementation improves on the Python reference by:

- ✨ **Real Parsing**: Python has stub-only handlers with no parse() method
- 🔍 **Full Metadata**: We extract complete metadata from META.json and META.yml
- 📦 **Dependency Extraction**: All 4 dependency scopes (runtime, build, test, configure)
- 👥 **Author Extraction**: Complete party information from author fields
- 🔗 **Resource URLs**: Homepage, VCS, code view, and bug tracking URLs
- 📄 **File References**: MANIFEST file list extraction

## Problem in Python Reference

Python ScanCode has 5 CPAN handlers in `misc.py` lines 127-173:

- `CpanMetaJsonHandler`
- `CpanMetaYmlHandler`
- `CpanManifestHandler`
- `CpanMakefilePlHandler`
- `CpanDistIniHandler`

**All are stubs** - they only detect files but don't extract any metadata (no `parse()` method).

## Our Solution

We implemented real parsing for 3 formats:

### 1. META.json (CPAN::Meta::Spec v2.0+)

- Full metadata extraction
- Nested `prereqs` structure with 4 dependency types
- Repository resource objects
- License arrays

### 2. META.yml (CPAN::Meta::Spec v1.4)

- Complete v1.4 format support
- Flat dependency structure (`requires`, `build_requires`, etc.)
- Legacy resource format
- String or array license field

### 3. MANIFEST

- File list extraction as `file_references`
- Comment filtering
- Whitespace handling

## Before/After Comparison

### Python Output (stub)

```json
{
  "package_type": "cpan",
  "name": null,
  "version": null,
  "description": null,
  "parties": [],
  "dependencies": []
}
```

### Rust Output (real parsing)

```json
{
  "type": "cpan",
  "name": "Example-Web-Toolkit",
  "version": "1.042",
  "primary_language": "Perl",
  "description": "A modern Perl toolkit for web development",
  "extracted_license_statement": "perl_5",
  "parties": [
    {
      "type": "person",
      "role": "author",
      "name": "John Doe",
      "email": "john@example.com"
    },
    {
      "type": "person",
      "role": "author",
      "name": "Jane Smith",
      "email": "jane@example.com"
    }
  ],
  "homepage_url": "https://example.com/web-toolkit",
  "vcs_url": "https://github.com/example/web-toolkit.git",
  "code_view_url": "https://github.com/example/web-toolkit",
  "bug_tracking_url": "https://github.com/example/web-toolkit/issues",
  "dependencies": [
    {
      "purl": "pkg:cpan/Moose",
      "extracted_requirement": "2.2011",
      "scope": "runtime",
      "is_runtime": true,
      "is_optional": false,
      "is_direct": true
    },
    {
      "purl": "pkg:cpan/HTTP::Tiny",
      "extracted_requirement": "0.070",
      "scope": "runtime",
      "is_runtime": true,
      "is_optional": false,
      "is_direct": true
    },
    {
      "purl": "pkg:cpan/ExtUtils::MakeMaker",
      "extracted_requirement": "7.24",
      "scope": "build",
      "is_runtime": false,
      "is_optional": false,
      "is_direct": true
    },
    {
      "purl": "pkg:cpan/Test::More",
      "extracted_requirement": "0.98",
      "scope": "test",
      "is_runtime": false,
      "is_optional": false,
      "is_direct": true
    }
  ],
  "datasource_id": "cpan_meta_json"
}
```

## Field Mapping

### META.json (v2.0+)

| CPAN Field | PackageData Field | Notes |
|-----------|-------------------|-------|
| `name` | `name` | Module name with hyphens |
| `version` | `version` | String or number |
| `abstract` | `description` | Short description |
| `license` | `extracted_license_statement` | Array joined with " AND " |
| `author` | `parties` (role=`author`) | Array of "Name <email>" strings |
| `resources.homepage` | `homepage_url` | |
| `resources.repository.url` | `vcs_url` | |
| `resources.repository.web` | `code_view_url` | |
| `resources.bugtracker.web` | `bug_tracking_url` | |
| `prereqs.runtime.requires` | `dependencies` (scope=`runtime`) | |
| `prereqs.build.requires` | `dependencies` (scope=`build`) | |
| `prereqs.test.requires` | `dependencies` (scope=`test`) | |
| `prereqs.configure.requires` | `dependencies` (scope=`configure`) | |

### META.yml (v1.4)

| CPAN Field | PackageData Field | Notes |
|-----------|-------------------|-------|
| `name` | `name` | |
| `version` | `version` | |
| `abstract` / `description` | `description` | Fallback to `description` |
| `license` | `extracted_license_statement` | String or array |
| `author` | `parties` (role=`author`) | |
| `resources.homepage` | `homepage_url` | |
| `resources.repository` | `vcs_url` | Simple string in v1.4 |
| `resources.bugtracker` | `bug_tracking_url` | |
| `requires` | `dependencies` (scope=`runtime`) | Flat structure |
| `build_requires` | `dependencies` (scope=`build`) | |
| `test_requires` | `dependencies` (scope=`test`) | |
| `configure_requires` | `dependencies` (scope=`configure`) | |

### MANIFEST

Simple line-by-line file list extraction:

- Extracts file paths as `file_references`
- Strips comments (lines starting with `#` or text after whitespace)
- Filters empty lines

## Dependency Scopes

CPAN has 4 dependency scopes:

| Scope | is_runtime | Description |
|-------|-----------|-------------|
| `runtime` | `true` | Runtime dependencies |
| `build` | `false` | Build-time dependencies |
| `test` | `false` | Test dependencies |
| `configure` | `false` | Configuration dependencies |

**Note**: The `perl` dependency is filtered out (not a CPAN module).

## Implementation Details

### Parsing Strategy

- **JSON**: serde_json for META.json
- **YAML**: serde_yaml for META.yml
- **MANIFEST**: Line-by-line text parsing

### Author Parsing

Supports "Name <email>" format:

- `"John Doe <john@example.com>"` → name="John Doe", email="john@example.com"
- `"John Doe"` → name="John Doe", email=None

### License Handling

- Single license: `"perl_5"` → `"perl_5"`
- License array: `["apache_2_0", "mit"]` → `"apache_2_0 AND mit"`

### Error Handling

- Graceful fallback to default PackageData on parse errors
- Warning logs for debugging
- No panics in library code

## References

### Python Reference Issues

- [misc.py lines 127-173](../../reference/scancode-toolkit/src/packagedcode/misc.py#L127-L173): Stub-only handlers with no parse() method
- No metadata extraction
- No dependency extraction
- No party extraction

### CPAN Documentation

- [CPAN::Meta::Spec v2](https://metacpan.org/pod/CPAN::Meta::Spec): JSON format specification
- [CPAN::Meta::Spec v1.4](https://metacpan.org/pod/distribution/CPAN-Meta/lib/CPAN/Meta/Spec.pm): YAML format specification
- [Module::Manifest](https://metacpan.org/pod/Module::Manifest): MANIFEST file format

## Status

- ✅ **Implementation**: Complete, validated, production-ready
- ✅ **Testing**: 16+ unit tests covering all features
- ✅ **Documentation**: Complete
- ⏳ **Future**: Makefile.PL and dist.ini parsers (deferred - very complex)

## Future Work

### Makefile.PL Parser

Makefile.PL is Perl code that generates Makefiles. Parsing it safely requires:

- Full Perl AST parsing (complex)
- Or sandboxed code execution (security risk)
- Current Python implementation doesn't parse it either

### dist.ini Parser

dist.ini is used by Dist::Zilla (modern Perl build system):

- INI-like format but with complex plugin system
- Requires understanding Dist::Zilla plugin configuration
- Current Python implementation doesn't parse it either

Both formats are deferred to future work due to complexity.
