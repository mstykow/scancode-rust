# npm Parser: Git URL and Non-Version Dependency Handling

## Summary

The npm parser in scancode-rust **fixes a bug** present in the Python reference implementation:

- **🐛 Bug Fix**: Dependencies with Git URLs, GitHub shortcuts, and local paths were incorrectly treated as pinned versions, producing malformed PURLs

## Bug Description

### Python Implementation (BROKEN)

**Location**: `reference/scancode-toolkit/src/packagedcode/npm.py`

**Problem**: Python treats all dependency values as potential versions, including Git URLs and local paths:

```json
{
  "dependencies": {
    "lodash": "git+https://github.com/lodash/lodash.git#v4.17.21",
    "my-lib": "github:user/repo#main",
    "local-pkg": "file:../local-pkg"
  }
}
```

**Python Output** (INCORRECT):

```json
{
  "dependencies": [
    {
      "purl": "pkg:npm/lodash@git+https://github.com/lodash/lodash.git#v4.17.21",
      "is_pinned": true
    }
  ]
}
```

This is wrong because:

1. The PURL includes the Git URL as a "version" - this is invalid PURL syntax
2. `is_pinned` should be `false` for non-version dependencies
3. The dependency cannot be resolved to a specific version without network access

### Our Rust Implementation (FIXED)

**Location**: `src/parsers/npm.rs`

**Approach**: Detect and properly handle non-version dependency values:

```rust
fn is_non_version_dependency(version: &str) -> bool {
    let v = version.trim();

    // Git URLs
    if v.starts_with("git://")
        || v.starts_with("git+ssh://")
        || v.starts_with("git+https://")
        || v.starts_with("git@")
    {
        return true;
    }

    // Platform shortcuts
    if v.starts_with("github:")
        || v.starts_with("gitlab:")
        || v.starts_with("bitbucket:")
        || v.starts_with("gist:")
    {
        return true;
    }

    // URLs and local paths
    if v.starts_with("http://")
        || v.starts_with("https://")
        || v.starts_with("file:")
        || v.starts_with("link:")
    {
        return true;
    }

    // GitHub shorthand: user/repo#branch
    if v.contains('/') && !v.starts_with('@') {
        // Check for GitHub shorthand pattern
        if let Some((_, after_hash)) = v.rsplit_once('#') {
            if after_hash.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_') {
                return true;
            }
        }
    }

    false
}
```

**Rust Output** (CORRECT):

```json
{
  "dependencies": [
    {
      "purl": "pkg:npm/lodash",
      "extracted_requirement": "git+https://github.com/lodash/lodash.git#v4.17.21",
      "is_pinned": false,
      "is_direct": true
    }
  ]
}
```

## Supported Non-Version Formats

| Format | Example | PURL Generated | is_pinned |
|--------|---------|----------------|-----------|
| Git URL | `git://github.com/user/repo.git` | `pkg:npm/package-name` | `false` |
| Git+HTTPS | `git+https://github.com/user/repo.git` | `pkg:npm/package-name` | `false` |
| Git+SSH | `git+ssh://git@github.com:user/repo.git` | `pkg:npm/package-name` | `false` |
| GitHub shorthand | `github:user/repo` | `pkg:npm/package-name` | `false` |
| GitHub with branch | `user/repo#main` | `pkg:npm/package-name` | `false` |
| GitLab shorthand | `gitlab:user/repo` | `pkg:npm/package-name` | `false` |
| Bitbucket shorthand | `bitbucket:user/repo` | `pkg:npm/package-name` | `false` |
| HTTP URL | `https://example.com/pkg.tgz` | `pkg:npm/package-name` | `false` |
| Local file | `file:../local-pkg` | `pkg:npm/package-name` | `false` |
| Link protocol | `link:../linked-pkg` | `pkg:npm/package-name` | `false` |

## Comparison with Python

| Aspect | Python | Rust |
|--------|--------|------|
| Git URL detection | ❌ None | ✅ Full support |
| GitHub shorthand | ❌ Treated as version | ✅ Properly detected |
| Local paths | ❌ Treated as version | ✅ Properly detected |
| is_pinned for non-version | ❌ Incorrectly `true` | ✅ Correctly `false` |
| PURL for non-version deps | ❌ Invalid (includes URL) | ✅ Valid (package name only) |

## Test Coverage

- `test_git_url_dependencies()` - Git URLs, GitHub shortcuts
- `test_url_dependencies()` - HTTP/HTTPS tarball URLs
- `test_local_path_dependencies()` - `file:` and `link:` protocols
- `test_mixed_dependencies()` - Mix of regular versions and non-version deps

## Impact

This fix ensures:

1. **Valid PURLs**: No malformed PURLs with URLs as versions
2. **Correct metadata**: `is_pinned` accurately reflects dependency resolution status
3. **Downstream compatibility**: Tools consuming ScanCode output can correctly identify unresolved dependencies
4. **SBOM accuracy**: Dependency graphs correctly distinguish between pinned and unpinned dependencies

## Reference

- **Issue**: ScanCode #2509 - npm weird dependency versions
- **Location**: `src/parsers/npm.rs` - `is_non_version_dependency()` function
