# Ruby/RubyGems Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Test Status

**Currently Passing:** 1/4 tests

- ✅ **cat-gemspec** - Passing with improvements
- ⏸️ **arel-gemspec** - Multi-line Ruby literals, conditional dependencies (complex)
- ⏸️ **oj-gemspec** - License detection engine required
- ⏸️ **rubocop-gemspec** - License detection engine required

## Intentional Improvements Over Python ScanCode

### Party/Author Extraction (Better Data Model)

**Python Behavior (Fragmented):**
```json
[
  {"name": "蒼時弦也", "email": null},
  {"name": null, "email": "elct9620@frost.tw"}
]
```

**Our Behavior (Combined & Semantic):**
```json
[
  {"name": "蒼時弦也", "email": "elct9620@frost.tw"}
]
```

**Rationale:**
- **Semantic correctness**: One person = one party record
- **Data integrity**: Email is associated with the correct author
- **RFC 5322 support**: Parses `"Name <email>"` format correctly
- **Better UX**: Downstream tools get complete party information

Python's approach fragments related data and loses the association between names and emails. Our implementation preserves data relationships and provides more useful output.

### Dependency Scope (Explicit vs Implicit)

**Python:** Runtime dependencies have `scope: null`  
**Ours:** Runtime dependencies have `scope: "runtime"`

**Rationale:** Explicit is better than implicit. All dependency scopes should be clearly labeled.

## Test Data

Test files sourced from Python ScanCode reference:
- `reference/scancode-toolkit/tests/packagedcode/data/rubygems/`
