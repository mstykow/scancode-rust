# Agent Guidelines for scancode-rust

This guide provides essential information for AI coding agents working on the `scancode-rust` codebase - a high-performance Rust tool for detecting licenses, copyrights, and package metadata in source code.

## Documentation Map

**Finding Information Quickly:**

- **Architecture & Design Decisions**: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) - System design, components, principles
- **How-To Guides**: [`docs/HOW_TO_ADD_A_PARSER.md`](docs/HOW_TO_ADD_A_PARSER.md) - Step-by-step guide for adding new parsers
- **Architectural Decision Records**: [`docs/adr/`](docs/adr/) - Why key decisions were made (5 ADRs)
- **Beyond-Parity Features**: [`docs/improvements/`](docs/improvements/) - Where Rust exceeds Python (7 parsers documented)
- **Supported Formats**: [`docs/SUPPORTED_FORMATS.md`](docs/SUPPORTED_FORMATS.md) - Auto-generated list of all supported package formats
- **API Reference**: Run `cargo doc --open` - Complete API documentation
- **This File**: Quick start, code style, common pitfalls

## Project Context

**scancode-rust** is a complete rewrite of [ScanCode Toolkit](https://github.com/aboutcode-org/scancode-toolkit) in Rust, designed to be a **drop-in replacement** with all features and requirements of the original, but with less complexity, zero bugs, and Rust-specific optimizations. The original Python codebase is available as a reference submodule at `reference/scancode-toolkit/`.

### Core Philosophy: Correctness and Feature Parity Above All

**The primary goal is to create a functionally identical replacement for ScanCode Toolkit that users can trust completely.**

When implementing features:

- **Maximize correctness and feature parity**: Every feature, edge case, and requirement from the original must be preserved
- **Effort is irrelevant**: Take whatever time and effort needed to get it right. No shortcuts, no compromises
- **Zero tolerance for bugs**: Identify bugs in the original Python code and fix them in the Rust implementation
- **Leverage Rust advantages**: Use Rust's type system, ownership model, and ecosystem to create more robust, performant code
- **Never cut corners**: Proper error handling, comprehensive tests, and thorough edge case coverage are non-negotiable

### Using the Reference Submodule

The `reference/scancode-toolkit/` submodule contains the original Python implementation and serves as:

- **Feature specification**: Understand what the original does, including all edge cases and requirements
- **Behavioral reference**: Verify expected output formats and results against the original
- **Bug identification**: Find known issues and technical debt to avoid replicating
- **Logic inspiration**: Understand the problem domain and solution approaches

⚠️ **Critical: This is a Rewrite, Not a Line-by-Line Port**

You **cannot** and **should not** follow the reference Python implementation line by line. Here's why:

- The original has architectural issues, bugs, and technical debt that must not be replicated
- Python patterns don't translate directly to idiomatic Rust
- Rust's type system and ownership model enable fundamentally better designs
- We must leverage Rust-specific optimizations (zero-copy parsing, compile-time guarantees, etc.)
- The goal is to achieve the same **outcomes** through better **implementation**

**Use the reference to understand WHAT to build, not HOW to build it.** Implement features using clean, idiomatic Rust that leverages the language's strengths while maintaining complete functional compatibility with the original.

## Quick Start

```bash
# Setup (first time only)
./setup.sh                    # Initialize git submodules (SPDX license data)
git submodule update --init   # Ensure all submodules are initialized

# Build & Test
cargo build                   # Development build
cargo build --release         # Optimized build
cargo test                    # Run all tests
cargo test <test_name>        # Run specific test (e.g., test_extract_from_testdata)
cargo test --lib              # Test library code only (faster)

# Code Quality
cargo fmt                     # Format code
cargo clippy                  # Lint and catch mistakes
cargo clippy --fix            # Auto-fix clippy suggestions

# Run Tool
cargo run -- <dir> -o output.json --exclude "*.git*" "target/*"
```

## Running Single Tests

To run a specific test, use its full path from `cargo test -- --list`:

```bash
cargo test parsers::npm_test::tests::test_extract_from_testdata
cargo test askalono::strategy::tests::single_optimize
cargo test test_is_match       # Runs all tests with "test_is_match" in name
```

## Project Architecture

**High-Level Structure:**

- `src/parsers/` - Package manifest parsers (trait-based, one per ecosystem)
- `src/models/` - Core data structures (PackageData, Dependency, etc.)
- `src/scanner/` - File system traversal and parallel processing
- `src/main.rs` - CLI entry point

**Key Patterns**: Trait-based parsers, Result-based errors, parallel processing with rayon

**For detailed architecture**: See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)

## Code Style Guidelines

### Imports

Organize imports in this order:

1. Standard library (`std::`)
2. External crates (alphabetical)
3. Internal crate modules (`crate::`)
4. Parent/sibling modules (`super::`, `self::`)

```rust
use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::models::PackageData;
use super::PackageParser;
```

### Naming Conventions

- **Types**: `PascalCase` (structs, enums, traits)
- **Functions/Variables**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`
- **Test modules**: Use `#[cfg(test)] mod tests { ... }` or separate `_test.rs` files

```rust
const LICENSE_DETECTION_THRESHOLD: f32 = 0.9;

struct PackageParser;

fn extract_package_data(path: &Path) -> PackageData {
    let package_type = "npm";
}
```

### Types and Error Handling

- **Use `Result<T, E>` for fallible operations**: Prefer `anyhow::Error` for general errors
- **Pattern matching over unwrap**: Use `?` operator for error propagation
- **Avoid `.unwrap()` in library code**: Only acceptable in tests or when panic is intentional
- **Use `Option` methods**: `.and_then()`, `.map()`, `.unwrap_or()`, etc.

```rust
// Good
fn read_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read: {}", e))?;
    Ok(content)
}

// Bad - avoid unwrap in library code
fn read_file_bad(path: &Path) -> String {
    let mut file = File::open(path).unwrap();  // DON'T DO THIS
    // ...
}
```

### Formatting

- **Line length**: No strict limit, but keep reasonable (~100 chars)
- **Indentation**: 4 spaces (enforced by `cargo fmt`)
- **Trailing commas**: Use in multi-line expressions
- **String literals**: Use `"double quotes"` for strings

```rust
PackageData {
    package_type: Some("npm".to_string()),
    name,
    version,
    homepage_url: None,  // Trailing comma
}
```

### Documentation

- **Public APIs**: Document with `///` doc comments
- **Examples**: Include examples for complex functions
- **Inline comments**: Explain "why" not "what"

```rust
/// Extracts package metadata from a manifest file.
///
/// Returns `PackageData` with all available fields populated.
/// Returns a default/empty structure if parsing fails.
pub fn extract_package_data(path: &Path) -> PackageData {
    // Use log::warn for parse errors rather than panicking
    // to allow the scan to continue for other files
    match parse_file(path) {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to parse {:?}: {}", path, e);
            default_package_data()
        }
    }
}
```

## Adding a New Package Parser

1. **Create parser file**: `src/parsers/<ecosystem>.rs`
2. **Implement trait**:

   ```rust
   use crate::models::PackageData;
   use super::PackageParser;

   pub struct MyParser;

   impl PackageParser for MyParser {
       const PACKAGE_TYPE: &'static str = "my-ecosystem";

       fn is_match(path: &Path) -> bool {
           path.file_name().is_some_and(|name| name == "my-manifest.json")
       }

       fn extract_package_data(path: &Path) -> PackageData {
           // Implementation
       }
   }
   ```

3. **Add test file**: `src/parsers/<ecosystem>_test.rs`
4. **Update mod.rs**: Add module declaration and public re-export
5. **Add test data**: Place sample manifests in `testdata/<ecosystem>/`

## Testing Strategy

scancode-rust uses a **four-layer testing approach** for comprehensive quality assurance:

1. **Doctests** - API documentation examples that run as tests (verifies public API examples work)
2. **Unit Tests** - Component-level tests for individual functions and edge cases
3. **Golden Tests** - Regression tests comparing output against Python ScanCode reference
4. **Integration Tests** - End-to-end tests validating the full scanner pipeline

**For complete testing philosophy and guidelines**: See [`docs/TESTING_STRATEGY.md`](docs/TESTING_STRATEGY.md)

### Quick Testing Reference

**Co-located tests**: Use `#[cfg(test)] mod tests { ... }` in implementation files
**Separate test files**: For larger suites, use `<module>_test.rs` and `<module>_golden_test.rs`
**Test data**: Place in `testdata/` directory, organized by ecosystem

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_matches_correct_filename() {
        assert!(MyParser::is_match(&PathBuf::from("package.json")));
        assert!(!MyParser::is_match(&PathBuf::from("readme.md")));
    }
}
```

## CI/CD

**Pre-commit hooks** (install with `pre-commit install`):

- `cargo fmt --all` - Format code and stage changes
- `cargo clippy --all-targets --all-features -- -D warnings` - Lint with warnings as errors

**GitHub Actions** (runs on push to main and PRs):

- Code formatting check: `cargo fmt --all -- --check`
- Clippy linting: `cargo clippy --all-targets --all-features -- -D warnings`
- Compilation: `cargo check --all --verbose`
- Test suite: `cargo test --all --verbose`

**All checks must pass before merging.**

## Performance Considerations

- **Parallel processing**: File scanning uses `rayon` - maintain thread safety
- **Read once**: File contents read once into memory for all analysis operations
- **Early filtering**: Exclusion patterns applied early during traversal
- **Atomic progress**: Progress bar updates use atomic operations
- **Release optimizations**: LTO enabled, single codegen unit, symbols stripped

## Common Pitfalls

1. **Taking shortcuts**: Never compromise on correctness for speed of implementation. Take the time to do it right.
2. **Following Python code line-by-line**: The reference is for understanding requirements, not for copying implementation patterns.
3. **Skipping edge cases**: The original has edge cases that must be handled. Study the tests thoroughly.
4. **License data missing**: Run `./setup.sh` to initialize submodule
5. **Cross-platform paths**: Use `Path` and `PathBuf`, not string concatenation
6. **Line endings**: Be careful with `\n` vs `\r\n` in tests
7. **Unwrap in library code**: Use `?` or `match` instead
8. **Breaking parallel processing**: Ensure modifications maintain thread safety
9. **Incomplete testing**: Every feature needs comprehensive test coverage including edge cases

## Porting Features from Original ScanCode

When implementing features from the original Python codebase at `reference/scancode-toolkit/`:

### Implementation Principles

1. **Research exhaustively**: Read the original implementation, tests, and documentation to understand:
   - The complete feature specification and all edge cases
   - Input formats, output structures, and error conditions
   - Known bugs, workarounds, and technical debt
   - User expectations and real-world usage patterns

2. **Achieve feature parity, not code parity**:
   - Every capability of the original must be preserved
   - Every edge case must be handled (correctly this time)
   - Output must be functionally equivalent (same JSON structure, same semantics)
   - **DO NOT** replicate line-by-line - use the reference to understand requirements, not implementation

3. **Design for correctness**:
   - Use Rust's type system to make invalid states unrepresentable
   - Leverage compiler guarantees instead of runtime checks where possible
   - Implement proper error handling with `Result<T, E>` (no exception-based control flow)
   - Write code that's self-documenting through strong types and clear interfaces

4. **Never compromise on quality**:
   - Take the time to implement comprehensive test coverage
   - Include test cases for bugs present in the original (document what you fixed)
   - Handle all error conditions explicitly - no silent failures
   - Don't ship until it's correct, complete, and well-tested

5. **Leverage Rust advantages**:
   - Use zero-copy parsing where possible (e.g., `&str` instead of `String`)
   - Apply compile-time optimizations (const evaluation, inlining)
   - Exploit the ownership system for memory safety without runtime cost
   - Use iterators and functional patterns for clarity and performance

6. **Document intentional differences**: If the Rust implementation differs behaviorally from the original:
   - Explain why (usually: fixing a bug or edge case)
   - Document the original behavior vs new behavior
   - Add tests demonstrating the improvement

### Example Workflow

```bash
# STEP 1: Study the original implementation thoroughly
cd reference/scancode-toolkit/
grep -r "relevant_function_name" src/
cat src/packagedcode/npm.py

# Look at tests to understand expected behavior and edge cases
find tests/ -name "*npm*" -type f
cat tests/packagedcode/test_npm.py

# Check for known issues
git log --all --grep="npm" --grep="bug" --oneline

# STEP 2: Return to main project and design the Rust implementation
cd ../..

# Create comprehensive test cases FIRST (TDD approach)
# Include edge cases found in original tests + cases for known bugs
vim src/parsers/npm_test.rs

# STEP 3: Implement in idiomatic Rust with proper error handling
vim src/parsers/npm.rs

# STEP 4: Verify correctness against original behavior
cargo test npm
# Run on real-world testdata and compare outputs with original
```

### Quality Checklist

Before considering a feature complete:

- [ ] All original functionality is preserved
- [ ] All edge cases from original tests are covered
- [ ] Known bugs from original are fixed (and tested)
- [ ] Error handling is comprehensive and explicit
- [ ] Code is idiomatic Rust (passes `clippy` without warnings)
- [ ] Performance is equal to or better than original
- [ ] Real-world testdata produces correct output
- [ ] Documentation explains any intentional behavioral differences

## Parser Implementation Guidelines

**Comprehensive step-by-step guide**: [`docs/HOW_TO_ADD_A_PARSER.md`](docs/HOW_TO_ADD_A_PARSER.md)

### Key Principles

1. **Feature parity**: Every field Python extracts, Rust must extract
2. **Security first**: AST-only parsing, no code execution (see [ADR 0004](docs/adr/0004-security-first-parsing.md))
3. **Beyond parity**: Fix bugs, implement TODOs (document in `docs/improvements/`)
4. **Validation**: Golden tests against Python reference (see [ADR 0003](docs/adr/0003-golden-test-strategy.md))

## Dependency Scope Conventions

The `Dependency.scope` field uses **native ecosystem terminology** to preserve semantic fidelity. This enables accurate round-tripping and maintains compatibility with ecosystem-specific tooling.

### npm Ecosystem

- **npm/yarn/pnpm package.json**:
  - `"dependencies"` - Regular runtime dependencies
  - `"devDependencies"` - Development-only dependencies
  - `"peerDependencies"` - Required peer dependencies
  - `"optionalDependencies"` - Optional runtime dependencies
  - `"bundledDependencies"` - Dependencies bundled with the package

- **npm lockfiles** (package-lock.json, npm-shrinkwrap.json):
  - `"dependencies"` - Regular or optional runtime dependencies
  - `"devDependencies"` - Development dependencies

- **yarn lockfiles**:
  - `"dependencies"` - Regular runtime dependencies (v1 and v2+)
  - `"peerDependencies"` - Peer dependencies (v2+ only; v1 doesn't distinguish)

- **pnpm lockfiles** (nested dependencies):
  - `None` - Top-level packages (no scope)
  - `"dev"` - Development dependencies
  - `"peer"` - Peer dependencies
  - `"optional"` - Optional dependencies

### Python Ecosystem

- **pyproject.toml (PEP 621)**:
  - `None` - Regular runtime dependencies (from `dependencies` array)
  - `"<extra_name>"` - Optional dependency groups (from `optional-dependencies.<extra_name>`)

- **pyproject.toml (Poetry)**:
  - `"dependencies"` - Regular runtime dependencies (from `[tool.poetry.dependencies]`)
  - `"dev-dependencies"` - Development dependencies (from `[tool.poetry.dev-dependencies]`)
  - `"<group_name>"` - Dependency groups (from `[tool.poetry.group.<group_name>.dependencies]`)

- **setup.py, setup.cfg**:
  - `"install"` - Regular runtime dependencies
  - `"<extra_name>"` - Optional dependency groups (from `extras_require`)

- **poetry.lock**:
  - `None` - All dependencies (no scope distinction in lockfile)
  - `is_optional` flag indicates dev dependencies

- **Pipfile.lock**:
  - `"install"` - Regular runtime dependencies (from `default` section)
  - `"develop"` - Development dependencies (from `develop` section)

- **requirements.txt** (filename-based):
  - `"install"` - Regular requirements.txt
  - `"develop"` - requirements-dev.txt or requirements/dev.txt
  - `"test"` - requirements-test.txt or requirements/test.txt
  - `"docs"` - requirements-doc.txt or requirements/doc.txt

### Rust Ecosystem

- **Cargo.toml**:
  - `"dependencies"` - Regular runtime dependencies
  - `"dev-dependencies"` - Development-only dependencies
  - `"build-dependencies"` - Build-time dependencies

- **Cargo.lock**:
  - `"dependencies"` - All runtime dependencies (dev/build deps not in lockfile by design)

### Java Ecosystem

- **Maven pom.xml** (`<scope>` element):
  - `None` - Default scope (equivalent to `compile`)
  - `"compile"` - Compile and runtime (default)
  - `"test"` - Test-time only
  - `"provided"` - Provided by runtime environment
  - `"runtime"` - Runtime only (not compile-time)
  - `"system"` - System-provided JARs

### Cross-Ecosystem Normalization

The `scope` field is intentionally **not standardized** across ecosystems. For cross-ecosystem analysis:

- Use `is_runtime` flag: `true` for runtime dependencies, `false` for dev/test/build
- Use `is_optional` flag: `true` for optional dependencies
- Future: Consider adding `normalized_scope` enum for standardized queries

## Additional Notes

- **Rust toolchain**: Version pinned in `rust-toolchain.toml` (currently 1.93.0)
- **Output format**: ScanCode Toolkit-compatible JSON with `SCANCODE_OUTPUT_FORMAT_VERSION`
- **License detection**: Uses SPDX license data, threshold of 0.9 confidence
- **Exclusion patterns**: Supports glob patterns (e.g., `*.git*`, `node_modules/*`)
- **Git submodules**: Two submodules - `resources/licenses/` (SPDX data) and `reference/scancode-toolkit/` (original Python codebase for reference)
