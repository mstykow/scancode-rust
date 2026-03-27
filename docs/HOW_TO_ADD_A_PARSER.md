# How To Add A Parser

This guide walks you through adding a new package parser to Provenant.

## Prerequisites

- Rust development environment set up
- Git submodules initialized: `git submodule update --init --filter=blob:none` (or `./setup.sh` on Linux, macOS, or WSL)
- Familiarity with the target package ecosystem
- Access to sample package manifest files

If you are contributing from Windows, prefer WSL for shell-based setup and helper commands.

## Overview

Adding a parser involves:

1. **Research** - Understand the package format
2. **Implementation** - Create the parser module
3. **Testing** - Add comprehensive tests
4. **Registration** - Register the parser with the scanner
5. **Golden Tests** - Add regression tests (expected for new production parsers)
6. **Assembly Support** - Add manifest/lockfile merging (if applicable)
7. **Validation** - Verify against reference implementation
8. **Documentation** - Document the implementation
9. **Quality Checks** - Run linting and formatting

**Time estimate**: 2-8 hours depending on complexity

## Step 1: Research the Package Format

**Choose your path:**

### Path A: Parser with Python Reference (Most Common)

If the ecosystem exists in `reference/scancode-toolkit/src/packagedcode/`:

```bash
cd reference/scancode-toolkit/
find src/packagedcode/ -name "*<ecosystem>*"
cat src/packagedcode/<ecosystem>.py
find tests/packagedcode/ -name "*<ecosystem>*"
```

**Use the Python reference to understand:**

- What file formats are handled
- What fields should be extracted
- Edge cases and known bugs to fix
- Expected output structure

**⚠️ CRITICAL**: Use the reference to understand **WHAT** to build, not **HOW**. Never port line-by-line. See [AGENTS.md](../AGENTS.md#using-the-reference-submodule) for details.

### Path B: Parser without Python Reference (New Ecosystem)

If adding a parser for an ecosystem **not** in the original Python ScanCode:

1. **Find the Official Specification**
   - Locate package format documentation (e.g., TOML spec, JSON schema)
   - Identify authoritative sources (language docs, package registry docs)
   - Check for version history (format evolution)

2. **Study Real-World Examples**

   ```bash
   # Clone popular projects using this format
   git clone https://github.com/<popular-project>
   find . -name "<manifest-file>" | head -10
   ```

3. **Analyze Format Structure**
   - What fields are required vs optional?
   - How are dependencies specified?
   - What metadata is standard?
   - Are there multiple file formats (manifest + lockfile)?

4. **Research Existing Tooling**
   - How do official tools parse this format?
   - Are there parsing libraries in the ecosystem?
   - What edge cases do tools handle?

5. **Define Extraction Scope**
   - What information is valuable for SBOM/licensing?
   - Package identity (name, version, namespace)
   - Dependencies (with versions, scopes)
   - Licensing information (declarations, URLs)
   - Maintainer/author information
   - URLs (homepage, repository, download)

### Common Step: Gather Test Data

Collect real-world package manifest examples:

```bash
mkdir -p testdata/<ecosystem>/
# Add 3-5 representative manifest files
# Include edge cases: empty, complex, minimal
```

**For Path A (Python Reference)**: Use files from `reference/scancode-toolkit/tests/`

**For Path B (New Ecosystem)**: Collect from popular open-source projects

## Step 2: Create the Parser Module

### File Structure

Create `src/parsers/<ecosystem>.rs`:

```rust
//! Parser for <Ecosystem> package manifests.
//!
//! ## Supported Formats
//! - `manifest.ext` - Main package manifest
//! - `lockfile.ext` - Dependency lockfile (if applicable)
//!
//! ## Key Features
//! - Extracts package metadata
//! - Handles dependencies with version constraints
//! - Supports all standard fields
//!
//! ## Implementation Notes
//! - Uses serde for JSON/TOML/YAML parsing
//! - Graceful error handling with structured scan diagnostics
//! - No code execution (AST parsing only)

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::models::{DatasourceId, Dependency, PackageData, Party};
use crate::parser_warn as warn;

use super::PackageParser;

/// Parser for <Ecosystem> package manifests
pub struct MyEcosystemParser;

impl PackageParser for MyEcosystemParser {
    const PACKAGE_TYPE: &'static str = "<ecosystem>";

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| {
            matches!(
                name.to_str(),
                Some("manifest.json") | Some("package.lock")
            )
        })
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read file {:?}: {}", path, e);
                return vec![PackageData {
                    package_type: Some(Self::PACKAGE_TYPE.to_string()),
                    datasource_id: Some(DatasourceId::MyEcosystemManifest),
                    ..Default::default()
                }];
            }
        };

        vec![parse_manifest(&content)]
    }
}

// Serde struct matching the manifest format
#[derive(Debug, Deserialize)]
struct ManifestFile {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    dependencies: Option<std::collections::HashMap<String, String>>,
    // Add fields as needed
}

fn parse_manifest(content: &str) -> PackageData {
    let manifest: ManifestFile = match serde_json::from_str(content) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to parse manifest: {}", e);
            return PackageData {
                package_type: Some("<ecosystem>".to_string()),
                datasource_id: Some(DatasourceId::MyEcosystemManifest),
                ..Default::default()
            };
        }
    };

    // Extract dependencies
    let dependencies = manifest
        .dependencies
        .unwrap_or_default()
        .into_iter()
        .map(|(name, version)| Dependency {
            purl: Some(build_purl(&name, Some(&version))),
            extracted_requirement: Some(version),
            scope: Some("dependencies".to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: None,
            is_direct: Some(true),
            ..Default::default()
        })
        .collect();

    PackageData {
        package_type: Some("<ecosystem>".to_string()),
        datasource_id: Some(DatasourceId::MyEcosystemManifest),
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        homepage_url: manifest.homepage,
        extracted_license_statement: manifest.license,
        dependencies,
        ..Default::default()
    }
}

fn build_purl(name: &str, version: Option<&str>) -> String {
    match version {
        Some(v) => format!("pkg:<ecosystem>/{}@{}", name, v),
        None => format!("pkg:<ecosystem>/{}", name),
    }
}

crate::register_parser!(
    "<Ecosystem> package manifest",
    &["**/manifest.json", "**/package.lock"],
    "<ecosystem>",
    "<Language>",
    Some("https://example.com/docs"),
);
```

> **Important**: The `register_parser!` macro at the end of the file registers metadata for
> auto-generating `docs/SUPPORTED_FORMATS.md`. Without it, your parser works for scanning but
> won't appear in the supported formats documentation.

### Key Principles

**DO**:

- ✅ Use `PackageParser` trait
- ✅ Return `PackageData` struct
- ✅ Handle errors gracefully with `parser_warn!()` / the local `warn!` alias so scanner output captures parser failures in `scan_errors`
- ✅ Extract all fields Python does
- ✅ Use established Rust parsers (serde_json, toml, yaml)

**DON'T**:

- ❌ Execute code (use AST parsing)
- ❌ Panic on errors
- ❌ Use plain `log::warn!()` in parser code for file-scoped failures; that bypasses the structured parser diagnostics path
- ❌ Use `.unwrap()` in library code
- ❌ Run broad file-content license detection in parser code
- ❌ Normalize ambiguous prose/URL/file-hint license metadata into declared expressions
- ❌ Detect copyrights (separate pipeline stage)

Parser-side declared-license normalization is allowed only for **trustworthy declared metadata** (for example, SPDX-expression-compatible manifest fields) and should use the shared parser license-normalization helper rather than one-off parser logic.

## Step 3: Add Comprehensive Tests

### Create Test File

`src/parsers/<ecosystem>_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_match() {
        assert!(MyEcosystemParser::is_match(&PathBuf::from("package.json")));
        assert!(!MyEcosystemParser::is_match(&PathBuf::from("README.md")));
    }

    #[test]
    fn test_extract_basic() {
        let path = PathBuf::from("testdata/<ecosystem>/basic.json");
        let data = MyEcosystemParser::extract_packages(&path);

        assert_eq!(data.name, Some("my-package".to_string()));
        assert_eq!(data.version, Some("1.0.0".to_string()));
        assert!(!data.dependencies.is_empty());
    }

    #[test]
    fn test_extract_with_all_fields() {
        let path = PathBuf::from("testdata/<ecosystem>/complete.json");
        let data = MyEcosystemParser::extract_packages(&path);

        assert!(data.description.is_some());
        assert!(data.homepage_url.is_some());
        assert!(data.extracted_license_statement.is_some());
    }

    #[test]
    fn test_extract_malformed_json() {
        let path = PathBuf::from("testdata/<ecosystem>/malformed.json");
        let data = MyEcosystemParser::extract_packages(&path);

        // Should not panic, returns default
        assert_eq!(data.package_type, Some("<ecosystem>".to_string()));
    }

    #[test]
    fn test_dependency_scopes() {
        let path = PathBuf::from("testdata/<ecosystem>/dependencies.json");
        let data = MyEcosystemParser::extract_packages(&path);

        let runtime_deps: Vec<_> = data.dependencies
            .iter()
            .filter(|d| d.is_runtime == Some(true))
            .collect();

        let dev_deps: Vec<_> = data.dependencies
            .iter()
            .filter(|d| d.scope == Some("dev".to_string()))
            .collect();

        assert!(!runtime_deps.is_empty());
        assert!(!dev_deps.is_empty());
    }
}
```

### Test Coverage Checklist

- [ ] `is_match()` correctly identifies manifest files
- [ ] Basic extraction works (name, version)
- [ ] All fields are extracted
- [ ] Dependencies with version constraints
- [ ] Different dependency scopes
- [ ] Malformed input handled gracefully
- [ ] Edge cases (empty, minimal, complex)
- [ ] Golden fixtures added for representative parser outputs
- [ ] At least one fixture-backed scan/assembly contract test added when the parser emits meaningful downstream package or dependency data

### Integration Test Verification

After implementing your parser, verify it's properly wired up to the scanner:

**Run the integration test suite:**

```bash
cargo test --test scanner_integration
```

The `test_all_parsers_are_registered_and_exported` test will verify your parser is:

1. Listed in the `register_package_handlers!` macro
2. Exported from the parsers module
3. Accessible to the scanner

**If this test fails**, it means you forgot to add your parser to the `register_package_handlers!` macro in Step 4.2.

### Ecosystem-Level Scan/Assembly Tests (Default for Downstream Package Contracts)

Unit tests and parser golden tests are the baseline. Some ecosystems also benefit from a small
number of **fixture-backed scanner/assembly tests** that exercise the higher-level flow:

1. file discovery via the scanner
2. parser extraction
3. package/file-reference assignment
4. assembly behavior when relevant

These tests are valuable when parser correctness depends on more than parsing a single file in
isolation. In practice, this is the default for parsers that emit top-level package identity,
meaningful dependencies, or file/package linkage consumed by assembly and output stages.

**Good candidates**:

- installed metadata that must attach referenced files correctly (`RECORD`, `installed-files.txt`,
  Debian `status.d` + `.list` / `.md5sums` sidecars)
- ecosystems with multiple competing metadata surfaces where scanner/assembly ordering matters
- archive or extracted layouts where normalized paths or file references affect final behavior
- intentionally unassembled formats whose scanner behavior must stay stable
- package/lockfile pairs whose final package visibility, dependency hoisting, or `for_packages`
  linkage would not be proven by parser-only goldens
- parsers whose downstream contract depends on package-shape fields such as `namespace`/`name`,
  `purl`, declared-license fields, `datasource_id`, or assembled `datafile_paths`

**Recommended location**: keep these tests near the owning ecosystem under `src/parsers/` in a
dedicated file such as `src/parsers/<ecosystem>_scan_test.rs`. For broad retroactive audits across
multiple existing ecosystems, add parser-local scan files for each covered ecosystem.

If your parser emits meaningful `PackageData.file_references`, treat one of these scan tests as
effectively required. Parser unit tests can prove that references were extracted; only a
scanner/assembly test proves that those references are actually resolved back onto scanned files via
final `for_packages` links.

This keeps them distinct from:

- parser unit tests in `src/parsers/<ecosystem>_test.rs`
- parser golden tests in `src/parsers/<ecosystem>_golden_test.rs`
- top-level scanner integration tests in `tests/scanner_integration.rs`, which should stay focused
  on cross-parser/system behavior rather than ecosystem-specific fixtures

**Rule of thumb**: if your parser emits package data that downstream assembly or output consumes,
add at least one fixture-backed scan-and-assemble contract test. Unit tests and parser goldens do
not prove final package visibility, `for_packages`, dependency hoisting, or file-link behavior.

## Step 4: Register the Parser

### Update `src/parsers/mod.rs`

**Step 4.1: Add module declaration and public re-export**

```rust
mod my_ecosystem;
#[cfg(test)]
mod my_ecosystem_test;

pub use self::my_ecosystem::MyEcosystemParser;
```

**Step 4.2: Register in `register_package_handlers!` macro**

This is **CRITICAL** - if you skip this step, your parser will be implemented but never called by the scanner!

Find the `register_package_handlers!` macro in `src/parsers/mod.rs` and add your parser to the `parsers` list:

```rust
register_package_handlers! {
    parsers: [
        NpmWorkspaceParser,
        NpmParser,
        // ... other parsers ...
        MyEcosystemParser,  // <-- ADD YOUR PARSER HERE
        // ... more parsers ...
    ],
    recognizers: [
        // ... file-type recognizers (don't add parsers here) ...
    ],
}
```

**Why this matters**: The `register_package_handlers!` macro generates the `try_parse_file()` function that the scanner uses to match files to parsers. If your parser isn't in this list, it will never be invoked, even if fully implemented and tested.

**Verification**: After adding your parser, verify it's registered:

```bash
# Should include "MyEcosystemParser" in output
cargo run --manifest-path xtask/Cargo.toml --bin update-parser-golden -- --list | grep MyEcosystemParser
```

## Step 5: Add Golden Tests (Expected for New Production Parsers)

Golden tests compare parser output against reference `.expected.json` files to catch regressions.

For new production parser work in this repository, treat golden tests as the default expectation rather than a nice-to-have. The only reasonable exception is an explicitly incremental parser slice with a documented follow-up task to add the missing goldens before calling the ecosystem support complete.

### Generate Expected Output

Use the test generator utility to create expected output files:

```bash
# List all available parser types
cargo run --manifest-path xtask/Cargo.toml --bin update-parser-golden -- --list

# Generate expected output using parser struct name
cargo run --manifest-path xtask/Cargo.toml --bin update-parser-golden -- MyEcosystemParser \
  testdata/<ecosystem>/sample.json \
  testdata/<ecosystem>/sample.json.expected.json

# Or use the convenience wrapper script
./scripts/update_parser_golden.sh MyEcosystemParser \
  testdata/<ecosystem>/sample.json \
  testdata/<ecosystem>/sample.json.expected.json
```

For canonical script purpose and full CLI argument reference, see [`scripts/README.md`](../scripts/README.md).

**Auto-Discovery**: The generator automatically discovers ALL parsers registered in `src/parsers/mod.rs` via the `register_package_handlers!` macro. When you add your parser to that list, it becomes immediately available - no manual updates needed!

### Create Golden Test File

Create `src/parsers/<ecosystem>_golden_test.rs`:

```rust
#[cfg(test)]
mod golden_tests {
    use crate::parsers::PackageParser;
    use crate::parsers::my_ecosystem::MyEcosystemParser;
    use crate::test_utils::compare_package_data_parser_only;
    use std::path::PathBuf;

    #[test]
    fn test_golden_basic() {
        let test_file = PathBuf::from("testdata/<ecosystem>/basic.json");
        let expected_file = PathBuf::from("testdata/<ecosystem>/basic.json.expected.json");

        let package_data = MyEcosystemParser::extract_packages(&test_file);

        match compare_package_data_parser_only(&package_data, &expected_file) {
            Ok(_) => (),
            Err(e) => panic!("Golden test failed: {}", e),
        }
    }
}
```

### Update Module Registration

Add golden test module to `src/parsers/mod.rs`:

```rust
mod my_ecosystem_golden_test;
```

## Step 6: Add Assembly Support (If Applicable)

Assembly merges related manifest/lockfile pairs into logical packages. If your ecosystem has multiple related files (e.g., manifest + lockfile), you need assembly support.

### Understanding Datasource IDs

**Datasource IDs** are unique identifiers for each type of package data source your parser handles. They serve as the bridge between parsers and the assembly system.

**Key Concepts**:

- **`DatasourceId` enum**: A type-safe enum in `src/models/datasource_id.rs` with variants for every supported file format. Using an enum instead of strings provides compile-time checking and prevents typos.
- **`datasource_id` field**: Set in each `PackageData` instance to indicate which specific file type was parsed
- **Assembly matching**: The assembler uses `datasource_id` values to group related files (e.g., manifest + lockfile)

**Example - Single Datasource Parser**:

```rust
use crate::models::DatasourceId;

// In extract_packages():
PackageData {
    datasource_id: Some(DatasourceId::CargoToml),
    // ...
}
```

**Example - Multi-Datasource Parser**:

```rust
use crate::models::DatasourceId;

// In extract_packages():
if path.ends_with("pyproject.toml") {
    PackageData {
        datasource_id: Some(DatasourceId::PypiPyprojectToml),
        // ...
    }
} else if path.ends_with("setup.py") {
    PackageData {
        datasource_id: Some(DatasourceId::PypiSetupPy),
        // ...
    }
}
```

**Naming Convention**: Enum variants use `PascalCase` (e.g., `NpmPackageJson`, `CargoLock`, `MavenPom`). They serialize to `snake_case` strings for JSON output.

**Critical Rules**:

1. Every new file format needs a corresponding `DatasourceId` variant in `src/models/datasource_id.rs`
2. Datasource IDs are globally unique — enforced at compile time by the enum
3. The `datasource_id` field must NEVER be `None` in production code paths
4. Every new `DatasourceId` must be classified in `src/assembly/assemblers.rs` — either in an `AssemblerConfig` or in `UNASSEMBLED_DATASOURCE_IDS`

### Classify Every Datasource for Assembly Accounting

Even when your parser does **not** need manifest/lockfile merging, you still need to classify its datasource for assembly accounting.

- If the datasource participates in package assembly, add it to the appropriate `AssemblerConfig` in `src/assembly/assemblers.rs`.
- If it is intentionally standalone or otherwise not assembled, add it to `UNASSEMBLED_DATASOURCE_IDS` in that same file.

This is enforced by the `assembly::assemblers::tests::test_every_datasource_id_is_accounted_for` test. If you skip this step, CI fails even if your parser logic and parser tests are correct.

### Register File-Reference Resolution Ownership (When Applicable)

Some parsers emit `PackageData.file_references` that should later be resolved back onto scanned
files and attached to the assembled package via `FileInfo.for_packages`.

If your parser emits meaningful file references, you must do **both** of the following:

1. **Register the ownership path in assembly**:
   - either add the datasource to the declarative resolver registry in
     `src/assembly/file_ref_resolve.rs`
   - or handle it in another explicit post-assembly pass such as a dedicated
     `*_resource_assign.rs` / merger step

2. **Add a parser-adjacent scan test** in `src/parsers/<ecosystem>_scan_test.rs` proving the final
   behavior on real scanned files.

Without this, it is easy to end up in a partial state where the parser extracts file references but
the scanner output never links those files back to the package.

### Check if Assembly is Needed

Does your ecosystem have:

- ✅ A manifest file (package.json, Cargo.toml, go.mod, etc.)
- ✅ A lockfile (package-lock.json, Cargo.lock, go.sum, etc.)
- ✅ Multiple related metadata files that describe the same package?

If **YES** to any, your parser needs assembly support.

If **NO**, your datasource still needs an explicit entry in `UNASSEMBLED_DATASOURCE_IDS` so the assembly-accounting test knows the omission is intentional.

### Add Assembler Configuration

Edit `src/assembly/assemblers.rs` and add your ecosystem to the `ASSEMBLERS` array:

```rust
// Add to the ASSEMBLERS array
AssemblerConfig {
    datasource_ids: &[
        DatasourceId::MyEcosystemManifest,
        DatasourceId::MyEcosystemLock,
    ],
    sibling_file_patterns: &["manifest.ext", "lockfile.ext"],
    mode: AssemblyMode::SiblingMerge,
},
```

**Key points**:

- `datasource_ids`: Must **exactly match** the `datasource_id` values your parsers emit in `PackageData`
- `sibling_file_patterns`: Filenames to look for in the same directory (order matters - first is primary)
- Patterns support exact match, case-insensitive match, and glob wildcards (`*.podspec`)
- The assembler will only merge packages whose `datasource_id` values are listed in the same `AssemblerConfig`

If your parser is intentionally **not** assembled, add it to `UNASSEMBLED_DATASOURCE_IDS` instead:

```rust
pub static UNASSEMBLED_DATASOURCE_IDS: &[DatasourceId] = &[
    // ... existing entries ...
    DatasourceId::MyEcosystemManifest,
];
```

### Add Assembly Golden Tests

Create test fixtures in `testdata/assembly-golden/<ecosystem>-basic/`:

1. **Create directory**:

   ```bash
   mkdir -p testdata/assembly-golden/<ecosystem>-basic
   ```

2. **Add test files**:
   - Add a minimal manifest file
   - Add a minimal lockfile (if applicable)
   - Keep files small and focused (5-20 lines each)

3. **Generate expected output**:
   The test will auto-generate `expected.json` on first run:

   ```bash
   cargo test test_assembly_<ecosystem>_basic
   ```

4. **Review generated output**:
   - Check `testdata/assembly-golden/<ecosystem>-basic/expected.json`
   - Verify UUIDs are normalized to `fixed-uid-done-for-testing-5642512d1758`
   - Verify packages and dependencies are correctly assembled
   - Verify `datafile_paths` includes both files
   - Verify `datasource_ids` includes both parser IDs

5. **Add test function**:
   Edit `src/assembly/assembly_golden_test.rs` and add:

   ```rust
   #[test]
   fn test_assembly_<ecosystem>_basic() {
       match run_assembly_golden_test("<ecosystem>-basic") {
           Ok(_) => (),
           Err(e) => panic!("Assembly golden test failed for <ecosystem>-basic: {}", e),
       }
   }
   ```

6. **Verify test passes**:

   ```bash
   cargo test test_assembly_<ecosystem>_basic
   ```

### Assembly Checklist

- [ ] Datasource classified in `src/assembly/assemblers.rs` (`ASSEMBLERS` or `UNASSEMBLED_DATASOURCE_IDS`)
- [ ] Assembler config added to `src/assembly/assemblers.rs` when assembly is needed
- [ ] If parser emits meaningful `file_references`, its resolution ownership is registered in `src/assembly/file_ref_resolve.rs` or another explicit post-assembly pass
- [ ] `datasource_ids` match parser `datasource_id` values
- [ ] `sibling_file_patterns` match actual filenames
- [ ] Test fixtures created in `testdata/assembly-golden/<ecosystem>-basic/`
- [ ] Golden test function added to `src/assembly/assembly_golden_test.rs`
- [ ] Assembly test passes: `cargo test test_assembly_<ecosystem>_basic`
- [ ] Datasource accounting test passes: `cargo test test_every_datasource_id_is_accounted_for --lib`
- [ ] Expected JSON reviewed and committed

### Common Assembly Patterns

**Sibling-Merge** (most common):

- Files in same directory (package.json + package-lock.json)
- Pattern: `["manifest.ext", "lockfile.ext"]`
- See `src/assembly/assemblers.rs` for current implementations

**Multi-File Merge**:

- Multiple related files (\*.podspec + Podfile + Podfile.lock)
- Pattern: `["*.podspec", "Podfile", "Podfile.lock"]`
- Supports glob patterns for variable filenames

**Metadata Pairs**:

- Two metadata files (metadata.json + metadata.rb)
- Pattern: `["metadata.json", "metadata.rb"]`
- For ecosystems with multiple metadata formats

### Related Documentation

- [Assembly Golden Tests README](../testdata/assembly-golden/README.md) - Test structure and UUID normalization
- [Architecture](ARCHITECTURE.md#package-assembly-system) - Assembly architecture and design principles
- [Assembler Configurations](../src/assembly/assemblers.rs) - All registered assemblers

## Step 7: Validate Implementation

### Path A: Validation with Python Reference

If you followed Path A (Python reference exists):

```bash
cd reference/scancode-toolkit/
scancode -p testdata/<ecosystem>/basic.json --json reference_output.json
```

Compare outputs:

```bash
cargo run -- --json-pp rust_output.json testdata/<ecosystem>/
# Compare fields manually or use diff tool
```

**Validation Checklist**:

- [ ] All fields Python extracts are present
- [ ] Field values match (or are improved)
- [ ] Dependencies are equivalent
- [ ] PURLs are correctly formatted
- [ ] Edge cases handled
- [ ] Golden tests pass (if added)

### Path B: Validation without Python Reference

If you followed Path B (new ecosystem):

1. **Test Against Official Tools**

   ```bash
   # Run ecosystem's native tool
   <ecosystem-tool> show <package>

   # Compare with our extraction
   cargo run -- --json-pp output.json testdata/<ecosystem>/
   cat output.json | jq '.packages[0]'
   ```

2. **Validate Against Specification**
   - Check that all required fields are extracted
   - Verify dependency syntax parsing is correct
   - Ensure PURL format follows [purl-spec](https://github.com/package-url/purl-spec)

3. **Cross-Reference with Package Registries**
   - Compare extracted metadata with registry API
   - Verify version constraints are parsed correctly
   - Check that URLs and identifiers match

**Validation Checklist**:

- [ ] All fields from format spec are extracted
- [ ] Dependencies parsed according to spec
- [ ] PURLs follow purl-spec format
- [ ] Output matches registry metadata
- [ ] Edge cases from real projects handled
- [ ] Golden tests pass (if added)

## Step 8: Document Your Work

### Add Module Documentation

Ensure your parser file has comprehensive `//!` module docs (already shown in Step 2).

### Document Improvements (If Beyond Parity)

If you fixed bugs or added features Python doesn't have, create `docs/improvements/<ecosystem>-parser.md`:

```markdown
# <Ecosystem> Parser: Improvements Over Python

## Summary

Our Rust implementation improves on the Python reference by:

- 🐛 **Bug Fix**: [Describe what was broken in Python]
- ✨ **New Feature**: [Describe what Python has as TODO]
- 🔍 **Enhanced**: [Describe where we extract more data]

## Problem in Python Reference

[Explain the issue with code examples if possible]

## Our Solution

[Explain how Rust implementation improves it]

## Before/After Comparison

**Python Output**:
\`\`\`json
{ "field": null }
\`\`\`

**Rust Output**:
\`\`\`json
{ "field": "correct-value" }
\`\`\`

## References

### Python Reference Issues

- Bug/TODO description

### <Ecosystem> Documentation

- [Official Docs](https://...)

## Status

- ✅ **Implementation**: Complete, validated, production-ready
- ✅ **Documentation**: Complete
```

### Update Supported Formats

The pre-commit hook will automatically regenerate `docs/SUPPORTED_FORMATS.md` when you commit,
**but only if your parser file includes the `register_parser!` macro** (shown in Step 2).
This macro registers metadata that the generator uses to build the formats table.

## Step 9: Quality Checks

### Run All Tests

```bash
cargo test <ecosystem>
cargo test --lib  # Fast: tests only library code
cargo test        # Full: includes all tests
```

### Check Code Quality

```bash
cargo fmt                    # Format code
cargo clippy                 # Lint
cargo clippy --fix          # Auto-fix suggestions
```

### Verify Documentation

```bash
cargo doc --open  # Generate and view API docs
```

### Pre-commit Hook

```bash
pre-commit run --all-files
```

For hook installation and contributor setup, see [`README.md`](../README.md).

## Common Pitfalls & Solutions

### Issue: Parser Not Being Called

**Problem**: Your parser's `is_match()` returns false

**Solution**: Check file name patterns carefully. Use debug prints to verify:

```rust
fn is_match(path: &Path) -> bool {
    let matches = path.file_name().is_some_and(|name| {
        name.to_str() == Some("manifest.json")
    });
    eprintln!("Checking {:?}: {}", path, matches);  // Debug
    matches
}
```

### Issue: Parsing Fails Silently

**Problem**: serde deserialization fails but you don't know why

**Solution**: Add detailed error logging:

```rust
let manifest: ManifestFile = match serde_json::from_str(content) {
    Ok(m) => m,
    Err(e) => {
        warn!("Failed to parse manifest: {}. Content: {}", e, content);
        return PackageData {
            package_type: Some("<ecosystem>".to_string()),
            datasource_id: Some(DatasourceId::MyEcosystemManifest),
            ..Default::default()
        };
    }
};
```

### Issue: Tests Pass But Golden Tests Fail

**Problem**: Your output structure doesn't match Python's

**Solution**: Compare field-by-field with Python output. Common issues:

- Different field names (check `PackageData` struct)
- Missing `package_type` field
- Incorrect PURL format
- Wrong dependency scope values

### Issue: Dependency Version Constraints Wrong

**Problem**: Version strings not parsed correctly

**Solution**: Don't parse version constraints. Extract as-is:

```rust
// Good: Extract raw string
extracted_requirement: Some("^1.2.3".to_string())

// Bad: Try to parse/normalize
// Don't do this - keep original format
```

## Beyond Parity Opportunities

Look for opportunities to improve on Python:

**Fix Known Bugs**: Check Python code for TODOs, FIXMEs, or incorrect behavior

**Extract More Data**: If the manifest has fields Python ignores, extract them into `extra_data`

**Handle Edge Cases**: If Python crashes on malformed input, handle it gracefully

**Security Improvements**: If Python executes code, use AST parsing instead

**Performance**: Use efficient parsers and zero-copy where possible

## Getting Help

**Resources**:

- [ARCHITECTURE.md](ARCHITECTURE.md) - System design
- [ADR 0001](adr/0001-trait-based-parsers.md) - Parser architecture
- [ADR 0003](adr/0003-golden-test-strategy.md) - Testing strategy
- [ADR 0004](adr/0004-security-first-parsing.md) - Security guidelines
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture and design principles

**Questions?**: Check existing parsers as examples:

- Simple: `src/parsers/cargo.rs` (TOML-based)
- Medium: `src/parsers/npm.rs` (JSON with multiple formats)
- Complex: `src/parsers/python.rs` (Multiple file formats, setup.py)

## Checklist

Before submitting your parser:

### Implementation

- [ ] Parser implements `PackageParser` trait
- [ ] `DatasourceId` variant(s) added to `src/models/datasource_id.rs` for each file format
- [ ] `is_match()` correctly identifies files
- [ ] `extract_packages()` extracts all fields
- [ ] `datasource_id` field set correctly in ALL code paths (never `None` in production)
- [ ] Dependencies extracted with proper scopes
- [ ] PURLs correctly formatted
- [ ] Graceful error handling (no panics)
- [ ] Uses appropriate parser (serde_json, toml, yaml)

### Testing

- [ ] Unit tests for `is_match()`
- [ ] Unit tests for basic extraction
- [ ] Tests for all fields
- [ ] Tests for dependencies
- [ ] Tests for malformed input
- [ ] Tests pass: `cargo test <ecosystem>`

### Code Quality

- [ ] Code formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy`
- [ ] No compiler warnings
- [ ] Documentation builds: `cargo doc --open`

### Documentation

- [ ] Module `//!` comments present
- [ ] Public functions have `///` doc comments
- [ ] Improvement doc created if beyond parity
- [ ] Test data added to `testdata/<ecosystem>/`

### Integration

- [ ] Parser module declared in `src/parsers/mod.rs`
- [ ] Parser exported with `pub use`
- [ ] Parser added to `register_package_handlers!` macro
- [ ] `register_parser!` macro added at end of parser file
- [ ] Integration test passes: `cargo test test_all_parsers_are_registered_and_exported`
- [ ] Datasource classified in `ASSEMBLERS` or `UNASSEMBLED_DATASOURCE_IDS`
- [ ] If parser emits meaningful downstream package/dependency data, final package visibility / `for_packages` / dependency hoisting is proven by a parser-adjacent `*_scan_test.rs`
- [ ] Pre-commit hooks pass
- [ ] SUPPORTED_FORMATS.md auto-updated

### Assembly (If Applicable)

- [ ] Datasource classified in `src/assembly/assemblers.rs`
- [ ] Assembler config added to `src/assembly/assemblers.rs`
- [ ] Assembly golden test created in `testdata/assembly-golden/`
- [ ] Assembly test function added to `src/assembly/assembly_golden_test.rs`
- [ ] Assembly test passes: `cargo test test_assembly_<ecosystem>_basic`
- [ ] Datasource accounting test passes: `cargo test test_every_datasource_id_is_accounted_for --lib`

### Validation

- [ ] Output compared with Python reference
- [ ] All Python-extracted fields present
- [ ] Edge cases handled
- [ ] Known Python bugs fixed (if any)

## Conclusion

You now have a complete parser integrated into Provenant! Your contribution helps achieve feature parity with ScanCode Toolkit while leveraging Rust's safety and performance advantages.

**Next Steps**:

1. Test on real-world package repositories
2. Compare results with Python ScanCode
3. Document any intentional differences
4. Submit pull request

Welcome to the Provenant project! 🦀
