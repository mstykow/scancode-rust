# Output Formats Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P1 - High Priority (User-Facing Feature)
> **Estimated Effort**: 4-6 weeks
> **Dependencies**: None (can be implemented independently)

## Overview

Support multiple output formats beyond JSON for different use cases: SPDX (compliance), CycloneDX (SBOM), CSV (spreadsheet analysis), YAML (human-readable), HTML (reports).

## Scope

### What This Covers

- **SPDX Tag-Value** - SPDX 2.3 standard format
- **SPDX RDF/XML** - SPDX RDF format
- **CycloneDX JSON** - CycloneDX 1.4+ SBOM format (JSON)
- **CycloneDX XML** - CycloneDX 1.4+ SBOM format (XML)
- **CSV** - Tabular format for spreadsheet analysis
- **YAML** - Human-readable structured format
- **HTML** - Static HTML report
- **HTML App** - Interactive HTML application
- **JSON Lines** - Streaming format (one JSON object per line)
- **Custom Templates** - Jinja2-style template rendering

### What This Doesn't Cover

- JSON output (already implemented)
- License policy evaluation (separate feature)
- Output filtering (e.g., only-findings) - separate feature

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/formattedcode/`

**Key Files**:

- `output_spdx.py` - SPDX Tag-Value and RDF/XML
- `output_cyclonedx.py` - CycloneDX JSON and XML
- `output_csv.py` - CSV tabular format
- `output_yaml.py` - YAML format
- `output_html.py` - HTML report
- `output_jsonlines.py` - JSON Lines format
- `output_json.py` - JSON format (reference)

## Current State in Rust

### Implemented

- ✅ JSON output (compact and pretty-printed)
- ✅ Basic output structure

### Missing

- ❌ SPDX Tag-Value format
- ❌ SPDX RDF/XML format
- ❌ CycloneDX JSON format
- ❌ CycloneDX XML format
- ❌ CSV format
- ❌ YAML format
- ❌ HTML report
- ❌ HTML app
- ❌ JSON Lines format
- ❌ Custom template rendering

## Architecture Considerations

### Design Questions

1. **Output Plugin System**: Trait-based like parsers or separate module?
2. **Template Engine**: Use existing Rust template crate (e.g., Tera, Handlebars) or custom?
3. **SPDX/CycloneDX**: Use existing Rust crates or implement from scratch?
4. **CLI Interface**: `--format` flag or separate flags per format?

### Integration Points

- CLI: Add `--format` option with format selection
- Output module: Create `OutputFormatter` trait
- Scanner: Route results to selected formatter

## Implementation Phases (TBD)

1. **Phase 1**: Output formatter trait and infrastructure
2. **Phase 2**: SPDX Tag-Value (most requested compliance format)
3. **Phase 3**: CycloneDX JSON (SBOM standard)
4. **Phase 4**: CSV (spreadsheet analysis)
5. **Phase 5**: YAML (human-readable)
6. **Phase 6**: JSON Lines (streaming)
7. **Phase 7**: HTML report (static)
8. **Phase 8**: SPDX RDF/XML (less common)
9. **Phase 9**: CycloneDX XML (less common)
10. **Phase 10**: HTML app (interactive)
11. **Phase 11**: Custom templates (advanced)

## Success Criteria

- [ ] All formats generate valid output per spec
- [ ] SPDX output validates with SPDX tools
- [ ] CycloneDX output validates with CycloneDX tools
- [ ] CSV opens correctly in Excel/LibreOffice
- [ ] HTML report renders correctly in browsers
- [ ] Golden tests pass for all formats

## Related Documents

- **Evergreen**: `ARCHITECTURE.md` (output pipeline)
- **Implementation**: All other plans (provide data for output)

## Notes

- SPDX and CycloneDX are critical for compliance and SBOM use cases
- Consider using existing Rust crates:
  - `spdx` crate for SPDX format
  - `cyclonedx-bom` crate for CycloneDX
  - `csv` crate for CSV
  - `serde_yaml` for YAML
  - `tera` or `handlebars` for templates
- HTML app requires embedding JavaScript/CSS assets
