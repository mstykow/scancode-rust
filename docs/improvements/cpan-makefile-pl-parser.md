# CpanMakefilePlParser: WriteMakefile Metadata Extraction

## Summary

**✨ New Feature + 🔍 Enhanced**: Python implementation is a stub with no parse method. Rust implementation provides WriteMakefile function parsing with metadata extraction, including bounded static resolution of `VERSION_FROM` and `ABSTRACT_FROM`.

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
   - `VERSION` or resolved `VERSION_FROM` → `version`
   - `ABSTRACT` or resolved `ABSTRACT_FROM` → `description`
   - `LICENSE` → `extracted_license_statement`

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
6. Optionally reads bounded sibling module files referenced by `VERSION_FROM` / `ABSTRACT_FROM` and extracts literal `$VERSION` plus POD `=head1 NAME` abstract text without executing Perl

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
  "extracted_license_statement": "perl_5",
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

| Parameter        | PackageData Field                | Notes                                                |
| ---------------- | -------------------------------- | ---------------------------------------------------- |
| `NAME`           | `name`                           | Module name                                          |
| `VERSION`        | `version`                        | Version string                                       |
| `VERSION_FROM`   | `version`                        | Resolve literal `$VERSION` from sibling module file  |
| `ABSTRACT`       | `description`                    | Short description                                    |
| `ABSTRACT_FROM`  | `description`                    | Resolve POD `NAME` abstract from sibling module file |
| `AUTHOR`         | `parties` (role=`author`)        | Author info                                          |
| `LICENSE`        | `extracted_license_statement`    | License identifier                                   |
| `PREREQ_PM`      | `dependencies` (scope=`runtime`) | Runtime deps                                         |
| `BUILD_REQUIRES` | `dependencies` (scope=`build`)   | Build deps                                           |
| `TEST_REQUIRES`  | `dependencies` (scope=`test`)    | Test deps                                            |

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

For `VERSION_FROM` and `ABSTRACT_FROM`, Rust only reads local referenced files within the `Makefile.PL` directory tree, caps file size, and extracts conservative literal patterns rather than evaluating Perl.

## Value

- **Legacy CPAN support**: Many older CPAN modules use Makefile.PL instead of META files
- **Complete dependency graph**: Extract build, test, and runtime dependencies
- **Author attribution**: Structured author information
- **License compliance**: License information for compliance tracking
- **Better package identity**: Real version and abstract text can now be recovered from the referenced module file when MakeMaker keeps them out of the `WriteMakefile(...)` call
- **Secure parsing**: No code execution risk

## Limitations

Still intentionally not implemented:

- Dynamic version computation (e.g., `VERSION => do { ... }`)
- Complex Perl expressions in parameter values
- Non-literal or generated `$VERSION` patterns in referenced module files
- Non-standard abstract extraction outside the documented POD `=head1 NAME` / `Package - abstract` form

These remaining cases would require a fuller Perl parser or code execution, which poses security risks. The current bounded static approach covers the common CPAN MakeMaker patterns without expanding evaluation scope.

## Coverage

Coverage includes:

- Basic WriteMakefile parsing
- Dependency extraction with all scopes
- Author email parsing
- Author without email
- Minimal Makefile.PL handling
- `VERSION_FROM` recovery from a sibling `.pm` file
- `ABSTRACT_FROM` recovery from POD in a sibling `.pm` file
- Empty content handling
- Malformed WriteMakefile handling

## References

- ExtUtils::MakeMaker spec: <https://metacpan.org/pod/ExtUtils::MakeMaker>
