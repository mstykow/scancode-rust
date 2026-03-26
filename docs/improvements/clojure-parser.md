# Clojure Parser Improvements

## Summary

Rust now ships bounded static support for Clojure `deps.edn` and Leiningen `project.clj` even though the Python ScanCode reference still has no production Clojure package parser.
The supported surface focuses on the most useful static data from the official docs: direct dependency declarations in `deps.edn`, and literal `defproject` metadata plus dependency vectors in `project.clj`.

## Python Status

- Python ScanCode does not currently ship packagedcode support for `deps.edn` or `project.clj`.
- Upstream interest exists, but the current upstream behavior is limited to summary and classification recognition for `project.clj` rather than package extraction.
- This gives Rust direct packagedcode support for Clojure project manifests that the Python reference does not currently provide.

## Rust Improvements

### Static `deps.edn` dependency extraction

- Rust now recognizes `deps.edn` and extracts top-level `:deps` entries.
- It also extracts alias-scoped dependency additions from bounded alias keys like `:extra-deps` and `:deps`.
- Maven coordinates become Maven-style purls, while git/local dependency source information is preserved in `extra_data` instead of being guessed as resolved package versions.

### Bounded `project.clj` metadata extraction

- Rust now recognizes literal top-level `(defproject ...)` forms.
- It extracts project namespace/name, version, description, homepage URL, SCM URL, license metadata, and literal dependency vectors.
- Literal profile dependency vectors for common scopes like `:dev`, `:test`, and `:provided` are also preserved.

### Explicit no-evaluation guardrails

- Rust does **not** evaluate Leiningen forms, unquote expressions, functions, profile code, or reader-eval constructs.
- Non-literal metadata and dependency forms are skipped instead of guessed.
- This keeps the parser aligned with the repository’s security-first bounded parsing model while still covering the common real-world static cases.

## Guardrails

- Rust does **not** run Leiningen, execute `project.clj`, or build an effective merged profile environment.
- `deps.edn` is treated as dependency/config data, not as a source of invented package identity when the file does not declare one.
- `deps.edn` and `project.clj` are intentionally treated as standalone unassembled manifests in the supported static surface described here.

## Coverage

Coverage spans `deps.edn` and `project.clj` parsing, alias and profile support, and the documented literal-only guardrails.

## References

- [Clojure deps.edn reference](https://clojure.org/reference/deps_edn)
- [Leiningen sample.project.clj](https://github.com/technomancy/leiningen/blob/stable/sample.project.clj)
- [Leiningen tutorial](https://github.com/technomancy/leiningen/blob/stable/doc/TUTORIAL.md)
