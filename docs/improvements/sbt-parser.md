# SBT Parser Improvements

## Summary

Rust now ships a bounded, static parser for `build.sbt` files even though the Python ScanCode reference has no production SBT parser today.
This first slice is intentionally narrow: it extracts only top-level literal metadata, literal license/homepage forms, and literal external library dependencies, and it skips anything that would require Scala or sbt evaluation.

## Python Status

- Python ScanCode does not currently ship a production SBT parser.
- Upstream demand exists, but there is no reference implementation to port directly.
- That makes this parser a net-new Rust improvement rather than parity work.

## Rust Improvements

### Safe `build.sbt` metadata extraction

- Rust now recognizes files literally named `build.sbt` and extracts top-level literal package metadata for `organization`, `name`, `version`, and `description`.
- The same bounded slice also supports `ThisBuild / organization|name|version|description` and lets direct root settings override broader `ThisBuild` defaults.
- `homepage := Some(url("..."))` and `organizationHomepage := Some(url("..."))` are recovered when they are directly literal.
- `licenses += "Name" -> url("https://...")` is recovered when it is directly literal.

### Literal dependency extraction without Scala evaluation

- Rust now extracts literal external dependencies from `libraryDependencies += ...` and `libraryDependencies ++= Seq(...)`.
- Supported dependency forms include both `%` and `%%` operators plus literal scope suffixes such as `% Test`, `% "test"`, and `% "provided"`.
- Rust now also supports top-level config-prefixed dependency statements such as `Test / libraryDependencies += ...`, `Runtime / libraryDependencies += ...`, and `Provided / libraryDependencies ++= Seq(...)`.
- When a config prefix exists and the dependency itself does not already declare an explicit trailing `% scope`, Rust maps the prefix into the same dependency scope/runtime semantics it already uses for trailing SBT scopes.
- Dependencies are emitted as Maven package URLs because SBT resolves external artifacts from the JVM/Maven ecosystem.
- For `%%`, Rust preserves the literal artifact name written in `build.sbt` and records that the dependency used SBT cross-version syntax instead of guessing a Scala binary suffix that would require evaluation.

### Small bounded alias support

- Rust resolves simple same-file string `val` aliases when they are used in supported top-level string settings or supported dependency coordinates/versions.
- Alias handling stays intentionally narrow and does not widen into general Scala constant folding or expression evaluation.

### Root-safe `.settings(...)` and shared bundle support

- Rust now descends into top-level root-safe `.settings(...)` wrappers such as `lazy val root = project.settings(...)` and `lazy val root = (project in file(".")).settings(...)`.
- The parser also supports same-file literal `Seq(...)` bundles reused as shared dependency or settings groups when they are referenced from supported positions.
- Inline settings inside a root-safe `.settings(...)` wrapper override earlier bundle-provided values using the same bounded precedence rules as the rest of the parser.

### Explicit guardrails

- Unsupported constructs are skipped instead of guessed.
- Rust does **not** execute Scala, invoke `sbt`, parse arbitrary `*.sbt`, parse `plugins.sbt`, parse `project/*.scala`, or attempt multi-project graph semantics.
- Rust only descends into top-level root-safe `.settings(...)` wrappers; broader nested or non-root project setting graphs are still skipped.

## Validation

- `cargo test sbt --lib`
- `cargo test --features golden-tests sbt_golden --lib`
- `cargo run --manifest-path xtask/Cargo.toml --bin generate-supported-formats`
- `cargo build`

## Related Issues

- #69
