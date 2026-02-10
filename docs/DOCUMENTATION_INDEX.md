# Documentation Index

This index helps you find the right documentation for your needs.

## For Users

- **[README.md](../README.md)** - Installation, usage, and quick start
- **[SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md)** - List of all 68 supported package formats

## For Contributors

### Getting Started

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and components
- **[HOW_TO_ADD_A_PARSER.md](HOW_TO_ADD_A_PARSER.md)** - Step-by-step parser implementation guide
- **[TESTING_STRATEGY.md](TESTING_STRATEGY.md)** - Four-layer testing approach

### Design Decisions

- **[adr/](adr/)** - Architectural Decision Records (5 ADRs)
  - Why we chose trait-based parsers
  - Extraction vs detection separation
  - Golden test strategy
  - Security-first parsing
  - Auto-generated documentation

### Beyond-Parity Features

- **[improvements/](improvements/)** - Features where Rust exceeds Python (18 parsers documented)

## For Maintainers

### Implementation Plans (Temporary)

- **[implementation-plans/](implementation-plans/)** - Active and placeholder implementation plans
  - **Active Plans**: Parser parity (~98% complete), Assembly (not started)
  - **Placeholder Plans**: License detection, copyright detection, output formats, etc.
  - See [implementation-plans/README.md](implementation-plans/README.md) for full list

### Document Organization

```text
docs/
├── ARCHITECTURE.md                    # Evergreen: System design
├── HOW_TO_ADD_A_PARSER.md            # Evergreen: Parser guide
├── TESTING_STRATEGY.md                # Evergreen: Testing philosophy
├── SUPPORTED_FORMATS.md               # Evergreen: Auto-generated format list
├── DOCUMENTATION_INDEX.md             # This file
│
├── adr/                               # Evergreen: Design decisions
│   ├── 0001-trait-based-parsers.md
│   ├── 0002-extraction-vs-detection.md
│   ├── 0003-golden-test-strategy.md
│   ├── 0004-security-first-parsing.md
│   └── 0005-auto-generated-docs.md
│
├── improvements/                      # Evergreen: Beyond-parity features
│   ├── alpine-parser.md
│   ├── composer-parser.md
│   ├── ... (18 parsers total)
│   └── README.md
│
└── implementation-plans/              # Temporary: Porting progress
    ├── README.md                      # Plan lifecycle and organization
    ├── PARSER_PARITY_PLAN.md         # ~98% complete
    ├── ASSEMBLY_IMPLEMENTATION_PLAN.md # Not started
    ├── LICENSE_DETECTION_PLAN.md      # Placeholder
    ├── COPYRIGHT_DETECTION_PLAN.md    # Placeholder
    ├── ... (10 plans total)
    └── PYTHON_ASSEMBLERS_*.md         # Reference docs
```

## Quick Links by Task

### I want to

**...understand the overall architecture**
→ [ARCHITECTURE.md](ARCHITECTURE.md)

**...add a new package parser**
→ [HOW_TO_ADD_A_PARSER.md](HOW_TO_ADD_A_PARSER.md)

**...understand testing strategy**
→ [TESTING_STRATEGY.md](TESTING_STRATEGY.md)

**...see what formats are supported**
→ [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md)

**...understand a design decision**
→ [adr/](adr/)

**...see where Rust exceeds Python**
→ [improvements/](improvements/)

**...track implementation progress**
→ [implementation-plans/](implementation-plans/)

**...implement a specific feature**
→ [implementation-plans/](implementation-plans/) (find the relevant plan)

## Document Lifecycle

### Evergreen Documents (Permanent)

- **ARCHITECTURE.md** - Updated as architecture evolves
- **HOW_TO_ADD_A_PARSER.md** - Updated as parser patterns change
- **TESTING_STRATEGY.md** - Updated as testing approach evolves
- **SUPPORTED_FORMATS.md** - Auto-generated, always current
- **adr/** - Immutable once written (new ADRs added as needed)
- **improvements/** - Documents added as beyond-parity features are implemented

### Temporary Documents (Implementation Plans)

- **implementation-plans/** - Active during porting, archived when complete
- Lifecycle: Placeholder → Planning → Active → Complete → Archived
- Once a feature is complete, relevant decisions move to ADRs

## Contributing

When adding documentation:

1. **Evergreen docs** go in `docs/` root or subdirectories (`adr/`, `improvements/`)
2. **Implementation plans** go in `docs/implementation-plans/`
3. **ADRs** are immutable - create new ADRs instead of editing old ones
4. **Beyond-parity features** get documented in `improvements/` with examples
5. **Auto-generated docs** (like `SUPPORTED_FORMATS.md`) should not be edited manually

## Maintenance

- **SUPPORTED_FORMATS.md**: Regenerate with `cargo run --bin generate-supported-formats`
- **Implementation plans**: Update status as work progresses
- **ADRs**: Add new ADRs for significant design decisions
- **Improvements**: Document beyond-parity features as they're implemented
