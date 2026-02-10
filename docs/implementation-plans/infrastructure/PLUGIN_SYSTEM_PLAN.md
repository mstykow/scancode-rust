# Plugin System Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P3 - Low Priority (Extensibility Feature)
> **Estimated Effort**: 3-4 weeks
> **Dependencies**: None (infrastructure feature)

## Overview

Extensible plugin architecture allowing third-party plugins for custom parsers, output formats, and post-processing logic. Enables users to extend scancode-rust without modifying core code.

## Scope

### What This Covers

- Plugin discovery and loading mechanism
- Plugin trait definitions (PreScan, Scan, PostScan, Output)
- Plugin registration system
- Plugin lifecycle management
- Plugin configuration and CLI options
- Plugin dependencies and ordering

### What This Doesn't Cover

- Specific plugin implementations (those are separate features)
- Plugin marketplace or distribution (future consideration)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/plugincode/`

**Key Concepts**:

- **Entry Points**: Uses setuptools entry points for plugin discovery
- **Plugin Base Classes**: PreScanPlugin, ScanPlugin, PostScanPlugin, OutputPlugin
- **Plugin Manager**: Central plugin discovery and lifecycle management
- **Plugin Options**: CLI option registration per plugin
- **Plugin Ordering**: sort_order attribute for execution sequencing

## Current State in Rust

### Implemented

- ✅ Trait-based parser system (PackageParser trait)
- ✅ Static parser registration (register_package_handlers! macro)

### Missing

- ❌ Dynamic plugin loading
- ❌ Plugin discovery mechanism
- ❌ Plugin trait hierarchy (PreScan, Scan, PostScan, Output)
- ❌ Plugin manager
- ❌ Plugin configuration system
- ❌ Plugin CLI option registration

## Architecture Considerations

### Design Questions

1. **Plugin Loading**: Dynamic library loading (.so/.dylib/.dll) or compile-time only?
2. **Plugin Discovery**: Cargo features, separate crates, or both?
3. **ABI Stability**: Use stable ABI (e.g., C FFI) or Rust-only plugins?
4. **Security**: Sandboxing, capability-based security, or trust-based?

### Rust-Specific Challenges

- No stable ABI for Rust (unlike Python's C API)
- Dynamic library loading requires careful version management
- Trait objects have limitations (no generic methods)

### Possible Approaches

1. **Compile-Time Plugins**: Plugins as Cargo features/dependencies (safest, no runtime loading)
2. **Dynamic Loading with C FFI**: Stable ABI via C interface (complex, but flexible)
3. **WebAssembly Plugins**: WASM-based plugins (sandboxed, portable, but limited)
4. **Hybrid**: Core plugins compile-time, optional plugins dynamic

## Implementation Phases (TBD)

1. **Phase 1**: Define plugin trait hierarchy
2. **Phase 2**: Implement plugin manager
3. **Phase 3**: Add plugin discovery mechanism
4. **Phase 4**: Implement plugin lifecycle management
5. **Phase 5**: Add plugin configuration system
6. **Phase 6**: Integrate with CLI

## Success Criteria

- [ ] Third-party plugins can be loaded
- [ ] Plugins can register CLI options
- [ ] Plugins can extend scanner functionality
- [ ] Plugin system is well-documented
- [ ] Example plugins demonstrate capabilities

## Related Documents

- **Evergreen**: `ARCHITECTURE.md` (plugin architecture)
- **ADR**: TBD - Plugin loading strategy

## Notes

- Plugin system is a "nice-to-have" not a "must-have"
- Rust's lack of stable ABI makes this more complex than Python
- Consider starting with compile-time plugins (Cargo features) for simplicity
- Dynamic loading can be added later if needed
- WebAssembly plugins are an interesting future direction
