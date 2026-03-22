# Consolidation Implementation Plan

> **Status**: ⚪ Deferred / Not Planned — retained as an explicit non-goal for current Provenant scope
> **Priority**: Deferred (compatibility-only feature, not on current roadmap)
> **Estimated Effort**: 2-3 weeks
> **Dependencies**: [LICENSE_DETECTION_ARCHITECTURE.md](../../LICENSE_DETECTION_ARCHITECTURE.md), [COPYRIGHT_DETECTION_PLAN.md](../text-detection/COPYRIGHT_DETECTION_PLAN.md), [ASSEMBLY_PLAN.md](../package-detection/ASSEMBLY_PLAN.md)

## Overview

Consolidation is an opt-in post-scan plugin (`--consolidate`) that groups scanned resources by origin and enriches packages with license/copyright data discovered in their files. It produces two output structures:

- **ConsolidatedPackage**: A detected package enriched with "core" licenses/copyrights (from its manifest) and "other" licenses/copyrights (discovered by scanning files within the package).
- **ConsolidatedComponent**: A group of files sharing the same copyright holder that aren't part of any detected package.

This is **not** package deduplication. It's about combining package metadata with scan findings.

## Recommendation

**Do not build consolidation for the current Provenant roadmap.** Keep this document as a decision record and future reference only.

Why:

- Official ScanCode docs still expose `--consolidate`, but they also mark it for future deprecation because newer top-level package/dependency/license data already provide a better consolidated view.
- Provenant's goal is parity with current high-value functionality, not compatibility with every legacy-compatible reporting surface.
- The feature is primarily compatibility-oriented and does not justify its implementation cost relative to the remaining summary/tally work.
- Provenant can revisit this later only if a concrete user or compatibility requirement emerges.

If this decision is ever revisited, the order should still be:

1. shared provenance cleanup
2. summarization parity work
3. consolidation parity/compatibility work

## Why This Feature Is Deferred

For current Provenant scope, consolidation is best treated as a **legacy-compatible output surface that we are intentionally not implementing**:

- it does not add enough strategic value beyond the newer top-level package/dependency/license view
- upstream already treats it as a deprecating compatibility layer
- it would add non-trivial implementation and maintenance cost for a feature Provenant does not plan to promote

This document is retained so the non-implementation is explicit, reviewable, and reversible if user demand changes.

## Architectural Boundary

Consolidation is **not** the place to decide a package's declared license.

- **Parsers** own `extracted_license_statement` and, for trustworthy manifest fields, may also populate:
  - `declared_license_expression`
  - `declared_license_expression_spdx`
  - parser-origin `license_detections`
- **Consolidation** should enrich packages with **discovered** license/copyright evidence from assigned files.
- **Consolidation must not silently overwrite manifest-derived declared license fields**. If it derives additional package-level license conclusions from file evidence, those should remain explicitly modeled as enriched/discovered data rather than replacing declared package metadata.

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

**Reference status note**: Python emits a deprecation warning for this plugin and also documents it as headed toward deprecation because improved top-level packages, dependencies, and licenses now provide better consolidated data.

**Important reference nuance**:

- In Python ScanCode, package declared-license fields are typically populated earlier on `PackageData` / `Package`, before consolidation.
- Consolidation then consumes those package-declared fields as the package's **core** license and adds file-discovered evidence as **other** or consolidated evidence.
- Provenant should keep that same high-level separation even if the exact implementation is cleaner and more explicit than the reference.

### Upstream Value Surface

- `--consolidate` gives users a grouped compatibility view that rolls package-level and non-package findings into larger units.
- It is still part of the current CLI/output contract.
- It is no longer the strategic center of ScanCode's reporting model, so parity is the main reason to implement it.

## Current State in Rust

## Current Product Decision

- **Not planned** for the current Provenant roadmap.
- **Not required** for the project's current interpretation of drop-in replacement goals.
- **May be reconsidered** only if latest-user workflows or downstream integrations demonstrate a real need for `--consolidate` and its output structures.

### Implemented

- ✅ Package assembly with `for_packages` linking (resources → packages)
- ✅ PURL generation
- ✅ Output format structure

### Missing

- ❌ Rich discovered license data integration for package/file enrichment
- ❌ Copyright detection integration for package/file enrichment
- ❌ Consolidation logic
- ❌ Consolidation models/output structures (`ConsolidatedPackage`, `ConsolidatedComponent`, resource-level `consolidated_to`)
- ❌ `--consolidate` CLI flag
- ❌ Regression coverage against the Python compatibility surface

### Already handled elsewhere

- ✅ Parser-side normalization of trustworthy declared package-license metadata
- ✅ Package assembly and `for_packages` linking
- ✅ Initial key-file promotion and summary foundations

### Concrete follow-up before/alongside consolidation

- Revisit the current key-file promotion path in `src/main.rs`.
- Today `promote_package_metadata_from_key_files(...)` can backfill package `declared_license_expression*` and `license_detections` from key files when package fields are empty.
- Before full consolidation lands, narrow this behavior so file-derived package enrichment uses explicit enriched/discovered provenance instead of mutating manifest-derived declared package fields implicitly.

## Implementation Phases

> These phases are retained only as future reference if this decision changes.

1. **Phase 0**: Shared provenance cleanup so package declared-license fields remain distinct from file-derived enrichment.
2. **Phase 1**: Define consolidation data models and output wiring for `ConsolidatedPackage`, `ConsolidatedComponent`, and resource-level `consolidated_to` links.
3. **Phase 2**: Build package-owned resource grouping from assembly-provided `for_packages` links and manifest/file ownership context.
4. **Phase 3**: Split package evidence into manifest-derived "core" findings and file-discovered "other" findings without mutating declared package fields.
5. **Phase 4**: Group non-package resources into consolidated components using stable holder-based grouping and deterministic identifiers.
6. **Phase 5**: Synthesize consolidated expressions, holders, and copyright output fields from grouped evidence.
7. **Phase 6**: Wire `--consolidate` CLI gating, output serialization, and regression coverage (unit, integration, and golden tests).

Sequencing note: implementation still depends on richer discovered license/copyright data for package/file enrichment, but the work is now broken into explicit phases instead of remaining an undetailed placeholder. This work should start only after the summarization path and shared provenance cleanup are stable.

## Success Criteria

> These success criteria apply only if the feature is reactivated.

- [ ] Enriches packages with discovered license/copyright data from their files without overwriting manifest-derived declared license fields
- [ ] Groups orphan resources by copyright holder
- [ ] Generates simplified combined license expressions
- [ ] `consolidated_to` links resources to their consolidation group
- [ ] Matches the current ScanCode compatibility surface for `consolidated_packages` / `consolidated_components`
- [ ] Preserves explicit core-vs-other provenance more clearly than the Python reference
- [ ] Exposes the `--consolidate` CLI flag with parity-compatible semantics and deprecation-aware documentation
- [ ] Golden tests pass against Python reference

## Related Documents

- **Prerequisite**: [`docs/LICENSE_DETECTION_ARCHITECTURE.md`](../../LICENSE_DETECTION_ARCHITECTURE.md) — implemented license-detection engine and file evidence pipeline
- **Prerequisite**: [`COPYRIGHT_DETECTION_PLAN.md`](../text-detection/COPYRIGHT_DETECTION_PLAN.md)
- **Prerequisite**: [`ASSEMBLY_PLAN.md`](../package-detection/ASSEMBLY_PLAN.md)
- **Sibling**: [`SUMMARIZATION_PLAN.md`](SUMMARIZATION_PLAN.md) — related post-processing

## Notes

- Opt-in feature (`--consolidate`), not enabled by default
- Requires package scans plus discovered license/copyright data from the file-scanning pipeline
- Should treat parser-declared package license data as the package's "core" license input, not as something consolidation itself computes
- The current `src/main.rs` key-file promotion behavior is a temporary bridge and should be narrowed as part of the package-enrichment architecture cleanup.
- Python has a deprecation warning on this plugin; that upstream signal is part of why Provenant is intentionally deferring it
- If this feature is ever reactivated, it should still be treated as compatibility-oriented rather than as a preferred long-term reporting model
