# ADR 0002: Extraction vs Detection Separation

**Status**: Accepted  
**Authors**: scancode-rust team  
**Supersedes**: None

## Context

ScanCode Toolkit performs several distinct operations on code:

1. **Package metadata extraction** - Reading manifests, lockfiles, archives
2. **License detection** - Analyzing license text and normalizing to SPDX identifiers
3. **Copyright detection** - Finding copyright statements and extracting holder names
4. **Email/author detection** - Parsing author information from file content

In the Python reference implementation, these responsibilities are mixed within parser code, leading to:

- **Unclear boundaries** - Hard to tell what's extraction vs analysis
- **Complex parsers** - Single parser does too many things
- **Coupled testing** - Can't test extraction without detection logic
- **Inconsistent behavior** - Some parsers normalize licenses, others don't
- **Code duplication** - License normalization logic spread across parsers

**Critical Question**: Should parsers handle license detection and copyright extraction, or just extract raw data?

## Decision

**Parsers MUST extract ONLY. License detection, copyright detection, and email parsing are SEPARATE pipeline stages.**

### Parser Responsibilities (✅ ALLOWED)

| Operation | Example | Rationale |
|-----------|---------|-----------|
| **Extract license statements** | `"license": "MIT"` → `extracted_license_statement: "MIT"` | Raw data from manifest |
| **Extract license URLs** | `"license": {"url": "https://..."}` → store URL | Preserve metadata |
| **Extract license text** | `"license": {"text": "Permission is..."}` → store text | Preserve full content |
| **Parse author/email from manifests** | `"author": "Jane <jane@example.com>"` → `Party` object | Structured manifest data |
| **Extract dependencies with scopes** | `dependencies: {...}` → `Dependency` array | Manifest structure |
| **Extract checksums** | `"sha256": "abc123..."` → `sha256: "abc123..."` | Direct field mapping |

### Parser Prohibitions (❌ FORBIDDEN)

| Operation | Why Forbidden | Handled By |
|-----------|---------------|------------|
| **Normalize licenses to SPDX** | Complex analysis, confidence scoring | License detection engine |
| **Resolve license URLs to identifiers** | Requires URL → SPDX mapping database | License detection engine |
| **Populate `declared_license_expression`** | Requires normalization and SPDX parsing | License detection engine |
| **Populate `license_detections`** | Requires fuzzy matching with confidence | License detection engine |
| **Extract copyright holders** | Requires grammar-based pattern matching | Copyright detection engine |
| **Populate `holder` field** | Requires ClueCODE-style analysis | Copyright detection engine |
| **Parse emails from file content** | Requires regex patterns and scanning | Email detection engine |

### Responsibility Matrix

#### License Detection

| Parser Responsibility | Detection Engine Responsibility |
|----------------------|--------------------------------|
| ✅ Populate `extracted_license_statement` with raw data | ✅ Populate `declared_license_expression` with normalized SPDX |
| ✅ Extract license URLs, text, fields AS-IS | ✅ Populate `declared_license_expression_spdx` with proper case |
| ❌ NEVER call `normalize_license()` | ✅ Populate `license_detections` array with Match objects |
| ❌ NEVER call `resolve_license_url()` | ✅ Map URLs to SPDX identifiers |
| ❌ NEVER populate `declared_license_expression*` | ✅ Analyze license text with confidence scoring |
| ❌ NEVER populate `license_detections` | ✅ Handle SPDX expression parsing |

#### Copyright & Holder Detection

| Parser Responsibility | Detection Engine Responsibility |
|----------------------|--------------------------------|
| ✅ Extract raw copyright text (if in manifest) | ✅ Populate `holder` field from copyright analysis |
| ❌ NEVER parse/extract holder names | ✅ Use grammar-based copyright detection (ClueCODE) |
| ❌ NEVER populate `holder` field | ✅ Scan file content for copyright statements |
| ✅ Set `holder: None` | ✅ Extract holder names with pattern matching |

#### Author/Email Parsing

| Parser Responsibility (Manifests) | Detection Engine Responsibility (File Content) |
|-----------------------------------|----------------------------------------------|
| ✅ Parse author/email from manifests (e.g., npm) | ✅ Scan source files for email patterns |
| ✅ Create `Party` objects with name/email/role | ✅ Parse Linux CREDITS files for authors |
| ✅ Use utilities like `parse_name_email()` | ✅ Separate plugin for email/author detection |

### Data Flow Architecture

```text
┌─────────────────┐     ┌──────────────────┐     ┌─────────────┐
│ manifest.json   │────>│ License Engine   │────>│ PackageData │
│                 │     │                  │     │             │
│ "license": "MIT"│     │ normalize()      │     │ declared:   │
│                 │     │ confidence: 1.0  │     │   "mit"     │
│                 │     │                  │     │ spdx: "MIT" │
└─────────────────┘     └──────────────────┘     └─────────────┘
     PARSER                 DETECTION                 OUTPUT
     (extraction)           (analysis)                (combined)
     
extracted_license_statement = "MIT"
                                    ──> declared_license_expression = "mit"
                                    ──> declared_license_expression_spdx = "MIT"
```

## Consequences

### Benefits

1. **Clear Separation of Concerns**
   - Parsers are simpler and focused on one task
   - Detection engines can be tested independently
   - Easier to understand and maintain

2. **Better Testing**
   - Test extraction logic without detection complexity
   - Mock detection engines for parser tests
   - Verify detection accuracy independently

3. **Consistency**
   - All parsers follow same pattern (no special cases)
   - License normalization uses same algorithm
   - Copyright detection uses same grammar

4. **Performance Optimization**
   - Detection can be parallelized separately
   - Can skip detection if only metadata needed
   - Cache detection results across files

5. **Feature Parity**
   - Matches Python's separation (licensedcode/, cluecode/)
   - Easier to verify against reference implementation
   - Clear migration path for detection algorithms

### Trade-offs

1. **Two-Stage Processing**
   - Must run extraction, then detection
   - Slight complexity in orchestration
   - Acceptable: mirrors Python architecture

2. **Delayed License Normalization**
   - Can't provide normalized SPDX in first pass
   - Must process detection stage separately
   - Acceptable: more accurate results worth the wait

3. **Detection Engine Not Yet Built**
   - Current parsers only extract (detection TODO)
   - Some golden tests blocked on detection engine
   - Acceptable: prioritizing parser coverage first

## Alternatives Considered

### 1. Unified Parser/Detector

**Approach**: Each parser handles extraction AND detection.

```rust
impl PackageParser for NpmParser {
    fn extract_package_data(path: &Path) -> PackageData {
        let raw_license = parse_manifest(path).license;
        let normalized = normalize_license(raw_license); // ❌
        PackageData {
            extracted_license_statement: Some(raw_license),
            declared_license_expression: Some(normalized), // ❌
            // ...
        }
    }
}
```

**Rejected because**:

- Mixes concerns (extraction + analysis)
- Harder to test independently
- Duplicates detection logic across parsers
- Inconsistent normalization behavior
- Doesn't match Python architecture

### 2. Detection in Post-Processing Hook

**Approach**: Parsers extract, then call hook for detection.

```rust
let mut pkg = extract_package_data(path);
pkg.apply_license_detection(); // Hook
```

**Rejected because**:

- Still couples detection to parser lifecycle
- Hard to control when detection runs
- Difficult to parallelize effectively

### 3. Detection During Scan Orchestration

**Approach**: Scanner coordinates extraction, then detection.

```rust
// Extraction phase
let packages = scan_for_packages(dir);

// Detection phase (separate)
for pkg in &mut packages {
    detect_licenses(pkg);
    detect_copyrights(pkg);
}
```

**✅ ACCEPTED**: This is our approach (matches Python).

## Python Reference Comparison

The Python reference implementation already follows this separation:

**Extraction** (packagedcode/):

- `src/packagedcode/*.py` - Parser modules
- Extract raw data from manifests
- Populate `extracted_license_statement`

**Detection** (licensedcode/, cluecode/):

- `src/licensedcode/` - License detection engine
- `src/cluecode/copyrights.py` - Copyright detection
- `src/cluecode/plugin_email.py` - Email detection
- Separate pipeline stages

**Our Rust implementation mirrors this proven architecture.**

## Related ADRs

- [ADR 0001: Trait-Based Parser Architecture](0001-trait-based-parsers.md) - Parser structure
- [ADR 0003: Golden Test Strategy](0003-golden-test-strategy.md) - Why some tests are blocked on detection engine
- [ADR 0004: Security-First Parsing](0004-security-first-parsing.md) - Why we don't execute code

## References

- Python reference implementation follows the same separation:
  - Extraction in `packagedcode/` modules
  - License detection in `licensedcode/` modules
  - Copyright/email detection in `cluecode/` modules
