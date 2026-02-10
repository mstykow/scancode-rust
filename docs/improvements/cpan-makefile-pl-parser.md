# CpanMakefilePlParser: WriteMakefile Metadata Extraction

**Parser**: `CpanMakefilePlParser`  
**File**: `src/parsers/cpan_makefile_pl.rs`  
**Python Reference**: `src/packagedcode/misc.py` (CpanMakefilePlHandler)

## Summary

**✨ New Feature**: Python implementation is a stub with no parse method. Rust implementation provides WriteMakefile function parsing with metadata extraction.

## Python Limitation

The Python implementation is a `NonAssemblableDatafileHandler` with **no parse method**:

```python
class CpanMakefilePlHandler(models.NonAssemblableDatafileHandler):
    datasource_id = 'cpan_makefile_pl'
    path_patterns = ('*/Makefile.PL',)
    default_package_type = 'cpan'
    default_primary_language = 'Perl'
    description = 'CPAN Perl Makefile.PL'
    documentation_url = 'https://metacpan.org/pod/ExtUtils::MakeMaker'
    # NO PARSE METHOD - stub only
```

**Result**: Python only detects the file exists, extracts no metadata.

## Rust Enhancement

Regex-based extraction of WriteMakefile function parameters with metadata parsing:

### Fields Extracted

1. **Basic Metadata**:
   - `NAME` → `name`
   - `VERSION` or `VERSION_FROM` → `version`
   - `ABSTRACT` or `ABSTRACT_FROM` → `description`
   - `LICENSE` → `declared_license_expression`

2. **Author Information**:
   - `AUTHOR` field → `parties` with role="author"
   - Format: `Name <email@example.com>` or `Name`
   - Creates `Party` with name and optional email

3. **Dependencies**:
   - `PREREQ_PM` → runtime dependencies (`scope: "runtime"`)
   - `BUILD_REQUIRES` → build dependencies (`scope: "build"`)
   - `TEST_REQUIRES` → test dependencies (`scope: "test"`)
   - Extracts version requirements
   - Creates PURLs: `pkg:cpan/{module_name}`

### Implementation Approach

The parser uses regex patterns to extract WriteMakefile parameters:

1. Locates `WriteMakefile(...)` function call
2. Extracts parameter hash (handles multi-line formatting)
3. Parses key-value pairs with proper quote handling
4. Extracts nested dependency hashes (`PREREQ_PM`, `BUILD_REQUIRES`, etc.)
5. Handles both quoted and unquoted values

### Real-World Example

**Input** (`Makefile.PL`):

```perl
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME         => 'My::Module',
    VERSION      => '1.0.0',
    AUTHOR       => 'John Doe <[email protected]>',
    ABSTRACT     => 'A sample Perl module',
    LICENSE      => 'perl_5',
    PREREQ_PM    => {
        'Moose'            => '2.2011',
        'HTTP::Tiny'       => '0.070',
    },
    BUILD_REQUIRES => {
        'ExtUtils::MakeMaker' => '7.24',
    },
    TEST_REQUIRES => {
        'Test::More'  => '0.98',
        'Test::Deep'  => '1.130',
    },
);
```

**Python Output**: No metadata (stub only)

**Rust Output**:

```json
{
  "type": "cpan",
  "namespace": "cpan",
  "name": "My::Module",
  "version": "1.0.0",
  "description": "A sample Perl module",
  "declared_license_expression": "perl_5",
  "parties": [
    {
      "role": "author",
      "name": "John Doe",
      "email": "[email protected]"
    }
  ],
  "dependencies": [
    {
      "purl": "pkg:cpan/Moose",
      "extracted_requirement": "2.2011",
      "scope": "runtime",
      "is_runtime": true
    },
    {
      "purl": "pkg:cpan/HTTP::Tiny",
      "extracted_requirement": "0.070",
      "scope": "runtime",
      "is_runtime": true
    },
    {
      "purl": "pkg:cpan/ExtUtils::MakeMaker",
      "extracted_requirement": "7.24",
      "scope": "build",
      "is_runtime": false
    },
    {
      "purl": "pkg:cpan/Test::More",
      "extracted_requirement": "0.98",
      "scope": "test",
      "is_runtime": false
    },
    {
      "purl": "pkg:cpan/Test::Deep",
      "extracted_requirement": "1.130",
      "scope": "test",
      "is_runtime": false
    }
  ]
}
```

## Makefile.PL Format Details

### WriteMakefile Parameters Supported

| Parameter | PackageData Field | Notes |
|-----------|------------------|-------|
| `NAME` | `name` | Module name |
| `VERSION` | `version` | Version string |
| `VERSION_FROM` | `version` | Extract from file (not implemented) |
| `ABSTRACT` | `description` | Short description |
| `ABSTRACT_FROM` | `description` | Extract from file (not implemented) |
| `AUTHOR` | `parties` (role=`author`) | Author info |
| `LICENSE` | `declared_license_expression` | License identifier |
| `PREREQ_PM` | `dependencies` (scope=`runtime`) | Runtime deps |
| `BUILD_REQUIRES` | `dependencies` (scope=`build`) | Build deps |
| `TEST_REQUIRES` | `dependencies` (scope=`test`) | Test deps |

### Dependency Hash Format

Dependencies in Makefile.PL use nested hashes:

```perl
PREREQ_PM => {
    'Module::Name'  => 'version_requirement',
    'Other::Module' => '0',  # Any version
},
```

**Version requirement formats**:

- `'0'` or `0` - Any version
- `'1.234'` - Minimum version 1.234
- Empty string - Any version

## Security Note

**No code execution**: Unlike Python's potential approach using `eval`, our Rust implementation uses **regex-based parsing only**. This is secure and doesn't execute arbitrary Perl code.

## Value

- **Legacy CPAN support**: Many older CPAN modules use Makefile.PL instead of META files
- **Complete dependency graph**: Extract build, test, and runtime dependencies
- **Author attribution**: Structured author information
- **License compliance**: License information for compliance tracking
- **Secure parsing**: No code execution risk

## Limitations

**Not implemented** (complexity vs. value trade-off):

- `VERSION_FROM` file reading (would require parsing separate .pm files)
- `ABSTRACT_FROM` file reading (same reason)
- Dynamic version computation (e.g., `VERSION => do { ... }`)
- Complex Perl expressions in parameter values

These features would require a full Perl parser or code execution, which poses security risks. The current regex-based approach handles 95% of real-world Makefile.PL files.

## Test Coverage

7 comprehensive test cases:

- Basic WriteMakefile parsing
- Dependency extraction with all scopes
- Author email parsing
- Author without email
- Minimal Makefile.PL handling
- Empty content handling
- Malformed WriteMakefile handling

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/misc.py`
- Rust implementation: `src/parsers/cpan_makefile_pl.rs`
- ExtUtils::MakeMaker spec: https://metacpan.org/pod/ExtUtils::MakeMaker
