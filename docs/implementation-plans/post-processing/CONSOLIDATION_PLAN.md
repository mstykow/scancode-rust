# Consolidation Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P2 - Medium Priority (Post-Processing Feature)
> **Estimated Effort**: 2-3 weeks
> **Dependencies**: LICENSE_DETECTION_PLAN.md, COPYRIGHT_DETECTION_PLAN.md, ASSEMBLY_PLAN.md

## Overview

Consolidation is an opt-in post-scan plugin (`--consolidate`) that groups scanned resources by origin and enriches packages with license/copyright data discovered in their files. It produces two output structures:

- **ConsolidatedPackage**: A detected package enriched with "core" licenses/copyrights (from its manifest) and "other" licenses/copyrights (discovered by scanning files within the package).
- **ConsolidatedComponent**: A group of files sharing the same copyright holder that aren't part of any detected package.

This is **not** package deduplication. It's about combining package metadata with scan findings.

## Scope

### What This Covers

- Grouping resources by detected package (using `for_packages` linking from assembly)
- Grouping remaining resources by copyright holder
- Splitting license/copyright findings into "core" (from manifest) vs "other" (from file scanning)
- Generating `consolidated_license_expression` (simplified combination of core + other)
- Generating `consolidated_holders` and `consolidated_copyright`
- Adding `consolidated_to` field on each resource (links resource to its consolidation group)

### What This Doesn't Cover

- Package assembly (covered by `../package-detection/ASSEMBLY_PLAN.md`)
- Tallies, scoring, classification (covered by `SUMMARIZATION_PLAN.md`)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/summarycode/plugin_consolidate.py`

**Key Classes**:

- `Consolidator` — PostScanPlugin, entry point (`--consolidate` flag)
- `Consolidation` — Holds core/other license expressions and holders for a group of files
- `ConsolidatedPackage` — Package + its Consolidation data
- `ConsolidatedComponent` — Component type + its Consolidation data

**Algorithm** (from `process_codebase`):

1. For each detected package, collect all resources assigned to it
2. Split their license/copyright findings into core vs other
3. Create `ConsolidatedPackage` with combined data
4. For remaining resources (not in any package), group by copyright holder
5. Create `ConsolidatedComponent` for each holder group
6. Mark each resource with `consolidated_to` identifier

**Note**: Python has a deprecation warning on this plugin. Verify whether it's still maintained upstream before porting.

## Current State in Rust

### Implemented

- ✅ Package assembly with `for_packages` linking (resources → packages)
- ✅ PURL generation
- ✅ Output format structure

### Missing (all blocked on license + copyright detection)

- ❌ License detection (prerequisite — no license data to consolidate yet)
- ❌ Copyright detection (prerequisite — no copyright data to consolidate yet)
- ❌ Consolidation logic
- ❌ `--consolidate` CLI flag

## Implementation Phases (TBD)

Blocked until license and copyright detection are implemented.

## Success Criteria

- [ ] Enriches packages with discovered license/copyright data from their files
- [ ] Groups orphan resources by copyright holder
- [ ] Generates simplified combined license expressions
- [ ] `consolidated_to` links resources to their consolidation group
- [ ] Golden tests pass against Python reference

## Related Documents

- **Prerequisite**: `../text-detection/LICENSE_DETECTION_PLAN.md`
- **Prerequisite**: `../text-detection/COPYRIGHT_DETECTION_PLAN.md`
- **Prerequisite**: `../package-detection/ASSEMBLY_PLAN.md`
- **Sibling**: `SUMMARIZATION_PLAN.md` (related post-processing)

## Notes

- Opt-in feature (`--consolidate`), not enabled by default
- Requires license + copyright + package scans to all be active
- Python has a deprecation warning on this plugin — check upstream status before investing effort
