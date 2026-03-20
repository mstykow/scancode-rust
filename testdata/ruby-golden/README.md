# Ruby/RubyGems Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Coverage Summary

This fixture set covers representative gemspec metadata, required-file constant resolution, Gemfile source provenance, Gemfile.lock Git and path metadata, and the current parser-only boundary where some cases still depend on license-detection integration.

## Intentional Improvements Over Python ScanCode

### Party/Author Extraction (Better Data Model)

**Python Behavior (Fragmented):**

```json
[
  { "name": "蒼時弦也", "email": null },
  { "name": null, "email": "elct9620@frost.tw" }
]
```

**Our Behavior (Combined & Semantic):**

```json
[{ "name": "蒼時弦也", "email": "elct9620@frost.tw" }]
```

**Rationale:**

- **Semantic correctness**: One person = one party record
- **Data integrity**: Email is associated with the correct author
- **RFC 5322 support**: Parses `"Name <email>"` format correctly
- **Better UX**: Downstream tools get complete party information

Python's approach fragments related data and loses the association between names and emails. Our implementation preserves data relationships and provides more useful output.

### Dependency Scope (Explicit vs Implicit)

**Python:** Runtime dependencies have `scope: null`  
**Ours:** Runtime dependencies also keep `scope: null`

**Rationale:** Explicit is better than implicit. All dependency scopes should be clearly labeled.

Ruby now follows the ecosystem-native convention documented in this repository: plain runtime Gemfile dependencies keep a null scope, while install-time context such as group membership and source provenance is preserved in dedicated fields.

### Extracted Gem Assembly (Improved Deduplication)

Rust now assembles `metadata.gz-extract` together with sibling `data.gz-extract/*.gemspec` content and deduplicates the overlapping package/dependency results, while also assigning nested files like `data.gz-extract/LICENSE.txt` and `data.gz-extract/lib/example-gem.rb` to the assembled gem package.

## Test Data

Test files sourced from Python ScanCode reference:

- `reference/scancode-toolkit/tests/packagedcode/data/rubygems/`
