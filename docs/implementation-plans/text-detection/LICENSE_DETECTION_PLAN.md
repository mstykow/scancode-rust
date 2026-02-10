# License Detection Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P0 - Critical Core Feature
> **Estimated Effort**: 6-8 weeks
> **Dependencies**: None (foundational feature)

## Overview

License detection is the core feature of ScanCode - identifying license text in source files and generating SPDX license expressions. This involves text matching, rule-based detection, and license expression composition.

## Scope

### What This Covers

- License text detection using fingerprinting/matching algorithms
- SPDX license expression generation
- License rule database and matching engine
- License detection confidence scoring
- Multi-license file handling (dual licensing, license stacks)
- License text normalization and comparison

### What This Doesn't Cover

- License policy evaluation (separate feature)
- License compatibility checking (separate feature)
- SPDX document generation (covered by OUTPUT_FORMATS_PLAN.md)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/`

**Key Components**:

- `cache.py` - License rule caching and indexing
- `match.py` - License match algorithms (exact, near-exact, fuzzy)
- `models.py` - License rule data structures
- `detection.py` - License detection orchestration
- `spans.py` - Text span matching and alignment
- `tokenize.py` - Text tokenization for matching

**Data Sources**:

- SPDX license list (already available in `resources/licenses/`)
- Custom license rules and patterns
- License expression parser (license-expression library)

## Current State in Rust

### Implemented

- ✅ SPDX license data embedded at compile time (`resources/licenses/` submodule)
- ✅ Basic license field structures in `PackageData` (license_expression, declared_license)
- ✅ License detection placeholders in output format

### Missing

- ❌ License text matching engine
- ❌ License rule database and indexing
- ❌ Text tokenization and normalization
- ❌ Match scoring and confidence calculation
- ❌ License expression composition
- ❌ Multi-license detection logic

## Architecture Considerations

### Design Questions

1. **Matching Algorithm**: Use Python's approach (token-based fingerprinting) or explore Rust-native alternatives (e.g., Aho-Corasick, suffix arrays)?
2. **Rule Storage**: Embed rules at compile time or load at runtime?
3. **Caching Strategy**: In-memory cache, disk cache, or both?
4. **Parallelization**: Per-file parallel detection or batch processing?

### Integration Points

- Scanner pipeline: Add license detection phase after file enumeration
- Output format: Populate `license_detections` array in file data
- Package parsers: Merge detected licenses with declared licenses

## Implementation Phases (TBD)

This section will be expanded when work begins. Preliminary phases:

1. **Phase 1**: Text tokenization and normalization
2. **Phase 2**: License rule database and indexing
3. **Phase 3**: Exact match detection
4. **Phase 4**: Fuzzy/near-exact matching
5. **Phase 5**: License expression composition
6. **Phase 6**: Integration with scanner pipeline

## Success Criteria

- [ ] Detects all SPDX-listed licenses with >95% accuracy
- [ ] Handles multi-license files correctly
- [ ] Generates valid SPDX license expressions
- [ ] Performance: <100ms per file on average
- [ ] Golden tests pass against Python reference output

## Related Documents

- **Evergreen**: `ARCHITECTURE.md` (scanner pipeline)
- **Evergreen**: `TESTING_STRATEGY.md` (golden test approach)
- **Implementation**: `PARSER_PARITY_PLAN.md` (package detection integration)
- **ADR**: TBD - License detection algorithm selection

## Notes

- This is the most complex feature to port from Python
- Consider using existing Rust crates (e.g., `askalono` for inspiration, though it's Apache-2.0 focused)
- License detection is independent of package detection - can be implemented in parallel
- Golden tests will be critical for validating parity with Python
