# Caching & Incremental Scanning Implementation Plan

> **Status**: 🔴 Placeholder - Not Started
> **Priority**: P2 - Medium Priority (Performance Feature)
> **Estimated Effort**: 2-3 weeks
> **Dependencies**: LICENSE_DETECTION_PLAN.md (license index caching)

## Overview

Persistent caching of scan results and license index to speed up repeated scans and enable incremental scanning (only scan changed files).

## Scope

### What This Covers

- **License Index Caching**: Persistent cache of compiled license index
- **Scan Result Caching**: Cache scan results per file (by hash)
- **Incremental Scanning**: Only scan files that changed since last scan
- **Cache Invalidation**: Detect when cache is stale (code/data changes)
- **Multi-Process Safety**: Lock-based cache access for parallel scans

### What This Doesn't Cover

- Distributed caching (e.g., Redis, shared cache across machines)
- Cache compression (future optimization)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/cache.py`, `packagedcode/cache.py`

**Key Features**:

- **Pickle-based Caching**: Serializes Python objects to disk
- **Lock Files**: Multi-process synchronization
- **Automatic Rebuild**: Detects code/data changes and rebuilds cache
- **Configurable Location**: Environment variable for cache directory

## Current State in Rust

### Implemented

- ✅ SPDX license data embedded at compile time (no runtime loading)

### Missing

- ❌ License index caching
- ❌ Scan result caching
- ❌ Incremental scanning
- ❌ Cache invalidation logic
- ❌ Multi-process cache locking

## Architecture Considerations

### Design Questions

1. **Serialization Format**: Bincode, MessagePack, or JSON?
2. **Cache Location**: XDG cache directory, project-local, or configurable?
3. **Cache Key**: File hash (SHA256), path, or both?
4. **Invalidation Strategy**: Timestamp-based, hash-based, or version-based?

### Integration Points

- Scanner: Check cache before scanning each file
- License detection: Load cached license index
- CLI: Add `--no-cache` and `--cache-dir` options

## Implementation Phases (TBD)

1. **Phase 1**: License index caching (prerequisite for license detection)
2. **Phase 2**: Scan result caching infrastructure
3. **Phase 3**: Cache invalidation logic
4. **Phase 4**: Incremental scanning
5. **Phase 5**: Multi-process locking
6. **Phase 6**: CLI integration

## Success Criteria

- [ ] License index loads from cache (faster startup)
- [ ] Scan results cached per file
- [ ] Incremental scans only process changed files
- [ ] Cache invalidates correctly on code/data changes
- [ ] Multi-process scans don't corrupt cache
- [ ] Performance: 10x+ speedup on repeated scans

## Related Documents

- **Implementation**: `LICENSE_DETECTION_PLAN.md` (license index caching)
- **Evergreen**: `ARCHITECTURE.md` (scanner pipeline)

## Notes

- Caching is critical for large codebases (thousands of files)
- Incremental scanning enables CI/CD integration (scan only changed files)
- Consider using existing Rust crates:
  - `bincode` for serialization
  - `fs2` for file locking
  - `dirs` for XDG cache directory
- Cache invalidation is the hard part (must detect code/data changes reliably)
