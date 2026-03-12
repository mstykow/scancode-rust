# Docker Parser: Containerfile and OCI Label Support

**Area**: Dockerfile / Containerfile package detection  
**Files**: `src/parsers/docker.rs`, `src/main_test.rs`, `src/utils/language.rs`  
**Upstream Context**: `aboutcode-org/scancode-toolkit#3471`, `aboutcode-org/scancode-toolkit#3561`

## Summary

**✨ New Feature + 🔍 Enhanced Extraction**: Rust now recognizes both `Dockerfile` and `Containerfile` as package data files and extracts OCI image labels from them while intentionally leaving them non-assembled.

## Upstream Context

The current Python reference does not expose a normal packagedcode Docker parser to port directly, but the upstream issue set defines the desired behavior:

- support `Containerfile` as an alternative Dockerfile name, and
- treat Dockerfile-like files as package data that can yield OCI image metadata.

This made Docker a greenfield parser family in Rust rather than a line-by-line parity port.

## Rust Improvement

Rust now implements a dedicated Docker parser with three stable behaviors:

1. **Dockerfile and Containerfile recognition**
   - matches `Dockerfile`
   - matches `Dockerfile.*`
   - matches `Containerfile`
   - matches `Containerfile.*`

2. **OCI label extraction**

   Rust reads `LABEL org.opencontainers.image.*=...` instructions and maps key fields into normal package metadata:
   - `org.opencontainers.image.title` → `name`
   - `org.opencontainers.image.description` → `description`
   - `org.opencontainers.image.url` → `homepage_url`
   - `org.opencontainers.image.source` → `vcs_url`
   - `org.opencontainers.image.version` → `version`
   - `org.opencontainers.image.licenses` → `extracted_license_statement`

   All collected OCI labels are also preserved in `extra_data.oci_labels` so data is not lost when a label does not map neatly to an existing top-level field.

3. **Non-assembled package data behavior**

   Dockerfile and Containerfile results are intentionally treated as package data on the file itself rather than as sibling-merged top-level assembled packages.

## Why this matters

- **Alternative naming support**: projects using `Containerfile` are no longer invisible to package detection.
- **Container metadata visibility**: OCI image labels now produce structured package metadata instead of being ignored as plain text.
- **Safe scope control**: Docker files participate in scan output without introducing unnecessary assembly semantics.

## Coverage

Coverage includes:

- parser matching for both Dockerfile and Containerfile naming patterns,
- unit coverage for OCI label extraction and raw `oci_labels` preservation,
- parser goldens for a real Dockerfile fixture and a real Containerfile fixture, and
- scan-level coverage proving Containerfile package data remains non-assembled.

## References

- Local issues: `#199`, `#200`
- Upstream issues: `aboutcode-org/scancode-toolkit#3471`, `aboutcode-org/scancode-toolkit#3561`
- OCI annotations spec: <https://github.com/opencontainers/image-spec/blob/main/annotations.md>
