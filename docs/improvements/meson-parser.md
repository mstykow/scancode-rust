# Meson Parser Improvements

## Summary

Rust now ships a bounded, static parser for `meson.build` files even though the Python ScanCode reference still has no production Meson parser.
This first slice focuses on the highest-value metadata surface from Meson’s own docs and introspection behavior: literal `project()` metadata and top-level literal `dependency()` declarations.

## Python Status

- Python ScanCode does not currently ship a production Meson parser.
- Upstream demand exists in `aboutcode-org/scancode-toolkit#2586`, but there is no packagedcode implementation or test suite to port directly.
- That makes this parser a net-new Rust improvement rather than parity work.

## Rust Improvements

### Safe `project()` metadata extraction

- Rust now recognizes files literally named `meson.build` and extracts the literal project name from the first top-level `project(...)` call.
- The same bounded slice also recovers literal project languages, version, license, `license_files`, and `meson_version` values when they are directly present in `project(...)`.
- Root packages are emitted with Meson package identities such as `pkg:meson/demo-project@1.2.3`.

### Top-level `dependency()` extraction without Meson evaluation

- Rust now extracts top-level literal `dependency(...)` declarations, including calls assigned to variables and direct standalone calls.
- Literal `version`, `required`, `method`, `modules`, `fallback`, and `native` kwargs are preserved.
- Dependencies are emitted as generic PURLs under the Meson namespace, such as `pkg:generic/meson/zlib`, while Meson version constraints remain in `extracted_requirement` instead of being guessed as concrete package versions.
- When a dependency is marked `native: true`, Rust records it as a non-runtime dependency because it targets the build machine rather than the produced package runtime.

### Explicit guardrails

- Unsupported constructs are skipped instead of guessed.
- Rust does **not** execute Meson, run `meson introspect`, follow `fallback` resolution, evaluate feature options, resolve variable indirection, or honor control-flow-dependent dependency declarations.
- Non-literal `project()` values and non-literal dependency names are intentionally ignored rather than guessed.

## Validation

- `cargo test meson --lib`
- `cargo test --features golden-tests meson_golden --lib`
- `cargo test test_all_parsers_are_registered_and_exported --test scanner_integration`
- `cargo run --manifest-path xtask/Cargo.toml --bin generate-supported-formats`
- `cargo build`

## Related Issues

- #73
- `aboutcode-org/scancode-toolkit#2586`

## References

- [Meson Syntax](https://mesonbuild.com/Syntax.html)
- [Meson Dependencies](https://mesonbuild.com/Dependencies.html)
- [Meson `project()` reference](https://raw.githubusercontent.com/mesonbuild/meson/master/docs/yaml/functions/project.yaml)
- [Meson `dependency()` reference](https://raw.githubusercontent.com/mesonbuild/meson/master/docs/yaml/functions/dependency.yaml)
