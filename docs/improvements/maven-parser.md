# Maven Parser: Beyond-Parity Improvements

**Parser**: `MavenParser`  
**File**: `src/parsers/maven.rs`  
**Python Reference**: `reference/scancode-toolkit/src/packagedcode/maven.py`

## Summary

The Maven POM parser in scancode-rust now improves on the Python reference in eight important areas:

1. **🔍 Enhanced Extraction**: description handling now matches Maven `name` + `description` semantics without duplicating identical values
2. **✨ New Feature**: `dependencyManagement` entries are surfaced as dependency records instead of being preserved only as opaque metadata
3. **✨ New Feature**: package qualifiers, source package PURLs, and packaging-aware download URLs are emitted for Maven packages
4. **✨ New Feature**: relocation metadata from `distributionManagement.relocation` is extracted and preserved
5. **🐛 Bug Fix**: extracted license statements are rendered as structured Maven license records instead of flattened display text
6. **🐛 Bug Fix**: organization parties use the correct `owner` role instead of the Python typo tracked by issue #211
7. **✨ New Feature**: Maven 4.1.0 POMs are accepted and tested
8. **🔍 Enhanced Extraction**: packaging aliases and property-resolved dependency scope/optional values are normalized after property resolution

## Improvement 1: Description De-duplication

### Python Behavior

Python combines `<name>` and `<description>` when both exist, but the existing Rust implementation previously dropped top-level POM descriptions entirely.

### Rust Behavior

Rust now emits `description` using Maven-aware rules:

- if only `<name>` exists, use it
- if only `<description>` exists, use it
- if both exist and are identical, keep one value
- if both exist and differ, join them with a newline

This matches the useful ScanCode behavior while fixing the missing-description gap and the duplicated-name issue.

## Improvement 2: dependencyManagement Dependencies

### Python Behavior

Python surfaces `dependencyManagement` entries as dependency records with synthetic scopes such as `dependencymanagement` and `import`.

### Rust Behavior

Rust now does the same. Managed entries are no longer hidden only in `extra_data.dependency_management`; they are also emitted as first-class dependencies, while the raw management metadata is still preserved in `extra_data`.

This improves dependency visibility for BOMs and managed dependency sets without losing the original Maven structure.

## Improvement 3: Maven Qualifiers, Source Packages, and Packaging-aware Downloads

### Python Behavior

Python emits package qualifiers for Maven `classifier` and non-default package `type`, and it also generates a `?classifier=sources` source package PURL.

### Rust Behavior

Rust now emits:

- package qualifiers for `classifier` and normalized `type`
- source package PURLs such as `pkg:maven/foo/bar@1.2.3?classifier=sources`
- packaging-aware repository download URLs

Rust also normalizes Maven packaging aliases like `maven-plugin` to the correct jar-style artifact extension instead of using raw packaging values verbatim.

## Improvement 4: Relocation Metadata Support

### Python Behavior

The current Python reference does not preserve Maven `distributionManagement.relocation` as structured output.

### Rust Behavior

Rust extracts relocation coordinates and messages, preserves them in `extra_data.relocation`, and emits a relocation dependency when coordinates are present. Message-only relocation notices are also retained so no relocation warning text is silently lost.

## Improvement 5: Structured License Statement Rendering

### Python Behavior

Python keeps Maven license records as structured normalized data before license detection, including name and URL, instead of reducing them to plain display text.

### Rust Behavior

Rust now renders `extracted_license_statement` as structured Maven license records, preserving `name`, `url`, and `comments` when present. This fixes the earlier “weird display” behavior where only the license name survived.

## Improvement 6: Correct Owner Party Role

### Python Behavior

The issue backlog documents a typo/problem around party role handling for Maven organization ownership metadata.

### Rust Behavior

Rust now emits organization ownership parties with the correct `owner` role, preserving organization name and URL as structured party data.

## Improvement 7: Maven 4.1.0 Support

### Python Behavior

Python-era assumptions were centered on 4.0.0 POMs.

### Rust Behavior

Rust now explicitly tests and accepts `modelVersion` 4.1.0 POMs, including qualifier-bearing packages, managed imports, and relocation metadata. This keeps the parser compatible with modern Maven metadata.

## Improvement 8: Post-resolution Dependency Normalization

### Python Behavior

The Python reference still carries TODOs around some dependency qualifier/type handling.

### Rust Behavior

Rust now resolves dependency scope, optional flags, classifier, and type after property substitution and then recomputes `scope`, `is_runtime`, `is_optional`, dependency PURLs, and pinning from the resolved values. This avoids incorrect dependency flags when Maven properties drive dependency metadata.

## Why This Matters

- **Parity with Maven semantics**: dependency management, relocations, classifiers, and modern POM versions are now represented explicitly
- **Better SBOM fidelity**: richer descriptions, source packages, structured licenses, and normalized dependency flags improve downstream package analysis
- **Less lossy output**: key Maven metadata is preserved instead of being flattened or discarded
- **Stronger regression coverage**: unit, parser-golden, and assembly-golden tests now lock in these behaviors

## References

- Python implementation: `reference/scancode-toolkit/src/packagedcode/maven.py`
- Rust implementation: `src/parsers/maven.rs`
- Maven POM reference: <https://maven.apache.org/pom.html>
- Maven dependency mechanism: <https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html>
