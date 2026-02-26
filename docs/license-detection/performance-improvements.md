# License Detection Performance Improvements

This document outlines performance bottlenecks identified in the license detection system and provides actionable suggestions for improvement using idiomatic Rust patterns.

## Executive Summary

The license detection system uses a multi-phase matching pipeline with:
1. Hash-based exact matching (O(1))
2. Aho-Corasick pattern matching
3. Set-similarity candidate selection
4. Sequence matching for fuzzy detection

Key performance issues identified:
- **No global caching** of loaded rules/licenses - files read from disk on every engine creation
- **Heavy cloning** of `Rule` structs during candidate selection
- **Per-file allocations** of Query objects with many HashMaps
- **String allocations** during tokenization and text processing

---

## 1. Caching and Lazy Initialization

### Current Problem

**File**: `src/license_detection/rules/loader.rs`, lines 469-549

```rust
pub fn load_rules_from_directory(path: &Path) -> Result<Vec<Rule>> {
    // Reads files from disk every time engine is created
}
```

The `LicenseDetectionEngine` is created fresh for each scan, causing:
- Reading **2,615 files** from disk
- Loading **37,572 rules**
- Re-parsing YAML frontmatter
- Re-building the entire index (Aho-Corasick automaton, token dictionaries)

### Measured Performance

| Metric | Rust | Python |
|--------|------|--------|
| Engine creation time | **~8.5 seconds** | ~60 seconds |
| Rules loaded | 37,572 | ~same |
| Files read | 2,615 | ~same |
| Memory usage | ~1.2 GB | ~similar |

Rust is already **~7x faster** than Python for engine creation. However, 8.5 seconds is still significant for CLI usage where each run is a new process.

### Engine Immutability (Important for Caching)

The engine is **immutable** - safe to share across threads and tests:

```rust
// src/license_detection/mod.rs:75-79
#[derive(Debug, Clone)]
pub struct LicenseDetectionEngine {
    index: Arc<index::LicenseIndex>,  // Arc = shared ownership
    spdx_mapping: SpdxMapping,
}

// detect takes &self, not &mut self
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>>
```

The `LicenseIndex` is never mutated after construction. All mutable patterns (`&mut self`, `mut index`) are in test code building test indices, not in the detection pipeline.

### Golden Tests Already Cache Correctly

```rust
// src/license_detection/golden_test.rs:39-55
static TEST_ENGINE: Lazy<Option<LicenseDetectionEngine>> = Lazy::new(|| {
    LicenseDetectionEngine::new(&data_path)  // Created ONCE
});

fn ensure_engine() -> Option<&'static LicenseDetectionEngine> {
    TEST_ENGINE.as_ref()  // Reused for all tests
}
```

### Suggested Solution: Global Lazy-Static Cache

For CLI and library usage, add a global cache:

```rust
// src/license_detection/mod.rs

use std::sync::LazyLock;
use parking_lot::RwLock;

static LICENSE_INDEX: LazyLock<RwLock<Option<Arc<LicenseIndex>>>> = LazyLock::new(|| {
    RwLock::new(None)
});

impl LicenseDetectionEngine {
    pub fn new(rules_path: &Path) -> Result<Self> {
        // Check cache first
        {
            let cache = LICENSE_INDEX.read();
            if let Some(ref index) = *cache {
                return Ok(Self {
                    index: Arc::clone(index),
                    spdx_mapping: build_spdx_mapping(&index),
                });
            }
        }
        
        // Build new index
        let rules = load_rules_from_directory(&rules_dir, false)?;
        let licenses = load_licenses_from_directory(&licenses_dir, false)?;
        let index = Arc::new(build_index(rules, licenses));
        
        // Cache it
        *LICENSE_INDEX.write() = Some(Arc::clone(&index));
        
        Ok(Self { index, spdx_mapping })
    }
}
```

**Benefits**:
- Zero-cost engine creation after first initialization
- Memory shared across all scans in same process
- Thread-safe with `parking_lot::RwLock`
- No test isolation issues (engine is immutable)

### How to Measure

```bash
# Current: Each run takes ~8.5s for engine creation
time target/release/scancode-rust test_scan/ --license-rules-path reference/scancode-toolkit/src/licensedcode/data -o /dev/null
# Elapsed: 8.5 seconds

# After caching: Library users would see instant subsequent calls
# CLI still needs on-disk caching for process isolation
```

### Future: On-Disk Caching

Python uses on-disk pickle caching (`~/.scancode/license_index/index_cache`). For full CLI parity, consider adding `bincode` serialization:

```rust
static CACHE_PATH: &str = "~/.cache/scancode-rust/license_index.bin";

// Load from disk if exists and fresh
// Build and save otherwise
```

This would make subsequent CLI runs near-instant, matching Python's behavior after first-run cache build.

---

## 2. Reduce Rule Cloning in Sequence Matching

### Current Problem

**File**: `src/license_detection/seq_match.rs`, line 317

```rust
step1_candidates.push((svr, svf, rid, rule.clone(), high_set_intersection));
```

The `Rule` struct (`src/license_detection/models.rs`) contains:
- `identifier: String`
- `license_expression: String`
- `text: String` (can be several KB)
- `tokens: Vec<u16>` (hundreds/thousands of elements)
- 30+ other fields

Cloning rules during candidate selection creates significant memory pressure.

**Impact**: Medium-High - Thousands of clones per file during matching.

### Suggested Solution: Use `Arc<Rule>` or References

**Option A: Arc<Rule>** (breaking change to index structure)

```rust
// src/license_detection/models.rs
pub struct Rule {
    // Keep inner data, wrap in Arc at construction
}

// src/license_detection/index/mod.rs
pub struct LicenseIndex {
    pub rules_by_rid: Vec<Arc<Rule>>,  // Changed from Vec<Rule>
}

// src/license_detection/seq_match.rs
// Cloning Arc is cheap (atomic increment)
step1_candidates.push((svr, svf, rid, Arc::clone(&rule), high_set_intersection));
```

**Option B: Store Rule Index, Lookup on Demand** (non-breaking)

```rust
// Only store rid in candidates, lookup rule when needed
step1_candidates.push((svr, svf, rid, high_set_intersection));

// Later when rule data is needed:
let rule = &index.rules_by_rid[rid];
```

**Benefits**:
- `Arc::clone()` is O(1) vs O(n) for deep cloning
- Reduces memory allocations significantly
- Better cache locality with smaller candidate vectors

### How to Measure

Add metrics to track clone operations:

```rust
// In seq_match.rs
use std::sync::atomic::{AtomicU64, Ordering};
static CLONE_COUNT: AtomicU64 = AtomicU64::new(0);
static CLONE_BYTES: AtomicU64 = AtomicU64::new(0);

// After change, CLONE_COUNT should drop significantly
```

Use `cargo flamegraph` to visualize time spent in `alloc::alloc` before/after.

---

## 3. Reduce Per-File Allocations

### Current Problem

**File**: `src/license_detection/query.rs`, lines 171-257

The `Query` struct is created fresh for each file and contains:
- `tokens: Vec<u16>` - Token IDs for the input text
- `line_by_pos: Vec<usize>` - Line number mapping
- `high_matchables: HashSet<u16>` - Positions of legalese tokens
- `low_matchables: HashSet<u16>` - Positions of non-legalese tokens
- `unknowns: HashMap<u16, Vec<PositionSpan>>` - Unknown token tracking

**Impact**: Medium - Thousands of allocations per file scanned.

### Suggested Solution: Thread-Local Buffer Pooling

```rust
// src/license_detection/query.rs

use std::cell::RefCell;

thread_local! {
    static QUERY_BUFFERS: RefCell<QueryBuffers> = RefCell::new(QueryBuffers::new());
}

struct QueryBuffers {
    tokens: Vec<u16>,
    line_by_pos: Vec<usize>,
    high_matchables: HashSet<u16>,
    low_matchables: HashSet<u16>,
}

impl QueryBuffers {
    fn new() -> Self {
        // Pre-allocate with reasonable capacity
        Self {
            tokens: Vec::with_capacity(1000),
            line_by_pos: Vec::with_capacity(1000),
            high_matchables: HashSet::with_capacity(100),
            low_matchables: HashSet::with_capacity(100),
        }
    }
    
    fn clear(&mut self) {
        self.tokens.clear();
        self.line_by_pos.clear();
        self.high_matchables.clear();
        self.low_matchables.clear();
    }
}

impl Query {
    pub fn new(text: &str, index: &LicenseIndex) -> Result<Self> {
        QUERY_BUFFERS.with(|buffers| {
            let mut buffers = buffers.borrow_mut();
            buffers.clear();
            // Use buffers.tokens, buffers.high_matchables, etc.
            // instead of creating new Vec/HashSet
        })
    }
}
```

**Benefits**:
- Reuses memory across files in same thread
- Avoids repeated heap allocations
- Clear() is O(n) vs allocation which requires heap search

### How to Measure

```bash
# Before/after: Count allocations with dhat or valgrind
cargo build --release
valgrind --tool=massif target/release/scancode-rust testdata/

# Or use dhat for allocation profiling
cargo install dhat
cargo dhat -- testdata/
```

---

## 4. Optimize String Processing

### Current Problem

**File**: `src/license_detection/tokenize.rs`

```rust
// Line ~100: Creates new String for every tokenized text
let text = text.to_lowercase();

// Line ~150: Regex creates new String for each match
let token = m.as_str().to_string();
```

**Impact**: Medium - String allocations add up for large files.

### Suggested Solutions

**A. Use Cow<str> for Case-Insensitive Comparison**

```rust
use std::borrow::Cow;

fn tokenize_text(text: &str) -> Vec<&str> {
    // Check if text is already lowercase
    let normalized: Cow<str> = if text.chars().all(|c| !c.is_ascii_uppercase()) {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.to_lowercase())
    };
    // Process normalized text
}
```

**B. Pre-Compiled Regex with `lazy_static` or `LazyLock`**

Already done for some patterns, verify all regexes are cached:

```rust
// Current pattern in tokenize.rs uses Lazy correctly
static QUERY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"...").unwrap()
});
```

**C. Use `aho-corasick` for Stopword Filtering**

Current stopword filtering iterates through a HashSet. For bulk filtering:

```rust
// Build automaton from stopwords
static STOPWORD_AUTOMATON: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasick::new(STOPWORDS.iter()).unwrap()
});

// Replace multiple contains() checks with single pass
fn filter_stopwords(tokens: &mut Vec<&str>) {
    // Single-pass automaton matching
}
```

### How to Measure

Add counters for string allocations:

```rust
static STRING_ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);

// Instrument to_lowercase calls
fn to_lowercase_tracked(s: &str) -> String {
    STRING_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    s.to_lowercase()
}
```

---

## 5. Memory Layout Optimizations

### Current Problem

**File**: `src/license_detection/models.rs`

The `Rule` struct has poor cache locality due to many `String` and `Vec` fields scattered in memory.

### Suggested Solutions

**A. Use `Box<str>` instead of `String` for Immutable Text**

```rust
pub struct Rule {
    pub identifier: Box<str>,  // No capacity field, smaller
    pub license_expression: Box<str>,
    // ...
}
```

**B. Consider `SmallVec` for Small Token Lists**

```rust
use smallvec::SmallVec;

pub struct Rule {
    // Many rules have < 16 tokens, store inline
    pub tokens: SmallVec<[u16; 16]>,
}
```

**C. Use `ThinVec` for Rarely-Used Fields**

```rust
use thin_vec::ThinVec;

pub struct Rule {
    // Zero allocation for empty vectors
    pub extra_data: ThinVec<ExtraData>,
}
```

### How to Measure

```rust
// Measure struct sizes
println!("Rule size: {}", std::mem::size_of::<Rule>());
println!("Rule alignment: {}", std::mem::align_of::<Rule>());

// Use pahole for detailed layout analysis
// cargo build --release && pahole target/release/scancode-rust
```

---

## Implementation Priority

| Priority | Improvement | Effort | Impact |
|----------|-------------|--------|--------|
| 1 | Global index caching | Medium | High |
| 2 | Reduce Rule cloning | Medium | High |
| 3 | Thread-local buffer pooling | Medium | Medium |
| 4 | String processing optimization | Low | Medium |
| 5 | Memory layout optimization | High | Low-Medium |

---

## Benchmarking Strategy

### 1. Establish Baseline

```bash
# Create benchmark script
cat > benchmark.sh << 'EOF'
#!/bin/bash
for size in small medium large; do
    echo "Benchmarking $size dataset..."
    hyperfine --warmup 2 \
        "cargo run --release -- testdata/$size -o /dev/null" \
        --export-markdown results_$size.md
done
EOF
```

### 2. Profile with Flamegraph

```bash
cargo install flamegraph
cargo flamegraph --root -- testdata/large -o /dev/null
# Open flamegraph.svg to identify hotspots
```

### 3. Allocation Profiling

```bash
# Use dhat for detailed allocation tracking
cargo install dhat-rs
# Add to Cargo.toml: dhat = "0.3"
DHAT_DIR=dhat_out cargo run --release -- testdata/
```

### 4. Continuous Performance Tracking

```bash
# Use criterion for regression testing
# Add to Cargo.toml:
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "license_detection"
harness = false

# Run benchmarks
cargo bench
```

### 5. Real-World Test Cases

Test against diverse repositories:
- Small: Single license file (~100 lines)
- Medium: Typical npm package (~1000 files)
- Large: Linux kernel or similar (~50k+ files)

```bash
# Clone real-world test cases
git clone --depth 1 https://github.com/torvalds/linux testdata/linux
git clone --depth 1 https://github.com/nodejs/node testdata/node
```

---

## Verification Checklist

After implementing each improvement:

- [ ] Run `cargo test` - all tests pass
- [ ] Run `cargo clippy -- -D warnings` - no warnings
- [ ] Run benchmark script - measure improvement
- [ ] Check flamegraph - hotspot reduced
- [ ] Verify output unchanged - compare JSON output
- [ ] Document improvement in CHANGELOG

---

## References

- `src/license_detection/mod.rs` - Main detection engine
- `src/license_detection/seq_match.rs` - Sequence matching with candidate selection
- `src/license_detection/query.rs` - Per-file query processing
- `src/license_detection/index/builder.rs` - Index construction
- `src/license_detection/tokenize.rs` - Text tokenization
- `src/license_detection/models.rs` - Rule and License data structures
