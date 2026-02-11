# Implementation Plans

This directory contains **temporary planning documents** for porting Python ScanCode features to Rust. These are working documents that track implementation progress and will be archived once features are complete.

## Directory Structure

Plans are organized by major feature area:

```text
implementation-plans/
├── package-detection/     # Package manifest parsing and assembly
├── text-detection/        # License, copyright, email/URL detection
├── post-processing/       # Summarization, tallies, classification
├── output/                # Output format support (SPDX, CycloneDX, etc.)
└── infrastructure/        # Plugin system, caching, progress tracking
```

## Active Plans

### Package Detection (`package-detection/`)

- **[PARSER_PLAN.md](package-detection/PARSER_PLAN.md)** - Individual file format parser implementations
  - Status: ~98% complete — only complex binary formats remain (low priority)

- **[ASSEMBLY_PLAN.md](package-detection/ASSEMBLY_PLAN.md)** - Package assembly roadmap
  - Status: Phase 1-3 Complete + Phase 4a npm workspace assembly complete
  - Next: File reference resolution (RPM/Alpine/Debian), archive extraction

## Placeholder Plans (To Be Fleshed Out)

These represent major architectural components not yet implemented. Each will be expanded into a detailed implementation plan when work begins.

### Text Detection (`text-detection/`)

- **[LICENSE_DETECTION_PLAN.md](text-detection/LICENSE_DETECTION_PLAN.md)** - License text detection and matching
  - Priority: P0 - Critical, Effort: 6-8 weeks

- **[COPYRIGHT_DETECTION_PLAN.md](text-detection/COPYRIGHT_DETECTION_PLAN.md)** - Copyright statement extraction
  - Priority: P1 - High, Effort: 3-4 weeks

- **[EMAIL_URL_DETECTION_PLAN.md](text-detection/EMAIL_URL_DETECTION_PLAN.md)** - Email and URL extraction
  - Priority: P2 - Medium, Effort: 1-2 weeks

### Post-Processing (`post-processing/`)

- **[SUMMARIZATION_PLAN.md](post-processing/SUMMARIZATION_PLAN.md)** - License/copyright tallies, facets, classification
  - Priority: P2 - Medium, Effort: 3-4 weeks

- **[CONSOLIDATION_PLAN.md](post-processing/CONSOLIDATION_PLAN.md)** - Resource grouping by origin, package enrichment with discovered licenses/copyrights
  - Priority: P2 - Medium, Effort: 2-3 weeks
  - Dependencies: License detection, copyright detection, package assembly

### Output Formats (`output/`)

- **[OUTPUT_FORMATS_PLAN.md](output/OUTPUT_FORMATS_PLAN.md)** - SPDX, CycloneDX, CSV, YAML, HTML output
  - Priority: P1 - High, Effort: 4-6 weeks

### Infrastructure (`infrastructure/`)

- **[PLUGIN_SYSTEM_PLAN.md](infrastructure/PLUGIN_SYSTEM_PLAN.md)** - Extensible plugin architecture
  - Priority: P3 - Low, Effort: 3-4 weeks

- **[CACHING_PLAN.md](infrastructure/CACHING_PLAN.md)** - Scan result caching and incremental scanning
  - Priority: P2 - Medium, Effort: 2-3 weeks

- **[PROGRESS_TRACKING_PLAN.md](infrastructure/PROGRESS_TRACKING_PLAN.md)** - Enhanced progress reporting
  - Priority: P3 - Low, Effort: 1-2 weeks

## Document Lifecycle

1. **Placeholder** - Brief description of component, scope, and dependencies
2. **Planning** - Detailed analysis, design decisions, implementation phases
3. **Active** - Work in progress, updated with status
4. **Complete** - Feature implemented, document archived
5. **Archived** - Moved to `docs/archived/` for historical reference

## Relationship to Evergreen Docs

These implementation plans are **temporary** and complement the **evergreen** documentation in `docs/`:

| Evergreen (Permanent) | Implementation Plans (Temporary) |
|-----------------------|----------------------------------|
| `ARCHITECTURE.md` | Component-specific implementation plans |
| `HOW_TO_ADD_A_PARSER.md` | `PARSER_PLAN.md` |
| `TESTING_STRATEGY.md` | Test plans within implementation docs |
| `adr/` | Design decisions made during implementation |
| `improvements/` | Beyond-parity features documented here |

Once a feature is complete, relevant architectural decisions move to ADRs, and the implementation plan is archived.
