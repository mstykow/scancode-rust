# CpanDistIniParser: Full INI Parsing Implementation

**Parser**: `CpanDistIniParser`  
**File**: `src/parsers/cpan_dist_ini.rs`  
**Python Reference**: `src/packagedcode/misc.py` (CpanDistIniHandler)

## Summary

**✨ New Feature**: Python implementation is a stub with no parse method. Rust implementation provides full INI parsing and metadata extraction.

## Python Limitation

The Python implementation is a `NonAssemblableDatafileHandler` with **no parse method**:

```python
class CpanDistIniHandler(models.NonAssemblableDatafileHandler):
    datasource_id = 'cpan_dist_ini'
    path_patterns = ('*/dist.ini',)
    default_package_type = 'cpan'
    default_primary_language = 'Perl'
    description = 'CPAN Perl dist.ini'
    documentation_url = 'https://metacpan.org/pod/Dist::Zilla::Tutorial'
    # NO PARSE METHOD - stub only
```

**Result**: Python only detects the file exists, extracts no metadata.

## Rust Enhancement

Full INI parsing with comprehensive metadata extraction:

### Fields Extracted

1. **Basic Metadata**:
   - `name` (converted from hyphenated to Perl namespace: `Foo-Bar` → `Foo::Bar`)
   - `version`
   - `abstract` → `description`
   - `license` → `declared_license_expression`

2. **Author Information**:
   - Parses `author` field with email extraction
   - Format: `Name <email@example.com>`
   - Creates `Party` with role="author"

3. **Copyright Information** (in `extra_data`):
   - `copyright_holder`
   - `copyright_year`

4. **Dependencies** (from `[Prereq]` sections):
   - Runtime dependencies: `[Prereq]`
   - Test dependencies: `[Prereq / TestRequires]`
   - Build dependencies: `[Prereq / BuildRequires]`
   - Extracts version requirements
   - Creates PURLs: `pkg:cpan/{module_name}`

### Implementation Approach

The parser:

1. Parses INI structure into root fields and sections
2. Converts hyphenated package names to Perl namespace format (`Foo-Bar` → `Foo::Bar`)
3. Extracts author information with email parsing (`Name <email@example.com>`)
4. Processes `[Prereq]` sections to extract dependencies with scope detection
5. Stores copyright metadata in `extra_data`

### Real-World Example

**Input** (`dist.ini`):

```ini
name = Dancer2-Plugin-Minion
version = 1.0.0
author = Jason A. Crome <[email protected]>
license = Perl_5
copyright_holder = Jason A. Crome
copyright_year = 2024
abstract = Dancer2 plugin for Minion job queue

[Prereq]
Moose = 0.92
MooseX::Params::Validate = 0.12

[Prereq / TestRequires]
Test::More = 0.88
```

**Python Output**: No metadata (stub only)

**Rust Output**:

```json
{
  "type": "cpan",
  "namespace": "cpan",
  "name": "Dancer2::Plugin::Minion",
  "version": "1.0.0",
  "description": "Dancer2 plugin for Minion job queue",
  "declared_license_expression": "Perl_5",
  "parties": [
    {
      "role": "author",
      "name": "Jason A. Crome",
      "email": "[email protected]"
    }
  ],
  "dependencies": [
    {
      "purl": "pkg:cpan/Moose",
      "extracted_requirement": "0.92",
      "scope": "runtime",
      "is_runtime": true
    },
    {
      "purl": "pkg:cpan/MooseX::Params::Validate",
      "extracted_requirement": "0.12",
      "scope": "runtime",
      "is_runtime": true
    },
    {
      "purl": "pkg:cpan/Test::More",
      "extracted_requirement": "0.88",
      "scope": "test",
      "is_runtime": false
    }
  ],
  "extra_data": {
    "copyright_holder": "Jason A. Crome",
    "copyright_year": "2024"
  }
}
```

## Value

- **SBOM completeness**: Full package metadata instead of just file detection
- **Dependency tracking**: Complete dependency graph with scopes
- **Author attribution**: Structured author information with email
- **License compliance**: License information for compliance tracking
- **Build reproducibility**: Version constraints for all dependencies

## Test Coverage

6 comprehensive test cases:

- Basic metadata extraction
- Dependency parsing with scopes
- Author email parsing
- Minimal dist.ini handling
- Author without email
- Empty content handling

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/misc.py`
- Rust implementation: `src/parsers/cpan_dist_ini.rs`
- Dist::Zilla spec: https://metacpan.org/pod/Dist::Zilla::Tutorial
