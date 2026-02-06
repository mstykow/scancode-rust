# Agent Guidelines for scancode-rust

This guide provides essential information for AI coding agents working on the `scancode-rust` codebase - a high-performance Rust tool for detecting licenses, copyrights, and package metadata in source code.

## Project Context

**scancode-rust** is a complete rewrite of [ScanCode Toolkit](https://github.com/aboutcode-org/scancode-toolkit) in Rust, designed to be faster, more reliable, and bug-free. The original Python codebase is available as a reference submodule at `reference/scancode-toolkit/`.

### Using the Reference Submodule

The `reference/scancode-toolkit/` submodule contains the original Python implementation and serves as:

- **Inspiration for porting features**: When implementing new functionality, examine the original code to understand the logic and edge cases
- **Reference for behavior**: Verify expected behavior and output formats against the original implementation
- **Bug avoidance**: Identify known issues in the original and implement cleaner solutions in Rust

⚠️ **Important**: This is a _rewrite_, not a port. Use the original code as reference only. Do not replicate its bugs, architectural issues, or outdated patterns. Focus on clean, idiomatic Rust code that improves upon the original.

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

**Module Structure:**

- `src/main.rs` - CLI entry point, orchestrates scanning workflow
- `src/scanner/` - Core file system traversal and parallel processing
- `src/parsers/` - Package manifest parsers (npm, Cargo, Maven, Python)
- `src/models/` - Data structures for scan results and output format
- `src/askalono/` - License detection using n-gram analysis
- `src/utils/` - File operations, hashing, language detection, SPDX handling

**Key Design Patterns:**

1. **Trait-Based Parsers**: All parsers implement `PackageParser` trait
2. **Builder Pattern**: Complex structs (e.g., `FileInfo`) use derive_builder
3. **Result-Based Errors**: Use `Result<T, E>` with `anyhow::Error` for error propagation
4. **Parallel Processing**: Uses `rayon` for multi-threaded file scanning
5. **Compile-Time Embedding**: License data embedded via `include_dir!` macro

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

## Testing Conventions

- **Co-located tests**: Use `#[cfg(test)] mod tests { ... }` in implementation files
- **Separate test files**: For larger test suites, use `<module>_test.rs` pattern
- **Test data**: Place in `testdata/` directory, organized by ecosystem
- **Helper functions**: Create helpers for common test setup (e.g., `create_temp_file()`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

1. **License data missing**: Run `./setup.sh` to initialize submodule
2. **Cross-platform paths**: Use `Path` and `PathBuf`, not string concatenation
3. **Line endings**: Be careful with `\n` vs `\r\n` in tests
4. **Unwrap in library code**: Use `?` or `match` instead
5. **Breaking parallel processing**: Ensure modifications maintain thread safety

## Porting Features from Original ScanCode

When implementing features inspired by the original Python codebase at `reference/scancode-toolkit/`:

1. **Research first**: Read the original implementation to understand the problem and approach
2. **Don't copy blindly**: The original has known bugs, performance issues, and technical debt
3. **Rethink the design**: Leverage Rust's type system, ownership model, and modern patterns
4. **Improve error handling**: Use `Result<T, E>` instead of exception-based control flow
5. **Add comprehensive tests**: Include test cases that cover bugs present in the original
6. **Document deviations**: If you intentionally differ from the original behavior, document why

### Example Workflow

```bash
# Explore the original implementation
cd reference/scancode-toolkit/
grep -r "relevant_function_name" src/

# Understand the data structures and logic
cat src/packagedcode/npm.py

# Return to main project and implement in Rust
cd ../..
# Implement in src/parsers/npm.rs with improvements
```

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

- **pyproject.toml, setup.py, setup.cfg**:
  - `None` or `"install"` - Regular runtime dependencies
  - `"dev"`, `"test"`, `"docs"` - Optional dependency groups (from `[tool.poetry.dev-dependencies]` or `extras_require`)

- **poetry.lock**:
  - `"dependencies"` - All direct dependencies (doesn't distinguish dev vs regular)
  - `"<extra_name>"` - Dependencies from extras groups

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

## Useful References

- README: `README.md` (user documentation)
- Cargo manifest: `Cargo.toml` (dependencies and project config)
