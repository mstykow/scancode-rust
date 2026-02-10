# Copyright Detection Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P1 - High Priority Core Feature
> **Estimated Effort**: 3-4 weeks
> **Dependencies**: None

## Overview

Copyright detection extracts copyright statements, holder names, and authorship information from source files using pattern matching and natural language processing techniques.

## Scope

### What This Covers

- Copyright statement detection (© 2024 Company Name, Copyright (c) 2024, etc.)
- Copyright holder extraction
- Year/year range parsing
- Author name extraction
- Email address extraction (related to copyright holders)

### What This Doesn't Cover

- Email/URL extraction from general source code (covered by EMAIL_URL_DETECTION_PLAN.md)
- Copyright policy evaluation
- License-copyright correlation

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/cluecode/`

**Key Components**:

- `copyrights.py` - Copyright detection patterns and logic
- `finder.py` - Pattern matching and text extraction
- `plugin_copyright.py` - Scanner plugin integration

**Detection Patterns**:

- Regex patterns for copyright statements
- Holder name extraction heuristics
- Year parsing and normalization
- Multi-line copyright statement handling

## Current State in Rust

### Implemented

- ✅ Copyright field structures in file data (`copyrights`, `holders`, `authors`)
- ✅ Output format placeholders

### Missing

- ❌ Copyright pattern matching engine
- ❌ Holder name extraction
- ❌ Year parsing logic
- ❌ Multi-line statement handling
- ❌ Scanner integration

## Architecture Considerations

### Design Questions

1. **Pattern Engine**: Regex-based (like Python) or custom parser?
2. **Performance**: Per-file or batch processing?
3. **Accuracy**: Balance between precision and recall

### Integration Points

- Scanner pipeline: Add copyright detection phase
- Output format: Populate copyright arrays in file data
- Text extraction: Reuse tokenization from license detection if applicable

## Implementation Phases (TBD)

Preliminary phases:

1. **Phase 1**: Copyright pattern database
2. **Phase 2**: Basic statement detection
3. **Phase 3**: Holder name extraction
4. **Phase 4**: Year parsing and normalization
5. **Phase 5**: Multi-line handling
6. **Phase 6**: Scanner integration

## Success Criteria

- [ ] Detects standard copyright formats (©, (c), Copyright)
- [ ] Extracts holder names accurately
- [ ] Parses year ranges correctly
- [ ] Handles multi-line statements
- [ ] Golden tests pass against Python reference

## Related Documents

- **Implementation**: `EMAIL_URL_DETECTION_PLAN.md` (related text extraction)
- **Implementation**: `LICENSE_DETECTION_PLAN.md` (similar pattern matching approach)
- **Evergreen**: `ARCHITECTURE.md` (scanner pipeline)

## Notes

- Simpler than license detection (pattern-based, not fingerprinting)
- Can leverage Rust regex crate for performance
- Consider using `fancy-regex` for lookahead/lookbehind patterns
