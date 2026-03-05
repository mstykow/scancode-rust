# License Expression Parsing and Handling: Python vs Rust Comparison

**Audit Date**: 2025-03-05  
**Python Reference**: `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/`  
**Rust Implementation**: `src/license_detection/expression/`

## Executive Summary

The Rust implementation provides a focused, type-safe expression parsing library that covers the core functionality of the Python `license-expression` package. The Python library is significantly more feature-rich, providing advanced boolean algebra operations, symbol management, and validation. The Rust implementation focuses on the practical needs of ScanCode license detection.

**Key Finding**: The Rust implementation handles the core expression parsing and SPDX conversion correctly, but lacks some advanced features present in Python (symbol validation, advanced simplification, symbol aliases, deduplication of complex expressions).

---

## 1. Expression Parsing

### Python Implementation

**Location**: `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/__init__.py:476-559`

**Architecture**:
- Uses `boolean.BooleanAlgebra` base class (from `boolean.py` package)
- Tokenization via two strategies:
  - **Simple tokenizer** (line 659-705): Regex-based, assumes license keys have no spaces
  - **Advanced tokenizer** (line 615-650): Aho-Corasick automaton for recognizing license keys with spaces and keywords in their names
- Creates `LicenseSymbol` and `LicenseWithExceptionSymbol` objects
- Supports license key aliases and case-insensitive matching

**Key Classes**:
```python
class LicenseSymbol(BaseSymbol):
    # Represents a single license key
    # Has: key, aliases, is_deprecated, is_exception
    
class LicenseWithExceptionSymbol(BaseSymbol):
    # Represents "license WITH exception" as a single atomic symbol
    # Contains: license_symbol, exception_symbol
    
class Licensing(boolean.BooleanAlgebra):
    # Main entry point for expression parsing
```

**Tokenization Process** (line 561-613):
1. Tokenize expression string into `(token_obj, token_string, position)` tuples
2. Recognize known license symbols from the Licensing object's symbol registry
3. Build symbols from unknown tokens
4. Replace "XXX WITH YYY" sequences with `LicenseWithExceptionSymbol`
5. Yield tokens for the boolean parser

**Error Handling**:
- `ExpressionError`: General errors
- `ExpressionParseError`: Parse syntax errors (inherits from `ParseError`)
- Custom error codes: `PARSE_INVALID_EXCEPTION`, `PARSE_INVALID_SYMBOL_AS_EXCEPTION`, etc.

### Rust Implementation

**Location**: `src/license_detection/expression/parse.rs:41-212`

**Architecture**:
- Simple recursive descent parser
- Hand-written tokenizer (no external dependencies)
- AST represented as `LicenseExpression` enum
- No concept of license key aliases or external symbol registry

**Key Types**:
```rust
pub enum LicenseExpression {
    License(String),
    LicenseRef(String),
    And { left: Box<LicenseExpression>, right: Box<LicenseExpression> },
    Or { left: Box<LicenseExpression>, right: Box<LicenseExpression> },
    With { left: Box<LicenseExpression>, right: Box<LicenseExpression> },
}
```

**Tokenization Process** (line 52-100):
1. Scan character-by-character
2. Recognize `(`, `)`, operators (AND, OR, WITH - case insensitive)
3. Treat all other alphanumeric sequences as license keys (lowercased)
4. No recognition of license keys with spaces (e.g., "GPL 2.0")

**Operator Precedence** (line 131-179):
```
OR (lowest) → AND (medium) → WITH (highest)
```

**Error Handling**:
- `ParseError` enum with variants: `EmptyExpression`, `UnexpectedToken`, `MismatchedParentheses`, `InvalidLicenseKey`, `InvalidOperator`, `ParseError(String)`

### Differences and Behavioral Variations

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| **License keys with spaces** | Supported via Advanced tokenizer | Not supported | LOW - ScanCode keys are typically dash-separated |
| **License key aliases** | Supported (symbol can have multiple names) | Not supported | LOW - Rust works with canonical keys only |
| **Case sensitivity** | Case-insensitive matching with known symbols | Always lowercases keys | LOW - Both normalize to lowercase |
| **Tokenization strategy** | Aho-Corasick automaton for efficiency | Simple char-by-char scan | MEDIUM - Python more efficient for long expressions |
| **Symbol validation** | Validates against known symbol registry | No validation during parsing | MEDIUM - Validation done separately |
| **WITH expression handling** | Creates atomic `LicenseWithExceptionSymbol` | Creates `With` node in AST | LOW - Semantically equivalent |
| **Position tracking** | Tracks exact position in original string | No position tracking | LOW - Only affects error messages |

**Potential Behavioral Difference**: Python's advanced tokenizer can recognize license keys that contain spaces or keywords (e.g., "GPL 2.0" instead of "GPL-2.0"). The Rust implementation cannot parse such keys. However, ScanCode license keys are consistently dash-separated, so this is unlikely to cause issues in practice.

---

## 2. Expression Simplification

### Python Implementation

**Location**: `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/__init__.py:707-761`

**Methods**:
1. **`simplify()`** (from `boolean.BooleanAlgebra`):
   - Full boolean algebra simplification
   - Removes redundant terms
   - Flattens nested expressions of the same operator
   
2. **`dedup()`** (line 707-761):
   - License-specific deduplication
   - Does not sort licenses (preserves order)
   - Treats OR choices as non-simplifiable (preserves all options)
   - Recursively processes nested AND/OR expressions

**Example**:
```python
>>> l = Licensing()
>>> expr = l.parse("mit OR mit OR apache-2.0")
>>> str(expr.simplify())
'mit OR apache-2.0'
>>> str(l.dedup("mit AND mit AND apache-2.0"))
'mit AND apache-2.0'
```

### Rust Implementation

**Location**: `src/license_detection/expression/simplify.rs:14-148`

**Function**: `simplify_expression(expr: &LicenseExpression) -> LicenseExpression`

**Strategy**:
1. Recursively traverse the expression tree
2. For AND/OR nodes: collect all arguments, deduplicate by string representation
3. For WITH nodes: no deduplication (treated as atomic)
4. Preserve left-to-right order

**Implementation Details** (line 37-123):
- Uses `HashSet` to track seen license keys
- `collect_unique_and()`: flattens nested AND expressions and deduplicates
- `collect_unique_or()`: flattens nested OR expressions and deduplicates
- WITH expressions are deduplicated as complete units (license + exception together)

**Key Difference from Python**:
- Python's `dedup()` preserves OR choices (doesn't simplify "MIT OR Apache" to just "MIT")
- Rust's `simplify_expression()` only removes exact duplicates, preserving choices

### Differences and Behavioral Variations

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| **Boolean simplification** | Full boolean algebra simplification | Duplicate removal only | MEDIUM - Python can simplify more complex expressions |
| **OR choice preservation** | Preserves all OR options | Preserves all OR options | NONE - Both behave correctly |
| **WITH deduplication** | Atomic unit (license WITH exception) | Atomic unit (license WITH exception) | NONE - Both behave correctly |
| **Order preservation** | Preserves order | Preserves order | NONE - Both behave correctly |
| **Complex expression simplification** | Can simplify `(A OR B) AND (A OR B)` to `A OR B` | Only removes exact duplicates | LOW - Uncommon in practice |

**Example**:
```python
# Python
expr = "mit OR gpl-2.0 OR mit"
# simplify() → "mit OR gpl-2.0"

expr = "(mit OR apache) AND (mit OR apache)"
# simplify() → "mit OR apache" (boolean simplification)
```

```rust
// Rust
expr = "mit OR gpl-2.0 OR mit"
// simplify_expression() → "mit OR gpl-2.0"

expr = "(mit OR apache) AND (mit OR apache)"
// simplify_expression() → "mit OR apache" (duplicate removal)
```

**Potential Behavioral Difference**: Python's boolean simplification can reduce more complex boolean expressions. For example, `(MIT OR Apache) AND (MIT OR Apache)` simplifies to `MIT OR Apache`. Rust only removes exact duplicate subexpressions. However, such expressions are rare in license detection, and Rust's behavior is correct for practical cases.

---

## 3. SPDX Conversion

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/cache.py:507-524`

**Function**: `build_spdx_license_expression(license_expression, licensing=None)`

**Process**:
1. Parse the expression using Licensing
2. Render using template: `'{symbol.wrapped.spdx_license_key}'`
3. Maps each license key to its SPDX equivalent from the License database

**License Key Mapping** (`reference/scancode-toolkit/src/licensedcode/models.py:231-246`):
- `spdx_license_key`: Primary SPDX identifier
- `other_spdx_license_keys`: Additional SPDX aliases (e.g., deprecated identifiers)
- For licenses without SPDX listing: use `LicenseRef-scancode-<key>`

**Example**:
```python
>>> build_spdx_license_expression("mit OR gpl-2.0-plus")
'MIT OR GPL-2.0-or-later'

>>> build_spdx_license_expression("gpl-2.0 WITH classpath-exception-2.0")
'GPL-2.0-only WITH Classpath-exception-2.0'

>>> build_spdx_license_expression("custom-license")
'LicenseRef-scancode-custom-license'
```

### Rust Implementation

**Location**: `src/license_detection/spdx_mapping/mod.rs:38-222`

**Structure**: `SpdxMapping` with bidirectional mapping

**Process** (line 156-160):
1. Parse expression string to `LicenseExpression`
2. Recursively convert each license key to SPDX key
3. Serialize back to string

**Key Conversion Logic** (line 163-196):
```rust
match expr {
    LicenseExpression::License(key) => {
        if let Some(spdx_key) = self.scancode_to_spdx(key) {
            if spdx_key.starts_with("LicenseRef-") {
                LicenseExpression::LicenseRef(spdx_key)
            } else {
                LicenseExpression::License(spdx_key)
            }
        } else {
            LicenseExpression::LicenseRef(format!("LicenseRef-scancode-{}", key))
        }
    }
    // ... recursive handling of And, Or, With
}
```

**Building the Mapping** (line 70-97):
- Extract `spdx_license_key` from each License
- If present: use it directly
- If absent: generate `LicenseRef-scancode-<key>`
- Build bidirectional mapping for reverse lookups

### Differences and Behavioral Variations

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| **Template rendering** | Uses template system | Direct AST transformation | LOW - Same result |
| **LicenseRef handling** | Preserves existing LicenseRef | Generates LicenseRef for unknown keys | LOW - Consistent behavior |
| **Bidirectional mapping** | No reverse mapping | Provides `spdx_to_scancode()` | LOW - Rust has extra feature |
| **Validation** | Raises `InvalidLicenseKeyError` on unknown keys | No validation during conversion | MEDIUM - Errors caught later |
| **other_spdx_license_keys** | Uses as aliases | Ignored | LOW - Only affects display, not parsing |

**Potential Behavioral Difference**: Python validates that license keys exist in the database during SPDX conversion and raises `InvalidLicenseKeyError` if a key is unknown. Rust generates a `LicenseRef-scancode-<key>` for unknown keys without validation. This is a difference in error handling strategy, not functional behavior.

---

## 4. Expression Combination

### Python Implementation

**Location**: `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/__init__.py:1746-1802`

**Function**: `combine_expressions(expressions, relation="AND", unique=True, licensing=Licensing())`

**Process**:
1. Parse each expression string to `LicenseExpression`
2. If `unique=True`: remove duplicates (preserving order) using dict deduplication
3. If single expression: return it directly
4. Otherwise: combine using `licensing.AND(*exprs)` or `licensing.OR(*exprs)`

**Example**:
```python
>>> combine_expressions(["mit", "gpl-2.0", "mit"])
'mit AND gpl-2.0'  # unique=True removes duplicate 'mit'

>>> combine_expressions(["mit", "gpl-2.0"], relation="OR")
'mit OR gpl-2.0'

>>> combine_expressions(["mit OR apache", "gpl-2.0"])
'(mit OR apache) AND gpl-2.0'  # wraps OR in parens
```

**Use in ScanCode** (`reference/scancode-toolkit/src/licensedcode/detection.py:2000-2004`):
```python
detected_license_expression = combine_expressions(
    expressions=license_expressions,
    relation='AND',
    unique=True,
    licensing=get_cache().licensing
)
```

### Rust Implementation

**Location**: `src/license_detection/expression/simplify.rs:435-473`

**Function**: `combine_expressions(expressions: &[&str], relation: CombineRelation, unique: bool) -> Result<String, ParseError>`

**Process**:
1. Parse each expression string
2. If `unique=True`: apply `simplify_expression()` to remove duplicates
3. Combine using `LicenseExpression::and()` or `LicenseExpression::or()` helpers
4. Serialize to string

**Helper Methods** (`src/license_detection/expression/mod.rs:125-160`):
```rust
impl LicenseExpression {
    pub fn and(expressions: Vec<LicenseExpression>) -> Option<LicenseExpression> {
        // Build left-nested binary tree
        // (a AND b) AND c
    }
    
    pub fn or(expressions: Vec<LicenseExpression>) -> Option<LicenseExpression> {
        // Build left-nested binary tree
        // (a OR b) OR c
    }
}
```

### Differences and Behavioral Variations

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| **Expression structure** | Flat args `AND(a, b, c)` | Nested binary `(a AND b) AND c` | MEDIUM - Affects rendering |
| **Duplicate removal** | Dict-based deduplication | HashSet-based deduplication | NONE - Same result |
| **Single expression** | Returns as-is | Returns as-is (with optional simplification) | NONE - Same behavior |
| **Parse error handling** | Raises `ExpressionError` | Returns `Err(ParseError)` | LOW - Different error types |

**Critical Behavioral Difference**: Python's boolean algebra library stores expressions with flat argument lists: `AND(a, b, c)`. When rendered, this produces `a AND b AND c` without internal parentheses.

Rust's implementation uses a binary tree structure: `(a AND b) AND c`. When rendered, the `expression_to_string()` function adds parentheses to preserve the structural grouping: `(a AND b) AND c`.

**Example**:
```python
# Python
combine_expressions(["a", "b", "c"], relation="AND", unique=False)
# Result: "a AND b AND c"  (flat)
```

```rust
// Rust
combine_expressions(&["a", "b", "c"], CombineRelation::And, false)
// Result: "(a AND b) AND c"  (nested with parens)
```

**Impact**: The extra parentheses in Rust's output are semantically correct (the expression is equivalent), but they make the output less readable and differ from Python's output. This is flagged as a potential golden test failure.

**Recommendation**: Consider modifying `expression_to_string()` to detect when left child is the same operator and render without intermediate parentheses, matching Python's flat rendering.

---

## 5. Special Cases

### 5.1 License References (LicenseRef-scancode-*)

**Python**: 
- Stored in License objects: `spdx_license_key: LicenseRef-scancode-<key>`
- Used for licenses not in the official SPDX list
- Rendered via template: `{symbol.wrapped.spdx_license_key}`

**Rust**:
- Generated on-the-fly for licenses without SPDX mapping
- Stored as `LicenseExpression::LicenseRef(String)`
- Distinguishes from regular `License(String)` variants

**Behavior**: Both implementations handle `LicenseRef` correctly. No behavioral differences.

### 5.2 Unknown Licenses

**Python** (`reference/scancode-toolkit/src/licensedcode/models.py:2773-2798`):
```python
UNKNOWN_LICENSE_KEY = 'unknown'

class UnknownRule(SynthethicRule):
    def __attrs_post_init__(self):
        self.license_expression = UNKNOWN_LICENSE_KEY
        self.license_expression_object = self.licensing.parse(UNKNOWN_LICENSE_KEY)
```

- Special license key `'unknown'` for unrecognized licenses
- Parsed as a regular license symbol
- Can be combined with other expressions

**Rust**:
- No special handling for unknown licenses in expression parser
- Unknown keys are parsed as regular `License(String)` nodes
- SPDX conversion maps unknown keys to `LicenseRef-scancode-<key>`

**Behavior**: Both handle unknown licenses as regular symbols. No behavioral differences in parsing.

### 5.3 License Intros/Notices

**Python** (`reference/scancode-toolkit/src/licensedcode/models.py:2748-2771`):
```python
class LicenseDetectedRule(SynthethicRule):
    # Rules for SPDX license identifier tags like "SPDX-License-Identifier: MIT"
    is_license_tag = True
```

- Special handling for license identifier tags
- Created dynamically during detection
- Treated as synthetic rules

**Rust**:
- No special handling in expression parser
- License tags handled at detection level, not expression level

**Behavior**: This is a detection-level feature, not an expression parsing feature. No impact on expression handling.

### 5.4 Exceptions (WITH expressions)

**Python** (`reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/__init__.py:1376-1496`):
```python
class LicenseWithExceptionSymbol(BaseSymbol):
    def __init__(self, license_symbol, exception_symbol, strict=False):
        # Validates that license_symbol.is_exception is False
        # Validates that exception_symbol.is_exception is True (if strict=True)
```

- `WITH` expressions are treated as atomic symbols
- Can validate that left side is a license and right side is an exception
- Has `is_exception` attribute on symbols

**Rust**:
```rust
LicenseExpression::With {
    left: Box<LicenseExpression>,
    right: Box<LicenseExpression>,
}
```

- `WITH` expressions are AST nodes, not atomic symbols
- No validation of exception vs license during parsing
- No `is_exception` attribute tracking

**Behavioral Difference**: Python can validate that WITH expressions use proper exception symbols on the right side (if `strict=True`). Rust does not perform this validation. This is unlikely to cause issues in practice since the license database ensures correct metadata, but it's a potential data quality check that Rust doesn't replicate.

### 5.5 Deprecated Licenses

**Python**:
- `License.is_deprecated` attribute
- `License.replaced_by` list of replacement keys
- Used for license key migration

**Rust** (`src/license_detection/models/license.rs`):
```rust
pub struct License {
    pub is_deprecated: bool,
    pub replaced_by: Vec<String>,
    // ...
}
```

- Stores the same metadata
- Not used in expression parsing
- Handled at detection/license resolution level

**Behavior**: No difference in expression handling. Both store the metadata but handle deprecation at the detection level.

---

## 6. String Representation (expression_to_string)

### Python Implementation

**Location**: `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/__init__.py:1498-1547`

**Rendering Strategy** (`RenderableFunction.render()`):
1. Get flat argument list from expression
2. For each argument:
   - If literal (symbol): render directly
   - If non-literal (sub-expression): wrap in parentheses
3. Join with operator (" AND " or " OR ")

**Example**:
```python
AND(a, b, c) → "a AND b AND c"  # flat args, no parens

AND(OR(a, b), c) → "(a OR b) AND c"  # OR is non-literal, needs parens
```

**WITH Rendering** (`LicenseWithExceptionSymbol.__str__()`):
```python
def __str__(self):
    return f"{self.license_symbol.key} WITH {self.exception_symbol.key}"
```

### Rust Implementation

**Location**: `src/license_detection/expression/simplify.rs:358-409`

**Rendering Strategy**:
1. Traverse AST recursively
2. Track parent operator precedence
3. Add parentheses when:
   - Child is same operator as parent (to preserve structural grouping)
   - Parent has lower precedence than child

**Example**:
```rust
And { left: And { left: a, right: b }, right: c }
// Renders as: "(a AND b) AND c"  // preserves binary tree structure

Or { left: And { left: a, right: b }, right: c }
// Renders as: "(a AND b) OR c"  // AND has higher precedence, needs parens
```

**WITH Rendering** (line 403-407):
```rust
LicenseExpression::With { left, right } => {
    let left_str = expression_to_string_internal(left, Some(Precedence::With));
    let right_str = expression_to_string_internal(right, Some(Precedence::With));
    format!("{} WITH {}", left_str, right_str)
}
```

### Critical Difference in Parentheses

**Python**:
- Stores expressions with flat args: `AND(a, b, c)`
- Renders flat: `a AND b AND c`
- Only adds parens for sub-expressions of different operators

**Rust**:
- Stores expressions as binary trees: `(a AND b) AND c`
- Renders with structural parens: `(a AND b) AND c`
- Adds parens to preserve binary tree structure

**Impact**: This is a **golden test failure risk**. When combining multiple expressions, Rust will produce output with more parentheses than Python.

**Example**:
```python
# Python
combine_expressions(["mit", "apache-2.0", "gpl-2.0"], relation="AND")
# Output: "mit AND apache-2.0 AND gpl-2.0"
```

```rust
// Rust
combine_expressions(&["mit", "apache-2.0", "gpl-2.0"], CombineRelation::And, true)
// Output: "(mit AND apache-2.0) AND gpl-2.0"
```

**Recommendation**: Modify Rust's `expression_to_string()` to detect when left child is the same operator and render flat, matching Python's behavior:

```rust
LicenseExpression::And { left, right } => {
    let left_str = expression_to_string_internal(left, Some(Precedence::And));
    let right_str = expression_to_string_internal(right, Some(Precedence::And));
    
    // Don't add parens if left is also AND (flatten)
    let left_str = if matches!(left.as_ref(), LicenseExpression::And { .. }) {
        format!("({})", left_str)  // <-- REMOVE THIS
    } else {
        left_str
    };
    
    format!("{} AND {}", left_str, right_str)
}
```

---

## 7. Containment Checks

### Python Implementation

**Location**: `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/__init__.py:314-334`

**Function**: `Licensing.contains(expression1, expression2, **kwargs)`

**Process**:
1. Parse and simplify both expressions
2. Use boolean algebra containment: `expression2 in expression1`
3. Handles deduplication automatically

**Example**:
```python
>>> l.contains("mit AND apache", "mit")
True

>>> l.contains("mit OR apache", "mit AND apache")
False
```

### Rust Implementation

**Location**: `src/license_detection/expression/simplify.rs:246-308`

**Function**: `licensing_contains(container: &str, contained: &str) -> bool`

**Process**:
1. Parse and simplify both expressions
2. Match on expression types:
   - `And/Or` contains `And/Or`: check if all contained args are in container args
   - `And/Or` contains `License/LicenseRef`: decompose WITH expressions
   - `With` contains `License`: check if license is in decomposed parts
   - Single licenses: direct equality

**WITH Decomposition** (line 188-197):
```rust
fn decompose_expr(expr: &LicenseExpression) -> Vec<LicenseExpression> {
    match expr {
        LicenseExpression::With { left, right } => {
            let mut parts = decompose_expr(left);
            parts.extend(decompose_expr(right));
            parts
        }
        _ => vec![expr.clone()],
    }
}
```

**Behavior**: `"gpl-2.0 WITH classpath-exception"` contains `"gpl-2.0"` ✓

### Differences and Behavioral Variations

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| **Implementation** | Delegates to boolean algebra library | Custom logic | LOW - Both correct |
| **WITH decomposition** | Implicit in boolean algebra | Explicit decomposition | NONE - Same behavior |
| **Edge cases** | Battle-tested in boolean.py | May have edge cases | LOW - Tests cover common cases |

**Behavior**: Both implementations correctly handle containment checks. Rust's explicit logic is more transparent but may miss edge cases that Python's battle-tested boolean algebra library handles correctly.

---

## 8. Validation

### Python Implementation

**Location**: `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/__init__.py:763-807`

**Function**: `Licensing.validate(expression, strict=True, **kwargs)`

**Returns**: `ExpressionInfo` object with:
- `original_expression`: The input string
- `normalized_expression`: The normalized/parsed expression
- `errors`: List of error messages
- `invalid_symbols`: List of invalid license keys

**Validation Checks**:
1. Parse expression (syntax validation)
2. Validate license keys exist in known symbols
3. If `strict=True`: validate WITH expressions use proper exception symbols

**Example**:
```python
>>> info = l.validate("MIT AND UnknownLicense")
>>> info.errors
['Unknown license key(s): UnknownLicense']
>>> info.invalid_symbols
['UnknownLicense']
```

### Rust Implementation

**Location**: `src/license_detection/expression/simplify.rs:319-336`

**Function**: `validate_expression(expr: &LicenseExpression, known_keys: &HashSet<String>) -> ValidationResult`

**Returns**: `ValidationResult` enum:
- `Valid`: Expression is valid
- `UnknownKeys { unknown: Vec<String> }`: Contains unknown license keys
- `Invalid { errors: Vec<String> }`: Has other validation errors

**Validation Checks**:
1. Extract all license keys from expression
2. Check each against known keys set

**Example**:
```rust
let mut known = HashSet::new();
known.insert("mit".to_string());

let expr = parse_expression("MIT AND UnknownKey")?;
let result = validate_expression(&expr, &known);
// Result: UnknownKeys { unknown: ["unknownkey"] }
```

### Differences and Behavioral Variations

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| **Return type** | Structured `ExpressionInfo` object | Simple enum | LOW - Both provide necessary info |
| **Strict mode** | Validates exception symbols in WITH | No strict mode | LOW - Exception metadata handled separately |
| **Normalized expression** | Returns normalized form | Doesn't normalize | LOW - Can call simplify separately |
| **Error aggregation** | Collects all errors | Returns on first error type | LOW - Both report unknown keys |

**Behavior**: Rust's validation is simpler but covers the essential case (unknown keys). Python's `strict=True` mode provides additional validation for exception symbols, but this is rarely needed in practice since the license database ensures correct metadata.

---

## 9. Key Architectural Differences

### Python: Boolean Algebra Approach

**Advantages**:
- Full boolean algebra operations (equality, equivalence, simplification)
- Battle-tested implementation (boolean.py library)
- Advanced features: symbol aliases, exception validation, Aho-Corasick tokenization
- Supports license keys with spaces and special characters

**Disadvantages**:
- Heavyweight (depends on boolean.py library)
- Complex codebase (1800+ lines)
- Overkill for basic expression parsing

### Rust: Minimalist AST Approach

**Advantages**:
- Simple, focused implementation (~1000 lines total)
- Type-safe AST with exhaustive pattern matching
- Zero external dependencies for parsing
- Clear, readable code

**Disadvantages**:
- Limited boolean algebra operations
- No license key aliases support
- Cannot handle license keys with spaces
- Parentheses rendering differs from Python

---

## 10. Recommendations

### Critical Issues

1. **Parentheses Rendering in Combined Expressions** (HIGH PRIORITY)
   - **Issue**: Rust renders `(mit AND apache) AND gpl` while Python renders `mit AND apache AND gpl`
   - **Impact**: Golden test failures, output differs from Python
   - **Fix**: Modify `expression_to_string()` to flatten same-operator chains
   - **Location**: `src/license_detection/expression/simplify.rs:369-384`

### Medium Priority Issues

2. **Unknown Key Handling in SPDX Conversion** (MEDIUM PRIORITY)
   - **Issue**: Rust generates `LicenseRef-scancode-<key>` for unknown keys, Python validates and raises error
   - **Impact**: Different error handling, but both produce correct SPDX output
   - **Recommendation**: Add optional validation step before SPDX conversion

3. **License Key Aliases** (MEDIUM PRIORITY)
   - **Issue**: Rust cannot recognize license key aliases (e.g., "GPLv2" for "gpl-2.0")
   - **Impact**: User-provided expressions with aliases will fail to parse
   - **Recommendation**: Implement alias resolution in `parse_expression()` using License database

### Low Priority Issues

4. **Exception Symbol Validation** (LOW PRIORITY)
   - **Issue**: Rust doesn't validate that WITH right-hand side is an exception
   - **Impact**: Could allow malformed expressions like "MIT WITH Apache"
   - **Recommendation**: Add strict mode validation if needed for data quality checks

5. **License Keys with Spaces** (LOW PRIORITY)
   - **Issue**: Rust tokenizer cannot parse license keys containing spaces
   - **Impact**: Unlikely to occur in practice (ScanCode uses dash-separated keys)
   - **Recommendation**: Document limitation, no code change needed

---

## 11. Test Coverage Comparison

### Python Test Cases

From `reference/scancode-toolkit/venv/lib/python3.12/site-packages/license_expression/`:

- Simple license keys
- AND/OR/WITH operators
- Parenthetical grouping
- License key aliases
- Case-insensitive operators
- Unknown license keys
- Exception validation (strict mode)
- Containment checks
- Equivalence checks
- Simplification
- Deduplication
- SPDX conversion
- License keys with spaces

### Rust Test Cases

From `src/license_detection/expression/parse_test.rs` and `simplify_test.rs`:

- Simple license keys ✓
- AND/OR/WITH operators ✓
- Parenthetical grouping ✓
- Case-insensitive operators ✓
- License keys with special chars (-, ., +) ✓
- Unknown license keys ✓
- Containment checks ✓
- Simplification ✓
- SPDX conversion ✓

### Missing Test Coverage in Rust

1. **License key aliases** - Not supported
2. **Exception validation** - Not implemented
3. **License keys with spaces** - Not supported
4. **Equivalence checking** - Not implemented (can use `expressions_equal()` helper)
5. **Error position tracking** - Not implemented
6. **Advanced boolean simplification** - Not implemented

---

## 12. Code References

### Python

| Feature | File | Lines |
|---------|------|-------|
| Licensing class | `license_expression/__init__.py` | 189-808 |
| parse() method | `license_expression/__init__.py` | 476-559 |
| tokenize() method | `license_expression/__init__.py` | 561-613 |
| dedup() method | `license_expression/__init__.py` | 707-761 |
| combine_expressions() | `license_expression/__init__.py` | 1746-1802 |
| LicenseSymbol | `license_expression/__init__.py` | 1182-1312 |
| LicenseWithExceptionSymbol | `license_expression/__init__.py` | 1376-1496 |
| build_spdx_license_expression() | `licensedcode/cache.py` | 507-524 |
| UNKNOWN_LICENSE_KEY | `licensedcode/models.py` | 2773 |

### Rust

| Feature | File | Lines |
|---------|------|-------|
| LicenseExpression enum | `expression/mod.rs` | 74-99 |
| parse_expression() | `expression/parse.rs` | 41-49 |
| tokenize() | `expression/parse.rs` | 52-100 |
| parse_tokens() | `expression/parse.rs` | 114-128 |
| simplify_expression() | `expression/simplify.rs` | 14-35 |
| combine_expressions() | `expression/simplify.rs` | 435-473 |
| expression_to_string() | `expression/simplify.rs` | 358-409 |
| licensing_contains() | `expression/simplify.rs` | 246-308 |
| validate_expression() | `expression/simplify.rs` | 319-336 |
| SpdxMapping | `spdx_mapping/mod.rs` | 24-36 |
| expression_scancode_to_spdx() | `spdx_mapping/mod.rs` | 156-160 |

---

## 13. Conclusion

The Rust implementation provides a solid foundation for license expression parsing with correct handling of the core use cases. The main behavioral difference is in parentheses rendering for combined expressions, which should be addressed to match Python's output format.

**Strengths of Rust Implementation**:
- Clean, type-safe AST design
- Comprehensive test coverage for core functionality
- Correct SPDX conversion
- Proper operator precedence handling
- Efficient simplification

**Areas for Improvement**:
- Flatten same-operator chains in `expression_to_string()`
- Consider adding license key alias support
- Add optional strict validation mode
- Improve error messages with position tracking

**Overall Assessment**: The Rust implementation is production-ready for ScanCode's primary use case (parsing and converting license expressions detected from source code). The missing features (aliases, advanced boolean operations, license keys with spaces) are not commonly needed in practice and can be added incrementally if required.
