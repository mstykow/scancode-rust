# ADR 0001: Trait-Based Parser Architecture

**Status**: Accepted  
**Authors**: scancode-rust team  
**Supersedes**: None

## Context

scancode-rust needs a unified, type-safe way to handle multiple package ecosystems and file formats while maintaining:

1. **Compile-time guarantees** - Catch errors before runtime
2. **Easy extensibility** - Adding new parsers should be straightforward
3. **Testability** - Each parser should be independently testable
4. **Clear contracts** - Implementers should know exactly what to provide

The Python reference implementation uses runtime class inspection and dynamic dispatch, which works but lacks compile-time type safety and can lead to subtle runtime errors.

## Decision

We use a **trait-based parser system** where all parsers implement the `PackageParser` trait:

```rust
pub trait PackageParser {
    /// The package ecosystem identifier (e.g., "npm", "cargo", "maven")
    const PACKAGE_TYPE: &'static str;

    /// Determines if this parser can handle the given file
    fn is_match(path: &Path) -> bool;

    /// Extracts package metadata from the file
    fn extract_package_data(path: &Path) -> PackageData;
}
```

### Implementation Pattern

Each parser is a zero-sized type (struct with no fields) that implements this trait:

```rust
pub struct NpmParser;

impl PackageParser for NpmParser {
    const PACKAGE_TYPE: &'static str = "npm";

    fn is_match(path: &Path) -> bool {
        matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some("package.json" | "package-lock.json" | "npm-shrinkwrap.json")
        )
    }

    fn extract_package_data(path: &Path) -> PackageData {
        // Implementation
    }
}
```

### Unified Data Model

All parsers return the same `PackageData` struct, which normalizes differences across ecosystems:

```rust
pub struct PackageData {
    // Package identity
    pub package_type: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub namespace: Option<String>,
    
    // Metadata
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub parties: Vec<Party>,
    
    // Dependencies
    pub dependencies: Vec<Dependency>,
    
    // Licenses (extraction only - detection is separate)
    pub extracted_license_statement: Option<String>,
    
    // Checksums
    pub sha256: Option<String>,
    pub sha1: Option<String>,
    
    // URLs
    pub repository_homepage_url: Option<String>,
    pub repository_download_url: Option<String>,
    
    // Additional data
    pub extra_data: serde_json::Value,
}
```

## Consequences

### Benefits

1. **Type Safety**
   - Rust compiler ensures all parsers implement the required methods
   - Impossible to "forget" to implement a method
   - Refactoring is safe - compiler catches breaking changes

2. **Zero Runtime Cost**
   - Trait methods can be statically dispatched
   - No vtable lookups for performance-critical paths
   - Zero-sized types have no memory overhead

3. **Clear Contract**
   - New parser authors know exactly what to implement
   - Documentation is self-evident from the trait definition
   - IDE autocomplete shows required methods

4. **Easy Testing**
   - Each parser can be tested in isolation
   - No need for complex test harnesses
   - Simple unit tests: `assert!(NpmParser::is_match(path))`

5. **Ecosystem Normalization**
   - Single `PackageData` struct unifies all formats
   - Easier to generate SBOM/SPDX output
   - Consistent JSON serialization

### Trade-offs

1. **Less Dynamic**
   - Cannot add parsers at runtime (but we don't need to)
   - Parser registration is compile-time only
   - Acceptable trade-off for type safety

2. **Boilerplate**
   - Each parser needs a struct declaration + impl block
   - More verbose than Python's class-based approach
   - Mitigated by IDE snippets and clear patterns

3. **Learning Curve**
   - Contributors need to understand Rust traits
   - Not as immediately obvious as Python classes
   - Mitigated by comprehensive documentation and examples

## Alternatives Considered

### 1. Enum-Based Dispatch

```rust
enum Parser {
    Npm(NpmParser),
    Cargo(CargoParser),
    // ...
}
```

**Rejected because**:

- Requires modifying central enum for every new parser
- Doesn't scale to 40+ parsers
- Makes testing harder (can't import parsers independently)

### 2. Dynamic Dispatch with `Box<dyn Parser>`

```rust
trait Parser {
    fn is_match(&self, path: &Path) -> bool;
    fn extract(&self, path: &Path) -> PackageData;
}
```

**Rejected because**:

- Runtime overhead (vtable lookups)
- Heap allocation for trait objects
- Less idiomatic for stateless parsers
- Loses `const` benefits

### 3. Function Pointer Registry

```rust
type ParserFn = fn(&Path) -> PackageData;

fn register_parser(name: &str, parser: ParserFn) { ... }
```

**Rejected because**:

- No compile-time guarantees
- Hard to type-check
- Error-prone registration
- Resembles Python too closely (loses Rust advantages)

## Python Reference Comparison

**Python Approach** (from `reference/scancode-toolkit/`):

```python
class DatafileHandler:
    datasource_id = 'npm_package_json'
    path_patterns = ('*/package.json',)
    default_package_type = 'npm'
    
    @classmethod
    def parse(cls, location):
        # Implementation
```

**Key Differences**:

| Aspect | Python | Rust (Our Approach) |
|--------|--------|---------------------|
| **Type Safety** | Runtime (duck typing) | Compile-time (traits) |
| **Dispatch** | Dynamic (class inspection) | Static (monomorphization) |
| **Performance** | Interpreted + vtables | Zero-cost abstractions |
| **Extensibility** | Plugin system (runtime) | Traits (compile-time) |
| **Error Detection** | Runtime errors | Compile-time errors |

## Related ADRs

- [ADR 0002: Extraction vs Detection Separation](0002-extraction-vs-detection.md) - Why parsers only extract, never detect
- [ADR 0003: Golden Test Strategy](0003-golden-test-strategy.md) - How we validate parser correctness
- [ADR 0005: Auto-Generated Documentation](0005-auto-generated-docs.md) - How parser metadata is documented

## References

- [Rust Book: Traits](https://doc.rust-lang.org/book/ch10-02-traits.html)
- [Zero-Sized Types in Rust](https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts)
