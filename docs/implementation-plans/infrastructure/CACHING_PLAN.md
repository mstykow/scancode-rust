# Caching & Incremental Scanning Implementation Plan

> **Status**: 🟡 Planning Complete — Ready for Implementation
> **Priority**: P2 - Medium Priority (Performance Feature)
> **Estimated Effort**: 2-3 weeks
> **Dependencies**: License detection (for license index caching benefits)

## Table of Contents

- [Overview](#overview)
- [Python Reference Analysis](#python-reference-analysis)
- [Rust Architecture Design](#rust-architecture-design)
- [Implementation Phases](#implementation-phases)
- [Beyond-Parity Improvements](#beyond-parity-improvements)
- [Testing Strategy](#testing-strategy)
- [Success Criteria](#success-criteria)

---

## Overview

Persistent caching of scan results and compiled data structures to speed up repeated scans. The caching system has two independent layers:

1. **Index Caching**: Persistent cache of the compiled license index (expensive to build from source text)
2. **Scan Result Caching**: Per-file cache of scan results keyed by content hash (the major performance win)

### Critical Finding: Python Has No Scan Result Caching

**Python ScanCode does NOT cache per-file scan results.** It only caches the license index and package pattern index (compiled data structures). There is no mechanism to skip re-scanning unchanged files. This means **scan result caching and incremental scanning are entirely beyond-parity features** — Rust will be the first ScanCode implementation to support them.

### Scope

**In Scope:**

- **License Index Caching**: Persistent cache of compiled askalono `Store` (currently rebuilt from embedded SPDX text on each run, taking 200-300ms)
- **Scan Result Caching**: Cache `FileInfo` results per file keyed by SHA256 content hash
- **Incremental Scanning**: Only scan files that changed since last scan (mtime + content hash check)
- **Cache Invalidation**: Version-stamped caches with tool version + data version embedded in cache metadata
- **Multi-Process Safety**: File locking for cache writes (parallel scans on same codebase)
- **Cache Management CLI**: `--no-cache`, `--cache-dir`, `--cache-clear` flags
- **Configurable Cache Location**: XDG cache directory by default, overridable via environment variable and CLI flag

**Out of Scope:**

- Distributed caching (Redis, shared network cache)
- Cache compression beyond what's inherited from askalono (zstd)
- Cache size limits / eviction policies (deferred — disk is cheap)

### Current State in Rust

**Implemented:**

- ✅ SPDX license data embedded at compile time via `include_dir!()` macro
- ✅ Askalono `Store` with existing `from_cache()`/`to_cache()` methods (MessagePack + zstd, version header)
- ✅ SHA256 hash computation per file in `process_file()` (already available as cache key)
- ✅ `FileInfo` struct with all scannable fields (package_data, license_detections, copyrights, etc.)

**Missing:**

- ❌ Persistent license index cache (Store is rebuilt from text each run)
- ❌ Scan result cache infrastructure
- ❌ Incremental scanning logic
- ❌ Cache invalidation
- ❌ Multi-process file locking
- ❌ CLI flags for cache control
- ❌ XDG cache directory support

---

## Python Reference Analysis

### Architecture Overview

Python ScanCode's caching spans 4 files:

| File | Lines | Purpose |
|------|-------|---------|
| `licensedcode/cache.py` | 567 | License index caching: `LicenseCache` class, pickle serialization, file locking |
| `packagedcode/cache.py` | 278 | Package pattern caching: `PkgManifestPatternsCache`, regex patterns, pickle |
| `scancode_config.py` | 223 | Cache directory configuration, environment variables, version detection |
| `scancode/lockfile.py` | 34 | File locking wrapper around `fasteners.InterProcessLock` |

Total: ~1,102 lines.

### What Python Caches

#### 1. License Index Cache (`LicenseCache`)

The most expensive data structure to build — a compiled `LicenseIndex` used for license text matching.

**Cached objects:**

- `db`: Mapping of License objects by key (the full license database)
- `index`: Compiled `LicenseIndex` (the search index)
- `licensing`: `license_expression.Licensing` object (expression parser)
- `spdx_symbols`: Mapping of SPDX keys to license symbols
- `unknown_spdx_symbol`: Fallback symbol for unknown SPDX keys
- `additional_license_directory`/`additional_license_plugins`: Custom license sources

**Lifecycle:**

```text
get_cache() → populate_cache() → LicenseCache.load_or_build()
  ├── Cache exists + not force? → load_cache_file() [pickle.load]
  └── Cache missing/corrupt/force?
      └── Lock file (6 min timeout)
          → Build license index from text files
          → Build SPDX symbols, licensing objects
          → Dump to pickle file
```

#### 2. Package Pattern Cache (`PkgManifestPatternsCache`)

Compiled regex patterns for matching file paths to package handlers.

**Cached objects:**

- `handler_by_regex`: Mapping from regex pattern to datasource ID(s)
- `system_package_matcher`: Compiled multiregex for system packages
- `application_package_matcher`: Compiled multiregex for app packages
- `all_package_matcher`: Combined matcher

### Cache Directory Structure

```text
~/.cache/scancode-tk/<version>/          # scancode_cache_dir
├── scancode_license_index_lockfile      # Lock file for license index
├── scancode_package_index_lockfile      # Lock file for package index
└── scancode-version-check.json          # Version check state

<src>/licensedcode/data/cache/           # licensedcode_cache_dir
└── license_index/
    └── index_cache                      # Pickled LicenseCache

<src>/packagedcode/data/cache/           # packagedcode_cache_dir
└── package_patterns_index/
    └── index_cache                      # Pickled PkgManifestPatternsCache
```

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `SCANCODE_CACHE` | `~/.cache/scancode-tk/<version>` | General cache directory (lock files, version check) |
| `SCANCODE_LICENSE_INDEX_CACHE` | `<src>/licensedcode/data/cache` | License index cache location |
| `SCANCODE_PACKAGE_INDEX_CACHE` | `<src>/packagedcode/data/cache` | Package pattern cache location |
| `SCANCODE_TEMP` | System temp dir | Temporary files directory |

### Serialization Format

**Python uses pickle protocol 4** (binary, Python-specific). Key characteristics:

- Fast for Python objects, but not portable
- No schema evolution — version changes require rebuild
- Vulnerable to arbitrary code execution on load (security concern)
- Typical license index size: ~50-100MB pickled

### File Locking

**Python wraps `fasteners.InterProcessLock`** with a timeout:

```python
class FileLock(fasteners.InterProcessLock):
    @contextmanager
    def locked(self, timeout):
        acquired = self.acquire(timeout=timeout)
        if not acquired:
            raise LockTimeout(timeout)
        try:
            yield
        finally:
            self.release()
```

- License index lock timeout: **6 minutes** (building the index is slow)
- Package pattern lock timeout: **1 minute**

### Cache Invalidation Strategy

Python's invalidation is **minimal**:

- No automatic content-based invalidation
- No version stamp in cache file itself
- Cache is per-version directory (`~/.cache/scancode-tk/<version>/`)
- Force rebuild via `--reindex-licenses` CLI command
- Corrupt cache detected via pickle load failure → automatic rebuild

### Known Issues in Python

1. **No scan result caching**: Every file is re-scanned on every run
2. **No incremental scanning**: No way to skip unchanged files
3. **Global mutable singleton**: `_LICENSE_CACHE = None` is not thread-safe
4. **Pickle security**: `pickle.load()` can execute arbitrary code on corrupted/malicious cache files
5. **Silent error swallowing**: `except Exception` catches all errors during cache load with only a print statement
6. **No cache size management**: Cache grows without bounds
7. **Lock timeout is generous**: 6-minute timeout for license index is needed because Python builds it slowly
8. **Version-directory isolation**: Each version gets its own cache directory, but old versions are never cleaned up

---

## Rust Architecture Design

### Design Philosophy

1. **Content-addressed scan result cache** — the major beyond-parity win
2. **Version-stamped index caches** — embed tool version + data version in cache metadata
3. **Leverage existing askalono cache infrastructure** — already uses MessagePack + zstd
4. **XDG-compliant cache location** — platform-native defaults, overridable
5. **Thread-safe by design** — no global mutable state, file locking for multi-process
6. **Safe serialization** — `postcard` or `rmp-serde`, never pickle-equivalent

### High-Level Architecture

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                        Caching Architecture                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────┐     ┌──────────────────────────────────────┐  │
│  │ License Index Cache  │     │ Scan Result Cache                    │  │
│  │                      │     │                                      │  │
│  │ • askalono Store     │     │ • Per-file FileInfo results          │  │
│  │ • MsgPack + zstd     │     │ • Keyed by SHA256 content hash      │  │
│  │ • Version-stamped    │     │ • Version-stamped metadata           │  │
│  │ • Built once, loaded │     │ • Written during scan, read on       │  │
│  │   on subsequent runs │     │   subsequent scans                   │  │
│  │ • ~3-5 MB compressed │     │ • Sharded by hash prefix             │  │
│  └─────────────────────┘     └──────────────────────────────────────┘  │
│                                                                          │
│  ┌─────────────────────┐     ┌──────────────────────────────────────┐  │
│  │ Cache Manager        │     │ File Locking                         │  │
│  │                      │     │                                      │  │
│  │ • XDG cache dir      │     │ • fd-lock (RwLock pattern)           │  │
│  │ • Env var override   │     │ • Shared read, exclusive write       │  │
│  │ • CLI flag override  │     │ • Timeout on lock acquisition        │  │
│  │ • Cache clear/reset  │     │ • Atomic file writes (temp + rename) │  │
│  │ • Version management │     │                                      │  │
│  └─────────────────────┘     └──────────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Cache Directory Layout

```text
~/.cache/scancode-rust/                    # XDG cache dir (or SCANCODE_RUST_CACHE env var)
├── metadata.json                          # Cache version, tool version, timestamps
├── license-index/
│   ├── store.bin.zstd                     # Cached askalono Store (MsgPack + zstd)
│   └── store.lock                         # Lock file for index rebuild
├── scans/
│   ├── ab/
│   │   ├── ab3f...a1c2.postcard           # Cached FileInfo for file with that SHA256
│   │   └── ab91...f3d0.postcard           # (sharded by first 2 hex chars)
│   ├── cd/
│   │   └── cd12...8e9f.postcard
│   └── ...
└── scans.lock                             # Lock file for scan cache writes
```

**Sharding rationale**: With 100K+ cached files, flat directories become slow on some filesystems. Two-character hex prefix = 256 subdirectories, each holding ~400 files for a 100K-file codebase.

### Core Data Types

```rust
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Top-level cache metadata (stored as JSON for human readability)
#[derive(Serialize, Deserialize, Debug)]
pub struct CacheMetadata {
    /// scancode-rust version that created this cache
    pub tool_version: String,
    /// SPDX license data version
    pub spdx_version: String,
    /// Timestamp of cache creation
    pub created_at: u64,
    /// Number of cached scan results
    pub scan_count: usize,
}

/// Cached scan result for a single file (content-addressed)
#[derive(Serialize, Deserialize, Debug)]
pub struct CachedScanResult {
    /// Cache format version (for schema evolution)
    pub cache_version: u32,
    /// Tool version that produced this result
    pub tool_version: String,
    /// SHA256 of the file content (redundant with key, for verification)
    pub content_hash: String,
    /// The actual scan result (package data, licenses, copyrights, etc.)
    /// Note: path-dependent fields (name, path, etc.) are NOT cached —
    /// they're reconstructed from the file's current path.
    pub package_data: Vec<PackageData>,
    pub license_expression: Option<String>,
    pub license_detections: Vec<LicenseDetection>,
    pub copyrights: Vec<Copyright>,
    pub programming_language: Option<String>,
}

/// Configuration for caching behavior
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache root directory
    pub cache_dir: PathBuf,
    /// Whether caching is enabled
    pub enabled: bool,
    /// Whether to force rebuild of all caches
    pub force_rebuild: bool,
}

/// The cache manager — thread-safe, created once per scan run
pub struct CacheManager {
    config: CacheConfig,
    metadata: CacheMetadata,
}
```

### Key Design Decisions

#### 1. Serialization: `rmp-serde` (MessagePack) for Scan Results

**Decision**: Use `rmp-serde` (MessagePack) rather than `postcard` or `bincode`.

**Rationale**:

- Askalono already uses `rmp-serde` for its Store cache — one less dependency
- MessagePack is well-specified and portable (unlike pickle)
- Schema evolution is easier with MessagePack (tolerates extra fields)
- `bincode` is unmaintained as of 2025
- `postcard` is slightly faster but less mature for schema evolution
- Performance difference is negligible for our use case (I/O-bound, not serialization-bound)

#### 2. Cache Location: XDG via `dirs` Crate

**Decision**: Use `dirs::cache_dir()` for platform-native defaults.

**Rationale**:

- Linux: `~/.cache/scancode-rust/`
- macOS: `~/Library/Caches/scancode-rust/`
- Windows: `{FOLDERID_LocalAppData}/scancode-rust/`
- Overridable via `SCANCODE_RUST_CACHE` env var and `--cache-dir` CLI flag

#### 3. Cache Key: SHA256 Content Hash (Already Computed)

**Decision**: Use file content SHA256 as cache key.

**Rationale**:

- The scanner already computes SHA256 for every file in `process_file()` (line 145)
- Content-addressed: same file content always hits same cache entry regardless of path
- Enables cross-project cache sharing (a common `LICENSE` file is scanned once, ever)
- No false cache hits — hash collision probability is negligible

#### 4. File Locking: `fd-lock` Crate

**Decision**: Use `fd-lock` for multi-process cache safety.

**Rationale**:

- Modern Rust API (RwLock pattern: shared read, exclusive write)
- Cross-platform (Unix + Windows)
- Used by production tools (Solana, Spin framework)
- Replaces Python's `fasteners` library

#### 5. Atomic Writes (Crash Safety)

**Decision**: Write cache files via temp file + rename.

**Rationale**:

- `rename()` is atomic on POSIX — no corrupt cache files on crash
- Write to `<hash>.postcard.tmp` → rename to `<hash>.postcard`
- If process crashes mid-write, temp file is orphaned (harmless)

#### 6. What to Cache vs. What to Reconstruct

**Cached** (content-dependent, expensive to compute):

- Package data (parser results)
- License detections (askalono matching)
- Copyright detections
- Programming language

**Reconstructed** (path-dependent, cheap to compute):

- File name, base name, extension, path
- File type (file vs directory)
- MIME type
- Size, date
- SHA1, MD5, SHA256 (needed to look up cache anyway)

### Module Structure

```text
src/
├── cache/
│   ├── mod.rs              # Public API: CacheManager
│   ├── config.rs           # CacheConfig, CLI flag integration
│   ├── index_cache.rs      # License index (askalono Store) caching
│   ├── scan_cache.rs       # Per-file scan result caching
│   ├── metadata.rs         # CacheMetadata, version management
│   └── locking.rs          # File locking wrappers
├── cache_test.rs           # Unit tests
```

---

## Implementation Phases

### Phase 1: Cache Infrastructure (2-3 days)

**Goal**: Establish cache directory management, configuration, and metadata.

**Deliverables:**

1. `config.rs`: `CacheConfig` struct, XDG directory resolution, env var support
2. `metadata.rs`: `CacheMetadata`, version stamping, JSON serialization
3. `mod.rs`: `CacheManager::new()`, directory creation, metadata load/save
4. CLI integration: Add `--no-cache`, `--cache-dir <PATH>`, `--cache-clear` to `cli.rs`

**Dependencies**: `dirs` crate (XDG directory), `serde_json` (metadata file)

**Testing**: Directory creation, env var override, metadata read/write, CLI flag parsing.

### Phase 2: License Index Caching (2-3 days)

**Goal**: Cache the compiled askalono `Store` on disk to eliminate 200-300ms startup cost.

**Deliverables:**

1. `index_cache.rs`: `load_or_build_license_store()` — check cache → load or build → save
2. Leverage existing `Store::from_cache()`/`Store::to_cache()` methods
3. Version-stamp cache with tool version + SPDX data version
4. Invalidation: rebuild if version mismatch or cache corrupt/missing

**Integration**: Replace the `create_store_from_texts()` call in `main.rs` with `load_or_build_license_store()`.

**Expected speedup**: 200-300ms → 20-50ms (10x faster startup).

**Testing**: Cache hit/miss, version mismatch invalidation, corrupt cache recovery.

### Phase 3: Scan Result Cache — Write Path (2-3 days)

**Goal**: Cache per-file scan results during scanning.

**Deliverables:**

1. `scan_cache.rs`: `CachedScanResult` struct, serialization, sharded directory structure
2. `locking.rs`: File locking wrappers using `fd-lock`
3. Write path: After `process_file()` completes, cache the result keyed by SHA256
4. Atomic writes: temp file + rename pattern

**Dependencies**: `rmp-serde` (already present), `fd-lock` (new)

**Integration**: Add cache write call at end of `process_file()` in `scanner/process.rs`.

**Testing**: Write + read roundtrip, atomic write crash safety, concurrent writes.

### Phase 4: Scan Result Cache — Read Path (2-3 days)

**Goal**: Check cache before scanning each file, skip scan on cache hit.

**Deliverables:**

1. Read path: Before scanning a file, compute SHA256 → check cache → return cached result if valid
2. Version validation: Verify cached result was produced by same tool version
3. Cache miss handling: Fall through to normal scanning pipeline
4. Progress bar integration: Show cache hit/miss statistics

**Integration**: Add cache lookup at start of `process_file()` in `scanner/process.rs`.

**Expected speedup**: On repeated scan of unchanged codebase: 10-50x faster (I/O only, no parsing/matching).

**Testing**: Cache hit path, cache miss path, version mismatch, corrupt entry handling.

### Phase 5: Incremental Scanning (2-3 days)

**Goal**: Only scan files that changed since last scan of the same directory.

**Deliverables:**

1. Scan manifest: Save a manifest of `{path: (mtime, size, sha256)}` after each scan
2. Incremental mode: On subsequent scan, compare file metadata against manifest
3. Fast-path: If mtime + size unchanged, assume file unchanged (skip SHA256 computation)
4. Slow-path: If mtime/size changed, compute SHA256 and check scan result cache
5. CLI flag: `--incremental` to enable incremental mode

**Scan manifest location**: `<scan-output-dir>/.scancode-rust-cache/manifest.json`

**Integration**: Add incremental check in file discovery phase (`scanner/count.rs` or `scanner/process.rs`).

**Testing**: Changed file detection, new file detection, deleted file handling, manifest load/save.

### Phase 6: Polish and Benchmarks (1-2 days)

**Goal**: Cache management, statistics, documentation, and performance validation.

**Deliverables:**

1. `--cache-clear` implementation: Delete all cached data
2. Cache statistics: Report hit/miss ratio after scan
3. Cache size reporting: Show total cache disk usage
4. Performance benchmarks: Measure speedup on real codebases
5. Documentation updates

---

## Beyond-Parity Improvements

### 1. Scan Result Caching (Major Feature — Python Has None)

**Python**: Re-scans every file on every run. No per-file caching.
**Rust**: Content-addressed scan result cache. Same file scanned once, ever (across all projects).

**Impact**: 10-50x speedup on repeated scans of large codebases.

### 2. Incremental Scanning (Major Feature — Python Has None)

**Python**: No incremental scanning support.
**Rust**: Only scan files with changed mtime/size/content since last scan.

**Impact**: CI/CD integration — scan only changed files in each commit.

### 3. Cross-Project Cache Sharing (Enhancement)

**Python**: N/A (no scan result caching).
**Rust**: Content-addressed cache means the same `LICENSE` file is scanned once regardless of which project it appears in. Common files like `MIT License`, `Apache 2.0`, etc. benefit enormously.

### 4. Safe Serialization (Security Fix)

**Python**: Uses `pickle` — vulnerable to arbitrary code execution on malicious cache files.
**Rust**: Uses `rmp-serde` (MessagePack) — data-only format, no code execution possible.

### 5. Thread-Safe Cache Access (Bug Fix)

**Python**: Global mutable singleton `_LICENSE_CACHE = None`, not thread-safe.
**Rust**: `CacheManager` is `Send + Sync`, file locking for multi-process safety.

### 6. Proper Error Handling (Bug Fix)

**Python**: `except Exception: print(...)` silently swallows cache load errors.
**Rust**: `Result<T, E>` with proper error propagation, `log::warn!` for non-fatal cache errors.

### 7. Faster Lock Timeout (Performance)

**Python**: 6-minute lock timeout for license index (because building is slow in Python).
**Rust**: 30-second lock timeout (Rust builds the index 10x faster).

---

## Testing Strategy

### Unit Tests (`cache_test.rs`)

1. **Cache directory**: XDG resolution, env var override, CLI flag override
2. **Metadata**: Version stamping, JSON read/write, version mismatch detection
3. **License index cache**: Load/save roundtrip, version invalidation, corrupt cache recovery
4. **Scan result cache**: Write/read roundtrip, sharded directory structure, hash-based lookup
5. **Atomic writes**: Crash safety (temp file left behind), concurrent writes
6. **File locking**: Shared read, exclusive write, timeout behavior
7. **Incremental scanning**: Changed file detection, new file, deleted file, manifest persistence

### Integration Tests

1. **Full scan with caching**: Scan directory → verify cache populated → re-scan → verify cache hits
2. **Incremental scan**: Scan → modify one file → re-scan → verify only modified file re-scanned
3. **Cache invalidation**: Upgrade tool version → verify cache rebuilt
4. **Cross-process safety**: Two concurrent scans on same directory → no corruption

### Performance Benchmarks

| Scenario | Baseline (no cache) | Expected (with cache) | Speedup |
|----------|--------------------|-----------------------|---------|
| License index load | 200-300ms | 20-50ms | 5-10x |
| Full scan (1000 files) | 30-60s | 30-60s (first run) | 1x |
| Repeated scan (1000 files, unchanged) | 30-60s | 2-5s | 10-20x |
| Incremental scan (1000 files, 10 changed) | 30-60s | 1-3s | 20-50x |

---

## Success Criteria

- [ ] License index loads from cache (5-10x faster startup)
- [ ] Scan results cached per file by SHA256 content hash
- [ ] Repeated scans of unchanged files skip scanning (10-20x speedup)
- [ ] Incremental scans only process changed files
- [ ] Cache invalidates correctly on tool version change
- [ ] Corrupt cache entries are detected and rebuilt (never crash)
- [ ] Multi-process scans don't corrupt cache (file locking)
- [ ] `--no-cache`, `--cache-dir`, `--cache-clear` CLI flags work
- [ ] `SCANCODE_RUST_CACHE` environment variable overrides cache location
- [ ] Cross-project cache sharing works (same file content → same cache entry)
- [ ] Cache directory follows XDG standard (Linux: `~/.cache/`, macOS: `~/Library/Caches/`)
- [ ] Atomic writes prevent corrupt cache files on crash
- [ ] `cargo clippy` clean, `cargo fmt` clean
- [ ] Comprehensive test coverage

---

## Dependency Summary

| Crate | Version | Purpose | Status |
|-------|---------|---------|--------|
| `rmp-serde` | 1.3 | MessagePack serialization (already in Cargo.toml for askalono) | ✅ Existing |
| `zstd` | 0.13 | Compression for license index cache (already in Cargo.toml) | ✅ Existing |
| `sha2` | 0.10 | SHA256 hashing (already used for file hashing) | ✅ Existing |
| `dirs` | 5.0 | XDG cache directory resolution | 🆕 New |
| `fd-lock` | 4.0 | File locking for multi-process safety | 🆕 New |

Only 2 new dependencies needed — both small, well-maintained, and widely used.

---

## Related Documents

- **Architecture**: [`docs/ARCHITECTURE.md`](../../ARCHITECTURE.md) — Scanner pipeline, caching section
- **License Detection**: [`LICENSE_DETECTION_PLAN.md`](../text-detection/LICENSE_DETECTION_PLAN.md) — License index is primary cache beneficiary
- **Testing Strategy**: [`docs/TESTING_STRATEGY.md`](../../TESTING_STRATEGY.md) — Testing approach
- **Python Reference**: `reference/scancode-toolkit/src/licensedcode/cache.py` — License cache implementation
- **Python Reference**: `reference/scancode-toolkit/src/packagedcode/cache.py` — Package pattern cache
- **Python Reference**: `reference/scancode-toolkit/src/scancode_config.py` — Cache directory configuration

---

## Appendix: Python File Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `licensedcode/cache.py` | 567 | License index caching: LicenseCache class, pickle serialization, build/load lifecycle, SPDX symbol building |
| `packagedcode/cache.py` | 278 | Package pattern caching: PkgManifestPatternsCache, multiregex pattern compilation, pickle serialization |
| `scancode_config.py` | 223 | Cache directory config, 3 env vars (SCANCODE_CACHE, SCANCODE_LICENSE_INDEX_CACHE, SCANCODE_PACKAGE_INDEX_CACHE), version detection |
| `scancode/lockfile.py` | 34 | File locking wrapper: FileLock class around fasteners.InterProcessLock with timeout |
| `licensedcode/reindex.py` | 79 | CLI command: `scancode-reindex-licenses` with `--all-languages`, `--only-builtin` flags |

## Appendix: Existing Askalono Cache Format

The askalono `Store` already has cache support (`src/askalono/store/cache.rs`):

```text
┌──────────────┬──────────────────────────────────┐
│ Header (11B) │ zstd-compressed MessagePack body  │
│ "askalono-04"│                                    │
└──────────────┴──────────────────────────────────┘
```

- **Header**: 11-byte version string (`b"askalono-04"`) for cache compatibility check
- **Body**: MessagePack-serialized `Store` struct, compressed with zstd (level 21)
- **Uncompressed size**: ~3.7 MiB
- **Compressed size**: ~1-2 MiB
- **Load time**: 20-50ms (vs 200-300ms from text)

This infrastructure will be reused directly for the license index cache layer.
