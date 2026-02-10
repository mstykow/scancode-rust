# Package Consolidation Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P2 - Medium Priority (Post-Processing Feature)
> **Estimated Effort**: 2-3 weeks
> **Dependencies**: ASSEMBLY_IMPLEMENTATION_PLAN.md

## Overview

Package consolidation deduplicates and merges package instances across the codebase. For example, if multiple files detect the same package (e.g., `lodash@4.17.21` in different node_modules directories), consolidation creates a single canonical package instance.

## Scope

### What This Covers

- Package deduplication by PURL
- Merging package metadata from multiple sources
- Consolidating dependencies
- Handling package instances in different locations (e.g., nested node_modules)
- Creating consolidated package list

### What This Doesn't Cover

- Package assembly (covered by ASSEMBLY_IMPLEMENTATION_PLAN.md)
- Package summarization/tallies (covered by SUMMARIZATION_PLAN.md)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/summarycode/plugin_consolidate.py`

**Key Logic**:

- Groups packages by PURL
- Merges metadata from multiple detections
- Handles nested package instances
- Creates consolidated package list

## Current State in Rust

### Implemented

- ✅ Package detection (file-level)
- ✅ PURL generation

### Missing

- ❌ Package deduplication logic
- ❌ Metadata merging
- ❌ Consolidated package list generation
- ❌ Nested instance handling

## Implementation Phases (TBD)

1. **Phase 1**: Package grouping by PURL
2. **Phase 2**: Metadata merging logic
3. **Phase 3**: Nested instance handling
4. **Phase 4**: Consolidated output generation

## Success Criteria

- [ ] Deduplicates packages correctly
- [ ] Merges metadata without loss
- [ ] Handles nested instances
- [ ] Golden tests pass

## Related Documents

- **Implementation**: `ASSEMBLY_IMPLEMENTATION_PLAN.md` (prerequisite)
- **Implementation**: `SUMMARIZATION_PLAN.md` (uses consolidated data)

## Notes

- Consolidation runs after assembly
- Important for monorepos with duplicate dependencies
- Reduces output size and improves clarity
