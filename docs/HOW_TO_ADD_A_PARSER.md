# How To Add A Parser

This guide walks you through adding a new package parser to scancode-rust.

## Prerequisites

- Rust development environment set up
- Git submodules initialized: `./setup.sh`
- Familiarity with the target package ecosystem
- Access to sample package manifest files

## Overview

Adding a parser involves:

1. **Research** - Understand the package format
2. **Implementation** - Create the parser module
3. **Testing** - Add comprehensive tests
4. **Documentation** - Document the implementation
5. **Integration** - Register the parser

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
//! - Graceful error handling with warnings
//! - No code execution (AST parsing only)

use std::fs;
use std::path::Path;

use log::warn;
use serde::{Deserialize, Serialize};

use crate::models::{Dependency, PackageData, Party};
use crate::parsers::utils::create_default_package_data;

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

    fn extract_package_data(path: &Path) -> PackageData {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read file {:?}: {}", path, e);
                return create_default_package_data(Self::PACKAGE_TYPE, None);
            }
        };

        parse_manifest(&content)
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
            return create_default_package_data("<ecosystem>", None);
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
```

### Key Principles

**DO**:

- ✅ Use `PackageParser` trait
- ✅ Return `PackageData` struct
- ✅ Handle errors gracefully with `warn!()`
- ✅ Extract all fields Python does
- ✅ Use established Rust parsers (serde_json, toml, yaml)

**DON'T**:

- ❌ Execute code (use AST parsing)
- ❌ Panic on errors
- ❌ Use `.unwrap()` in library code
- ❌ Normalize licenses (extraction only)
- ❌ Detect copyrights (separate pipeline stage)

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
        let data = MyEcosystemParser::extract_package_data(&path);
        
        assert_eq!(data.name, Some("my-package".to_string()));
        assert_eq!(data.version, Some("1.0.0".to_string()));
        assert!(!data.dependencies.is_empty());
    }

    #[test]
    fn test_extract_with_all_fields() {
        let path = PathBuf::from("testdata/<ecosystem>/complete.json");
        let data = MyEcosystemParser::extract_package_data(&path);
        
        assert!(data.description.is_some());
        assert!(data.homepage_url.is_some());
        assert!(data.extracted_license_statement.is_some());
    }

    #[test]
    fn test_extract_malformed_json() {
        let path = PathBuf::from("testdata/<ecosystem>/malformed.json");
        let data = MyEcosystemParser::extract_package_data(&path);
        
        // Should not panic, returns default
        assert_eq!(data.package_type, Some("<ecosystem>".to_string()));
    }

    #[test]
    fn test_dependency_scopes() {
        let path = PathBuf::from("testdata/<ecosystem>/dependencies.json");
        let data = MyEcosystemParser::extract_package_data(&path);
        
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

## Step 4: Register the Parser

### Update `src/parsers/mod.rs`

Add module declaration and public re-export:

```rust
pub mod my_ecosystem;
pub mod my_ecosystem_test;

pub use my_ecosystem::MyEcosystemParser;
```

### Register in Parser List

Find the parser registration section and add your parser to the appropriate collection.

## Step 5: Add Golden Tests (Optional but Recommended)

Golden tests compare parser output against reference `.expected.json` files to catch regressions.

### Generate Expected Output

Use the test generator utility to create expected output files:

```bash
# List all available parser types
cargo run --bin generate-test-expected --list

# Generate expected output using parser struct name
cargo run --bin generate-test-expected MyEcosystemParser \
  testdata/<ecosystem>/sample.json \
  testdata/<ecosystem>/sample.json.expected.json

# Or use the convenience wrapper script
./scripts/generate_test_expected.sh MyEcosystemParser \
  testdata/<ecosystem>/sample.json \
  testdata/<ecosystem>/sample.json.expected.json
```

**Auto-Discovery**: The generator automatically discovers ALL parsers registered in `src/parsers/mod.rs` via the `define_parsers!` macro. When you add your parser to that list, it becomes immediately available - no manual updates needed!

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

        let package_data = MyEcosystemParser::extract_package_data(&test_file);

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

## Step 6: Validate Implementation

### Path A: Validation with Python Reference

If you followed Path A (Python reference exists):

```bash
cd reference/scancode-toolkit/
scancode -p testdata/<ecosystem>/basic.json --json reference_output.json
```

Compare outputs:

```bash
cargo run -- testdata/<ecosystem>/ -o rust_output.json
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
   cargo run -- testdata/<ecosystem>/ -o output.json
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

## Step 7: Document Your Work

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

The pre-commit hook will automatically regenerate `docs/SUPPORTED_FORMATS.md` when you add your parser. No manual action needed!

## Step 7: Quality Checks

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
pre-commit install   # One-time setup
git add .
git commit          # Hooks run automatically
```

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
        return create_default_package_data("<ecosystem>", None);
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
- [ ] `is_match()` correctly identifies files
- [ ] `extract_package_data()` extracts all Python fields
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

- [ ] Parser registered in `src/parsers/mod.rs`
- [ ] Pre-commit hooks pass
- [ ] SUPPORTED_FORMATS.md auto-updated

### Validation

- [ ] Output compared with Python reference
- [ ] All Python-extracted fields present
- [ ] Edge cases handled
- [ ] Known Python bugs fixed (if any)

## Conclusion

You now have a complete parser integrated into scancode-rust! Your contribution helps achieve feature parity with ScanCode Toolkit while leveraging Rust's safety and performance advantages.

**Next Steps**:

1. Test on real-world package repositories
2. Compare results with Python ScanCode
3. Document any intentional differences
4. Submit pull request

Welcome to the scancode-rust project! 🦀
