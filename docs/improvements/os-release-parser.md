# OsReleaseParser: Debian Naming Fix and URL Extraction

## Summary

This parser improves on the Python reference in two user-visible ways:

1. **🐛 Bug Fix**: regular Debian systems are no longer mislabeled as distroless images
2. **🔍 Enhanced Extraction**: `HOME_URL`, `SUPPORT_URL`, and `BUG_REPORT_URL` are preserved as structured package metadata

## Debian naming fix

The Python reference contains a Debian-specific naming bug. When `ID=debian`, it can still collapse ordinary Debian systems into the `distroless` name based on `PRETTY_NAME` matching.

Rust keeps the intended distinction:

- Debian systems stay `debian`
- distroless images derived from Debian can still be identified as `distroless`

This matters because distro identity affects how users interpret scan results, especially for container base images and operating-system inventories.

## URL field extraction

Rust also keeps more of the metadata that `os-release` files already provide:

- `HOME_URL` maps to `homepage_url`
- `SUPPORT_URL` maps to `code_view_url`
- `BUG_REPORT_URL` maps to `bug_tracking_url`

The Python reference largely ignores those URLs, which makes the result less useful for SBOM and provenance consumers that want links back to the operating system project, support channel, or bug tracker.

## Why this matters

- **Correct distro identification**: Debian systems are no longer confused with distroless images
- **Richer metadata**: operating-system package records can carry stable project, support, and bug-reporting links

## Reference

- [os-release specification](https://www.freedesktop.org/software/systemd/man/os-release.html)
