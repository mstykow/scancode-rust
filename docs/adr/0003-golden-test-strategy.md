# ADR 0003: Golden Test Strategy

**Status**: Accepted  
**Authors**: scancode-rust team  
**Supersedes**: None

## Context

We need a reliable way to verify that scancode-rust produces output functionally equivalent to the Python ScanCode Toolkit reference implementation. Key challenges:

1. **Feature Parity Verification** - How do we prove our parsers extract the same data?
2. **Regression Prevention** - How do we catch unintended behavior changes?
3. **Edge Case Coverage** - How do we ensure rare formats and corner cases work?
4. **Architectural Differences** - How do we handle intentional implementation differences?

The Python reference implementation has extensive test data and expected outputs, but our Rust implementation may legitimately differ in structure (e.g., single package vs array, field ordering).

## Decision

We use **golden testing** where parsers are validated against reference outputs from ScanCode Toolkit, with documented exceptions for intentional architectural differences.

### Golden Test Workflow

```text
┌──────────────────┐
│ testdata/        │
│ npm/package.json │
│                  │
└────────┬─────────┘
         │
         ├─────────────────────────┐
         │                         │
         ▼                         ▼
┌──────────────────┐      ┌──────────────────┐
│ Python ScanCode  │      │ scancode-rust    │
│                  │      │                  │
│ scancode -p ...  │      │ NpmParser::      │
│                  │      │ extract_package  │
└────────┬─────────┘      └────────┬─────────┘
         │                         │
         ▼                         ▼
┌──────────────────┐      ┌──────────────────┐
│ expected.json    │      │ actual output    │
│ (reference)      │      │                  │
└────────┬─────────┘      └────────┬─────────┘
         │                         │
         └─────────┬───────────────┘
                   │
                   ▼
            ┌─────────────┐
            │ JSON diff   │
            │ comparison  │
            └─────────────┘
```

### Implementation Pattern

**1. Generate Reference Output** (one-time setup per test case):

```bash
cd reference/scancode-toolkit
scancode -p testdata/npm/package.json --json expected/npm-package.json
```

**2. Create Golden Test** (in Rust):

```rust
#[test]
fn test_golden_npm_simple() {
    let path = Path::new("testdata/npm/package.json");
    let result = NpmParser::extract_package_data(path);
    
    let expected = read_expected_json("expected/npm-package.json");
    
    // Compare semantically, ignoring field order
    assert_package_data_eq(result, expected);
}
```

**3. Handle Intentional Differences**:

```rust
#[test]
#[ignore = "Architectural difference: Rust uses single PackageData with dependencies array, Python uses packages array. See ADR 0001."]
fn test_golden_cocoapods_podfile() {
    // Test code here
}
```

### Test Organization

```text
src/parsers/
├── npm.rs                    # Implementation
├── npm_test.rs               # Unit tests
└── npm_golden_test.rs        # Golden tests

testdata/
├── npm/
│   ├── package.json          # Test input
│   ├── package-lock.json
│   └── yarn.lock
└── expected/
    ├── npm-package.json      # Reference output
    ├── npm-lockfile.json
    └── npm-yarn.json
```

## Consequences

### Benefits

1. **Feature Parity Proof**
   - Direct comparison with Python reference
   - Catches missing fields or incorrect values
   - Validates edge case handling

2. **Regression Prevention**
   - Any change that breaks compatibility is caught immediately
   - Prevents accidental feature removal
   - Safe refactoring with confidence

3. **Documentation of Differences**
   - Ignored tests document WHY we differ from Python
   - Architectural decisions are explicit
   - Future maintainers understand context

4. **Real-World Test Data**
   - Uses actual package manifests from ecosystems
   - Covers edge cases found in production
   - Validates against proven reference implementation

5. **Continuous Validation**
   - Pre-commit hooks run tests
   - CI validates on every push
   - Automated regression detection

### Trade-offs

1. **Test Maintenance**
   - Must regenerate expected outputs if Python changes
   - Need to document intentional differences
   - Acceptable: Worth the confidence in correctness

2. **Blocked Tests**
   - Some tests blocked on detection engine (license normalization)
   - Can't validate full output until detection is implemented
   - Acceptable: Unit tests validate extraction correctness

3. **JSON Structure Differences**
   - Must handle field ordering differences
   - Some fields may be legitimately different (e.g., array vs single object)
   - Mitigated: Custom comparison logic, documented exceptions

### Documented Architectural Differences

#### 1. CocoaPods & Swift: Package Structure

**Python Approach**:

```json
{
  "packages": [
    {"name": "Alamofire", "version": "5.4.0"},
    {"name": "SwiftyJSON", "version": "5.0.0"}
  ]
}
```

**Rust Approach**:

```json
{
  "name": "MyApp",
  "dependencies": [
    {"name": "Alamofire", "version": "5.4.0"},
    {"name": "SwiftyJSON", "version": "5.0.0"}
  ]
}
```

**Rationale**: Both are valid representations. Rust uses normalized `PackageData` struct for consistency. Validated via comprehensive unit tests.

**Decision**: Document difference, ignore golden tests, rely on unit tests.

#### 2. Alpine: Provider Field (Beyond Parity)

**Python**: Provider field (`p:`) is ignored ("not used yet")

**Rust**: Provider field fully extracted and stored in `extra_data.providers`

**Rationale**: We implement features that Python has marked as TODO. This is intentional improvement.

**Decision**: Document as enhancement, ignore golden test for provider field.

## Alternatives Considered

### 1. Unit Tests Only (No Golden Tests)

**Approach**: Test individual parser functions without comparing to Python reference.

```rust
#[test]
fn test_npm_parser() {
    let result = parse_package_json("...");
    assert_eq!(result.name, Some("lodash".to_string()));
    assert_eq!(result.version, Some("4.17.21".to_string()));
    // ... manual assertions for every field
}
```

**Rejected because**:

- No proof of feature parity with Python reference
- Easy to miss fields or edge cases
- Manual assertion maintenance is error-prone
- Doesn't catch regressions against reference

### 2. Snapshot Testing (insta crate)

**Approach**: Generate snapshots of Rust output, review diffs manually.

```rust
#[test]
fn test_npm_parser() {
    let result = parse_package_json("...");
    insta::assert_json_snapshot!(result);
}
```

**Rejected because**:

- No comparison with Python reference (our source of truth)
- Snapshot becomes the truth (circular validation)
- Harder to verify feature parity
- Doesn't validate against proven reference implementation

### 3. Property-Based Testing (proptest)

**Approach**: Generate random inputs, verify properties hold.

```rust
proptest! {
    fn test_npm_parser_doesnt_panic(input: String) {
        let _ = parse_package_json(&input);
    }
}
```

**Partial acceptance**: We use property-based testing for security (DoS protection, invalid input handling), but NOT as primary validation strategy.

**Why not primary**:

- Can't verify feature parity with reference
- Doesn't test real-world manifests
- Hard to generate valid package manifests
- Golden tests are more effective for correctness

### 4. Integration Testing via CLI

**Approach**: Run full scancode-rust CLI, compare JSON output.

```bash
cargo run -- testdata/npm/ -o actual.json
diff actual.json expected.json
```

**Partial acceptance**: We do this at CI level, but NOT as primary test strategy.

**Why not primary**:

- Slower than unit/golden tests
- Harder to debug failures
- Can't test parsers in isolation
- Golden tests at parser level are more granular

## Implementation Guidelines

### When to Write a Golden Test

✅ **Write golden test when**:

- Parser is complete and stable
- Reference output available from Python ScanCode
- Edge cases covered by real test data

❌ **Don't write golden test when**:

- Feature depends on detection engine (not yet built)
- Architectural difference makes comparison meaningless
- Parser is still experimental/unstable

### When to Ignore a Golden Test

Document with `#[ignore = "reason"]` when:

1. **Detection Engine Dependency**: Test requires license normalization or copyright detection
2. **Architectural Difference**: Intentional implementation difference (e.g., data structure)
3. **Beyond Parity**: We implement features Python has as TODO/missing

**Always document WHY** in the ignore attribute.

### Custom Comparison Logic

Handle legitimate differences:

```rust
fn assert_package_data_eq(actual: PackageData, expected: PackageData) {
    // Ignore field order in dependencies
    let actual_deps = sort_by_name(actual.dependencies);
    let expected_deps = sort_by_name(expected.dependencies);
    assert_eq!(actual_deps, expected_deps);
    
    // Ignore null vs missing fields
    assert_eq_optional(actual.description, expected.description);
    
    // Normalize URLs (http vs https)
    assert_eq_normalized_url(actual.homepage_url, expected.homepage_url);
}
```

## Quality Gates

Before marking a parser complete:

- ✅ All relevant golden tests passing OR documented as ignored with reason
- ✅ Unit tests cover extraction logic
- ✅ Edge cases validated (empty files, malformed input, etc.)
- ✅ Real-world test data included
- ✅ Performance acceptable (benchmarked)

## Related ADRs

- [ADR 0001: Trait-Based Parser Architecture](0001-trait-based-parsers.md) - Parser structure enables golden testing
- [ADR 0002: Extraction vs Detection Separation](0002-extraction-vs-detection.md) - Why some tests are blocked on detection engine
- [ADR 0004: Security-First Parsing](0004-security-first-parsing.md) - Security property testing complements golden tests

## References

- Python reference test data: `reference/scancode-toolkit/tests/packagedcode/data/`
- Golden test examples: `src/parsers/*_golden_test.rs`
- Test infrastructure: `src/parsers/test_utils.rs`
- CI configuration: `.github/workflows/ci.yml`
