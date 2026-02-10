# RpmSpecfileParser: Full Preamble Parsing Implementation

**Parser**: `RpmSpecfileParser`  
**File**: `src/parsers/rpm_specfile.rs`  
**Python Reference**: `src/packagedcode/rpm.py` (RpmSpecfileHandler)

## Summary

**✨ New Feature**: Python implementation is a stub with `# TODO: implement me!!@` comment. Rust implementation provides full RPM spec preamble parsing with metadata extraction.

## Python Limitation

The Python implementation is a `NonAssemblableDatafileHandler` with **no parse method**:

```python
class RpmSpecfileHandler(NonAssemblableDatafileHandler):
    datasource_id = 'rpm_specfile'
    path_patterns = ('*.spec',)
    default_package_type = 'rpm'
    description = 'RPM specfile'
    documentation_url = 'https://rpm-software-management.github.io/rpm/manual/spec.html'

    @classmethod
    def parse(cls, location, package_only=False):
        # TODO: implement me!!@
        return
```

**Result**: Python only detects the file exists, extracts no metadata.

## Rust Enhancement

Full RPM spec preamble parsing with comprehensive metadata extraction:

### Fields Extracted

1. **Basic Metadata**:
   - `Name` → `name`
   - `Version` → `version`
   - `License` → `declared_license_expression`
   - `Summary` → `description`
   - `URL` → `homepage_url`

2. **Author/Maintainer Information**:
   - `Packager` → `parties` with role="maintainer"
   - Format: `Name <email@example.com>`
   - Creates `Party` with name and email

3. **Description**:
   - `%description` section → `description` (overwrites Summary if present)
   - Multi-line content support

4. **Dependencies**:
   - `Requires` → runtime dependencies (`scope: "dependencies"`)
   - `BuildRequires` → build dependencies (`scope: "build-dependencies"`)
   - Extracts version requirements with operators (`>=`, `<=`, `=`, `>`, `<`)
   - Creates PURLs: `pkg:rpm/{package_name}`

### Implementation Approach

The parser:

1. Parses RPM spec preamble line-by-line
2. Extracts key-value pairs (e.g., `Name: mypackage`)
3. Handles multi-line fields like `%description`
4. Parses dependency lines with version constraints
5. Converts Packager field to structured Party model

### Real-World Example

**Input** (`mypackage.spec`):

```spec
Name:           mypackage
Version:        1.0.0
License:        MIT
Summary:        A sample RPM package
URL:            https://example.com/mypackage
Packager:       John Doe <[email protected]>

Requires:       bash >= 4.0
Requires:       glibc
BuildRequires:  gcc >= 9.0
BuildRequires:  make

%description
This is a sample RPM package that demonstrates
the full capabilities of our spec file parser.
It can handle multi-line descriptions.

%prep
# ... rest of spec file ...
```

**Python Output**: No metadata (stub only)

**Rust Output**:

```json
{
  "type": "rpm",
  "name": "mypackage",
  "version": "1.0.0",
  "description": "This is a sample RPM package that demonstrates\nthe full capabilities of our spec file parser.\nIt can handle multi-line descriptions.",
  "homepage_url": "https://example.com/mypackage",
  "declared_license_expression": "MIT",
  "parties": [
    {
      "role": "maintainer",
      "name": "John Doe",
      "email": "[email protected]"
    }
  ],
  "dependencies": [
    {
      "purl": "pkg:rpm/bash",
      "extracted_requirement": "bash >= 4.0",
      "scope": "dependencies",
      "is_runtime": true
    },
    {
      "purl": "pkg:rpm/glibc",
      "extracted_requirement": "glibc",
      "scope": "dependencies",
      "is_runtime": true
    },
    {
      "purl": "pkg:rpm/gcc",
      "extracted_requirement": "gcc >= 9.0",
      "scope": "build-dependencies",
      "is_runtime": false
    },
    {
      "purl": "pkg:rpm/make",
      "extracted_requirement": "make",
      "scope": "build-dependencies",
      "is_runtime": false
    }
  ]
}
```

## RPM Spec Format Details

### Dependency Syntax

RPM dependencies follow this format:

```text
Requires: package_name [operator version]
```

**Operators**:

- `>=` - Greater than or equal
- `<=` - Less than or equal
- `=` - Exactly equal
- `>` - Strictly greater
- `<` - Strictly less

### Preamble Tags Supported

| Tag | PackageData Field | Notes |
|-----|------------------|-------|
| `Name` | `name` | Package name |
| `Version` | `version` | Package version |
| `License` | `declared_license_expression` | SPDX expression |
| `Summary` | `description` | Short description |
| `URL` | `homepage_url` | Project homepage |
| `Packager` | `parties` (role=`maintainer`) | Maintainer info |
| `%description` | `description` | Long description (overrides Summary) |
| `Requires` | `dependencies` (scope=`dependencies`) | Runtime deps |
| `BuildRequires` | `dependencies` (scope=`build-dependencies`) | Build deps |

## Value

- **Build system integration**: Extract metadata from source RPM specs
- **SBOM generation**: Complete dependency information from spec files
- **License compliance**: License information directly from spec
- **Maintainer tracking**: Packager information for accountability
- **Pre-build analysis**: Analyze dependencies before building RPM

## Limitations

**Not implemented** (matching Python's scope):

- Macro expansion (e.g., `%{version}`, `%{_libdir}`)
- Conditional directives (e.g., `%if`, `%ifdef`)
- Scriptlets (e.g., `%prep`, `%build`, `%install`)
- File lists (e.g., `%files`)

These are complex features that Python also doesn't implement. The focus is on extracting metadata from the preamble, which is sufficient for most SBOM use cases.

## Test Coverage

6 comprehensive test cases:

- Basic preamble parsing
- Dependency extraction with version constraints
- Multi-line %description handling
- Packager field parsing
- Minimal spec file handling
- Empty content handling

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/rpm.py` (line 449: "TODO: implement me!!@")
- Rust implementation: `src/parsers/rpm_specfile.rs`
- RPM spec format: https://rpm-software-management.github.io/rpm/manual/spec.html
