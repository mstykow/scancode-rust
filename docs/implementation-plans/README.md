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

- **[PARSER_PARITY_PLAN.md](package-detection/PARSER_PARITY_PLAN.md)** - Individual file format parser implementations
  - Status: ~98% complete (79 parsers implemented)
  - Remaining: Phase 5 complex binary formats (optional, low priority)

- **[ASSEMBLY_IMPLEMENTATION_PLAN.md](package-detection/ASSEMBLY_IMPLEMENTATION_PLAN.md)** - Package assembly roadmap
  - Status: Phase 1 Complete (8 ecosystems) - Feb 10, 2026
  - Scope: Merging related files into logical packages (e.g., package.json + package-lock.json)
  - Next: Phase 2 (Maven nested sibling-merge)

- **[CONSOLIDATION_PLAN.md](package-detection/CONSOLIDATION_PLAN.md)** - Package deduplication
  - Status: Placeholder

**Supporting Docs**:

- [ASSEMBLY_PARITY_ROADMAP.md](package-detection/ASSEMBLY_PARITY_ROADMAP.md) - Analysis of Python's 20 assemblers
- [ASSEMBLY_QUICK_REFERENCE.md](package-detection/ASSEMBLY_QUICK_REFERENCE.md) - Assembly concepts
- [PYTHON_ASSEMBLERS_SUMMARY.md](package-detection/PYTHON_ASSEMBLERS_SUMMARY.md) - Python framework overview
- [PYTHON_ASSEMBLERS_DETAILED.md](package-detection/PYTHON_ASSEMBLERS_DETAILED.md) - Detailed assembler analysis
- [ASSEMBLY_DOCUMENTATION_INDEX.md](package-detection/ASSEMBLY_DOCUMENTATION_INDEX.md) - Navigation guide

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
| `HOW_TO_ADD_A_PARSER.md` | `PARSER_PARITY_PLAN.md` |
| `TESTING_STRATEGY.md` | Test plans within implementation docs |
| `adr/` | Design decisions made during implementation |
| `improvements/` | Beyond-parity features documented here |

Once a feature is complete, relevant architectural decisions move to ADRs, and the implementation plan is archived.
