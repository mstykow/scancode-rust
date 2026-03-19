# Ruby/RubyGems Parser Golden Test Suite

## Purpose

Golden tests compare parser output against expected results from the original ScanCode Toolkit to ensure compatibility.

## Test Status

**Current state:** 6/8 Ruby parser goldens are active; the remaining 2 are still intentionally ignored because they require license detection engine integration.

- ✅ **cat-gemspec** - Passing with improvements
- ✅ **arel-gemspec** - Active after parser parity fixes for `%q{}` cleanup and conditional dependency extraction
- ✅ **with-variables** - Active with required-file constant resolution for gemspec metadata
- ✅ **Gemfile (source options)** - Active with manifest-level `git`/`path`/`source` provenance preservation
- ✅ **Gemfile.lock (git)** - Active with Bundler GIT source metadata preserved in dependency extra data
- ✅ **Gemfile.lock (path)** - Active with PATH primary-package behavior and lockfile metadata preserved
- ⏸️ **oj-gemspec** - License detection engine required
- ⏸️ **rubocop-gemspec** - License detection engine required

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
