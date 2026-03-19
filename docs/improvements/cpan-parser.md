# CPAN Parser: Improvements Over Python

## Summary

Rust improves on the Python reference in several practical ways:

- **✨ Real parsing**: stub-only CPAN metadata handlers become real parsers
- **🐛 Safer fallback identity**: malformed or unreadable inputs can still preserve parser identity instead of collapsing into generic package data
- **🔍 Fuller metadata**: package metadata, dependency scopes, author data, resource URLs, and manifest file references are extracted from real inputs

## Reference limitation

The Python reference can detect several CPAN metadata files, but the core CPAN handlers are stub-oriented. That means files can be recognized without yielding meaningful package metadata.

## Rust improvement

Rust performs real parsing for the main CPAN metadata surfaces:

### `META.json`

Rust extracts package metadata, structured resource URLs, author information, license fields, and nested dependency scopes from modern CPAN metadata.

### `META.yml`

Rust supports the older YAML metadata surface while preserving the dependency scopes and resource fields that matter for package consumers.

### `MANIFEST`

Rust turns MANIFEST entries into structured `file_references`, which helps preserve what files the package claims as part of its source distribution.

### Fallback identity hardening

When a CPAN input is malformed or unreadable, Rust still preserves the parser's package identity surface, including `package_type`, `datasource_id`, and `primary_language`, instead of losing those signals during fallback handling.

## Why this matters

- **Better Perl package visibility**: CPAN metadata files now produce real package records instead of mostly empty placeholders
- **Clearer dependency semantics**: runtime, build, test, and configure scopes remain visible
- **Safer scanner behavior**: malformed inputs do not silently lose the identity needed for assembly and downstream interpretation

## References

- [CPAN::Meta::Spec v2](https://metacpan.org/pod/CPAN::Meta::Spec)
- [CPAN::Meta::Spec v1.4](https://metacpan.org/pod/distribution/CPAN-Meta/lib/CPAN/Meta/Spec.pm)
- [Module::Manifest](https://metacpan.org/pod/Module::Manifest)
