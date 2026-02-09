# Gradle Lockfile Parser: Improvements Over Python

## Summary

Our Rust implementation improves on the Python reference by:

- ✨ **New Format: gradle.lockfile** — Python has no parser for Gradle's dependency locking format

## Problem in Python Reference

Python ScanCode handles `build.gradle` and `build.gradle.kts` files but has **no support** for `gradle.lockfile` — the text-based format Gradle uses to lock exact dependency versions.

Without lockfile parsing, Python ScanCode cannot report the pinned, resolved dependency versions that a Gradle project actually uses in production.

## Our Solution

We implemented `GradleLockfileParser` which extracts resolved dependency information from `gradle.lockfile` files.

### Format

The `gradle.lockfile` format is a simple text file with one dependency per line:

```text
# This is a Gradle generated file for dependency locking.
com.google.guava:guava:30.1-jre=abc123
org.springframework.boot:spring-boot-starter-web:2.7.0=def456
org.junit.jupiter:junit-jupiter-api:5.8.0=hash789
```

Each line follows the format: `group:artifact:version=hash`

### Before/After Comparison

**Python Output**: *(no parser exists)*

```json
// File not recognized — no output
```

**Rust Output**:

```json
{
  "type": "maven",
  "datasource_id": "gradle_lockfile",
  "dependencies": [
    {
      "purl": "pkg:maven/com.google.guava/guava@30.1-jre",
      "is_pinned": true,
      "is_runtime": true,
      "is_optional": false,
      "resolved_package": {
        "type": "maven",
        "namespace": "com.google.guava",
        "name": "guava",
        "version": "30.1-jre"
      },
      "extra_data": {
        "group": "com.google.guava",
        "artifact": "guava",
        "hash": "abc123"
      }
    }
  ]
}
```

## What Gets Extracted

| Field | Source | Description |
|-------|--------|-------------|
| `purl` | GAV coordinates | Maven-style Package URL |
| `is_pinned` | always `true` | Lockfiles contain exact versions |
| `resolved_package` | GAV coordinates | Full resolved package details |
| `extra_data.group` | group segment | Maven group ID |
| `extra_data.artifact` | artifact segment | Maven artifact ID |
| `extra_data.hash` | hash after `=` | Dependency hash for verification |

### Key Design Decisions

- **Package type is `maven`** — Gradle lockfile dependencies are Maven artifacts
- **All dependencies are pinned** — By definition, lockfiles contain resolved versions
- **Hash preservation** — The hash after `=` is stored in `extra_data` for integrity verification
- **Comments and empty lines** are skipped gracefully

## Impact

- **SBOM accuracy**: Lockfiles represent the actual dependency versions used in production, not just declared ranges
- **Reproducibility**: Enables auditing of exact dependency versions across builds
- **Supply chain security**: Pinned versions with hashes enable integrity verification

## References

### Python Reference

- Python ScanCode has no `gradle.lockfile` handler

### Gradle Documentation

- [Gradle Dependency Locking](https://docs.gradle.org/current/userguide/dependency_locking.html)

## Status

- ✅ **Implementation**: Complete, validated, production-ready
- ✅ **Testing**: Unit tests covering all formats, edge cases, and malformed input
- ✅ **Documentation**: Complete
