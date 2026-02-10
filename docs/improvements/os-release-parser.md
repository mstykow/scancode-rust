# OsReleaseParser: Bug Fix + URL Field Extraction

**Parser**: `OsReleaseParser`  
**File**: `src/parsers/os_release.rs`  
**Python Reference**: `src/packagedcode/distro.py` (EtcOsReleaseHandler)

## Summary

1. **🐛 Bug Fix**: Fixed incorrect logic in the Python implementation that would incorrectly identify regular Debian distributions as "distroless"
2. **🔍 Enhanced Extraction**: Extract URL fields (`HOME_URL`, `SUPPORT_URL`, `BUG_REPORT_URL`) that Python ignores

## Bug in Python Implementation

The Python code has a logic error in the namespace determination:

```python
if distro_identifier == 'debian':
    namespace = 'debian'

    if 'distroless' in pretty_name:
        name = 'distroless'
    elif pretty_name.startswith('debian'):
        name = 'distroless'  # BUG: Should be 'debian', not 'distroless'
```

**Problem**: When `PRETTY_NAME` starts with "debian" (which is the case for standard Debian distributions), the code incorrectly sets `name = 'distroless'` instead of `name = 'debian'`.

**Impact**: Regular Debian installations would be misidentified as Google's Distroless container images.

## Correct Implementation in Rust

```rust
fn determine_namespace_and_name<'a>(
    id: &'a str,
    id_like: Option<&'a str>,
    pretty_name: &'a str,
) -> (&'a str, &'a str) {
    match id {
        "debian" => {
            let name = if pretty_name.contains("distroless") {
                "distroless"
            } else {
                "debian"  // FIXED: Correctly defaults to "debian"
            };
            ("debian", name)
        }
        // ... rest of logic
    }
}
```

**Fix**: The Rust implementation correctly:

1. Checks if `pretty_name` contains "distroless" → sets name to "distroless"
2. Otherwise → defaults to "debian" (not "distroless")

## Test Coverage

The fix is validated by unit tests:

```rust
#[test]
fn test_parse_debian() {
    let content = r#"
ID=debian
PRETTY_NAME="Debian GNU/Linux 11 (bullseye)"
VERSION_ID="11"
"#;
    let pkg = parse_os_release(content);
    
    assert_eq!(pkg.namespace.as_deref(), Some("debian"));
    assert_eq!(pkg.name.as_deref(), Some("debian"));  // Not "distroless"
    assert_eq!(pkg.version.as_deref(), Some("11"));
}

#[test]
fn test_parse_distroless() {
    let content = r#"
ID=debian
PRETTY_NAME="Distroless"
VERSION_ID="11"
"#;
    let pkg = parse_os_release(content);
    
    assert_eq!(pkg.namespace.as_deref(), Some("debian"));
    assert_eq!(pkg.name.as_deref(), Some("distroless"));  // Correctly identified
}
```

## Real-World Impact

**Before (Python bug)**:

- Standard Debian 11 → Incorrectly identified as "distroless"
- Distroless containers → Correctly identified as "distroless"

**After (Rust fix)**:

- Standard Debian 11 → Correctly identified as "debian"
- Distroless containers → Correctly identified as "distroless"

## Enhancement: URL Field Extraction

### Python Limitation

The Python implementation only extracts:

- `namespace` (distro family)
- `name` (distro name)
- `version` (version ID)

It **ignores** all URL fields present in os-release files.

### Rust Enhancement

We extract additional fields from the os-release spec:

```rust
let homepage_url = fields.get("HOME_URL").cloned();
let bug_tracking_url = fields.get("BUG_REPORT_URL").cloned();
let code_view_url = fields.get("SUPPORT_URL").cloned();
```

**Mapping**:

- `HOME_URL` → `homepage_url`
- `BUG_REPORT_URL` → `bug_tracking_url`
- `SUPPORT_URL` → `code_view_url`

### Real-World Example

**Debian 11 os-release**:

```ini
ID=debian
VERSION_ID="11"
HOME_URL="https://www.debian.org/"
SUPPORT_URL="https://www.debian.org/support"
BUG_REPORT_URL="https://bugs.debian.org/"
```

**Python output**: No URLs extracted  
**Rust output**: All three URLs populated in PackageData

### Value

- **SBOM completeness**: Provides links to project resources
- **Vulnerability reporting**: Direct link to bug tracker
- **Documentation**: Homepage for distro information
- **Support**: Support URL for enterprise users

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/distro.py`
- Rust implementation: `src/parsers/os_release.rs`
- os-release spec: https://www.freedesktop.org/software/systemd/man/os-release.html
