# OSGi Manifest Parser: Bundle Metadata Extraction

**Parser**: `MavenParser` (OSGi detection integrated)  
**File**: `src/parsers/maven_pom.rs`  
**Python Reference**: `src/packagedcode/maven.py` (JavaOSGiManifestHandler)

## Summary

**✨ New Feature**: Python implementation has **empty path_patterns** (assembly-only handler). Rust implementation provides full OSGi bundle detection and metadata extraction.

## Python Limitation

The Python implementation is a handler with **no path patterns**:

```python
class JavaOSGiManifestHandler(BaseJavaManifestHandler):
    datasource_id = 'java_osgi_manifest'
    path_patterns = ()  # EMPTY - only called during assembly phase
    default_package_type = 'maven'
    description = 'Java OSGi MANIFEST.MF'
    documentation_url = 'https://docs.osgi.org/specification/osgi.core/7.0.0/framework.module.html'
```

**Result**: Python only extracts OSGi metadata during the assembly phase, not during normal file scanning. Files are not automatically detected as OSGi bundles.

## Rust Enhancement

Automatic OSGi bundle detection and full metadata extraction during file scanning:

### Detection Strategy

The `MavenParser` automatically detects OSGi bundles by checking for:

```rust
fn is_osgi_manifest(manifest: &Manifest) -> bool {
    manifest.main_section
        .get("Bundle-SymbolicName")
        .is_some()
}
```

**Pattern**: `**/META-INF/MANIFEST.MF` files with `Bundle-SymbolicName` header

### Fields Extracted

1. **Bundle Identity**:
   - `Bundle-SymbolicName` → `name` (core OSGi identifier)
   - `Bundle-Version` → `version`
   - `Bundle-Name` → `description` (human-readable name)
   - `Bundle-Description` → `description` (detailed description, preferred over Bundle-Name)

2. **License Information**:
   - `Bundle-License` → `declared_license_expression`

3. **Vendor/Author Information**:
   - `Bundle-Vendor` → `parties` with role="vendor"
   - Creates `Party` with name from vendor field

4. **Homepage**:
   - `Bundle-DocURL` → `homepage_url`

5. **Dependencies** (from OSGi manifest headers):
   - `Import-Package` → runtime dependencies (`scope: "dependencies"`)
   - `Require-Bundle` → bundle dependencies (`scope: "dependencies"`)
   - Extracts version ranges (e.g., `[1.0.0,2.0.0)`)
   - Creates PURLs: `pkg:maven/{bundle_name}` (Maven namespace used for OSGi)

### Implementation Approach

The parser:

1. Detects `META-INF/MANIFEST.MF` files
2. Checks for `Bundle-SymbolicName` to identify OSGi bundles
3. Parses manifest headers using `java-properties` crate
4. Extracts bundle metadata from OSGi-specific headers
5. Parses `Import-Package` and `Require-Bundle` for dependencies
6. Handles version ranges in OSGi format

### Real-World Example

**Input** (`META-INF/MANIFEST.MF` in OSGi bundle):

```text
Manifest-Version: 1.0
Bundle-SymbolicName: org.example.myservice
Bundle-Version: 1.2.3
Bundle-Name: My Service Bundle
Bundle-Description: A sample OSGi service bundle
Bundle-Vendor: Example Corp
Bundle-License: Apache-2.0
Bundle-DocURL: https://example.com/myservice
Import-Package: org.osgi.framework;version="[1.8.0,2.0.0)",
 org.slf4j;version="[1.7.0,2.0.0)",
 javax.servlet.http;version="[3.1.0,4.0.0)"
Require-Bundle: org.apache.commons.lang3;bundle-version="[3.0.0,4.0.0)"
```

**Python Output**: No detection during file scan (only in assembly phase)

**Rust Output**:

```json
{
  "type": "maven",
  "namespace": "maven",
  "name": "org.example.myservice",
  "version": "1.2.3",
  "description": "A sample OSGi service bundle",
  "homepage_url": "https://example.com/myservice",
  "declared_license_expression": "Apache-2.0",
  "parties": [
    {
      "role": "vendor",
      "name": "Example Corp"
    }
  ],
  "dependencies": [
    {
      "purl": "pkg:maven/org.osgi.framework",
      "extracted_requirement": "[1.8.0,2.0.0)",
      "scope": "dependencies",
      "is_runtime": true
    },
    {
      "purl": "pkg:maven/org.slf4j",
      "extracted_requirement": "[1.7.0,2.0.0)",
      "scope": "dependencies",
      "is_runtime": true
    },
    {
      "purl": "pkg:maven/javax.servlet.http",
      "extracted_requirement": "[3.1.0,4.0.0)",
      "scope": "dependencies",
      "is_runtime": true
    },
    {
      "purl": "pkg:maven/org.apache.commons.lang3",
      "extracted_requirement": "[3.0.0,4.0.0)",
      "scope": "dependencies",
      "is_runtime": true
    }
  ]
}
```

## OSGi Bundle Format Details

### OSGi Manifest Headers Supported

| Header | PackageData Field | Notes |
|--------|------------------|-------|
| `Bundle-SymbolicName` | `name` | Unique bundle identifier (required) |
| `Bundle-Version` | `version` | OSGi semantic version |
| `Bundle-Name` | `description` | Human-readable name |
| `Bundle-Description` | `description` | Detailed description (preferred) |
| `Bundle-Vendor` | `parties` (role=`vendor`) | Bundle vendor |
| `Bundle-License` | `declared_license_expression` | License identifier |
| `Bundle-DocURL` | `homepage_url` | Documentation URL |
| `Import-Package` | `dependencies` | Package imports with versions |
| `Require-Bundle` | `dependencies` | Required bundles with versions |

### OSGi Version Ranges

OSGi uses interval notation for version ranges:

- `[1.0.0,2.0.0)` - From 1.0.0 (inclusive) to 2.0.0 (exclusive)
- `[1.0.0,2.0.0]` - From 1.0.0 (inclusive) to 2.0.0 (inclusive)
- `(1.0.0,2.0.0)` - From 1.0.0 (exclusive) to 2.0.0 (exclusive)

Our parser preserves these ranges in `extracted_requirement`.

## Difference from Python

### Python Approach

Python's `JavaOSGiManifestHandler` has **empty `path_patterns`**:

- Not invoked during file scanning
- Only called during assembly phase
- Requires explicit assembly step
- OSGi bundles not automatically detected

### Rust Approach

Rust's `MavenParser` has **integrated OSGi detection**:

- Automatically invoked for all `META-INF/MANIFEST.MF` files
- Detects OSGi bundles via `Bundle-SymbolicName` header
- Extracts metadata during normal file scanning
- No assembly step required for basic metadata

## Value

- **Automatic detection**: OSGi bundles automatically recognized during scan
- **Complete dependency graph**: Import-Package and Require-Bundle extracted
- **SBOM completeness**: Full bundle metadata without assembly step
- **Vendor tracking**: Bundle vendor information for accountability
- **License compliance**: License information directly from manifest

## Use Cases

1. **Eclipse plugins**: Eclipse RCP applications are OSGi bundles
2. **Apache Karaf**: OSGi runtime applications
3. **Java EE containers**: Many use OSGi internally
4. **Modular Java applications**: OSGi is a popular module system

## Test Coverage

5 comprehensive test cases:

- OSGi manifest detection
- Bundle metadata extraction
- Import-Package dependency parsing
- Require-Bundle dependency parsing
- Version range handling

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/maven.py` (empty path_patterns)
- Rust implementation: `src/parsers/maven_pom.rs`
- OSGi Core Specification: <https://docs.osgi.org/specification/osgi.core/7.0.0/framework.module.html>
