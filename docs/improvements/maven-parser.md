# Maven Parser: Beyond-Parity Improvements

**Parser**: `MavenPomXmlParser`  
**File**: `src/parsers/maven.rs`  
**Python Reference**: `reference/scancode-toolkit/src/packagedcode/maven.py`

## Summary

The Maven POM parser in scancode-rust improves on the Python reference in four areas:

1. **🔍 Enhanced Extraction**: SCM field separation — preserves both `connection` and `developerConnection` independently
2. **🔍 Enhanced Extraction**: `inception_year` extraction from `<inceptionYear>`
3. **🔍 Enhanced Extraction**: Consistent, shorter `extra_data` key naming
4. **🔍 Enhanced Extraction**: SCM URL normalization applied independently to both SCM fields

## Improvement 1: SCM Field Separation

### Python Behavior

Python merges the `<scm>` element's `connection` and `developerConnection` fields into a single `vcs_url`. Whichever value is found last wins, and the other is silently discarded. This loses information when both fields are present — a common case in real-world POMs where `connection` provides anonymous read access and `developerConnection` provides authenticated write access.

### Rust Behavior

Rust maps them separately: `connection` populates `vcs_url`, while `developerConnection` is stored in `extra_data.scm_developer_connection`. Both values are preserved, giving downstream consumers full visibility into the project's SCM configuration.

## Improvement 2: Inception Year Extraction

### Python Behavior

Python does not extract the `<inceptionYear>` element from POM files. This field is part of the Maven POM specification and indicates when the project was first created.

### Rust Behavior

Rust extracts `<inceptionYear>` into `extra_data.inception_year`. This is useful for provenance tracking, license compliance (copyright date ranges), and understanding project maturity.

## Improvement 3: Consistent extra_data Key Naming

### Python Behavior

Python uses verbose, inconsistent key names in `extra_data` for CI and issue tracking metadata: `issue_management_system`, `ci_management_system`, and `ci_management_url`.

### Rust Behavior

Rust uses shorter, consistent keys: `issue_tracking_system`, `ci_system`, and `ci_url`. These names are cleaner, avoid redundant `_management` suffixes, and align better with the corresponding top-level field names (e.g., `bug_tracking_url` pairs naturally with `issue_tracking_system`).

## Improvement 4: SCM URL Normalization

### Python Behavior

Python normalizes the `scm:git:` prefix to `git+` on the merged `vcs_url`, but since it collapses both SCM fields into one, only the surviving value gets normalized.

### Rust Behavior

Rust applies `scm:git:` → `git+` prefix normalization independently to both `connection` (stored in `vcs_url`) and `developerConnection` (stored in `extra_data.scm_developer_connection`). Both URLs receive consistent normalization.

## Why This Matters

- **Data preservation**: No information is silently discarded from the POM
- **SBOM completeness**: Inception year and full SCM details improve software bill of materials quality
- **Downstream tooling**: Separate SCM URLs let tools distinguish read-only from read-write repository access
- **Consistency**: Shorter, uniform key names reduce confusion and simplify programmatic access to `extra_data`

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/maven.py`
- Rust implementation: `src/parsers/maven.rs`
- Maven POM reference: <https://maven.apache.org/pom.html>
