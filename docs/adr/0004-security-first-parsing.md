# ADR 0004: Security-First Parsing

**Status**: Accepted  
**Authors**: scancode-rust team  
**Supersedes**: None

## Context

Package parsers must handle untrusted input from arbitrary sources:

- Downloaded package manifests from public repositories
- User-provided codebases during scanning
- Potentially malicious or malformed files

The Python ScanCode Toolkit has several security issues:

1. **Code Execution**: Some parsers execute user code (setup.py with `eval()`, APKBUILD shell scripts, Gemfile/Podfile Ruby execution, Gradle with Groovy engine)
2. **DoS Vulnerabilities**: No limits on file size, recursion depth, or iteration count
3. **Archive Bombs**: Zip bomb protection is incomplete or missing
4. **Memory Exhaustion**: Large manifests can exhaust memory

**Critical Question**: How do we extract package metadata safely without introducing security vulnerabilities?

## Decision

**All parsers MUST follow security-first principles: No code execution, explicit resource limits, robust input validation.**

### Security Principles

#### 1. **No Code Execution** (MANDATORY)

Parsers **NEVER** execute user-provided code, regardless of ecosystem conventions.

| ❌ FORBIDDEN | ✅ REQUIRED | Example |
|-------------|------------|---------|
| `eval()`, `exec()` | AST parsing | Python setup.py |
| Subprocess calls | Static analysis | Shell scripts (APKBUILD) |
| Ruby `instance_eval` | Regex/AST parsing | Ruby Gemfile/Podfile |
| Groovy engine | Token-based lexer | Gradle build files |
| Jinja2 template rendering | String interpolation preservation | Conan conanfile.py |

**Rationale**:

- User code can be malicious (arbitrary code execution)
- Even benign code may have side effects (network calls, file writes)
- AST parsing provides same metadata without execution risk

#### 2. **DoS Protection** (MANDATORY)

All parsers enforce explicit limits:

| Resource | Limit | Enforcement | Example |
|----------|-------|-------------|---------|
| **File Size** | 100 MB default | Check before reading | Prevent memory exhaustion |
| **Recursion Depth** | 50 levels | Track in parser state | Prevent stack overflow |
| **Iteration Count** | 100,000 items | Break early with warning | Prevent infinite loops |
| **String Length** | 10 MB per field | Truncate with warning | Prevent memory attacks |

```rust
const MAX_FILE_SIZE: usize = 100 * 1024 * 1024; // 100 MB
const MAX_RECURSION_DEPTH: usize = 50;
const MAX_ITERATIONS: usize = 100_000;

fn extract_package_data(path: &Path) -> PackageData {
    // 1. Check file size
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_FILE_SIZE {
        warn!("File too large: {} bytes", metadata.len());
        return default_package_data();
    }
    
    // 2. Limit iterations
    for (i, dep) in dependencies.iter().enumerate() {
        if i >= MAX_ITERATIONS {
            warn!("Exceeded max iterations, stopping");
            break;
        }
        // Process dependency
    }
    
    // 3. Limit recursion (tracked in parser state)
    if recursion_depth > MAX_RECURSION_DEPTH {
        warn!("Exceeded max recursion depth");
        return default_value;
    }
}
```

#### 3. **Archive Safety** (Archives Only)

For parsers that extract archives (.deb, .rpm, .apk, .gem, .whl):

| Protection | Implementation | Threshold |
|------------|---------------|-----------|
| **Size Limits** | Check uncompressed size before extraction | 1 GB uncompressed |
| **Compression Ratio** | Reject excessive compression (zip bombs) | 100:1 ratio max |
| **Path Traversal** | Validate extracted paths don't escape temp dir | Block `../` patterns |
| **Decompression Limits** | Stop decompression after size threshold | 1 GB limit |

```rust
fn extract_archive(path: &Path) -> Result<TempDir> {
    let archive = Archive::open(path)?;
    
    // Check compression ratio
    let compressed_size = fs::metadata(path)?.len();
    let uncompressed_size = archive.total_uncompressed_size()?;
    let ratio = uncompressed_size / compressed_size;
    
    if ratio > MAX_COMPRESSION_RATIO {
        return Err("Suspicious compression ratio (possible zip bomb)");
    }
    
    // Check total uncompressed size
    if uncompressed_size > MAX_UNCOMPRESSED_SIZE {
        return Err("Archive too large when uncompressed");
    }
    
    // Extract with path validation
    for entry in archive.entries()? {
        let path = entry.path()?;
        
        // Prevent path traversal
        if path.components().any(|c| c == Component::ParentDir) {
            warn!("Skipping entry with parent dir: {:?}", path);
            continue;
        }
        
        entry.unpack(temp_dir.path().join(path))?;
    }
    
    Ok(temp_dir)
}
```

#### 4. **Input Validation** (MANDATORY)

All parsers validate input before processing:

| Validation | Check | Action on Failure |
|------------|-------|-------------------|
| **File Exists** | `fs::metadata()` | Return error, don't panic |
| **UTF-8 Encoding** | `String::from_utf8()` | Log warning, try lossy conversion |
| **JSON/YAML Validity** | `serde_json::from_str()` | Return default PackageData |
| **Required Fields** | Check `name`, `version` presence | Populate with `None`, continue |
| **URL Format** | Basic validation (not exhaustive) | Accept as-is, don't parse aggressively |

```rust
fn extract_package_data(path: &Path) -> PackageData {
    // 1. Validate file exists
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read {:?}: {}", path, e);
            return default_package_data();
        }
    };
    
    // 2. Validate JSON
    let manifest: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            warn!("Invalid JSON in {:?}: {}", path, e);
            return default_package_data();
        }
    };
    
    // 3. Extract fields with fallbacks (no unwrap)
    let name = manifest["name"].as_str().map(String::from);
    let version = manifest["version"].as_str().map(String::from);
    
    PackageData {
        name,
        version,
        // ... other fields
    }
}
```

#### 5. **Circular Dependency Detection** (Dependency Resolution Only)

For parsers that resolve transitive dependencies:

```rust
fn resolve_dependencies(
    package: &str,
    visited: &mut HashSet<String>,
) -> Vec<Dependency> {
    // Detect cycles
    if visited.contains(package) {
        warn!("Circular dependency detected: {}", package);
        return vec![];
    }
    
    visited.insert(package.to_string());
    
    // Resolve dependencies
    // ...
}
```

## Consequences

### Benefits

1. **Safe by Default**
   - No arbitrary code execution risk
   - Resistant to DoS attacks
   - Protected against zip bombs

2. **Predictable Resource Usage**
   - Known upper bounds on memory/CPU
   - Won't exhaust system resources
   - Safe to run in parallel

3. **Robust Error Handling**
   - Graceful degradation on malformed input
   - Warnings instead of panics
   - Continues scanning even if one file fails

4. **Auditability**
   - Clear security boundaries
   - Easy to review for vulnerabilities
   - Documented threat model

5. **Better than Python Reference**
   - Python executes setup.py, APKBUILD, Gemfiles (UNSAFE)
   - Python has no DoS limits (VULNERABLE)
   - Python zip bomb protection incomplete (VULNERABLE)

### Trade-offs

1. **Less Dynamic**
   - Can't evaluate dynamic expressions (e.g., Python `version = get_version()`)
   - Must extract static values only
   - **Acceptable**: Metadata should be static for reproducibility

2. **Incomplete Extraction in Edge Cases**
   - Some packages use dynamic version calculation
   - Template-based manifests (Jinja2 in conanfile.py)
   - **Acceptable**: Extract what's safe, document limitations

3. **Performance Overhead**
   - File size checks add syscalls
   - Iteration counting adds overhead
   - **Acceptable**: Safety > raw speed, overhead is minimal

## Alternatives Considered

### 1. Sandboxed Execution

**Approach**: Execute user code in isolated sandbox (Docker, seccomp, namespace isolation).

```rust
fn extract_from_setup_py(path: &Path) -> PackageData {
    let output = Command::new("docker")
        .args(&["run", "--rm", "--network=none", "python:3.11"])
        .arg("python")
        .arg(path)
        .output()?;
    
    parse_output(output.stdout)
}
```

**Rejected because**:

- Complex infrastructure requirement (Docker daemon)
- Slower (container startup overhead)
- Still vulnerable to malicious code (resource exhaustion inside container)
- Not portable (requires Docker/system-level sandboxing)
- Doesn't solve fundamental problem (untrusted code execution)

### 2. Static Analysis Only (No Parsing)

**Approach**: Use regex/heuristics instead of proper parsing.

```rust
fn extract_name(content: &str) -> Option<String> {
    let re = Regex::new(r#"name\s*=\s*"([^"]+)""#)?;
    re.captures(content)?.get(1).map(|m| m.as_str().to_string())
}
```

**Rejected because**:

- Too fragile for complex formats (JSON, TOML, YAML)
- Misses edge cases (multiline strings, escaping)
- Hard to maintain (regex soup)
- Less accurate than proper parsing

### 3. Trust User Input (No Limits)

**Approach**: Parse without validation or limits (like Python reference).

```rust
fn extract_package_data(path: &Path) -> PackageData {
    let content = fs::read_to_string(path).unwrap(); // ❌
    let manifest: Value = serde_json::from_str(&content).unwrap(); // ❌
    // No size checks, no iteration limits
}
```

**Rejected because**:

- Vulnerable to DoS (large files, deeply nested structures)
- Vulnerable to zip bombs
- Vulnerable to malicious input
- Not production-ready for security-sensitive contexts

### 4. Per-Ecosystem Security Policies

**Approach**: Different security levels per ecosystem.

```rust
match ecosystem {
    "npm" => parse_safely(),     // High security
    "gradle" => parse_unsafely(), // Low security (execute Groovy)
}
```

**Rejected because**:

- Inconsistent security posture
- Creates "secure" vs "insecure" parser classes
- Hard to document and reason about
- All parsers should be equally safe

## Python Reference Comparison

**Python Security Issues in Reference Implementation**:

| Issue | Risk | Our Solution |
|-------|------|--------------|
| `exec()` in setup.py parsing | Arbitrary code execution | AST parsing only |
| Ruby `instance_eval` | Code execution | Regex parsing |
| Shell execution (APKBUILD) | Command injection | Not implemented |
| Groovy engine for Gradle | Code execution | Custom lexer |
| No DoS limits | Memory exhaustion | Explicit limits |
| Incomplete zip bomb protection | DoS via decompression | Full protection |

**We significantly improve security compared to the Python reference.**

## Quality Gates

Before marking a parser complete:

- ✅ No code execution (verified by code review)
- ✅ DoS limits enforced (file size, iterations, recursion)
- ✅ Archive safety if applicable (size, compression ratio)
- ✅ Input validation with graceful degradation
- ✅ No `.unwrap()` in library code
- ✅ Security review documented (this ADR)

## Related ADRs

- [ADR 0001: Trait-Based Parser Architecture](0001-trait-based-parsers.md) - Parser structure enables security boundaries
- [ADR 0002: Extraction vs Detection Separation](0002-extraction-vs-detection.md) - Separating concerns simplifies security
- [ADR 0003: Golden Test Strategy](0003-golden-test-strategy.md) - Property-based tests for security (fuzzing, malicious input)

## References

- OWASP: [Deserialization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Deserialization_Cheat_Sheet.html)
- Wikipedia: [Zip Bomb](https://en.wikipedia.org/wiki/Zip_bomb)
- Rust security best practices: [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
