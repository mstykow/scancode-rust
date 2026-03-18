# Helm Parser Improvements

## Summary

Rust now ships static Helm chart support for `Chart.yaml` and `Chart.lock` even though the Python ScanCode reference still has no production Helm parser.
This slice focuses on the high-value official metadata surface from Helm itself: chart identity, maintainers, declared chart dependencies, and pinned lockfile dependency state.

## Python Status

- Python ScanCode does not currently ship a Helm packagedcode parser.
- Upstream demand exists in `aboutcode-org/scancode-toolkit#4816`, but there is no packagedcode implementation or test suite to port directly.
- That makes this parser a net-new Rust improvement rather than parity work.

## Rust Improvements

### Static `Chart.yaml` metadata extraction

- Rust now recognizes `Chart.yaml` and extracts chart identity from `name`, `version`, and `apiVersion`.
- The parser also preserves `description`, `home`, `keywords`, `maintainers`, and common Helm metadata such as `appVersion`, `kubeVersion`, `type`, `icon`, `sources`, and `annotations`.
- Root packages are emitted with Helm package identities such as `pkg:helm/nginx@22.1.1`.

### Declared dependency extraction from `Chart.yaml`

- Rust now extracts chart dependencies declared in `Chart.yaml`.
- It preserves dependency metadata including `repository`, `condition`, `tags`, `alias`, and `import-values`.
- Exact dependency versions are treated as pinned; range-style versions remain unpinned requirements.

### Pinned dependency state from `Chart.lock`

- Rust now parses `Chart.lock` for locked dependency versions.
- It preserves top-level lock metadata like `digest` and `generated`.
- Sibling assembly keeps both the declared dependency view from `Chart.yaml` and the pinned dependency view from `Chart.lock`, following the same manifest+lockfile pattern already used in Cargo and Composer.

## Guardrails

- Rust does **not** evaluate templates, parse `values.yaml`, fetch remote chart repositories, inspect packaged chart archives, or resolve charts from OCI registries.
- Legacy `apiVersion: v1` charts still have their core chart metadata parsed from `Chart.yaml`, but this slice does not implement `requirements.yaml` / `requirements.lock`.
- Malformed dependency entries are skipped instead of causing the whole chart parse to fail.

## Validation

- `cargo test helm --lib`
- `cargo test --features golden-tests helm_golden --lib`
- `cargo test test_assembly_helm_basic --lib`
- `cargo test test_every_datasource_id_is_accounted_for --lib`
- `cargo test test_all_parsers_are_registered_and_exported --test scanner_integration`
- `cargo run --bin generate-supported-formats`
- `npm run check:docs`
- `cargo build`
- `cargo clippy --all-targets --all-features -- -D warnings`

## Related Issues

- #343
- `aboutcode-org/scancode-toolkit#4816`

## References

- [Helm chart format docs](https://helm.sh/docs/v3/topics/charts/)
- [Helm dependency update](https://helm.sh/docs/helm/helm_dependency_update/)
- [Helm dependency build](https://helm.sh/docs/helm/helm_dependency_build/)
