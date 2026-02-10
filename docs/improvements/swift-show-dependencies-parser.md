# SwiftShowDependenciesParser: Full Dependency Graph Extraction

**Parser**: `SwiftShowDependenciesParser`  
**File**: `src/parsers/swift_show_dependencies.rs`  
**Python Reference**: `src/packagedcode/swift.py` (SwiftShowDependenciesDepLockHandler)

## Summary

**🔍 Enhanced Extraction**: Python only extracts package name. Rust extracts full dependency graph with versions, URLs, and direct/transitive marking.

## Python Limitation

The Python implementation extracts minimal metadata:

```python
@classmethod
def _parse(cls, swift_dependency_relation, package_only=False):
    dependencies = get_flatten_dependencies(
        dependency_tree=swift_dependency_relation.get("dependencies")
    )

    package_data = dict(
        datasource_id=cls.datasource_id,
        type=cls.default_package_type,
        primary_language=cls.default_primary_language,
        name=swift_dependency_relation.get("name"),  # ONLY NAME
        dependencies=dependencies,
    )
    
    return models.PackageData.from_data(package_data, package_only)
```

**Missing**:

- Package version
- Package URL
- Dependency versions
- Direct vs transitive dependency marking
- GitHub-based PURLs

## Rust Enhancement

Extracts complete dependency graph with full metadata:

### Additional Fields Extracted

1. **Root Package**:
   - `name` - Package name
   - `version` - Package version
   - `homepage_url` - Package URL (from `url` field)

2. **Dependencies** (flattened tree):
   - `purl` - Package URL (GitHub-aware: `pkg:swift/github.com/owner/repo`)
   - `extracted_requirement` - Resolved version
   - `is_direct` - `true` for direct deps, `false` for transitive
   - `is_pinned` - `true` if version is not "unspecified"
   - `scope` - Always "dependencies"
   - `is_runtime` - Always `true`

### Implementation Approach

The parser uses breadth-first traversal to flatten the nested dependency tree:

1. **Queue-based traversal**: Processes dependencies level-by-level
2. **Direct marking**: First-level dependencies marked as `is_direct=true`, nested ones as `false`
3. **GitHub-aware PURLs**: Detects GitHub URLs and generates proper namespaced PURLs (`pkg:swift/github.com/owner/repo`)
4. **Version pinning detection**: Marks dependencies with resolved versions as pinned
5. **Transitive expansion**: Recursively adds all nested dependencies to the flat list

### Real-World Example

**Input** (`swift-show-dependencies.deplock`):

```json
{
  "name": "VercelUI",
  "version": "1.0.0",
  "url": "https://github.com/vercel/VercelUI",
  "dependencies": [
    {
      "identity": "vercel",
      "name": "Vercel",
      "url": "https://github.com/swift-cloud/Vercel",
      "version": "1.15.2",
      "dependencies": [
        {
          "identity": "vapor",
          "name": "vapor",
          "url": "https://github.com/vapor/vapor",
          "version": "4.79.0",
          "dependencies": []
        }
      ]
    },
    {
      "identity": "swift-nio",
      "name": "swift-nio",
      "url": "https://github.com/apple/swift-nio.git",
      "version": "2.58.0",
      "dependencies": []
    }
  ]
}
```

**Python Output**:

```json
{
  "type": "swift",
  "primary_language": "Swift",
  "name": "VercelUI",
  "dependencies": [
    {
      "purl": "pkg:swift/Vercel",
      "scope": "dependencies"
    },
    {
      "purl": "pkg:swift/vapor",
      "scope": "dependencies"
    },
    {
      "purl": "pkg:swift/swift-nio",
      "scope": "dependencies"
    }
  ]
}
```

**Rust Output**:

```json
{
  "type": "swift",
  "primary_language": "Swift",
  "name": "VercelUI",
  "version": "1.0.0",
  "homepage_url": "https://github.com/vercel/VercelUI",
  "dependencies": [
    {
      "purl": "pkg:swift/github.com/swift-cloud/Vercel",
      "extracted_requirement": "1.15.2",
      "scope": "dependencies",
      "is_runtime": true,
      "is_optional": false,
      "is_pinned": true,
      "is_direct": true
    },
    {
      "purl": "pkg:swift/github.com/vapor/vapor",
      "extracted_requirement": "4.79.0",
      "scope": "dependencies",
      "is_runtime": true,
      "is_optional": false,
      "is_pinned": true,
      "is_direct": false
    },
    {
      "purl": "pkg:swift/github.com/apple/swift-nio",
      "extracted_requirement": "2.58.0",
      "scope": "dependencies",
      "is_runtime": true,
      "is_optional": false,
      "is_pinned": true,
      "is_direct": true
    }
  ]
}
```

## Key Improvements

1. **Version Information**: Resolved versions for all dependencies
2. **Direct vs Transitive**: `is_direct` flag distinguishes dependency levels
3. **GitHub-Aware PURLs**: Proper package URLs with GitHub namespace
4. **Package Metadata**: Root package version and URL
5. **Pinned Detection**: Identifies locked vs floating versions

## Value

- **Dependency auditing**: Know exact versions in use
- **Security scanning**: Identify vulnerable dependency versions
- **License compliance**: Track transitive dependency licenses
- **Build reproducibility**: Complete dependency graph with versions
- **Supply chain security**: Distinguish direct from transitive dependencies

## Test Coverage

5 comprehensive test cases:

- Basic package name extraction
- Full dependency graph with nesting
- Direct vs transitive marking
- GitHub PURL generation
- Empty dependencies handling
- Invalid JSON handling

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/swift.py`
- Rust implementation: `src/parsers/swift_show_dependencies.rs`
- Swift Package Manager: <https://docs.swift.org/swiftpm/documentation/packagemanagerdocs/packageshowdependencies/>
- Discussion: <https://forums.swift.org/t/swiftpm-show-dependencies-without-fetching-dependencies/51154>
