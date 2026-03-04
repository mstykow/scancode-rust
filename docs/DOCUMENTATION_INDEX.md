# Documentation Index

This index helps you find the right documentation for your needs.

## For Users

- **[README.md](../README.md)** - Installation, usage, and quick start
- **[SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md)** - List of all supported package formats

## For Contributors

### Getting Started

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and components
- **[HOW_TO_ADD_A_PARSER.md](HOW_TO_ADD_A_PARSER.md)** - Step-by-step parser implementation guide
- **[TESTING_STRATEGY.md](TESTING_STRATEGY.md)** - Four-layer testing approach

### Design Decisions

- **[adr/](adr/)** - Architectural Decision Records

### Beyond-Parity Features

- **[improvements/](improvements/)** - Features where Rust exceeds Python

## For Maintainers

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
│
└── improvements/                      # Evergreen: Beyond-parity features
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

**...track implementation quality and behavior**
→ [TESTING_STRATEGY.md](TESTING_STRATEGY.md)

**...configure cache behavior and controls**
→ [README.md](../README.md) and [implementation-plans/infrastructure/CACHING_PLAN.md](implementation-plans/infrastructure/CACHING_PLAN.md)

**...implement a specific feature**
→ [ARCHITECTURE.md](ARCHITECTURE.md) and [HOW_TO_ADD_A_PARSER.md](HOW_TO_ADD_A_PARSER.md)

## Document Lifecycle

### Evergreen Documents (Permanent)

- **ARCHITECTURE.md** - Updated as architecture evolves
- **HOW_TO_ADD_A_PARSER.md** - Updated as parser patterns change
- **TESTING_STRATEGY.md** - Updated as testing approach evolves
- **SUPPORTED_FORMATS.md** - Auto-generated, always current
- **adr/** - Immutable once written (new ADRs added as needed)
- **improvements/** - Documents added as beyond-parity features are implemented

## Contributing

When adding documentation:

1. **Evergreen docs** go in `docs/` root or subdirectories (`adr/`, `improvements/`)
2. **ADRs** are immutable - create new ADRs instead of editing old ones
3. **Beyond-parity features** get documented in `improvements/` with examples
4. **Auto-generated docs** (like `SUPPORTED_FORMATS.md`) should not be edited manually

## Maintenance

- **SUPPORTED_FORMATS.md**: Regenerate with `cargo run --bin generate-supported-formats`
- **ADRs**: Add new ADRs for significant design decisions
- **Improvements**: Document beyond-parity features as they're implemented
