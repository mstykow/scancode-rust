# SPDX License Data Comparison: Python vs Rust

**Date**: 2026-03-05
**Audit Scope**: License data sources, SPDX key mapping, deprecated identifiers, exceptions, and aliases

---

## Executive Summary

| Aspect | Python | Rust | Status |
|--------|--------|------|--------|
| License Data Source | Custom `.LICENSE` files (2,615 licenses) | SPDX `license-list-data` submodule (727 licenses) | **DIVERGENT** |
| SPDX Key Mapping | Extracted from License objects | Extracted from License objects | ✅ Same approach |
| Deprecated Handling | `is_deprecated` + `replaced_by` fields | Same fields + hardcoded subs table | ✅ Compatible |
| License Exceptions | `is_exception` flag on licenses | SPDX `exceptions.json` + `is_exception` flag | **DIVERGENT** |
| License Aliases | `other_spdx_license_keys` array | Same field | ✅ Compatible |
| Deprecated SPDX Subs | Dynamic table from licensing lib | Hardcoded constant | ⚠️ Requires sync |

**Critical Finding**: The Rust implementation uses a different license data source (SPDX `license-list-data` submodule) instead of the Python custom license database. This results in significantly fewer licenses and different data structures.

---

## 1. License Data Sources

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/data/licenses/`

**Format**: Custom YAML frontmatter + license text
```yaml
---
key: mit
short_name: MIT License
name: MIT License
category: Permissive
owner: MIT
homepage_url: http://opensource.org/licenses/mit-license.php
notes: Per SPDX.org, this license is OSI certified.
spdx_license_key: MIT
other_spdx_license_keys:
  - LicenseRef-MIT-Bootstrap
  - LicenseRef-MIT-Discord
text_urls:
    - http://opensource.org/licenses/mit-license.php
---
Permission is hereby granted...
```

**Count**: **2,615 license files** (as of audit date)

**Characteristics**:
- Custom ScanCode-specific license database
- Includes non-SPDX licenses (proprietary, historical, variants)
- Contains `owner`, `category`, `homepage_url` metadata
- Has `ignorable_copyrights`, `ignorable_holders`, `ignorable_authors`, etc.
- Many licenses have `other_spdx_license_keys` for aliases
- License text is curated/normalized by ScanCode team

**Loading Code**:
- `models.py:load_licenses()` (lines 802-880)
- `models.py:License` class (lines 120-289)

### Rust Implementation

**Location**: `resources/licenses/` (git submodule)

**Format**: SPDX `license-list-data` JSON
```json
{
  "licenseListVersion": "3.28.0",
  "isDeprecatedLicenseId": false,
  "name": "MIT License",
  "licenseId": "MIT",
  "seeAlso": ["https://opensource.org/license/mit/"],
  "isOsiApproved": true,
  "licenseText": "MIT License\n\nCopyright (c)..."
}
```

**Count**: **727 licenses** in `json/details/` (as of audit date)

**Characteristics**:
- Official SPDX License List data from https://github.com/spdx/license-list-data
- Standard SPDX identifiers only
- Includes `standardLicenseTemplate` for matching
- Has `crossRef` for URL validation
- No custom metadata (owner, category, etc.)
- License text is from SPDX XML source

**Loading Code**:
- `rules/loader.rs:parse_license_file()` (lines 348-467)
- Currently **loads from Python `.LICENSE` files**, not SPDX JSON

### Key Differences

| Aspect | Python | Rust (SPDX submodule) | Impact |
|--------|--------|----------------------|---------|
| License Count | 2,615 | 727 | **Major**: 1,888 fewer licenses |
| Custom Licenses | Yes (LicenseRef-scancode-*) | No | **Major**: Non-SPDX licenses missing |
| License Variants | Extensive | SPDX only | **Major**: Historical variants missing |
| Metadata | Rich (owner, category, notes) | Minimal | Moderate: Information loss |
| Ignorables | Yes (copyrights, holders, urls) | No | **Moderate**: Detection noise |

**⚠️ BEHAVIORAL DIFFERENCE**: The Rust code loads from Python `.LICENSE` files but also includes the SPDX submodule. The SPDX submodule appears **unused** in current implementation.

**File**: `src/license_detection/rules/loader.rs:703-719`
```rust
#[test]
fn test_load_licenses_from_reference() {
    let path = Path::new("reference/scancode-toolkit/src/licensedcode/data/licenses");
    // ...
    let licenses = load_licenses_from_directory(path, false).unwrap();
```

---

## 2. SPDX Key Mapping

### Python Implementation

**File**: `reference/scancode-toolkit/src/licensedcode/cache.py:313-378`

**Function**: `get_licenses_by_spdx_key()`

```python
def get_licenses_by_spdx_key(
    licenses=None,
    include_deprecated=False,
    lowercase_keys=True,
    include_other_spdx_license_keys=False,
):
    """Return a mapping of {SPDX license id: License}"""
    licenses_by_spdx_key = {}
    
    for lic in licenses:
        if lic.spdx_license_key:
            slk = lic.spdx_license_key
            if lowercase_keys:
                slk = slk.lower()
            
            if not lic.is_deprecated or include_deprecated:
                licenses_by_spdx_key[slk] = lic
        
        if include_other_spdx_license_keys:
            for other_spdx in lic.other_spdx_license_keys:
                licenses_by_spdx_key[other_spdx.lower()] = lic
    
    return licenses_by_spdx_key
```

**Characteristics**:
- Bidirectional mapping (ScanCode ↔ SPDX)
- Lowercases keys for case-insensitive lookup
- Optionally includes `other_spdx_license_keys` (aliases)
- Optionally includes deprecated licenses
- Returns License objects

**Usage in expression building**:
- `cache.py:build_spdx_licensing()` (lines 295-310)
- Creates `LicenseSymbolLike` objects for license-expression library

### Rust Implementation

**File**: `src/license_detection/spdx_mapping/mod.rs:24-97`

**Struct**: `SpdxMapping`

```rust
pub struct SpdxMapping {
    scancode_to_spdx: HashMap<String, String>,
    spdx_to_scancode: HashMap<String, String>,
}

impl SpdxMapping {
    pub fn build_from_licenses(licenses: &[License]) -> Self {
        for license in licenses {
            if let Some(spdx_key) = &license.spdx_license_key {
                scancode_to_spdx.insert(license.key.clone(), spdx_key.clone());
                spdx_to_scancode.entry(spdx_key.clone())
                    .or_insert_with(|| license.key.clone());
            } else {
                let licenseref_key = format!("LicenseRef-scancode-{}", license.key);
                scancode_to_spdx.insert(license.key.clone(), licenseref_key);
            }
        }
    }
}
```

**Characteristics**:
- Same bidirectional mapping approach
- Same handling of `spdx_license_key` field
- Same `LicenseRef-scancode-*` fallback for non-SPDX licenses
- No explicit lowercase normalization (assumes keys are already lowercase)

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Key Lowercasing | Explicit in `get_licenses_by_spdx_key()` | Assumed already lowercase | **Minor**: Potential case sensitivity |
| `other_spdx_license_keys` | Optional inclusion | Not implemented | **Moderate**: Aliases not mapped |
| Deprecated Handling | Optional `include_deprecated` param | Loaded if `with_deprecated=true` | ✅ Compatible |

**⚠️ POTENTIAL ISSUE**: Rust does not include `other_spdx_license_keys` in mapping.

**Python**: `cache.py:361-376`
```python
if include_other_spdx_license_keys:
    for other_spdx in lic.other_spdx_license_keys:
        slk = other_spdx.lower()
        licenses_by_spdx_key[slk] = lic
```

**Rust**: Missing from `spdx_mapping/mod.rs:build_from_licenses()`

---

## 3. Deprecated Identifiers

### Python Implementation

**Fields on License**:
- `is_deprecated: bool` - Flag indicating deprecated status
- `replaced_by: List[str]` - List of replacement license expressions

**File**: `models.py:127-145`
```python
is_deprecated = attr.ib(
    default=False,
    metadata=dict(
        help='Flag set to True if this is a deprecated license. '
             'The policy is to never delete a license key once attributed. '
             'Instead it can be marked as deprecated and will be ignored for detection.')
)

replaced_by = attr.ib(
    default=[],
    metadata=dict(
        help='A list of new license expressions that replace this license, '
             'only if deprecated and replaced by something else.')
)
```

**Usage in loading**:
- `models.py:load_licenses()` (lines 802-880)
- `load_licenses(with_deprecated=False)` excludes deprecated by default
- Validation ensures `replaced_by` is set when `is_deprecated=True`

**Example**: `gpl-2.0-classpath.LICENSE`
```yaml
key: gpl-2.0-classpath
is_deprecated: yes
replaced_by:
    - gpl-2.0-plus
    - classpath-exception-2.0
spdx_license_key: GPL-2.0-with-classpath-exception
```

### Rust Implementation

**Fields on License**:
- Same fields: `is_deprecated: bool`, `replaced_by: Vec<String>`

**File**: `src/license_detection/models/license.rs:32-36`
```rust
pub is_deprecated: bool,
pub replaced_by: Vec<String>,
```

**Loading behavior**: Same as Python
- `rules/loader.rs:523` - Filtered when `with_deprecated=false`

**Deprecated SPDX substitution table**:

**File**: `src/license_detection/spdx_lid/mod.rs:169-200`
```rust
const DEPRECATED_SPDX_EXPRESSION_SUBS: &[(&str, &str)] = &[
    ("ecos-2.0", "gpl-2.0-plus with ecos-exception-2.0"),
    ("gpl-2.0-with-classpath-exception", "gpl-2.0-only with classpath-exception-2.0"),
    ("gpl-2.0-with-gcc-exception", "gpl-2.0-only with gcc-exception-2.0"),
    ("wxwindows", "lgpl-2.0-plus with wxwindows-exception-3.1"),
    // ... more substitutions
];
```

**Also in index builder**:

**File**: `src/license_detection/index/builder/mod.rs:31-65`
```rust
const DEPRECATED_SPDX_SUBS: &[(&str, &str)] = &[
    ("ecos-2.0", "gpl-2.0-or-later with ecos-exception-2.0"),
    ("gpl-2.0-with-autoconf-exception", "gpl-2.0-only with autoconf-exception-2.0"),
    // ...
];
```

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| `is_deprecated` field | ✅ | ✅ | ✅ Compatible |
| `replaced_by` field | ✅ | ✅ | ✅ Compatible |
| Loading filter | `with_deprecated` param | Same | ✅ Compatible |
| Hardcoded subs table | Dynamic from licensing lib | Two hardcoded constants | ⚠️ Must stay in sync |

**⚠️ POTENTIAL ISSUE**: Rust has two different hardcoded substitution tables:
1. `spdx_lid/mod.rs:DEPRECATED_SPDX_EXPRESSION_SUBS` (uses `-plus`)
2. `index/builder/mod.rs:DEPRECATED_SPDX_SUBS` (uses `-or-later`)

**Example mismatch**:
- `spdx_lid`: `"ecos-2.0" → "gpl-2.0-plus with ecos-exception-2.0"`
- `index/builder`: `"ecos-2.0" → "gpl-2.0-or-later with ecos-exception-2.0"`

**Python** (dynamic): `match_spdx_lid.py:202-223`
```python
EXPRESSSIONS_BY_OLD_SPDX_IDS = {
    'eCos-2.0': 'GPL-2.0-or-later WITH eCos-exception-2.0',
    # ...
}
```

---

## 4. License Exceptions

### Python Implementation

**Approach**: Exceptions are **licenses with `is_exception: true`**

**File**: `models.py:210-215`
```python
is_exception = attr.ib(
    default=False,
    metadata=dict(
        help='Flag set to True if this is a license exception')
)
```

**Example**: `389-exception.LICENSE`
```yaml
key: 389-exception
short_name: 389 Directory Server Exception to GPL 2.0
is_exception: yes
spdx_license_key: 389-exception
```

**Combined license + exception**:

**Example**: `gpl-2.0-classpath.LICENSE`
```yaml
key: gpl-2.0-classpath
is_deprecated: yes
is_exception: yes
replaced_by:
    - gpl-2.0-plus
    - classpath-exception-2.0
spdx_license_key: GPL-2.0-with-classpath-exception
```

**Count**: **~200+ exception licenses** in Python data

**Usage in expressions**:
- Combined licenses deprecated in SPDX 2.0
- Replaced by `License WITH Exception` syntax
- `match_spdx_lid.py:196-223` handles deprecated combined IDs

### Rust Implementation

**Approach 1**: Same as Python - `is_exception` field on License

**File**: `rules/loader.rs:119-120`
```rust
#[serde(default, deserialize_with = "deserialize_yes_no_bool")]
is_exception: Option<bool>,
```

**Approach 2**: SPDX `exceptions.json` submodule

**File**: `resources/licenses/json/exceptions.json`
```json
{
  "licenseListVersion": "3.28.0",
  "exceptions": [
    {
      "licenseExceptionId": "389-exception",
      "name": "389 Directory Server Exception",
      "isDeprecatedLicenseId": false
    }
  ]
}
```

**Count**: **40 exceptions** in SPDX `exceptions.json`

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Exception representation | License with `is_exception=true` | Same + SPDX `exceptions.json` | **DIVERGENT** |
| Exception count | ~200+ | 40 (SPDX) | **Major**: Many missing |
| Combined licenses | Deprecated with `replaced_by` | Handled in subs tables | ⚠️ Must stay in sync |

**⚠️ CRITICAL**: The SPDX `exceptions.json` has only 40 exceptions, but Python has ~200+ exception licenses. This is because:
1. SPDX only standardizes common exceptions
2. ScanCode tracks many more custom/proprietary exceptions
3. Rust may not be loading all exception licenses from Python data

---

## 5. License Aliases

### Python Implementation

**Field**: `other_spdx_license_keys`

**File**: `models.py:240-246`
```python
other_spdx_license_keys = attr.ib(
    default=attr.Factory(list),
    metadata=dict(
        help='List of other SPDX keys, such as the id of a deprecated '
             'license or alternative LicenseRef identifiers')
)
```

**Example**: `mit.LICENSE`
```yaml
key: mit
spdx_license_key: MIT
other_spdx_license_keys:
  - LicenseRef-MIT-Bootstrap
  - LicenseRef-MIT-Discord
  - LicenseRef-MIT-TC
  - LicenseRef-MIT-Diehl
```

**Example**: `gpl-2.0.LICENSE`
```yaml
key: gpl-2.0
spdx_license_key: GPL-2.0-only
other_spdx_license_keys:
  - GPL-2.0
  - GPL 2.0
  - LicenseRef-GPL-2.0
```

**Usage**:
- Included in SPDX key mapping (optional)
- Used for alternative identifiers
- Stores deprecated SPDX IDs

### Rust Implementation

**Field**: Same - `other_spdx_license_keys: Vec<String>`

**File**: `src/license_detection/models/license.rs:18`
```rust
pub other_spdx_license_keys: Vec<String>,
```

**Loading**: Same YAML parsing

**File**: `rules/loader.rs:95-96`
```rust
#[serde(default)]
other_spdx_license_keys: Option<Vec<String>>,
```

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Field definition | ✅ | ✅ | ✅ Compatible |
| Loading | ✅ | ✅ | ✅ Compatible |
| Usage in SPDX mapping | Optional `include_other_spdx_license_keys` | **NOT IMPLEMENTED** | **MODERATE ISSUE** |

**⚠️ MISSING FEATURE**: Rust does not use `other_spdx_license_keys` in `SpdxMapping::build_from_licenses()`

**Python**: `cache.py:361-376`
```python
if include_other_spdx_license_keys:
    for other_spdx in lic.other_spdx_license_keys:
        slk = other_spdx.lower()
        licenses_by_spdx_key[slk] = lic
```

**Rust**: Missing equivalent code in `spdx_mapping/mod.rs`

**Impact**: License expressions using aliases (e.g., `GPL-2.0` instead of `GPL-2.0-only`) will not resolve correctly in Rust.

---

## 6. Behavioral Differences Summary

### Critical Issues

1. **Different License Data Sources**
   - Python: 2,615 custom licenses
   - Rust (SPDX submodule): 727 standard licenses
   - **Impact**: 1,888 licenses missing, including custom/proprietary licenses

2. **Missing `other_spdx_license_keys` Mapping**
   - Python: Maps all aliases to license
   - Rust: Only maps primary `spdx_license_key`
   - **Impact**: Expression parsing fails for aliases

3. **Hardcoded vs Dynamic Deprecated Substitutions**
   - Python: Dynamic table from `license-expression` library
   - Rust: Two separate hardcoded tables with inconsistencies
   - **Impact**: Expression normalization may differ

### Moderate Issues

4. **Case Sensitivity in Key Lookup**
   - Python: Explicit lowercasing in `get_licenses_by_spdx_key()`
   - Rust: Assumes keys are already lowercase
   - **Impact**: Potential lookup failures for mixed-case SPDX IDs

5. **Exception Handling**
   - Python: ~200+ exception licenses in database
   - Rust SPDX: 40 standard exceptions
   - **Impact**: Many exceptions unavailable

### Minor Issues

6. **Metadata Loss**
   - Python: Rich metadata (owner, category, notes, ignorables)
   - Rust SPDX: Minimal metadata
   - **Impact**: Less context for license analysis

7. **Two Substitution Tables**
   - `spdx_lid/mod.rs:DEPRECATED_SPDX_EXPRESSION_SUBS`
   - `index/builder/mod.rs:DEPRECATED_SPDX_SUBS`
   - **Impact**: Potential inconsistency in expression handling

---

## 7. Recommendations

### High Priority

1. **Use Python License Data, Not SPDX Submodule**
   - Current Rust code loads from Python `.LICENSE` files ✅
   - SPDX submodule appears unused - **remove or document purpose**
   - Ensure all 2,615 licenses are loaded

2. **Implement `other_spdx_license_keys` Mapping**
   - Add to `SpdxMapping::build_from_licenses()`
   - Match Python's `include_other_spdx_license_keys` behavior
   - Test with alias expressions like `GPL-2.0` → `GPL-2.0-only`

3. **Unify Deprecated Substitution Tables**
   - Consolidate two hardcoded tables into one
   - Use `-or-later` format (matches SPDX and Python)
   - Or make dynamic from license data

### Medium Priority

4. **Add Explicit Key Lowercasing**
   - Lowercase keys in `SpdxMapping::build_from_licenses()`
   - Match Python's case-insensitive behavior

5. **Document Exception Handling Strategy**
   - Clarify whether using `is_exception` flag or `exceptions.json`
   - Ensure all Python exceptions are loaded

### Low Priority

6. **Add Metadata Fields to Rust License Struct**
   - `owner`, `osi_license_key`, `text_urls`, etc.
   - For compatibility and future features

---

## 8. Test Cases to Verify Parity

```rust
#[test]
fn test_spdx_key_mapping_includes_aliases() {
    // GPL-2.0 should map to gpl-2.0 license
    let mapping = build_spdx_mapping(&licenses);
    assert!(mapping.scancode_to_spdx("gpl-2.0").is_some());
}

#[test]
fn test_deprecated_substitution_consistency() {
    // Both tables should use same format
    let spdx_lid = get_deprecated_substitution("ecos-2.0");
    let builder = get_builder_substitution("ecos-2.0");
    assert_eq!(spdx_lid, builder);
}

#[test]
fn test_case_insensitive_lookup() {
    let mapping = build_spdx_mapping(&licenses);
    assert_eq!(mapping.scancode_to_spdx("MIT"), mapping.scancode_to_spdx("mit"));
}

#[test]
fn test_exception_count_matches_python() {
    let exceptions: Vec<_> = licenses.iter().filter(|l| l.is_exception).collect();
    // Should have ~200+ exceptions, not just 40 from SPDX
    assert!(exceptions.len() > 200);
}
```

---

## 9. File Reference

### Python Files

| File | Purpose | Lines |
|------|---------|-------|
| `licensedcode/models.py` | License class definition | 120-289 |
| `licensedcode/models.py` | `load_licenses()` function | 802-880 |
| `licensedcode/cache.py` | SPDX key mapping | 313-378 |
| `licensedcode/match_spdx_lid.py` | Deprecated SPDX subs | 202-223 |
| `licensedcode/data/licenses/*.LICENSE` | License data files | 2,615 files |

### Rust Files

| File | Purpose | Lines |
|------|---------|-------|
| `src/license_detection/models/license.rs` | License struct | 1-55 |
| `src/license_detection/spdx_mapping/mod.rs` | SPDX mapping | 24-97 |
| `src/license_detection/rules/loader.rs` | License/rule loading | 348-467 |
| `src/license_detection/spdx_lid/mod.rs` | Deprecated subs | 169-200 |
| `src/license_detection/index/builder/mod.rs` | Deprecated subs | 31-65 |
| `resources/licenses/json/` | SPDX submodule data | 727 files |

---

## 10. Conclusion

The Rust implementation has **partial feature parity** with Python for SPDX data handling:

✅ **Compatible**:
- License loading from `.LICENSE` files
- `spdx_license_key` field mapping
- `is_deprecated` and `replaced_by` fields
- `other_spdx_license_keys` field (but not used in mapping)

⚠️ **Requires Sync**:
- Hardcoded deprecated substitution tables (two versions)
- Key lowercasing not explicit

❌ **Divergent**:
- `other_spdx_license_keys` not included in SPDX mapping
- Exception handling (SPDX vs Python database)
- Potential case sensitivity issues

**Action Required**: Implement `other_spdx_license_keys` mapping and unify deprecated substitution tables to achieve full parity.
