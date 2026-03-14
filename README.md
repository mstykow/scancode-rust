# scancode-rust

A high-performance Rust rewrite of [ScanCode Toolkit](https://github.com/aboutcode-org/scancode-toolkit) for scanning codebases for licenses, package metadata, file metadata, and related provenance data.

## Overview

`scancode-rust` is built as a ScanCode-compatible replacement project with a strong focus on correctness, feature parity, safety, and performance.

Today the repository covers high-level scanning workflows for:

- License detection and license reference output
- Package and dependency metadata extraction across many ecosystems
- Package assembly for related manifests and lockfiles
- File metadata and scan environment metadata
- Optional copyright, holder, and author detection
- Optional email and URL extraction
- Multiple output formats, including ScanCode-style JSON, YAML, SPDX, CycloneDX, HTML, and custom templates

For architecture, supported formats, testing, and contributor guidance, start with the [Documentation Index](docs/DOCUMENTATION_INDEX.md).

## Features

- Parallel scanning with Rust-native performance
- ScanCode-compatible JSON output and broad output-format support
- Broad package-manifest and lockfile coverage across many ecosystems
- Package assembly for sibling, nested, and workspace-style inputs
- Include/exclude filtering, path normalization, and scan-result filtering
- Persistent scan-cache controls for repeated runs
- Security-first parsing with explicit safeguards and compatibility-focused tradeoffs where needed

## Installation

### From Crates.io (Recommended)

```sh
cargo install scancode-rust
```

### Download Precompiled Binary

Download the appropriate binary for your platform from the [GitHub Releases](https://github.com/mstykow/scancode-rust/releases) page:

- **Linux (x64)**: `scancode-rust-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `scancode-rust-aarch64-unknown-linux-gnu.tar.gz`
- **macOS (Apple Silicon)**: `scancode-rust-aarch64-apple-darwin.tar.gz`
  - Intel Macs can use the ARM build via Rosetta 2
- **Windows**: `scancode-rust-x86_64-pc-windows-msvc.zip`

Extract and place the binary in your system's PATH:

```sh
# Example for Linux/macOS
tar xzf scancode-rust-*.tar.gz
sudo mv scancode-rust /usr/local/bin/
```

### Build from Source

```sh
git clone https://github.com/mstykow/scancode-rust.git
cd scancode-rust
./setup.sh  # Initialize submodules and configure sparse checkout for embedded license data
cargo build --release
```

The compiled binary will be available at `target/release/scancode-rust`.

## Usage

```sh
scancode-rust --json-pp <FILE> [OPTIONS] <DIR_PATH>...
```

At least one output option is required.

For the complete CLI surface, run:

```sh
scancode-rust --help
```

Commonly used options include:

- `--json`, `--json-pp`, `--json-lines`, `--yaml`, `--html`, `--csv`
- `--spdx-tv`, `--spdx-rdf`, `--cyclonedx`, `--cyclonedx-xml`
- `--custom-output`, `--custom-template`
- `--exclude/--ignore`, `--include`, `--max-depth`, `--processes`
- `--cache-dir`, `--cache-clear`, `--from-json`, `--no-assemble`
- `--filter-clues`, `--only-findings`, `--mark-source`
- `--copyright`, `--email`, `--url`

### Example

```sh
scancode-rust --json-pp scan-results.json ~/projects/my-codebase --ignore "*.git*" --ignore "target/*" --ignore "node_modules/*"
```

Use `-` as FILE to write an output stream to stdout (for example: `--json-pp -`).
Multiple output flags can be used in a single run, matching ScanCode CLI behavior.
When using `--from-json`, you can pass multiple JSON inputs; directory scan mode currently supports one input path.
Cache location can also be controlled with the `SCANCODE_RUST_CACHE` environment variable.

For the generated package-format support matrix, see [Supported Formats](docs/SUPPORTED_FORMATS.md).

## Performance

`scancode-rust` is designed to be significantly faster than the Python-based ScanCode Toolkit, especially for large codebases, thanks to native Rust performance and parallel processing. See [Architecture: Performance Characteristics](docs/ARCHITECTURE.md#performance-characteristics) for details.

## Output Formats

Implemented output formats:

- JSON (ScanCode-compatible baseline)
- YAML
- JSON Lines
- CSV
- SPDX (Tag-Value, RDF/XML)
- CycloneDX (JSON, XML)
- HTML report
- Custom template rendering

Additional parity-oriented outputs such as the HTML app surface are present in the codebase, but the README focuses on the primary user-facing formats above.

Output architecture and compatibility approach are documented in:

- [Architecture](docs/ARCHITECTURE.md)
- [Testing Strategy](docs/TESTING_STRATEGY.md)

## Documentation

- **[Documentation Index](docs/DOCUMENTATION_INDEX.md)** - Best starting point for navigating the docs set
- **[Architecture](docs/ARCHITECTURE.md)** - System design, processing pipeline, and design decisions
- **[Supported Formats](docs/SUPPORTED_FORMATS.md)** - Generated support matrix for package ecosystems and file formats
- **[How to Add a Parser](docs/HOW_TO_ADD_A_PARSER.md)** - Step-by-step guide for adding new parsers
- **[Testing Strategy](docs/TESTING_STRATEGY.md)** - Testing approach and guidelines
- **[ADRs](docs/adr/)** - Architectural decision records
- **[Beyond-Parity Improvements](docs/improvements/)** - Features where Rust exceeds the Python original

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Setting Up for Local Development

To contribute to `scancode-rust`, follow these steps to set up the repository for local development:

1. **Install Rust**  
   Ensure you have Rust installed on your system. You can install it using [rustup](https://rustup.rs/):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the Repository**  
   Clone the `scancode-rust` repository to your local machine:

   ```sh
   git clone https://github.com/mstykow/scancode-rust.git
   cd scancode-rust
   ```

3. **Initialize and Update Embedded License Data**  
   Use the following script to initialize submodules, configure sparse checkout, and update the embedded SPDX license-data submodule to the latest upstream state.  
   If `pre-commit` is installed, this script also installs Git pre-commit hooks automatically:

   ```sh
   ./setup.sh
   ```

4. **Build the Project**  
   Build the project with Cargo:

   ```sh
   cargo build
   ```

5. **Run Tests**  
   Run the test suite to ensure everything is working correctly:

   ```sh
   cargo test
   ```

6. **Install Pre-commit (if needed)**  
   This repository uses [pre-commit](https://pre-commit.com/) to run checks before each commit.  
   For documentation hooks and commands, install Node.js and npm first (`package.json` currently requires Node `>=24`).
   If you install `pre-commit` after running `./setup.sh`, run `pre-commit install` once:

   ```sh
   # Using pip
   pip install pre-commit

   # Or using brew on macOS
   brew install pre-commit

   # Install the hooks
   pre-commit install
   ```

   Common documentation quality commands:

   ```sh
   npm run check:docs  # markdownlint + prettier check
   npm run fix:docs    # markdownlint auto-fix + prettier write
   ```

7. **Start Developing**  
   You can now make changes and test them locally. Use `cargo run --bin scancode-rust` to execute the tool:

   ```sh
   cargo run --bin scancode-rust -- [OPTIONS] <DIR_PATH>
   ```

## Publishing a Release (Maintainers Only)

Releases are automated using [`cargo-release`](https://github.com/crate-ci/cargo-release) and GitHub Actions.

### Prerequisites

**One-time setup:**

1. Install `cargo-release` CLI tool:

   ```sh
   cargo install cargo-release
   ```

2. Authenticate with crates.io (one-time only):

   ```sh
   cargo login
   ```

   Enter your [crates.io API token](https://crates.io/me) when prompted. This is stored in `~/.cargo/credentials.toml` and persists across sessions.

### Release Process

Use the `release.sh` script:

```sh
# Dry-run first (recommended)
./release.sh patch

# Then execute the actual release
./release.sh patch --execute
```

Available release types:

- `patch`: Increments `X.Y.Z` to `X.Y.(Z+1)`
- `minor`: Increments `X.Y.Z` to `X.(Y+1).0`
- `major`: Increments `X.Y.Z` to `(X+1).0.0`

**What happens automatically:**

1. **Updates SPDX license data** to the latest version from upstream
2. Commits the license data update (if changes detected)
3. `cargo-release` updates the version in `Cargo.toml` and `Cargo.lock`
4. Creates a git commit: `chore: release vX.Y.Z`
5. Creates a GPG-signed git tag: `vX.Y.Z`
6. Publishes to crates.io
7. Pushes commits and tag to GitHub
8. GitHub Actions workflow is triggered by the tag
9. Builds binaries for all published targets:
   - Linux: x64 and ARM64
   - macOS: ARM64 (Apple Silicon; Intel Macs can use Rosetta 2 with the ARM build)
   - Windows: x64
10. Creates archives (.tar.gz/.zip) and SHA256 checksums
11. Creates a GitHub Release with all artifacts and auto-generated release notes

> **Note**: The release script ensures every release ships with the latest SPDX license definitions. It also handles a sparse checkout workaround for `cargo-release`.

Monitor the [GitHub Actions workflow](https://github.com/mstykow/scancode-rust/actions) to verify completion.

## License

This project is licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
