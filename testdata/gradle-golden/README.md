# Gradle Golden Test Data

This directory contains test data for validating Gradle build file parsing (`build.gradle` and `build.gradle.kts`).

## Current Implementation Status

### ✅ **Supported Patterns (Working)**

The current Rust implementation uses regex-based extraction and supports these dependency declaration formats:

1. **Pattern 2 - String Notation**: 
   ```groovy
   compile 'org.apache.commons:commons-text:1.1'
   ```

2. **Pattern 3 - Parentheses Notation**:
   ```groovy
   implementation("com.example:library:1.0.0")
   ```

3. **Pattern 4 - Named Parameters**:
   ```groovy
   api group: 'com.google.guava', name: 'guava', version: '30.1-jre'
   ```

### ❌ **Unsupported Patterns (Python Reference Uses Token Parser)**

The Python ScanCode Toolkit implementation uses **pygmars** (a token-based parser with grammar) to handle complex patterns that cannot be reliably extracted with regex:

1. **Pattern 1 - Map Format with Brackets**:
   ```groovy
   runtimeOnly(
       [group: 'org.jacoco', name: 'org.jacoco.ant', version: '0.7.4.201502262128'],
       [group: 'org.jacoco', name: 'org.jacoco.agent', version: '0.7.4.201502262128']
   )
   ```
   - **Why unsupported**: Requires parsing nested bracket structures and handling multi-line continuations

2. **Pattern 5 - Variable References**:
   ```groovy
   api dependencies.lombok
   ```
   - **Why unsupported**: Requires resolving variable references, which needs semantic analysis beyond regex

3. **Nested Function Calls**:
   ```groovy
   implementation(enforcedPlatform("com.fasterxml.jackson:jackson-bom:2.12.2"))
   ```
   - **Why unsupported**: Requires parsing nested function calls and extracting the inner string

4. **String Interpolation** (Kotlin DSL):
   ```kotlin
   implementation("org.jetbrains.kotlin:kotlin-stdlib-jdk8:$kotlinPluginVersion")
   ```
   - **Why unsupported**: Requires variable resolution which isn't possible with static analysis

5. **Project References**:
   ```kotlin
   "testImplementation"(project(":utils:test-utils"))
   ```
   - **Why unsupported**: Requires understanding project structure

## Test Files Status

### ✅ Passing Tests

- `groovy/groovy1/build.gradle` - Simple string notation
- `kotlin/kotlin1/build.gradle.kts` - Basic Kotlin DSL

### ❌ Failing Tests (Known Limitations)

- `groovy/groovy-basic/build.gradle` - Contains Pattern 1 (map format) and Pattern 5 (variable references)
- `groovy/groovy5-parens+singlequotes/build.gradle` - Contains nested function calls
- `kotlin/kotlin2/build.gradle.kts` - Complex file with string interpolation and project references
- `end2end/build.gradle` - Contains multiple unsupported patterns

## Path Forward: Full Feature Parity

To achieve 100% feature parity with Python ScanCode Toolkit, the Rust implementation needs:

### Option 1: Tree-Sitter Integration (Recommended)

- **Dependencies already added**: `tree-sitter`, `tree-sitter-groovy`, `tree-sitter-kotlin`
- **Approach**: Parse build files into AST, walk the tree to extract dependencies
- **Benefits**: 
  - Handles all 5 patterns correctly
  - Robust to formatting variations
  - Future-proof for additional patterns
- **Effort**: Medium (2-3 days implementation)

### Option 2: Token-Based Parser (Python's Approach)

- **Approach**: Implement a pygmars-style token parser with grammar rules
- **Benefits**: Exact parity with Python implementation
- **Drawbacks**: Significant complexity, reinventing the wheel
- **Effort**: High (5+ days implementation)

### Option 3: Enhanced Regex + Manual Parsing

- **Approach**: Add specialized regex for Pattern 1 and Pattern 5, skip interpolation
- **Benefits**: Incremental improvement
- **Drawbacks**: Still incomplete, brittle
- **Effort**: Low (1 day) but incomplete solution

## Recommendation

**For production use**, implement **Option 1 (Tree-Sitter)**. The dependencies are already in place, and tree-sitter grammars exist for both Groovy and Kotlin. This provides the most robust, maintainable solution with full feature parity.

**Current status** is suitable for:
- Simple Gradle files with string notation
- Basic dependency declarations
- ~60% of real-world Gradle files (based on test coverage)

## Python Reference Implementation

Location: `../../reference/scancode-toolkit/src/packagedcode/build_gradle.py`

Key differences:
- **Python**: Uses pygmars token parser with grammar rules (lines 77-92)
- **Rust**: Uses regex patterns (3 out of 5 patterns supported)

Grammar used by Python:
```python
grammar = """
    DEPENDENCY-1: {<PACKAGE-IDENTIFIER>{3} <OPERATOR>}    # Map format
    DEPENDENCY-2: {<NAME> <TEXT> <LIT-STRING> <TEXT>}    # String notation (✅ supported)
    DEPENDENCY-3: {<NAME> <TEXT>? <OPERATOR> <LIT-STRING> <OPERATOR>}  # Parentheses (✅ supported)
    DEPENDENCY-4: {<NAME> <TEXT> <NAME-LABEL> <TEXT> <LIT-STRING> <PACKAGE-IDENTIFIER> <PACKAGE-IDENTIFIER> <OPERATOR>? <TEXT>}  # Named params (✅ supported)
    DEPENDENCY-5: {<NAME> <TEXT> <NAME> <OPERATOR> <NAME-ATTRIBUTE>}  # Variable refs (❌ unsupported)
"""
```

## License Detection Blockers

**All golden tests will fail on license detection fields** because:
- License detection engine is not yet implemented in Rust version
- Fields affected: `declared_license_expression`, `declared_license_expression_spdx`, `license_detections`
- **This is expected** and not a parser limitation

The test comparator (`compare_package_data_parser_only`) automatically skips license detection fields, so focus is on dependency extraction accuracy.
