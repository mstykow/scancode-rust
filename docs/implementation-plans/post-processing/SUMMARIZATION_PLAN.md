# Summarization & Analysis Implementation Plan

> **Status**: 🟡 In Progress — foundational summary and key-file infrastructure is implemented; broader tallies/facets/generated-code parity remains open
> **Priority**: P2 - Medium Priority (Post-Processing Feature)
> **Estimated Effort**: 3-4 weeks
> **Dependencies**: LICENSE_DETECTION_PLAN.md, COPYRIGHT_DETECTION_PLAN.md, ASSEMBLY_PLAN.md

## Overview

Post-scan analysis and summarization features that aggregate findings across files to provide high-level insights: license tallies, copyright statistics, license clarity scoring, file classification, and facets.

## Architectural Boundary

Summarization is a **consumer**, not a normalizer.

- **Parsers** should already provide manifest-derived declared package-license data when the source field is trustworthy enough to normalize.
- **Summarization** should read package-declared metadata plus discovered file/resource evidence and turn them into:
  - tallies
  - clarity scoring
  - scan-level summary expressions
  - classification/facets
- **Summarization should not become the primary place that decides a package's declared license**.

## Scope

### What This Covers

- **License Tallies**: Count and categorize licenses across codebase
- **Copyright Tallies**: Aggregate copyright holders and statements
- **Package Tallies**: Count packages by ecosystem
- **License Clarity Score**: Calculate license clarity metrics
- **File Classification**: Classify files by type (source, test, doc, data, etc.)
- **Facet Assignment**: Tag files with facets (core, dev, test, doc, etc.)
- **Generated Code Detection**: Identify auto-generated files
- **Scan Summary**: High-level scan statistics

### What This Doesn't Cover

- License policy evaluation (separate feature)
- Package consolidation (covered by `CONSOLIDATION_PLAN.md` in this directory)
- Output formatting (covered by OUTPUT_FORMATS_PLAN.md)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/summarycode/`

**Key Components**:

- `tallies.py` - License, copyright, package tallies
- `score.py` - License clarity scoring
- `classify_plugin.py` - File classification
- `facet.py` - Facet assignment
- `generated.py` - Generated code detection
- `summarizer.py` - Scan summary generation
- `copyright_tallies.py` - Copyright statistics

## Current State in Rust

### Implemented

- ✅ Basic scan statistics (file count, scan time)
- ✅ Output format structure
- ✅ Top-level `summary` output block
- ✅ Key-file tagging foundations (`is_legal`, `is_manifest`, `is_readme`, `is_top_level`, `is_key_file`)
- ✅ Package metadata promotion from key files
- ✅ Initial `license_clarity_score` model/output
- ✅ Initial non-license-dependent summary fields:
  - `declared_holder`
  - `primary_language`
  - `other_languages`

### Missing

- ❌ License tallies
- ❌ Copyright tallies
- ❌ Package tallies
- ❌ Full Python-parity license clarity scoring heuristics
- ❌ Broader file classification beyond current key-file/source slices
- ❌ Facet assignment
- ❌ Generated code detection
- ❌ Comprehensive scan summary parity

### Already handled elsewhere

- ✅ Parser-side normalization of trustworthy declared package-license metadata
- ✅ Initial summary consumption of package/key-file declared license data
- ✅ Initial package metadata promotion from key files

## Implementation Phases

1. **Phase 1**: File classification and key-file tagging foundations ✅
2. **Phase 2**: Package/file metadata promotion foundations ✅
3. **Phase 3**: Initial summary model/output structure ✅
4. **Phase 4**: Initial non-license-dependent summary fields ✅
5. **Phase 5**: License tallies over existing declared/discovered evidence
6. **Phase 6**: Copyright tallies
7. **Phase 7**: Package tallies
8. **Phase 8**: Full license clarity parity
9. **Phase 9**: Facets and generated-code detection
10. **Phase 10**: Comprehensive scan summary parity

## Success Criteria

- [ ] Generates accurate tallies for licenses, copyrights, packages
- [ ] Calculates license clarity score matching Python
- [ ] Classifies files correctly beyond current key-file/source slices
- [ ] Detects generated code
- [ ] Golden tests pass

## Related Documents

- **Implementation**: `LICENSE_DETECTION_PLAN.md` (prerequisite)
- **Implementation**: `COPYRIGHT_DETECTION_PLAN.md` (prerequisite)
- **Implementation**: `../package-detection/ASSEMBLY_PLAN.md` (prerequisite)
- **Evergreen**: `ARCHITECTURE.md` (post-processing pipeline)

## Notes

- Some summarization foundations can land before full detector parity (for example key-file tagging, package metadata promotion, initial summary fields, and primary-language/holder derivation).
- Full parity for tallies and Python-style scoring still depends on richer discovered-license/copyright coverage and clearer package-vs-file provenance.
- The recent parser-side declared-license normalization work reduces one gap for summarization consumers, but it does not remove the need for summary tallies, facets, generated-code detection, or scan-level aggregation.
- Can be implemented incrementally (one tally type at a time)
- License clarity score is a key metric for compliance teams
