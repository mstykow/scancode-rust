# Phase 2: Nested Sibling-Merge Design

> **Status**: Design Phase
> **Date**: Feb 10, 2026
> **Branch**: feat/package-assembly-phase2

## Problem Statement

Phase 1 implemented **sibling-merge** for files in the **same directory** (e.g., `package.json` + `package-lock.json`).

Phase 2 needs **nested sibling-merge** for files in **different directories** within the same package (e.g., `pom.xml` + `META-INF/MANIFEST.MF`).

## Maven Use Case

**Typical Maven JAR structure:**

```text
my-library-1.0.0.jar/
├── pom.xml                                    # Primary manifest
├── META-INF/
│   ├── MANIFEST.MF                            # Secondary manifest (nested)
│   └── maven/com.example/my-library/
│       ├── pom.xml                            # Duplicate pom.xml (nested)
│       └── pom.properties                     # Properties file
└── com/example/mylibrary/
    └── MyClass.class
```

**Assembly requirement**: Merge `pom.xml` (root) + `META-INF/MANIFEST.MF` (nested) into a single Package.

## Python Reference Algorithm

From background research (task bg_df91c23a):

### Discovery Algorithm

1. **When MANIFEST.MF is found**:
   - Navigate up 1 level to `META-INF` directory
   - Walk entire `META-INF` subtree to find siblings

2. **When nested pom.xml is found**:
   - Navigate up 4 levels to `META-INF` directory
   - Path: `META-INF/maven/{namespace}/{name}/pom.xml`
   - Walk entire `META-INF` subtree to find siblings

3. **Sibling Discovery**:
   - Walk from common parent (META-INF)
   - Find all matching files using glob patterns:
     - `*/META-INF/MANIFEST.MF`
     - `*/META-INF/maven/**/pom.xml`

### Merge Strategy

1. **Process in order**: pom.xml FIRST, then MANIFEST.MF
2. **First file creates Package** (pom.xml)
3. **Subsequent files update Package** (MANIFEST.MF)
4. **Update rules**:
   - Empty fields → Fill from new data
   - Non-empty fields → Keep existing (don't replace)
   - List fields → Merge, avoiding duplicates
   - Always append datasource_ids and datafile_paths

## Design Decision: Two Approaches

### Approach A: Extend Current Grouping (REJECTED)

**Idea**: Group files by "package root" instead of parent directory.

**Problems**:

- How to determine "package root" without parsing?
- Requires heuristics (e.g., "if MANIFEST.MF exists, go up to find pom.xml")
- Breaks the clean directory-based grouping
- Complex edge cases (multiple packages in same tree)

### Approach B: Pattern-Based Discovery (SELECTED)

**Idea**: Support glob patterns in `sibling_file_patterns` that can match nested paths.

**Benefits**:

- Declarative configuration
- Reuses existing pattern matching
- Extensible to other nested patterns
- Clear semantics

**Implementation**:

- Extend `matches_pattern()` to support path patterns (not just filename)
- When pattern contains `/`, match against full path (not just filename)
- Walk directory tree to find all matching files

## Proposed Implementation

### 1. Extend Pattern Matching

**Current** (Phase 1):

```rust
pub(crate) fn matches_pattern(file_name: &str, pattern: &str) -> bool {
    // Only matches filename
    if let Some(suffix) = pattern.strip_prefix('*') {
        file_name.ends_with(suffix)
    } else {
        file_name == pattern || file_name.eq_ignore_ascii_case(pattern)
    }
}
```

**Proposed** (Phase 2):

```rust
pub(crate) fn matches_pattern(file_path: &str, pattern: &str) -> bool {
    // If pattern contains '/', match against full path
    if pattern.contains('/') {
        return matches_path_pattern(file_path, pattern);
    }
    
    // Otherwise, match against filename only (Phase 1 behavior)
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    if let Some(suffix) = pattern.strip_prefix('*') {
        file_name.ends_with(suffix)
    } else {
        file_name == pattern || file_name.eq_ignore_ascii_case(pattern)
    }
}

fn matches_path_pattern(file_path: &str, pattern: &str) -> bool {
    // Support glob patterns in paths:
    // - "META-INF/MANIFEST.MF" → exact path match
    // - "**/META-INF/MANIFEST.MF" → any depth
    // - "META-INF/**/pom.xml" → nested pom.xml
    
    // Use glob crate for proper glob matching
    use glob::Pattern;
    Pattern::new(pattern)
        .ok()
        .and_then(|p| p.matches(file_path).then_some(true))
        .unwrap_or(false)
}
```

### 2. Update Assembler Configuration

**Maven assembler config**:

```rust
AssemblerConfig {
    datasource_ids: &[
        "maven_pom",
        "java_jar_manifest",
        "java_osgi_manifest",
    ],
    sibling_file_patterns: &[
        "pom.xml",                      // Root pom.xml (Phase 1 pattern)
        "**/META-INF/MANIFEST.MF",      // Nested MANIFEST.MF (Phase 2 pattern)
    ],
}
```

### 3. Extend Directory Grouping

**Current** (Phase 1): Groups by parent directory

```rust
fn group_files_by_directory(files: &[FileInfo]) -> HashMap<PathBuf, Vec<usize>>
```

**Proposed** (Phase 2): Groups by "package root" for nested patterns

```rust
fn group_files_for_assembly(
    files: &[FileInfo],
    config: &AssemblerConfig,
) -> HashMap<PathBuf, Vec<usize>> {
    // For patterns without '/', use parent directory (Phase 1)
    // For patterns with '/', find common ancestor
    
    let has_nested_patterns = config.sibling_file_patterns
        .iter()
        .any(|p| p.contains('/'));
    
    if !has_nested_patterns {
        // Phase 1: group by parent directory
        return group_files_by_directory(files);
    }
    
    // Phase 2: group by package root
    group_files_by_package_root(files, config)
}
```

### 4. Package Root Detection

**Algorithm**:

1. For each file with matching datasource_id
2. Check if it matches any nested pattern
3. If yes, find the "package root" by walking up the tree
4. Group all files under the same package root

**Example**:

```text
Input files:
- /path/to/jar/pom.xml (datasource_id: maven_pom)
- /path/to/jar/META-INF/MANIFEST.MF (datasource_id: java_jar_manifest)

Pattern: "**/META-INF/MANIFEST.MF"
Match: /path/to/jar/META-INF/MANIFEST.MF
Package root: /path/to/jar (parent of META-INF)

Result: Both files grouped under /path/to/jar
```

## Alternative: Simpler Approach (RECOMMENDED)

After analyzing the Python implementation more carefully, I realize there's a **simpler approach** that doesn't require complex pattern matching:

### Key Insight from Python

Python doesn't use glob patterns for discovery. Instead:

1. It processes files **one by one** during scanning
2. When it finds a MANIFEST.MF or nested pom.xml, it **walks up** to find META-INF
3. Then it **walks down** from META-INF to find all siblings
4. This is a **local search** from each file, not a global grouping

### Simpler Rust Implementation

**Don't change the grouping logic.** Instead:

1. **Keep Phase 1 directory grouping** as-is
2. **Add a post-processing step** for nested patterns:
   - After directory-based assembly
   - For each unassembled file matching nested patterns
   - Walk up to find package root
   - Walk down to find siblings
   - Assemble them

**Pseudocode**:

```rust
pub fn assemble(files: &mut [FileInfo]) -> AssemblyResult {
    // Phase 1: Directory-based assembly (existing)
    let mut result = assemble_by_directory(files);
    
    // Phase 2: Nested pattern assembly (new)
    result.extend(assemble_nested_patterns(files));
    
    result
}

fn assemble_nested_patterns(files: &mut [FileInfo]) -> AssemblyResult {
    // Find unassembled files matching nested patterns
    let nested_configs = ASSEMBLERS.iter()
        .filter(|c| has_nested_patterns(c));
    
    for config in nested_configs {
        for (idx, file) in files.iter().enumerate() {
            if file.for_packages.is_empty() {
                // Not yet assembled
                if matches_nested_pattern(file, config) {
                    // Walk up to find package root
                    // Walk down to find siblings
                    // Assemble them
                }
            }
        }
    }
}
```

## Decision: Hybrid Approach

**Best of both worlds**:

1. **For simple nested patterns** (Maven): Use the simpler post-processing approach
2. **For complex nested patterns** (future): Extend to full glob support

**Phase 2 implementation**:

- Add `assemble_nested_patterns()` function
- Detect Maven files (pom.xml + MANIFEST.MF)
- Walk up to find common parent
- Walk down to find siblings
- Merge using existing `sibling_merge::assemble_siblings()`

## Implementation Plan

### Step 1: Add Nested Pattern Detection

```rust
fn has_nested_patterns(config: &AssemblerConfig) -> bool {
    config.sibling_file_patterns.iter().any(|p| p.contains("**"))
}
```

### Step 2: Add Package Root Discovery

```rust
fn find_package_root(file_path: &Path, pattern: &str) -> Option<PathBuf> {
    // For "**/META-INF/MANIFEST.MF", walk up to parent of META-INF
    // For "pom.xml", return parent directory
}
```

### Step 3: Add Sibling Discovery

```rust
fn find_nested_siblings(
    root: &Path,
    files: &[FileInfo],
    config: &AssemblerConfig,
) -> Vec<usize> {
    // Walk all files under root
    // Match against config.sibling_file_patterns
    // Return indices of matching files
}
```

### Step 4: Add Nested Assembly

```rust
fn assemble_nested_patterns(files: &mut [FileInfo]) -> AssemblyResult {
    // For each nested config
    // For each unassembled file
    // Find package root
    // Find siblings
    // Assemble using existing sibling_merge logic
}
```

### Step 5: Integrate into Main Assembly

```rust
pub fn assemble(files: &mut [FileInfo]) -> AssemblyResult {
    let mut result = assemble_by_directory(files);
    result.extend(assemble_nested_patterns(files));
    result
}
```

## Testing Strategy

### Unit Tests

1. Test `has_nested_patterns()` with various patterns
2. Test `find_package_root()` with Maven paths
3. Test `find_nested_siblings()` with mock file tree
4. Test `assemble_nested_patterns()` with Maven files

### Golden Tests

1. Create `testdata/assembly-golden/maven-basic/`:
   - `pom.xml` (root)
   - `META-INF/MANIFEST.MF` (nested)
   - `expected.json` (merged output)
2. Run assembly and compare with expected
3. Verify both datasource_ids are recorded
4. Verify merge order (pom.xml data takes precedence)

## Success Criteria

- [ ] Maven assembler config added to `assemblers.rs`
- [ ] Nested pattern detection implemented
- [ ] Package root discovery implemented
- [ ] Sibling discovery for nested patterns implemented
- [ ] Assembly integration complete
- [ ] Unit tests pass
- [ ] Golden tests pass for Maven
- [ ] Output matches Python reference

## Next Steps

1. Implement nested pattern detection functions
2. Add Maven assembler configuration
3. Create golden test data
4. Implement assembly logic
5. Test and validate
