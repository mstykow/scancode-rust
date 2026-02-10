# ConanDataParser: Patch and Mirror URL Extraction

**Parser**: `ConanDataParser`  
**File**: `src/parsers/conan_data.rs`  
**Python Reference**: `src/packagedcode/conan.py` (ConanDataHandler)

## Summary

**🔍 Enhanced Extraction**: Python only extracts sources. Rust also extracts patch metadata and mirror URLs.

## Python Limitation

The Python implementation only extracts from the `sources` section:

```python
@classmethod
def parse(cls, location, package_only=False):
    with io.open(location, encoding="utf-8") as loc:
        conandata = yaml.safe_load(loc)

    sources = conandata.get("sources") or {}
    
    for version, source_data in sources.items():
        url = source_data.get("url")
        sha256 = source_data.get("sha256")
        
        yield PackageData(
            type="conan",
            version=version,
            download_url=url,
            sha256=sha256,
        )
```

**Missing**:

- `patches` section (patch files, descriptions, types)
- Mirror URLs when multiple URLs provided
- Patch metadata for build reproducibility

## Rust Enhancement

Extracts both `sources` and `patches` sections with full metadata:

### Additional Fields Extracted

1. **Patch Metadata** (in `extra_data.patches`):
   - `patch_file` - Path to patch file
   - `patch_description` - What the patch does
   - `patch_type` - Type: "portability", "conan", "bugfix", etc.

2. **Mirror URLs** (in `extra_data.mirror_urls`):
   - All alternative download URLs
   - First URL used as `download_url`
   - Remaining URLs stored for fallback

### Implementation Approach

The parser:

1. Iterates through each version in the `sources` section
2. Handles both single URL and multiple URL formats
3. When multiple URLs present, uses first as `download_url` and stores all in `mirror_urls`
4. Looks up corresponding patches for each version from `patches` section
5. Supports both list-of-objects and string formats for patches
6. Stores patch metadata (file, description, type) in `extra_data.patches`

### Real-World Example

**Input** (`conandata.yml`):

```yaml
sources:
  "1.12.0":
    url:
      - "https://github.com/libcpr/cpr/archive/refs/tags/1.12.0.tar.gz"
      - "https://mirror.example.com/cpr-1.12.0.tar.gz"
    sha256: "f64b501de66e163d6a278fbb6a95f395ee873b7a66c905dd785eae107266a709"
patches:
  "1.12.0":
    - patch_file: "patches/008-1.12.0-remove-warning-flags.patch"
      patch_description: "disable warning flags and warning as error"
      patch_type: "portability"
    - patch_file: "patches/009-1.12.0-windows-msvc-runtime.patch"
      patch_description: "dont hardcode value of CMAKE_MSVC_RUNTIME_LIBRARY"
      patch_type: "conan"
```

**Python Output**:

```json
{
  "type": "conan",
  "version": "1.12.0",
  "download_url": "https://github.com/libcpr/cpr/archive/refs/tags/1.12.0.tar.gz",
  "sha256": "f64b501de66e163d6a278fbb6a95f395ee873b7a66c905dd785eae107266a709"
}
```

**Rust Output**:

```json
{
  "type": "conan",
  "version": "1.12.0",
  "download_url": "https://github.com/libcpr/cpr/archive/refs/tags/1.12.0.tar.gz",
  "sha256": "f64b501de66e163d6a278fbb6a95f395ee873b7a66c905dd785eae107266a709",
  "extra_data": {
    "mirror_urls": [
      "https://github.com/libcpr/cpr/archive/refs/tags/1.12.0.tar.gz",
      "https://mirror.example.com/cpr-1.12.0.tar.gz"
    ],
    "patches": [
      {
        "patch_file": "patches/008-1.12.0-remove-warning-flags.patch",
        "patch_description": "disable warning flags and warning as error",
        "patch_type": "portability"
      },
      {
        "patch_file": "patches/009-1.12.0-windows-msvc-runtime.patch",
        "patch_description": "dont hardcode value of CMAKE_MSVC_RUNTIME_LIBRARY",
        "patch_type": "conan"
      }
    ]
  }
}
```

## Value

- **Build reproducibility**: Patch information for exact source reconstruction
- **Download resilience**: Mirror URLs for fallback when primary source unavailable
- **Patch tracking**: Know what modifications applied to upstream sources
- **Compliance**: Document source modifications for license compliance
- **Security**: Track patches that fix vulnerabilities

## Test Coverage

9 comprehensive test cases:

- Basic sources extraction
- Multiple URLs (mirrors)
- Patches with full metadata
- Mirror URLs in extra_data
- Patches without matching source version
- Missing fields handling
- Empty sources
- Invalid YAML

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/conan.py`
- Rust implementation: `src/parsers/conan_data.rs`
- conandata.yml spec: https://docs.conan.io/2/tutorial/creating_packages/handle_sources_in_packages.html
- Conan Center format: https://github.com/conan-io/conan-center-index/blob/master/docs/adding_packages/conandata_yml_format.md
