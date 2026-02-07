# Composer Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Test Status

**Currently Passing:** 1/1 tests

- ✅ `test_golden_composer_lock` - Passing with enhancements

## Enhancements Over Python ScanCode

### Richer Dependency Metadata

The Rust implementation provides significantly more metadata in dependency `extra_data` compared to the original Python implementation:

**Fields added to dependency `extra_data`:**
- `source_type`: Version control type (e.g., "git")
- `source_url`: Repository URL (e.g., "https://github.com/doctrine/cache.git")
- `source_reference`: VCS commit hash
- `dist_type`: Distribution archive type (e.g., "zip")
- `dist_url`: Distribution download URL
- `dist_reference`: Distribution commit reference
- `type`: Package type (e.g., "library")

**Rationale:**
This metadata is useful for:
- **Package provenance tracking** - Know exactly where packages come from
- **Reproducible builds** - Pin to specific commits and distributions
- **Security auditing** - Verify package sources and integrity
- **Dependency resolution** - Better understanding of package relationships

**Example:**
```json
"extra_data": {
  "source_type": "git",
  "source_url": "https://github.com/doctrine/cache.git",
  "source_reference": "eb152c5100571c7a45470ff2a35095ab3f3b900b",
  "dist_type": "zip",
  "dist_url": "https://api.github.com/repos/doctrine/cache/zipball/eb152c5100571c7a45470ff2a35095ab3f3b900b",
  "dist_reference": "eb152c5100571c7a45470ff2a35095ab3f3b900b",
  "type": "library"
}
```

This is an **intentional improvement** over the Python implementation, providing users with more complete package information without compromising compatibility.

## Test Data

Test files sourced from Python ScanCode reference:
- `reference/scancode-toolkit/tests/packagedcode/data/phpcomposer/`
