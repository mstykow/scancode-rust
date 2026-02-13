# License Detection Comparison: Python vs Rust

This document compares the license detection results between Python scancode-toolkit and scancode-rust on a test project.

## Test Setup

### Test Project Structure

```text
/tmp/test-project/
├── Cargo.toml          # Rust package manifest (license = "Apache-2.0")
├── LICENSE             # Apache-2.0 license text (full text)
├── python/
│   ├── __init__.py     # ScanCode toolkit Python file with SPDX headers
│   └── index.py        # ScanCode toolkit Python file with SPDX headers
└── rust/
    ├── mod.rs          # scancode-rust license detection module
    └── spdx_lid.rs     # SPDX-LID detection implementation
```text

### Scan Commands

│   ├── __init__.py     # ScanCode toolkit Python file with SPDX headers
│   └── index.py        # ScanCode toolkit Python file with SPDX headers
└── rust/
    ├── mod.rs          # scancode-rust license detection module
    └── spdx_lid.rs     # SPDX-LID detection implementation
__Python scancode:__

```bash
scancode --license --json-pp /tmp/scancode.json /tmp/test-project
```text

__scancode-rust:__

```bash
cargo run --release --bin scancode-rust -- /tmp/test-project \
  -o /tmp/test-project/rust-output.json \
  --license-rules-path reference/scancode-toolkit/src/licensedcode/data
```text

## Results Summary

| File | Python Detection | Rust Detection | Match? |
|------|------------------|----------------|--------|
| `Cargo.toml` | `apache-2.0` (via 2-aho) | No detection | ❌ |
| `LICENSE` | `apache-2.0` (via 1-hash) | 11 mixed detections | ❌ |
| `python/__init__.py` | `apache-2.0` (via 1-spdx-id) | `apache-2.0 AND gpl-1.0-plus AND ...` | Partial |
| `python/index.py` | `apache-2.0`, `gpl-2.0` | `agpl-2.0 AND gpl-1.0-plus AND ...` | Partial |
| `rust/mod.rs` | `mit`, `gpl-1.0-plus`, `apache-2.0` | `mit AND mit AND ...` | Partial |
| `rust/spdx_lid.rs` | Multiple complex expressions | Multiple complex expressions | Partial |

## Detailed Comparison

### 1. Cargo.toml

__Python:__

```text
apache-2.0 (matcher: 2-aho, rule: apache-2.0_65.RULE)
```text

__Rust:__

```text
No detection
```text

__Analysis:__ Python detects the license from the `license = "Apache-2.0"` field in Cargo.toml using the Aho-Corasick matcher. Rust does not detect this. The Rust scan output file (`rust-output.json`) contains the results, which may have affected the Cargo.toml parsing.

### 2. LICENSE (Apache-2.0 Full Text)

__Python:__

```text
apache-2.0 (matcher: 1-hash, rule: apache-2.0.LICENSE)
```text

__Rust:__

```text
unknown-license-reference AND apache-2.0 AND apache-2.0 AND unknown-license-reference AND apache-2.0 AND gpl-1.0-plus AND unknown AND warranty-disclaimer AND lzma-sdk-pd AND unknown-license-reference AND apache-2.0
```text

__Analysis:__ This is the most significant difference. Python correctly identifies the LICENSE file as Apache-2.0 using the __hash matcher__ (exact content match). Rust instead produces 11 separate detections with mixed licenses, suggesting the hash matcher is not working correctly or the file content doesn't match the expected hash.

### 3. python/__init__.py

__Python:__

```text
apache-2.0 (matcher: 1-spdx-id, rule: spdx-license-identifier-apache_2_0-...)
```text

__Rust:__

```text
apache-2.0 AND gpl-1.0-plus AND lzma-sdk-pd AND apache-2.0 AND apache-2.0
```text

__Analysis:__ Both detect Apache-2.0, but Rust produces a more complex expression with additional matches. The SPDX-ID matcher works in both cases, but Rust appears to be combining additional Aho-Corasick matches.

### 4. python/index.py

__Python:__

```text
apache-2.0 (matcher: 1-spdx-id)
gpl-2.0 (matcher: 2-aho, rule: gpl-2.0_52.RULE)
```text

__Rust:__

```text
agpl-2.0 AND gpl-1.0-plus AND lzma-sdk-pd AND mit AND unknown-license-reference AND apache-2.0 AND mit-no-false-attribs AND gpl-1.0-plus AND apache-2.0 AND apache-2.0
```text

__Analysis:__ Python cleanly identifies two separate license detections. Rust produces a single complex combined expression. The expression composition/combination logic differs significantly.

### 5. rust/mod.rs

__Python:__

```text
mit (matcher: 2-aho, rule: mit_14.RULE)
mit (matcher: 2-aho, rule: mit.LICENSE)
mit (matcher: 1-spdx-id)
mit (matcher: 2-aho, rule: mit_12.RULE)
gpl-1.0-plus (matcher: 2-aho, rule: gpl_bare_word_only.RULE)
gpl-1.0-plus (matcher: 2-aho, rule: gpl_91.RULE)
apache-2.0 (matcher: 2-aho, rule: apache-2.0_151.RULE)
mit (matcher: 1-spdx-id)
```text

__Rust:__

```text
mit AND mit AND unknown-license-reference AND mit AND gpl-1.0-plus AND other-permissive AND mit AND lzma-sdk-pd AND gpl-1.0-plus AND apache-2.0 AND unknown-license-reference AND unknown-license-reference AND mit
```text

__Analysis:__ Both detect the same core licenses (mit, gpl-1.0-plus, apache-2.0), but Rust produces a single combined expression while Python keeps them separate. Rust also includes additional matches like `unknown-license-reference` and `other-permissive`.

### 6. rust/spdx_lid.rs

__Python:__

```text
(unknown-spdx AND unknown-spdx) AND unknown-spdx
mit (multiple matches)
apache-2.0 (multiple matches)
gpl-2.0-plus WITH classpath-exception-2.0 (multiple matches)
mit OR apache-2.0 (multiple matches)
bsd-new
(epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0) AND epl-2.0 AND apache-2.0 AND classpath-exception-2.0
... (many more)
```text

__Rust:__

```text
apache-2.0
gpl-2.0 AND apache-2.0 AND classpath-exception-2.0 AND epl-2.0
mit
classpath-exception-2.0
apache-2.0 AND mit
mit AND bsd-new AND mit AND lzma-sdk-pd AND epl-2.0 AND ngpl AND classpath-exception-2.0 AND gpl-1.0-plus AND apache-2.0 AND gpl-2.0 AND classpath-exception-2.0
```text

__Analysis:__ This file contains many SPDX license identifiers and expressions. Both tools detect similar licenses but with different grouping and combination. Rust produces fewer, more combined detections.

## Key Findings

### 1. Hash Matcher Not Working

The LICENSE file (full Apache-2.0 text) should be matched by the hash matcher (`1-hash`) for an exact match. Python does this correctly. Rust produces many fragment matches instead.

__Root Cause:__ The hash matcher in Rust may not be comparing against license file hashes correctly, or the hash computation differs from Python.

### 2. Expression Combination Differs

Python keeps many detections separate, while Rust combines them into larger expressions. This is a design difference in how the assembly/combination phase works.

### 3. SPDX-ID Matcher Works

Both tools correctly detect SPDX license identifiers (e.g., `SPDX-License-Identifier: Apache-2.0`) using the `1-spdx-id` matcher.

### 4. Aho-Corasick Matcher Works

The `2-aho` matcher produces matches in both tools, though the exact matches and their combination differ.

### 5. Cargo.toml License Field Not Detected

Python detects the `license = "Apache-2.0"` field in Cargo.toml, but Rust does not. This may be because:

- The Rust scan output file was present and affected the scan
- The TOML parsing for license detection is not implemented
- The matcher for this specific pattern differs

## Recommendations

1. __Fix hash matcher__: Investigate why the LICENSE file hash match fails in Rust
2. __Review expression combination__: Consider whether the current combination logic matches Python's behavior
3. __Test with clean directory__: Re-run without the `rust-output.json` file in the test project
4. __Add Cargo.toml license detection__: Ensure the license field in Cargo.toml is detected

## Technical Details

### Matchers Used

| Matcher | Python | Rust | Status |
|---------|--------|------|--------|
| `1-hash` | ✅ Used for LICENSE | ❌ Not used | Bug |
| `1-spdx-id` | ✅ Used | ✅ Used | Working |
| `2-aho` | ✅ Used | ✅ Used | Working |
| `3-seq` | ✅ Used once | ? | Unknown |

### Rule Counts

- __Python__: Loaded ~36,000+ rules
- __Rust__: Loaded 36,467 rules (same source)

## Next Steps

1. Debug the hash matcher to understand why LICENSE file doesn't match
2. Compare expression combination algorithms between Python and Rust
3. Add more golden tests for edge cases
4. Consider aligning detection grouping behavior with Python for parity
