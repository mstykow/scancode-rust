# Gitmodules Parser: New Feature

## Summary

**✨ New Feature**: Rust parses `.gitmodules` files and treats Git submodules as dependencies, a behavior the Python reference does not provide.

## Reference limitation

When a project uses Git submodules, the Python reference can scan the repository contents but does not surface the `.gitmodules` manifest itself as a dependency source. That leaves a gap in dependency graphs for projects that vendor code through submodules.

## Rust improvement

Rust reads `.gitmodules` as an INI-like manifest and emits one dependency edge per submodule.

Each dependency can preserve:

- the submodule path
- the original URL or host reference in `extracted_requirement`
- a GitHub or GitLab PURL when the host is recognizable

For other hosts, Rust still preserves the submodule relationship even if it cannot generate a platform-specific PURL.

## Why this matters

- **More complete dependency graphs**: Git submodules stop being invisible to package metadata extraction
- **Better provenance**: the repository path and remote origin both remain visible in scan output
- **Useful host-aware identities**: GitHub and GitLab submodules can be represented as structured package URLs when possible
