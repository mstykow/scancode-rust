# Summarization & Analysis Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P2 - Medium Priority (Post-Processing Feature)
> **Estimated Effort**: 3-4 weeks
> **Dependencies**: LICENSE_DETECTION_PLAN.md, COPYRIGHT_DETECTION_PLAN.md, ASSEMBLY_IMPLEMENTATION_PLAN.md

## Overview

Post-scan analysis and summarization features that aggregate findings across files to provide high-level insights: license tallies, copyright statistics, license clarity scoring, file classification, and facets.

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
- Package consolidation (covered by CONSOLIDATION_PLAN.md)
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

### Missing

- ❌ License tallies
- ❌ Copyright tallies
- ❌ Package tallies
- ❌ License clarity scoring
- ❌ File classification
- ❌ Facet assignment
- ❌ Generated code detection
- ❌ Comprehensive scan summary

## Implementation Phases (TBD)

1. **Phase 1**: File classification and facet assignment
2. **Phase 2**: License tallies
3. **Phase 3**: Copyright tallies
4. **Phase 4**: Package tallies
5. **Phase 5**: License clarity scoring
6. **Phase 6**: Generated code detection
7. **Phase 7**: Scan summary generation

## Success Criteria

- [ ] Generates accurate tallies for licenses, copyrights, packages
- [ ] Calculates license clarity score matching Python
- [ ] Classifies files correctly
- [ ] Detects generated code
- [ ] Golden tests pass

## Related Documents

- **Implementation**: `LICENSE_DETECTION_PLAN.md` (prerequisite)
- **Implementation**: `COPYRIGHT_DETECTION_PLAN.md` (prerequisite)
- **Implementation**: `ASSEMBLY_IMPLEMENTATION_PLAN.md` (prerequisite)
- **Evergreen**: `ARCHITECTURE.md` (post-processing pipeline)

## Notes

- Requires license and copyright detection to be implemented first
- Can be implemented incrementally (one tally type at a time)
- License clarity score is a key metric for compliance teams
