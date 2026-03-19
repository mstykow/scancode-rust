# Docker Parser: Containerfile and OCI Label Support

## Summary

**✨ New Feature + 🔍 Enhanced Extraction**: Rust recognizes both `Dockerfile` and `Containerfile` as package data files, extracts OCI image labels from them, and keeps those results attached to the file itself instead of forcing sibling assembly.

## Reference limitation

The Python reference does not provide a normal packagedcode Docker parser that covers this behavior directly. As a result, Dockerfile-like files are easier to miss as package metadata sources.

## Rust improvement

Rust implements three durable behaviors:

1. **Dockerfile and Containerfile recognition**
   Alternative Dockerfile names such as `Containerfile` are treated as package data instead of being ignored.

2. **OCI label extraction**
   `LABEL org.opencontainers.image.*=...` instructions are mapped into normal package metadata when they fit existing fields, and the full label set is preserved in `extra_data.oci_labels` so label data is not lost.

3. **Non-assembled package data behavior**
   Dockerfile and Containerfile results stay attached to the file that declared them. That keeps the behavior predictable and avoids introducing unnecessary sibling-merge semantics.

## Why this matters

- **Alternative naming support**: `Containerfile` projects are no longer invisible to package detection
- **Container metadata visibility**: OCI labels produce structured package metadata instead of disappearing into plain text
- **Safer scope control**: scan output can include container metadata without inventing unrelated assembly rules

## Reference

- [OCI image annotations specification](https://github.com/opencontainers/image-spec/blob/main/annotations.md)
