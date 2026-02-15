# Testing Strategy

## Philosophy

scancode-rust uses a **behavior-focused, multi-layered testing approach** that prioritizes intelligent coverage over arbitrary test quotas.

### Core Principles

1. **Test Behavior, Not Implementation**
   - Focus on what the code does, not how it does it
   - Tests should survive refactoring
   - Edge cases matter more than line coverage

2. **High-Value Tests Over High Counts**
   - One well-designed test beats ten redundant tests
   - Every test should verify meaningful behavior
   - No tests for the sake of reaching coverage targets

3. **Fast Feedback Loops**
   - Unit tests run in milliseconds (parallel execution)
   - Instant failure isolation
   - Developers get immediate actionable feedback

4. **Complementary Layers**
   - Doctests verify API documentation examples work
   - Unit tests verify component correctness
   - Golden tests catch regressions
   - Integration tests validate end-to-end behavior

---

## Test Architecture

### Layer 0: Doctests

**Purpose**: Verify API documentation examples work correctly and serve as living documentation

**Characteristics**:

- Code examples in `///` doc comments that run as tests
- Ensures documentation stays synchronized with code
- Provides working examples for users
- Runs with `cargo test --doc`

**When to Write**:

- For all public API functions with non-trivial usage
- When examples would help users understand the API
- For complex function signatures requiring setup examples

**Example**:

```rust
/// Extracts package metadata from a manifest file.
///
/// # Examples
///
/// ```rust
/// use scancode_rust::scanner::process;
/// use std::path::PathBuf;
///
/// let result = process(&PathBuf::from(".""), 50, progress, &patterns, &strategy)?;
/// println!("Found {} files", result.files.len());
/// ```
pub fn process(...) -> Result<ScanResult> {
    // Implementation
}
```

**Why This Matters**: Documentation examples that don't compile or fail are caught immediately. Users can trust that documented examples actually work.

**Location**: Inline in source code as `///` or `//!` doc comments

**Current Status**: 14 doctests covering main API entry points (all passing)

---

### Layer 1: Unit Tests

**Purpose**: Verify individual components work correctly in isolation

**Characteristics**:

- Test single functions or small groups of related functions
- Mock external dependencies where appropriate
- Fast execution (parallel, minimal I/O)
- Pinpoint exact failure location

**When to Write**:

- Every parser function (field extraction, validation, transformation)
- Every edge case (empty input, malformed data, extreme values)
- Every business rule (dependency resolution, version constraints, PURL generation)

**Example**:

```rust
#[test]
fn test_parse_dependency_with_alternatives() {
    let deps = parse_dependency_field("libc6 | libc6-udeb", ...);
    assert_eq!(deps.len(), 2);
    assert!(deps[0].is_optional.unwrap());
    assert!(deps[1].is_optional.unwrap());
}
```

**Why This Matters**: When this test fails, you immediately know which exact function and input combination failed.

**Location**: Inline `#[cfg(test)] mod tests { ... }` blocks in implementation files or separate `*_test.rs` files in `src/parsers/`

---

### Layer 2: Golden Tests

**Purpose**: Catch regressions by comparing output against reference implementation

**Characteristics**:

- Compare full PackageData output to expected JSON
- Use real-world manifest files as test data
- Validate against Python ScanCode Toolkit reference
- Document intentional differences (see ADR 0003)

**When to Write**:

- After parser is stable and fully implemented
- When reference output available from Python ScanCode
- For parsers with complex output structures

**Example**:

```rust
#[test]
fn test_golden_debian_control() {
    let result = DebianControlParser::extract_package_data("testdata/debian/control");
    let expected = read_expected_json("testdata/debian/control.expected.json");
    assert_package_data_eq(result, expected);
}
```

**Why This Matters**: Prevents accidentally breaking feature parity with Python reference.

**Location**: Separate `*_golden_test.rs` files in `src/parsers/` with test data in `testdata/<ecosystem>-golden/`

**Test Utilities**: Uses `test_utils::compare_package_data_parser_only()` which:

- Skips dynamic/time-sensitive fields (identifiers, line numbers, matched_text)
- Handles optional license detection fields gracefully
- Provides clear diff messages on mismatch

---

### Layer 3: Integration Tests

**Purpose**: Validate end-to-end scanner behavior (file discovery → parsing → output)

**Location**: `tests/scanner_integration.rs` (top-level integration test suite)

**Characteristics**:

- Test the full `process()` pipeline
- Verify multi-parser coordination
- Validate error handling and graceful degradation
- Test scanner options (exclusions, depth limits, etc.)

**When to Write**:

- After major scanner changes
- When adding new scanner features (filters, output formats)
- To verify cross-parser interactions
- To test error handling across the pipeline

**Example Scenarios Covered**:

- Multi-parser discovery (npm + pypi + cargo in same directory)
- Output format structure validation (all required fields present)
- Error handling (malformed manifests don't crash scanner)
- Exclusion patterns work correctly
- Max depth limits are respected
- Empty directories handled gracefully

**Why This Matters**: Unit tests verify components work; integration tests verify they work together correctly.

**Example**:

```rust
#[test]
fn test_scanner_discovers_all_registered_parsers() {
    let result = process("testdata/integration/multi-parser", ...);
    
    assert!(result.files.iter().any(|f| f.package_data[0].package_type == Some("npm")));
    assert!(result.files.iter().any(|f| f.package_data[0].package_type == Some("pypi")));
    assert!(result.files.iter().any(|f| f.package_data[0].package_type == Some("cargo")));
}
```

---

## Rust vs Python Comparison

### Python ScanCode Toolkit Approach

**Structure**:

- Primarily golden tests (parse file → compare to `.expected.json`)
- Tests entire pipeline at once
- Typically 10-15 test files per ecosystem

**Trade-offs**:

- ✅ Catches regressions in full output
- ❌ Hard to debug when tests fail (which field? which line?)
- ❌ Large JSON diffs are difficult to interpret
- ❌ Slower execution (file I/O, JSON serialization)

### scancode-rust Approach

**Structure**:

- Doctests for API documentation verification
- Comprehensive unit tests for component behavior
- Golden tests for regression detection
- Integration tests for end-to-end validation

**Trade-offs**:

- ✅ Immediate failure isolation (know exactly what broke)
- ✅ Fast parallel execution (minimal I/O in unit tests)
- ✅ Easy to maintain (update specific assertions, not large JSON files)
- ✅ Better coverage of edge cases
- ✅ Tests survive refactoring (test behavior, not implementation)
- ❌ More tests to write initially (but pays off long-term)

**Performance Advantage**: Rust tests typically run 3-5x faster than equivalent Python tests due to parallel execution and no interpreter overhead.

---

## Testing Guidelines

### What Makes a Good Test

**DO**:

- Test observable behavior (inputs → expected outputs)
- Use descriptive test names (`test_parse_debian_dependency_with_version_constraint`)
- Test edge cases (empty strings, Unicode, extreme values)
- Keep tests independent (no shared state between tests)
- Use real-world test data where possible

**DON'T**:

- Test implementation details (private functions, internal state)
- Write tests just to hit coverage targets
- Copy-paste tests (use helper functions for common patterns)
- Ignore failing tests (fix or remove them)
- Skip error cases (test both success and failure paths)

### When to Use Each Test Type

| Scenario | Test Type |
|----------|-----------|
| Public API function with complex usage | Doctest |
| New parser function | Unit test |
| Edge case discovered | Unit test |
| Parser fully implemented | Golden test |
| Scanner feature added | Integration test |
| Bug found in production | Unit test (reproduce) + fix + verify |
| Refactoring parser internals | Unit tests should still pass |
| Changing API signature | Doctests will break (expected) |
| Changing output format | Golden tests will break (expected) |

---

## Test Organization

### File Structure

```text
src/parsers/
├── npm.rs                    # Implementation
├── npm_test.rs               # Unit tests (co-located)
└── npm_golden_test.rs        # Golden tests (separate file)

tests/
└── scanner_integration.rs    # Integration tests (top-level)

testdata/
├── npm/                      # Unit test data
│   ├── package.json
│   └── package-lock.json
├── npm-golden/               # Golden test data with .expected files
│   ├── simple/
│   │   ├── package.json
│   │   └── package.json.expected
│   └── complex/
│       ├── yarn.lock
│       └── yarn.lock.expected
└── integration/              # Integration test data
    └── multi-parser/
        ├── package.json
        ├── pyproject.toml
        └── Cargo.toml
```

### Naming Conventions

**Unit Tests**:

- `test_<function_name>_<scenario>` (e.g., `test_parse_dependency_with_alternatives`)
- `test_<component>_<edge_case>` (e.g., `test_rfc822_parser_handles_empty_fields`)

**Golden Tests**:

- `test_golden_<ecosystem>_<format>` (e.g., `test_golden_npm_package_json`)

**Integration Tests**:

- `test_<scanner_feature>_<scenario>` (e.g., `test_scanner_discovers_all_registered_parsers`)

---

## Running Tests

### All Tests

```bash
cargo test                    # Run all tests except golden tests
cargo test --lib              # Run only library tests (faster, excludes integration)
cargo test --doc              # Run only doctests
cargo test --test '*'         # Run only integration tests
cargo test --features golden-tests  # Include golden tests (slower, compares against Python ScanCode)
```

> **Note**: Golden tests (comparing output against Python ScanCode reference) are gated behind the `golden-tests` feature flag because they are slow and require the reference submodule. They run automatically in CI but are excluded from `cargo test` by default for faster local development.

### Specific Test Categories

```bash
cargo test npm_test           # All npm unit tests
cargo test golden             # All golden tests
cargo test scanner_integration  # All integration tests
cargo test --doc              # All API documentation examples
```

### Single Test

```bash
cargo test test_parse_dependency_with_alternatives
```

### Ignored Tests

Golden tests are gated behind the `golden-tests` feature flag:

```bash
cargo test --features golden-tests             # Run all tests including golden tests
cargo test --lib --features golden-tests golden # Run only golden tests
```

### CI/CD

Tests run automatically on:

- Every commit (via pre-commit hooks: `cargo fmt`, `cargo clippy`)
- Every push to main
- Every pull request

All tests must pass before merging. Commands:

- `cargo test --all --verbose` — unit tests, doctests, integration tests
- `cargo test --all --verbose --features golden-tests` — all of the above plus golden tests

---

## Quality Gates

Before marking a parser complete, verify:

- [ ] **Unit tests** cover all public functions and edge cases
- [ ] **Golden tests** exist for at least one real-world file per format
- [ ] **Integration test** verifies parser is discovered and invoked correctly (if adding new ecosystem)
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)

---

## Related Documentation

- **[ADR 0003: Golden Test Strategy](adr/0003-golden-test-strategy.md)** - Why and how we use golden tests
- **[HOW_TO_ADD_A_PARSER.md](HOW_TO_ADD_A_PARSER.md)** - Step-by-step parser implementation guide
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and test infrastructure

---

## Summary

**Testing is about confidence, not coverage.**

Write tests that:

1. Verify meaningful behavior
2. Catch real bugs
3. Survive refactoring
4. Provide fast feedback

**Our multi-layered approach ensures**:

- Doctests verify API documentation examples actually work
- Unit tests verify components work correctly
- Golden tests ensure feature parity with Python reference
- Integration tests validate end-to-end behavior
- Fast CI/CD feedback loop (parallel execution, instant failure isolation)

**Result**: High-quality, maintainable test suite that gives developers confidence to refactor and evolve the codebase.
